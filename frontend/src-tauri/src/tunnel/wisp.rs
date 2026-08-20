use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use futures_util::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::io::Cursor;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, Mutex};
use tokio_tungstenite::client_async_tls;
use tokio_tungstenite::tungstenite::handshake::client::Request;
use tokio_tungstenite::tungstenite::protocol::Message;
use url::Url;
use sha2::{Sha256, Digest};
use std::time::{SystemTime, UNIX_EPOCH};
pub const WISP_TYPE_CONNECT: u8 = 0x01;
pub const WISP_TYPE_DATA: u8 = 0x02;
pub const WISP_TYPE_CONTINUE: u8 = 0x03;
pub const WISP_TYPE_CLOSE: u8 = 0x04;
pub const WISP_STREAM_TCP: u8 = 0x01;

#[derive(Clone)]
pub struct WispClient {
    ws_tx: mpsc::Sender<Message>,
    stream_channels: Arc<Mutex<HashMap<u32, mpsc::Sender<Vec<u8>>>>>,
    next_stream_id: Arc<Mutex<u32>>,
    /// Флаг жизни WebSocket-соединения. Когда sender или receiver задача завершается
    /// (WebSocket разорван), флаг ставится в false.
    alive: Arc<AtomicBool>,
}

impl WispClient {
    /// Проверяет, живо ли WebSocket-соединение к WISP-серверу.
    pub fn is_alive(&self) -> bool {
        self.alive.load(Ordering::SeqCst)
    }

    pub async fn connect(wisp_url: &str) -> Result<Self, String> {
        let parsed_url = Url::parse(wisp_url).map_err(|e| format!("Некорректный WISP URL: {}", e))?;
        let host = parsed_url.host_str().ok_or("Отсутствует хост в WISP URL")?.to_string();
        let port = parsed_url.port_or_known_default().unwrap_or(443);

        // Проверяем наличие апстрим-прокси
        let tcp_stream = TcpStream::connect((host.as_str(), port))
            .await
            .map_err(|e| format!("Не удалось подключиться к WISP хосту напрямую {}:{}: {}", host, port, e))?;

        let token = parsed_url.path().trim_start_matches("/wisp/");
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs().to_string();
        let salt = obfstr::obfstr!(env!("WISP_SALT")).to_string();
        
        let mut hasher = Sha256::new();
        hasher.update(format!("{}{}{}", token, timestamp, salt).as_bytes());
        let signature = format!("{:x}", hasher.finalize());

        // Устанавливаем TLS и поднимаем WebSocket
        let request = Request::builder()
            .uri(wisp_url)
            .header("Host", host.as_str())
            .header("Connection", "Upgrade")
            .header("Upgrade", "websocket")
            .header("Sec-WebSocket-Version", "13")
            .header("Sec-WebSocket-Key", tokio_tungstenite::tungstenite::handshake::client::generate_key())
            .header("X-Wisp-Timestamp", &timestamp)
            .header("X-Wisp-Signature", &signature)
            .body(())
            .map_err(|e| format!("Ошибка формирования WebSocket запроса: {}", e))?;

        let (ws_stream, _) = client_async_tls(request, tcp_stream)
            .await
            .map_err(|e| format!("Не удалось выполнить WebSocket handshake: {}", e))?;

        let (mut ws_sink, mut ws_source) = ws_stream.split();
        let (tx, mut rx) = mpsc::channel::<Message>(1000);

        let stream_channels = Arc::new(Mutex::new(HashMap::<u32, mpsc::Sender<Vec<u8>>>::new()));
        let stream_channels_rx = stream_channels.clone();

        let alive = Arc::new(AtomicBool::new(true));
        let alive_tx = alive.clone();
        let alive_rx = alive.clone();

        // Sender task: пересылает сообщения из канала в WebSocket
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if ws_sink.send(msg).await.is_err() {
                    break;
                }
            }
            alive_tx.store(false, Ordering::SeqCst);
            log::warn!("[WISP] WebSocket sender task завершилась — соединение разорвано");
        });

        // Receiver task: читает из WebSocket и раздаёт по stream-каналам
        tokio::spawn(async move {
            while let Some(Ok(msg)) = ws_source.next().await {
                if let Message::Binary(data) = msg {
                    if data.len() < 5 {
                        continue;
                    }

                    let packet_type = data[0];
                    let mut cursor = Cursor::new(&data[1..5]);
                    let stream_id = byteorder::ReadBytesExt::read_u32::<LittleEndian>(&mut cursor).unwrap_or(0);

                    if packet_type == WISP_TYPE_DATA {
                        let payload = data[5..].to_vec();
                        let channels = stream_channels_rx.lock().await;
                        if let Some(ch) = channels.get(&stream_id) {
                            let _ = ch.send(payload).await;
                        }
                    } else if packet_type == WISP_TYPE_CLOSE {
                        let mut channels = stream_channels_rx.lock().await;
                        channels.remove(&stream_id);
                    } else if packet_type == WISP_TYPE_CONTINUE {
                        // CONTINUE is used for flow control in WISP v2. We ignore it for now as we don't strictly enforce flow control.
                    }
                }
            }
            alive_rx.store(false, Ordering::SeqCst);
            log::warn!("[WISP] WebSocket receiver task завершилась — соединение разорвано");
        });

        Ok(Self {
            ws_tx: tx,
            stream_channels,
            next_stream_id: Arc::new(Mutex::new(1)),
            alive,
        })
    }

    pub async fn create_stream(&self, host: &str, port: u16) -> Result<WispStream, String> {
        let stream_id = {
            let mut id = self.next_stream_id.lock().await;
            let current = *id;
            *id += 1;
            current
        };

        let (data_tx, data_rx) = mpsc::channel::<Vec<u8>>(500);
        {
            let mut channels = self.stream_channels.lock().await;
            channels.insert(stream_id, data_tx);
        }

        let mut packet = Vec::new();
        packet.push(WISP_TYPE_CONNECT);
        byteorder::WriteBytesExt::write_u32::<LittleEndian>(&mut packet, stream_id).unwrap();
        packet.push(WISP_STREAM_TCP);
        byteorder::WriteBytesExt::write_u16::<LittleEndian>(&mut packet, port).unwrap();
        packet.extend_from_slice(host.as_bytes());

        self.ws_tx
            .send(Message::Binary(packet.into()))
            .await
            .map_err(|e| format!("Ошибка отправки CONNECT в WISP: {}", e))?;

        Ok(WispStream {
            stream_id,
            ws_tx: self.ws_tx.clone(),
            data_rx,
            read_buffer: Vec::new(),
        })
    }
}

