// frontend/src-tauri/src/tunnel/mod.rs
pub mod loopback;
pub mod wisp;

use loopback::LoopbackProxyServer;
use parking_lot::Mutex;
use std::sync::Arc;
use wisp::WispClient;

#[derive(Clone, Default)]
pub struct TunnelManager {
    inner: Arc<Mutex<Option<ActiveTunnel>>>,
}

#[allow(dead_code)]
struct ActiveTunnel {
    pub port: u16,
    pub session_secret: String,
    pub shutdown_tx: tokio::sync::watch::Sender<bool>,
    pub wisp_url: String,
    pub jwt_token: String,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct ProxyConnectionInfo {
    pub local_port: u16,
    pub session_secret: String,
    pub is_active: bool,
}

impl TunnelManager {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(None)),
        }
    }

    /// Запускает защищенный туннель к VPS через Wisp и поднимает локальный прокси
    pub async fn start(&self, wisp_base_url: &str, jwt_token: &str) -> Result<ProxyConnectionInfo, String> {
        // Если туннель уже был запущен — останавливаем старый
        self.stop();

        let formatted_url = if wisp_base_url.ends_with('/') {
            format!("{}{}", wisp_base_url, urlencoding::encode(jwt_token))
        } else {
            format!("{}/{}", wisp_base_url, urlencoding::encode(jwt_token))
        };

        // 1. Подключаемся к Wisp шлюзу на VPS
        let wisp_client = WispClient::connect(&formatted_url).await?;

        // 2. Поднимаем локальный прокси с секретом
        let server = LoopbackProxyServer::start(wisp_client).await?;

        let port = server.port;
        let session_secret = server.session_secret.clone();

        // Пробрасываем локальный прокси в системное окружение процесса
        let proxy_url = format!("http://127.0.0.1:{}", port);
        std::env::set_var("HTTP_PROXY", &proxy_url);
        std::env::set_var("HTTPS_PROXY", &proxy_url);
        std::env::set_var("ALL_PROXY", &proxy_url);

        {
            let mut guard = self.inner.lock();
            *guard = Some(ActiveTunnel {
                port,
                session_secret: session_secret.clone(),
                shutdown_tx: server.shutdown_tx,
                wisp_url: wisp_base_url.to_string(),
                jwt_token: jwt_token.to_string(),
            });
        }

        Ok(ProxyConnectionInfo {
            local_port: port,
            session_secret,
            is_active: true,
        })
    }

    /// Останавливает туннель и закрывает локальный порт
    pub fn stop(&self) {
        std::env::remove_var("HTTP_PROXY");
        std::env::remove_var("HTTPS_PROXY");
        std::env::remove_var("ALL_PROXY");

        let mut guard = self.inner.lock();
        if let Some(tunnel) = guard.take() {
            let _ = tunnel.shutdown_tx.send(true);
        }
    }

    /// Проверяет статус и возвращает текущую информацию о прокси
    pub fn get_info(&self) -> Option<ProxyConnectionInfo> {
        let guard = self.inner.lock();
        guard.as_ref().map(|t| ProxyConnectionInfo {
            local_port: t.port,
            session_secret: t.session_secret.clone(),
            is_active: true,
        })
    }
}
