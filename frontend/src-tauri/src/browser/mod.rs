use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{Emitter, Manager, WebviewBuilder, WebviewUrl};
use url::Url;

/// Менеджер нативных Chromium-вкладок (Multi-Webview)
#[derive(Clone, Default)]
pub struct TabManager {
    // Хранилище созданных вкладок: tab_id -> webview label
    tabs: Arc<Mutex<HashMap<String, String>>>,
    active_tab_id: Arc<Mutex<Option<String>>>,
    top_bar_height: Arc<Mutex<f64>>,
}

impl TabManager {
    pub fn new() -> Self {
        Self {
            tabs: Arc::new(Mutex::new(HashMap::new())),
            active_tab_id: Arc::new(Mutex::new(None)),
            top_bar_height: Arc::new(Mutex::new(84.0)), // Значение по умолчанию
        }
    }

    pub fn update_top_bar_height(&self, window: &tauri::Window, height: f64) {
        {
            let mut h = self.top_bar_height.lock();
            *h = height;
        }
        self.sync_main_window_geometry(window);
    }

    /// Создает новую нативную вкладку Chromium под панелью управления или переходит по URL
    pub fn create_or_navigate_tab(&self, window: &tauri::Window, tab_id: &str, target_url: &str) -> Result<(), String> {
        let label = format!("tab-wv-{}", tab_id);
        println!("[TAB_MANAGER] create_or_navigate_tab: tab_id={}, target_url={}, label={}", tab_id, target_url, label);
        
        let is_blank = target_url.is_empty() || target_url == "about:blank";
        if is_blank {
            println!("[TAB_MANAGER] Blank URL -> hiding all webviews");
            self.hide_all_webviews(window);
            let mut active = self.active_tab_id.lock();
            *active = Some(tab_id.to_string());
            return Ok(());
        }

        let parsed_url = if let Ok(u) = Url::parse(target_url) {
            WebviewUrl::External(u)
        } else {
            let search_url = format!("https://duckduckgo.com/?q={}", urlencoding::encode(target_url));
            WebviewUrl::External(Url::parse(&search_url).map_err(|e| e.to_string())?)
        };

        let app_handle = window.app_handle();

        // 1. Если вкладка уже создана — выполняем навигацию и переключаем
        if let Some(existing_wv) = app_handle.get_webview(&label) {
            println!("[TAB_MANAGER] Found existing webview for {}. Navigating to {:?}", label, parsed_url);
            if let WebviewUrl::External(u) = &parsed_url {
                if let Err(e) = existing_wv.navigate(u.clone()) {
                    println!("[TAB_MANAGER] ERROR navigating webview: {:?}", e);
                }
            }
            self.switch_to_tab(window, tab_id)?;
            return Ok(());
        }

        // 2. Рассчитываем геометрию (строго под панелью 84px)
        let window_size = window
            .inner_size()
            .map_err(|e| format!("Failed to get window size: {}", e))?;
        let scale_factor = window
            .scale_factor()
            .map_err(|e| format!("Failed to get scale factor: {}", e))?;
        let logical_size = window_size.to_logical::<f64>(scale_factor);
        let top_bar_height = *self.top_bar_height.lock();
        let content_height = (logical_size.height - top_bar_height).max(100.0);
        let t_id = tab_id.to_string();
        let t_id_ipc = t_id.clone();
        
        let base_script = r#"
                if (window !== window.top) return;

                // Спуфинг объекта Client Hints
                if (navigator.userAgentData) {
                  Object.defineProperty(navigator, 'userAgentData', {
                    get: () => ({
                      brands: [
                        { brand: 'Google Chrome', version: '131' },
                        { brand: 'Chromium', version: '131' },
                        { brand: 'Not_A Brand', version: '24' }
                      ],
                      mobile: false,
                      platform: 'Windows',
                      getHighEntropyValues: async () => ({
                        architecture: 'x86',
                        bitness: '64',
                        model: '',
                        platform: 'Windows',
                        platformVersion: '10.0.0',
                        uaFullVersion: '131.0.0.0'
                      })
                    })
                  });
                }

                window.addEventListener('contextmenu', (e) => {
                    e.preventDefault();
                    let href = '';
                    let src = '';
                    let target = e.target;
                    while (target && target !== document) {
                        if (target.tagName === 'A' && target.href) href = target.href;
                        if (target.tagName === 'IMG' && target.src) src = target.src;
                        target = target.parentNode;
                    }
                    let selection = window.getSelection().toString();
                    if (window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.core.invoke) {
                        window.__TAURI__.core.invoke('native_context_menu', {
                            window: '{}',
                            href: href,
                            src: src,
                            selection: selection
                        }).catch(() => {
                            if (window.chrome && window.chrome.webview) {
                                window.chrome.webview.postMessage(JSON.stringify({
                                    cmd: 'native_context_menu',
                                    href: href,
                                    src: src,
                                    selection: selection
                                }));
                            }
                        });
                    } else if (window.chrome && window.chrome.webview) {
                        window.chrome.webview.postMessage(JSON.stringify({
                            cmd: 'native_context_menu',
                            href: href,
                            src: src,
                            selection: selection
                        }));
                    }
                });

                // Track title changes reliably across all origins
                const notifyTitle = () => {
                    if (!document.title) return;
                    const title = document.title;
                    try {
                        if (window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.core.invoke) {
                            window.__TAURI__.core.invoke('native_title_changed', {
                                tabId: '{{TAB_ID}}',
                                title: title
                            }).catch(() => {});
                        } else if (window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke) {
                            window.__TAURI_INTERNALS__.invoke('native_title_changed', {
                                tabId: '{{TAB_ID}}',
                                title: title
                            }).catch(() => {});
                        }
                    } catch (_) {}
                };

                let lastTitle = document.title;
                const checkTitle = () => {
                    if (document.title && document.title !== lastTitle) {
                        lastTitle = document.title;
                        notifyTitle();
                    }
                };

                const notifyFavicon = () => {
                    let icon = '';
                    const links = document.querySelectorAll('link[rel="icon"], link[rel="shortcut icon"], link[rel="apple-touch-icon"]');
                    if (links.length > 0) {
                        for (let i = links.length - 1; i >= 0; i--) {
                            if (links[i].href) {
                                icon = links[i].href;
                                break;
                            }
                        }
                    }
                    if (!icon) {
                        icon = window.location.origin + '/favicon.ico';
                    }
                    try {
                        if (window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.core.invoke) {
                            window.__TAURI__.core.invoke('native_favicon_changed', {
                                tabId: '{{TAB_ID}}',
                                favicon: icon
                            }).catch(() => {});
                        } else if (window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke) {
                            window.__TAURI_INTERNALS__.invoke('native_favicon_changed', {
                                tabId: '{{TAB_ID}}',
                                favicon: icon
                            }).catch(() => {});
                        }
                    } catch (_) {}
                };

                let lastFavicon = '';
                const checkFavicon = () => {
                    const links = document.querySelectorAll('link[rel="icon"], link[rel="shortcut icon"], link[rel="apple-touch-icon"]');
                    let currentFavicon = window.location.origin + '/favicon.ico';
                    if (links.length > 0) {
                        for (let i = links.length - 1; i >= 0; i--) {
                            if (links[i].href) {
                                currentFavicon = links[i].href;
                                break;
                            }
                        }
                    }
                    if (currentFavicon !== lastFavicon) {
                        lastFavicon = currentFavicon;
                        notifyFavicon();
                    }
                };

                setInterval(() => {
                    checkTitle();
                    checkFavicon();
                }, 1000);

                window.addEventListener('DOMContentLoaded', notifyTitle);
                window.addEventListener('load', notifyTitle);

                // Track URL changes for SPAs
                const notifyUrl = (url) => {
                    try {
                        if (window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.core.invoke) {
                            window.__TAURI__.core.invoke('native_url_changed', {
                                tabId: '{{TAB_ID}}',
                                url: url
                            }).catch(() => {});
                        } else if (window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke) {
                            window.__TAURI_INTERNALS__.invoke('native_url_changed', {
                                tabId: '{{TAB_ID}}',
                                url: url
                            }).catch(() => {});
                        }
                    } catch (_) {}
                    setTimeout(notifyTitle, 200);
                };
                window.addEventListener('popstate', () => notifyUrl(window.location.href));
                const originalPushState = history.pushState;
                history.pushState = function() {
                    originalPushState.apply(this, arguments);
                    notifyUrl(window.location.href);
                };
                const originalReplaceState = history.replaceState;
                history.replaceState = function() {
                    originalReplaceState.apply(this, arguments);
                    notifyUrl(window.location.href);
                };
        "#;
        let init_script = base_script.replace("{{TAB_ID}}", &t_id_ipc);

        let t_id_load = t_id.clone();
        let mut builder = WebviewBuilder::new(&label, parsed_url)
            .incognito(tab_id.starts_with("fake_"))
            .initialization_script(&init_script)
            .on_page_load(move |webview, _payload| {
                // Автоматическая перезагрузка при первом открытии вкладки в сессии (решает проблему белого экрана ChatGPT/YouTube)
                let _ = webview.eval(r#"
                    if (!sessionStorage.getItem('first_load_done')) {
                        sessionStorage.setItem('first_load_done', 'true');
                        window.location.reload();
                    }
                "#);

                if let Ok(url) = webview.url() {
                    #[allow(non_snake_case)]
                    #[derive(serde::Serialize, Clone)]
                    struct Payload {
                        tabId: String,
                        url: String,
                    }
                    let _ = webview.emit("webview-loaded", Payload {
                        tabId: t_id.clone(),
                        url: url.to_string(),
                    });

                    // Trigger title evaluation on load
                    let script = format!(r#"
                        (function() {{
                            const t = document.title;
                            if (t) {{
                                if (window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.core.invoke) {{
                                    window.__TAURI__.core.invoke('native_title_changed', {{ tabId: '{}', title: t }});
                                }} else if (window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke) {{
                                    window.__TAURI_INTERNALS__.invoke('native_title_changed', {{ tabId: '{}', title: t }});
                                }}
                            }}
                        }})();
                    "#, t_id_load, t_id_load);
                    let _ = webview.eval(&script);
                }
            })
            .on_download(move |webview, event| {
                match event {
                    tauri::webview::DownloadEvent::Requested { url, destination } => {
                        println!("[DOWNLOAD] Requested: {:?} -> {:?}", url, destination);
                        let file_name = url.path_segments()
                            .and_then(|s| s.last())
                            .unwrap_or("download")
                            .to_string();
                        
                        #[derive(serde::Serialize, Clone)]
                        #[allow(non_snake_case)]
                        struct DlPayload {
                            tabId: String,
                            url: String,
                            filename: String,
                        }
                        use tauri::Emitter;
                        let _ = webview.emit("webview-download-started", DlPayload {
                            tabId: String::new(),
                            url: url.to_string(),
                            filename: file_name,
                        });
                        true
                    }
                    _ => true
                }
            });

        let _wv = window
            .add_child(
                builder,
                tauri::LogicalPosition::new(0.0, top_bar_height),
                tauri::LogicalSize::new(logical_size.width, content_height),
            )
            .map_err(|e| {
                println!("[TAB_MANAGER] ERROR adding child Webview: {:?}", e);
                format!("Failed to add child Webview: {}", e)
            })?;

        {
            let mut tabs_map = self.tabs.lock();
            tabs_map.insert(tab_id.to_string(), label.clone());
        }

        self.switch_to_tab(window, tab_id)?;

        Ok(())
    }

    /// Скрывает все дочерние webview
    pub fn hide_all_webviews(&self, window: &tauri::Window) {
        let app_handle = window.app_handle();
        let tabs_map = self.tabs.lock();
        for (_, label) in tabs_map.iter() {
            if let Some(wv) = app_handle.get_webview(label) {
                let _ = wv.hide();
            }
        }
    }

    /// Переключает видимость между нативными вкладками
    pub fn switch_to_tab(&self, window: &tauri::Window, tab_id: &str) -> Result<(), String> {
        println!("[TAB_MANAGER] switch_to_tab: {}", tab_id);
        let app_handle = window.app_handle();

        let window_size = window
            .inner_size()
            .unwrap_or(tauri::PhysicalSize::new(800, 600));
        let scale_factor = window
            .scale_factor()
            .unwrap_or(1.0);
        let logical_size = window_size.to_logical::<f64>(scale_factor);
        let top_bar_height = *self.top_bar_height.lock();
        let content_height = (logical_size.height - top_bar_height).max(100.0);

        let tabs_map = self.tabs.lock();
        let mut found_active = false;
        for (id, label) in tabs_map.iter() {
            if let Some(wv) = app_handle.get_webview(label) {
                if id == tab_id {
                    println!("[TAB_MANAGER] Showing webview: {}", label);
                    let _ = wv.set_position(tauri::LogicalPosition::new(0.0, top_bar_height));
                    let _ = wv.set_size(tauri::LogicalSize::new(logical_size.width, content_height));
                    let _ = wv.show();
                    let _ = wv.set_focus();
                    found_active = true;
                } else {
                    let _ = wv.hide();
                }
            }
        }

        if !found_active {
            println!("[TAB_MANAGER] No webview found for tab_id: {}. Hiding all.", tab_id);
            for (_, label) in tabs_map.iter() {
                if let Some(wv) = app_handle.get_webview(label) {
                    let _ = wv.hide();
                }
            }
            
            let mut active = self.active_tab_id.lock();
            *active = Some(tab_id.to_string());
            
            return Err("NOT_FOUND".to_string());
        }

        let mut active = self.active_tab_id.lock();
        *active = Some(tab_id.to_string());

        Ok(())
    }

    /// Закрывает нативную вкладку и уничтожает ее Webview
    pub fn close_tab(&self, window: &tauri::Window, tab_id: &str) -> Result<(), String> {
        let app_handle = window.app_handle();
        let label = {
            let mut tabs_map = self.tabs.lock();
            tabs_map.remove(tab_id)
        };

        println!("[TAB_MANAGER] Closing tab_id: {} -> label: {:?}", tab_id, label);

        if let Some(lbl) = label {
            if let Some(wv) = app_handle.get_webview(&lbl) {
                println!("[TAB_MANAGER] Found webview for label: {}. Destroying...", lbl);
                let _ = wv.hide();
                let _ = wv.close().map_err(|e| {
                    println!("[TAB_MANAGER] Error closing webview: {}", e);
                    e.to_string()
                });
            } else {
                println!("[TAB_MANAGER] Webview for label {} not found in app_handle!", lbl);
            }
        }

        let mut active = self.active_tab_id.lock();
        if active.as_deref() == Some(tab_id) {
            *active = None;
        }

        Ok(())
    }

    /// Устанавливает масштаб веб-страницы для конкретной вкладки
    pub fn set_zoom(&self, window: &tauri::Window, tab_id: &str, zoom_factor: f64) -> Result<(), String> {
        let app_handle = window.app_handle();
        let tabs_map = self.tabs.lock();
        if let Some(label) = tabs_map.get(tab_id) {
            if let Some(wv) = app_handle.get_webview(label) {
                let _ = wv.set_zoom(zoom_factor);
            }
        }
        Ok(())
    }

    /// Закрывает все вкладки (при смене пользователя / выходе)
    pub fn close_all_tabs(&self, window: &tauri::Window) -> Result<(), String> {
        let app_handle = window.app_handle();
        let labels: Vec<String> = {
            let mut tabs_map = self.tabs.lock();
            let list = tabs_map.values().cloned().collect();
            tabs_map.clear();
            list
        };

        for lbl in labels {
            if let Some(wv) = app_handle.get_webview(&lbl) {
                let _ = wv.hide();
                let _ = wv.close();
            }
        }

        let mut active = self.active_tab_id.lock();
        *active = None;

        Ok(())
    }

    /// Синхронизирует размер и координаты активной вкладки с главным окном при перемещении/ресайзе
    pub fn sync_main_window_geometry(&self, window: &tauri::Window) {
        let active_id = self.active_tab_id.lock().clone();
        if let Some(id) = active_id {
            let label = {
                let tabs_map = self.tabs.lock();
                tabs_map.get(&id).cloned()
            };
            
            if let Some(lbl) = label {
                if let Some(wv) = window.app_handle().get_webview(&lbl) {
                    if let Ok(window_size) = window.inner_size() {
                        let scale_factor = window.scale_factor().unwrap_or(1.0);
                        let logical_size = window_size.to_logical::<f64>(scale_factor);
                        let top_bar_height = *self.top_bar_height.lock();
                        let content_height = (logical_size.height - top_bar_height).max(100.0);
                        
                        let _ = wv.set_position(tauri::LogicalPosition::new(0.0, top_bar_height));
                        let _ = wv.set_size(tauri::LogicalSize::new(logical_size.width, content_height));
                    }
                }
            }
        }
    }

    /// Синхронизирует видимость активной вкладки при сворачивании/потере фокуса главного окна
    pub fn sync_main_window_focus(&self, window: &tauri::Window, focused: bool) {
        let app_handle = window.app_handle();
        let active_id = self.active_tab_id.lock().clone();

        if let Some(id) = active_id {
            let tabs_map = self.tabs.lock();
            if let Some(label) = tabs_map.get(&id) {
                if let Some(win) = app_handle.get_webview_window(label) {
                    if focused {
                        let _ = win.show();
                    } else {
                        let _ = win.hide();
                    }
                }
            }
        }
    }

    /// Навигация «Назад»
    pub fn go_back(&self, window: &tauri::Window, tab_id: &str) -> Result<(), String> {
        let app_handle = window.app_handle();
        let tabs_map = self.tabs.lock();
        if let Some(label) = tabs_map.get(tab_id) {
            if let Some(wv) = app_handle.get_webview(label) {
                let _ = wv.eval("window.history.back();");
            }
        }
        Ok(())
    }

    /// Навигация «Вперед»
    pub fn go_forward(&self, window: &tauri::Window, tab_id: &str) -> Result<(), String> {
        let app_handle = window.app_handle();
        let tabs_map = self.tabs.lock();
        if let Some(label) = tabs_map.get(tab_id) {
            if let Some(wv) = app_handle.get_webview(label) {
                let _ = wv.eval("window.history.forward();");
            }
        }
        Ok(())
    }

    /// Перезагрузка страницы
    pub fn reload(&self, window: &tauri::Window, tab_id: &str) -> Result<(), String> {
        let app_handle = window.app_handle();
        let tabs_map = self.tabs.lock();
        if let Some(label) = tabs_map.get(tab_id) {
            if let Some(wv) = app_handle.get_webview(label) {
                let _ = wv.eval("window.location.reload();");
            }
        }
        Ok(())
    }
}