pub struct WispStream {
    stream_id: u32,
    ws_tx: mpsc::Sender<Message>,
    data_rx: mpsc::Receiver<Vec<u8>>,
    read_buffer: Vec<u8>,
}

impl tokio::io::AsyncRead for WispStream {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        if !self.read_buffer.is_empty() {
            let to_read = std::cmp::min(buf.remaining(), self.read_buffer.len());
            buf.put_slice(&self.read_buffer[..to_read]);
            self.read_buffer.drain(..to_read);
            return std::task::Poll::Ready(Ok(()));
        }

        match self.data_rx.poll_recv(cx) {
            std::task::Poll::Ready(Some(data)) => {
                let to_read = std::cmp::min(buf.remaining(), data.len());
                buf.put_slice(&data[..to_read]);
                if to_read < data.len() {
                    self.read_buffer.extend_from_slice(&data[to_read..]);
                }
                std::task::Poll::Ready(Ok(()))
            }
            std::task::Poll::Ready(None) => std::task::Poll::Ready(Ok(())),
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}

impl tokio::io::AsyncWrite for WispStream {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        let mut packet = Vec::with_capacity(5 + buf.len());
        packet.push(WISP_TYPE_DATA);
        byteorder::WriteBytesExt::write_u32::<LittleEndian>(&mut packet, self.stream_id).unwrap();
        packet.extend_from_slice(buf);

        match self.ws_tx.try_send(Message::Binary(packet.into())) {
            Ok(_) => std::task::Poll::Ready(Ok(buf.len())),
            Err(mpsc::error::TrySendError::Full(_)) => std::task::Poll::Pending,
            Err(mpsc::error::TrySendError::Closed(_)) => {
                std::task::Poll::Ready(Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "WISP channel closed")))
            }
        }
    }

    fn poll_flush(self: std::pin::Pin<&mut Self>, _cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), std::io::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: std::pin::Pin<&mut Self>, _cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), std::io::Error>> {
        let mut packet = Vec::with_capacity(6);
        packet.push(WISP_TYPE_CLOSE);
        byteorder::WriteBytesExt::write_u32::<LittleEndian>(&mut packet, self.stream_id).unwrap();
        packet.push(0x01); // Reason code 0x01 = Voluntary
        let _ = self.ws_tx.try_send(Message::Binary(packet.into()));
        std::task::Poll::Ready(Ok(()))
    }
}
