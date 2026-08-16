// frontend/src-tauri/src/tunnel/wisp.rs
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use futures_util::{SinkExt, StreamExt};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::io::Cursor;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

const WISP_TYPE_CONNECT: u8 = 0x01;
const WISP_TYPE_DATA: u8 = 0x02;
const WISP_TYPE_CONTINUE: u8 = 0x03;
const WISP_TYPE_CLOSE: u8 = 0x04;
const _WISP_TYPE_INFO: u8 = 0x05;

const STREAM_TYPE_TCP: u8 = 0x01;

/// Клиент мультиплексированного Wisp-туннеля
#[derive(Clone)]
pub struct WispClient {
    tx_to_ws: mpsc::Sender<Vec<u8>>,
    stream_counter: Arc<AtomicU32>,
    active_streams: Arc<Mutex<HashMap<u32, mpsc::Sender<Vec<u8>>>>>,
}

impl WispClient {
    /// Устанавливает зашифрованное WebSocket-соединение с Wisp сервером на VPS
    pub async fn connect(wisp_url: &str) -> Result<Self, String> {
        let (ws_stream, _) = connect_async(wisp_url)
            .await
            .map_err(|e| format!("Wisp connection failed: {}", e))?;

        let (mut ws_sink, mut ws_source) = ws_stream.split();
        let (tx_to_ws, mut rx_from_client) = mpsc::channel::<Vec<u8>>(1024);

        let active_streams = Arc::new(Mutex::new(HashMap::<u32, mpsc::Sender<Vec<u8>>>::new()));
        let streams_for_reader = active_streams.clone();

        // 1. Фоновая задача отправки пакетов в WebSocket
        tokio::spawn(async move {
            while let Some(packet) = rx_from_client.recv().await {
                if let Err(e) = ws_sink.send(Message::Binary(packet.into())).await {
                    log::error!("[WISP] Ошибка отправки пакета в WebSocket: {:?}", e);
                    break;
                }
            }
        });

        // 2. Фоновая задача приема и демультиплексирования пакетов из WebSocket
        tokio::spawn(async move {
            while let Some(msg_res) = ws_source.next().await {
                match msg_res {
                    Ok(Message::Binary(bytes)) => {
                        if bytes.len() < 5 {
                            continue;
                        }
                        let packet_type = bytes[0];
                        let mut cursor = Cursor::new(&bytes[1..5]);
                        let stream_id = cursor.read_u32::<LittleEndian>().unwrap_or(0);

                        match packet_type {
                            WISP_TYPE_DATA => {
                                let payload = bytes[5..].to_vec();
                                let sender = {
                                    let map = streams_for_reader.lock();
                                    map.get(&stream_id).cloned()
                                };
                                if let Some(tx) = sender {
                                    let _ = tx.send(payload).await;
                                }
                            }
                            WISP_TYPE_CONTINUE => {
                                // Flow control подтверждение от сервера
                                log::trace!("[WISP] Получен CONTINUE для stream_id: {}", stream_id);
                            }
                            WISP_TYPE_CLOSE => {
                                log::debug!("[WISP] Получен сигнал закрытия потока #{}", stream_id);
                                let mut map = streams_for_reader.lock();
                                map.remove(&stream_id);
                            }
                            _ => {}
                        }
                    }
                    Ok(Message::Close(_)) => {
                        log::warn!("[WISP] Сервер закрыл WebSocket соединение");
                        break;
                    }
                    Err(e) => {
                        log::error!("[WISP] Ошибка чтения из WebSocket: {:?}", e);
                        break;
                    }
                    _ => {}
                }
            }
            // Очищаем все активные потоки при разрыве сокета
            streams_for_reader.lock().clear();
        });

        Ok(Self {
            tx_to_ws,
            stream_counter: Arc::new(AtomicU32::new(1)),
            active_streams,
        })
    }

    /// Открывает новый TCP-стрим через Wisp (Remote DNS + Connect)
    pub async fn create_tcp_stream(
        &self,
        host: &str,
        port: u16,
    ) -> Result<(u32, mpsc::Receiver<Vec<u8>>, WispStreamSender), String> {
        let stream_id = self.stream_counter.fetch_add(1, Ordering::SeqCst);

        let (stream_tx, stream_rx) = mpsc::channel::<Vec<u8>>(512);
        {
            let mut map = self.active_streams.lock();
            map.insert(stream_id, stream_tx);
        }

        // Формируем WISP CONNECT пакет: [0x01][stream_id (4B)][stream_type (1B)][port (2B)][host]
        let mut connect_pkt = Vec::with_capacity(1 + 4 + 1 + 2 + host.len());
        connect_pkt.push(WISP_TYPE_CONNECT);
        connect_pkt.write_u32::<LittleEndian>(stream_id).unwrap();
        connect_pkt.push(STREAM_TYPE_TCP);
        connect_pkt.write_u16::<LittleEndian>(port).unwrap();
        connect_pkt.extend_from_slice(host.as_bytes());

        self.tx_to_ws
            .send(connect_pkt)
            .await
            .map_err(|e| format!("Failed to send WISP CONNECT: {}", e))?;

        let sender = WispStreamSender {
            stream_id,
            tx_to_ws: self.tx_to_ws.clone(),
            active_streams: self.active_streams.clone(),
        };

        Ok((stream_id, stream_rx, sender))
    }
}

/// Структура для отправки данных в конкретный Wisp-стрим
#[derive(Clone)]
pub struct WispStreamSender {
    stream_id: u32,
    tx_to_ws: mpsc::Sender<Vec<u8>>,
    active_streams: Arc<Mutex<HashMap<u32, mpsc::Sender<Vec<u8>>>>>,
}

impl WispStreamSender {
    /// Отправляет данные (WISP DATA) в туннель
    pub async fn send_data(&self, payload: &[u8]) -> Result<(), String> {
        let mut data_pkt = Vec::with_capacity(1 + 4 + payload.len());
        data_pkt.push(WISP_TYPE_DATA);
        data_pkt.write_u32::<LittleEndian>(self.stream_id).unwrap();
        data_pkt.extend_from_slice(payload);

        self.tx_to_ws
            .send(data_pkt)
            .await
            .map_err(|e| format!("Failed to send WISP DATA: {}", e))
    }

    /// Закрывает стрим (WISP CLOSE)
    pub async fn close(&self, reason: u8) {
        let mut close_pkt = Vec::with_capacity(1 + 4 + 1);
        close_pkt.push(WISP_TYPE_CLOSE);
        close_pkt.write_u32::<LittleEndian>(self.stream_id).unwrap();
        close_pkt.push(reason);

        let _ = self.tx_to_ws.send(close_pkt).await;

        let mut map = self.active_streams.lock();
        map.remove(&self.stream_id);
    }
}
