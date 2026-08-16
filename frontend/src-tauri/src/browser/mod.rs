use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{Manager, WebviewBuilder, WebviewUrl, WebviewWindow};
use url::Url;

pub const TOP_BAR_HEIGHT: f64 = 84.0;

/// Менеджер нативных Chromium-вкладок (Multi-Webview)
#[derive(Clone, Default)]
pub struct TabManager {
    // Хранилище созданных вкладок: tab_id -> webview label
    tabs: Arc<Mutex<HashMap<String, String>>>,
    active_tab_id: Arc<Mutex<Option<String>>>,
}

impl TabManager {
    pub fn new() -> Self {
        Self {
            tabs: Arc::new(Mutex::new(HashMap::new())),
            active_tab_id: Arc::new(Mutex::new(None)),
        }
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

        // 1. Если вкладка уже создана — переключаем ее, а навигацию выполняем только если URL изменился
        if let Some(existing_wv) = app_handle.get_webview(&label) {
            println!("[TAB_MANAGER] Found existing webview for {}.", label);
            if let Ok(curr_url) = existing_wv.url() {
                if let WebviewUrl::External(u) = &parsed_url {
                    if curr_url.as_str() != u.as_str() {
                        println!("[TAB_MANAGER] URL changed from {} to {}. Navigating...", curr_url, u);
                        let _ = existing_wv.navigate(u.clone());
                    }
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
        let content_height = (logical_size.height - TOP_BAR_HEIGHT).max(100.0);

        // 3. Создаем настоящий дочерний Webview внутри окна
        println!("[TAB_MANAGER] Adding child Webview for label {}...", label);
        let builder = WebviewBuilder::new(&label, parsed_url);

        let _wv = window
            .add_child(
                builder,
                tauri::LogicalPosition::new(0.0, TOP_BAR_HEIGHT),
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
        let content_height = (logical_size.height - TOP_BAR_HEIGHT).max(100.0);

        let tabs_map = self.tabs.lock();
        let mut found_active = false;
        for (id, label) in tabs_map.iter() {
            if let Some(wv) = app_handle.get_webview(label) {
                if id == tab_id {
                    println!("[TAB_MANAGER] Showing webview: {}", label);
                    let _ = wv.set_position(tauri::LogicalPosition::new(0.0, TOP_BAR_HEIGHT));
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
            let _ = self.switch_to_tab(window, &id);
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
