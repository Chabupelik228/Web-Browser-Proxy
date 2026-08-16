// frontend/src-tauri/src/tunnel/loopback.rs
use crate::tunnel::wisp::WispClient;
use rand::distributions::Alphanumeric;
use rand::Rng;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;

/// Структура запущенного локального прокси-сервера
pub struct LoopbackProxyServer {
    pub port: u16,
    pub session_secret: String,
    pub shutdown_tx: watch::Sender<bool>,
}

impl LoopbackProxyServer {
    /// Запускает локальный HTTP/CONNECT прокси на 127.0.0.1 с динамическим портом и секретом
    pub async fn start(wisp_client: WispClient) -> Result<Self, String> {
        let listener = TcpListener::bind("127.0.0.1:11338")
            .await
            .map_err(|e| format!("Failed to bind local loopback listener: {}", e))?;

        let port = listener
            .local_addr()
            .map_err(|e| format!("Failed to get local port: {}", e))?
            .port();

        // Генерация случайного 32-символьного секрета сессии
        let session_secret: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(32)
            .map(char::from)
            .collect();

        let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
        let secret_clone = session_secret.clone();

        tokio::spawn(async move {
            log::info!("[LOOPBACK PROXY] Запущен на 127.0.0.1:{}", port);

            loop {
                tokio::select! {
                    accept_res = listener.accept() => {
                        match accept_res {
                            Ok((stream, _)) => {
                                let wisp = wisp_client.clone();
                                let secret = secret_clone.clone();
                                tokio::spawn(async move {
                                    if let Err(e) = handle_client_connection(stream, wisp, &secret).await {
                                        log::debug!("[LOOPBACK] Ошибка обработки соединения: {:?}", e);
                                    }
                                });
                            }
                            Err(e) => {
                                log::error!("[LOOPBACK] Ошибка accept: {:?}", e);
                            }
                        }
                    }
                    _ = shutdown_rx.changed() => {
                        if *shutdown_rx.borrow() {
                            log::info!("[LOOPBACK PROXY] Остановка локального прокси на порту {}", port);
                            break;
                        }
                    }
                }
            }
        });

        Ok(Self {
            port,
            session_secret,
            shutdown_tx,
        })
    }
}

/// Обрабатывает входящее HTTP / HTTPS (CONNECT) соединение от WebView2
async fn handle_client_connection(
    mut client_stream: TcpStream,
    wisp_client: WispClient,
    _expected_secret: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut buf = [0u8; 8192];
    let n = client_stream.read(&mut buf).await?;
    if n == 0 {
        return Ok(());
    }

    let request_str = String::from_utf8_lossy(&buf[..n]);
    let mut lines = request_str.lines();
    let request_line = match lines.next() {
        Some(l) => l,
        None => return Ok(()),
    };

    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let target = parts.next().unwrap_or("");

    // 2. Обработка HTTPS туннелирования (метод CONNECT)
    if method.eq_ignore_ascii_case("CONNECT") {
        let (host, port) = parse_host_port(target, 443);
        println!("[LOOPBACK] HTTPS CONNECT к {}:{}", host, port);

        // Открываем TCP стрим через Wisp на VPS (Remote DNS)
        let (_stream_id, mut wisp_rx, wisp_tx) = wisp_client.create_tcp_stream(&host, port).await?;
        println!("[LOOPBACK] WISP поток успешно создан для {}:{}", host, port);

        // Отвечаем браузеру, что туннель установлен
        client_stream
            .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
            .await?;

        let (mut client_read, mut client_write) = client_stream.into_split();

        // Пересылка: WebView2 -> Wisp Tunnel
        let tx_clone = wisp_tx.clone();
        let client_to_wisp = tokio::spawn(async move {
            let mut read_buf = [0u8; 16384];
            while let Ok(read_n) = client_read.read(&mut read_buf).await {
                if read_n == 0 {
                    break;
                }
                if tx_clone.send_data(&read_buf[..read_n]).await.is_err() {
                    break;
                }
            }
            let _ = tx_clone.close(0x01).await;
        });

        // Пересылка: Wisp Tunnel -> WebView2
        let wisp_to_client = tokio::spawn(async move {
            while let Some(chunk) = wisp_rx.recv().await {
                if client_write.write_all(&chunk).await.is_err() {
                    break;
                }
            }
        });

        tokio::select! {
            _ = client_to_wisp => {},
            _ = wisp_to_client => {},
        }
    } else {
        // 3. Прямой HTTP запрос
        let (host, port) = parse_http_target(target, &request_str);
        let (_stream_id, mut wisp_rx, wisp_tx) = wisp_client.create_tcp_stream(&host, port).await?;

        // Отправляем изначальный HTTP запрос в стрим
        wisp_tx.send_data(&buf[..n]).await?;

        let (mut client_read, mut client_write) = client_stream.into_split();

        let tx_clone = wisp_tx.clone();
        let client_to_wisp = tokio::spawn(async move {
            let mut read_buf = [0u8; 16384];
            while let Ok(read_n) = client_read.read(&mut read_buf).await {
                if read_n == 0 {
                    break;
                }
                if tx_clone.send_data(&read_buf[..read_n]).await.is_err() {
                    break;
                }
            }
            let _ = tx_clone.close(0x01).await;
        });

        let wisp_to_client = tokio::spawn(async move {
            while let Some(chunk) = wisp_rx.recv().await {
                if client_write.write_all(&chunk).await.is_err() {
                    break;
                }
            }
        });

        tokio::select! {
            _ = client_to_wisp => {},
            _ = wisp_to_client => {},
        }
    }

    Ok(())
}

fn parse_host_port(target: &str, default_port: u16) -> (String, u16) {
    if let Some((h, p)) = target.split_once(':') {
        (h.to_string(), p.parse::<u16>().unwrap_or(default_port))
    } else {
        (target.to_string(), default_port)
    }
}

fn parse_http_target(target: &str, request_str: &str) -> (String, u16) {
    if let Ok(url) = url::Url::parse(target) {
        let host = url.host_str().unwrap_or("127.0.0.1").to_string();
        let port = url.port().unwrap_or(80);
        (host, port)
    } else {
        // Проверяем заголовок Host
        for line in request_str.lines() {
            if line.to_lowercase().starts_with("host:") {
                let host_val = line[5..].trim();
                return parse_host_port(host_val, 80);
            }
        }
        parse_host_port(target, 80)
    }
}
