// frontend/src-tauri/src/browser/mod.rs
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use url::Url;

pub const TOP_BAR_HEIGHT: f64 = 76.0;

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
    pub fn create_or_navigate_tab(&self, window: &WebviewWindow, tab_id: &str, target_url: &str) -> Result<(), String> {
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

        // 1. Если вкладка уже создана — переходим по адресу и делаем ее видимой
        if let Some(existing_win) = app_handle.get_webview_window(&label) {
            println!("[TAB_MANAGER] Found existing webview window for {}. Navigating...", label);
            if let WebviewUrl::External(u) = &parsed_url {
                let _ = existing_win.navigate(u.clone());
            }
            self.switch_to_tab(window, tab_id)?;
            return Ok(());
        }

        // 2. Рассчитываем геометрию (строго под панелью 76px)
        let window_size = window
            .inner_size()
            .map_err(|e| format!("Failed to get window size: {}", e))?;
        let scale_factor = window
            .scale_factor()
            .map_err(|e| format!("Failed to get scale factor: {}", e))?;
        let logical_size = window_size.to_logical::<f64>(scale_factor);

        let window_pos = window
            .outer_position()
            .unwrap_or(tauri::PhysicalPosition::new(0, 0));
        let logical_pos = window_pos.to_logical::<f64>(scale_factor);

        let content_height = (logical_size.height - TOP_BAR_HEIGHT).max(100.0);
        println!("[TAB_MANAGER] Window size: {:?}, logical: {:?}, content_height: {}", window_size, logical_size, content_height);

        // 3. Создаем нативный WebviewWindow с собственным HWND
        println!("[TAB_MANAGER] Creating WebviewWindow for label {}...", label);
        let builder = WebviewWindowBuilder::new(app_handle, &label, parsed_url)
            .title("Tab")
            .decorations(false)
            .shadow(false)
            .skip_taskbar(true)
            .position(logical_pos.x, logical_pos.y + TOP_BAR_HEIGHT)
            .inner_size(logical_size.width, content_height);

        let _win = match builder.build() {
            Ok(w) => {
                println!("[TAB_MANAGER] Successfully created child WebviewWindow: {}", label);
                w
            },
            Err(e) => {
                println!("[TAB_MANAGER] ERROR building WebviewWindow: {:?}", e);
                return Err(format!("Failed to create child WebviewWindow: {}", e));
            }
        };

        {
            let mut tabs_map = self.tabs.lock();
            tabs_map.insert(tab_id.to_string(), label.clone());
        }

        self.switch_to_tab(window, tab_id)?;

        Ok(())
    }

    /// Скрывает все дочерние webview
    pub fn hide_all_webviews(&self, window: &WebviewWindow) {
        let app_handle = window.app_handle();
        let tabs_map = self.tabs.lock();
        for (_, label) in tabs_map.iter() {
            if let Some(win) = app_handle.get_webview_window(label) {
                let _ = win.hide();
            }
        }
    }

    /// Переключает видимость между нативными вкладками
    pub fn switch_to_tab(&self, window: &WebviewWindow, tab_id: &str) -> Result<(), String> {
        println!("[TAB_MANAGER] switch_to_tab: {}", tab_id);
        let app_handle = window.app_handle();

        let window_size = window
            .inner_size()
            .unwrap_or(tauri::PhysicalSize::new(800, 600));
        let scale_factor = window
            .scale_factor()
            .unwrap_or(1.0);
        let logical_size = window_size.to_logical::<f64>(scale_factor);

        let window_pos = window
            .outer_position()
            .unwrap_or(tauri::PhysicalPosition::new(0, 0));
        let logical_pos = window_pos.to_logical::<f64>(scale_factor);
        let content_height = (logical_size.height - TOP_BAR_HEIGHT).max(100.0);

        let tabs_map = self.tabs.lock();
        let mut found_active = false;
        for (id, label) in tabs_map.iter() {
            if let Some(win) = app_handle.get_webview_window(label) {
                if id == tab_id {
                    println!("[TAB_MANAGER] Showing webview window: {}", label);
                    let _ = win.set_position(tauri::LogicalPosition::new(logical_pos.x, logical_pos.y + TOP_BAR_HEIGHT));
                    let _ = win.set_size(tauri::LogicalSize::new(logical_size.width, content_height));
                    let _ = win.show();
                    let _ = win.set_focus();
                    found_active = true;
                } else {
                    let _ = win.hide();
                }
            }
        }

        if !found_active {
            println!("[TAB_MANAGER] No webview window found for tab_id: {}. Hiding all.", tab_id);
            for (_, label) in tabs_map.iter() {
                if let Some(win) = app_handle.get_webview_window(label) {
                    let _ = win.hide();
                }
            }
        }

        let mut active = self.active_tab_id.lock();
        *active = Some(tab_id.to_string());

        Ok(())
    }

    /// Закрывает нативную вкладку и уничтожает ее Webview
    pub fn close_tab(&self, window: &WebviewWindow, tab_id: &str) -> Result<(), String> {
        let app_handle = window.app_handle();
        let label = {
            let mut tabs_map = self.tabs.lock();
            tabs_map.remove(tab_id)
        };

        println!("[TAB_MANAGER] Closing tab_id: {} -> label: {:?}", tab_id, label);

        if let Some(lbl) = label {
            if let Some(win) = app_handle.get_webview_window(&lbl) {
                println!("[TAB_MANAGER] Found webview window for label: {}. Destroying...", lbl);
                let _ = win.close().map_err(|e| {
                    println!("[TAB_MANAGER] Error closing webview window: {}", e);
                    e.to_string()
                });
            } else {
                println!("[TAB_MANAGER] WebviewWindow for label {} not found in app_handle!", lbl);
            }
        }

        let mut active = self.active_tab_id.lock();
        if active.as_deref() == Some(tab_id) {
            *active = None;
        }

        Ok(())
    }

    /// Навигация «Назад»
    pub fn go_back(&self, window: &WebviewWindow, tab_id: &str) -> Result<(), String> {
        let app_handle = window.app_handle();
        let tabs_map = self.tabs.lock();
        if let Some(label) = tabs_map.get(tab_id) {
            if let Some(win) = app_handle.get_webview_window(label) {
                let _ = win.eval("window.history.back();");
            }
        }
        Ok(())
    }

    /// Навигация «Вперед»
    pub fn go_forward(&self, window: &WebviewWindow, tab_id: &str) -> Result<(), String> {
        let app_handle = window.app_handle();
        let tabs_map = self.tabs.lock();
        if let Some(label) = tabs_map.get(tab_id) {
            if let Some(win) = app_handle.get_webview_window(label) {
                let _ = win.eval("window.history.forward();");
            }
        }
        Ok(())
    }

    /// Перезагрузка страницы
    pub fn reload(&self, window: &WebviewWindow, tab_id: &str) -> Result<(), String> {
        let app_handle = window.app_handle();
        let tabs_map = self.tabs.lock();
        if let Some(label) = tabs_map.get(tab_id) {
            if let Some(win) = app_handle.get_webview_window(label) {
                let _ = win.eval("window.location.reload();");
            }
        }
        Ok(())
    }
}
