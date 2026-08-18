// frontend/src-tauri/src/commands.rs
use crate::browser::TabManager;
use crate::security::device::get_hardware_fingerprint;
use crate::tunnel::{ProxyConnectionInfo, TunnelManager};
use tauri::{Manager, State, WebviewWindow};

/// Возвращает уникальный аппаратный отпечаток устройства
#[tauri::command]
pub fn get_device_id() -> String {
    get_hardware_fingerprint()
}

/// Запускает Wisp-туннель к VPS и поднимает локальный прокси
#[tauri::command]
pub async fn start_tunnel(
    wisp_url: String,
    token: String,
    manager: State<'_, TunnelManager>,
) -> Result<ProxyConnectionInfo, String> {
    manager.start(&wisp_url, &token).await
}

/// Останавливает активный прокси-туннель
#[tauri::command]
pub fn stop_tunnel(manager: State<'_, TunnelManager>) -> Result<(), String> {
    manager.stop();
    Ok(())
}

/// Возвращает статус и порт локального прокси
#[tauri::command]
pub fn get_tunnel_status(manager: State<'_, TunnelManager>) -> Option<ProxyConnectionInfo> {
    manager.get_info()
}


// ----------------------------------------------------
// НАЦИОНАЛЬНЫЕ КОМАНДЫ ДЛЯ CHROMIUM-ВКЛАДОК (MULTI-WEBVIEW)
// ----------------------------------------------------

#[tauri::command]
pub async fn native_navigate_tab(
    app: tauri::AppHandle,
    tab_id: String,
    url: String,
    tab_manager: State<'_, TabManager>,
) -> Result<(), String> {
    let tm = tab_manager.inner().clone();
    let app_clone = app.clone();
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.run_on_main_thread(move || {
        let win = app_clone.get_window("main")
            .or_else(|| app_clone.webview_windows().into_values().next().map(|w| w.as_ref().window().clone()));
        let res = match win {
            Some(w) => tm.create_or_navigate_tab(&w, &tab_id, &url),
            None => Err("Main window not found".to_string()),
        };
        let _ = tx.send(res);
    }).map_err(|e| e.to_string())?;

    rx.await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn native_switch_tab(
    app: tauri::AppHandle,
    tab_id: String,
    tab_manager: State<'_, TabManager>,
) -> Result<(), String> {
    let tm = tab_manager.inner().clone();
    let app_clone = app.clone();
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.run_on_main_thread(move || {
        let win = app_clone.get_window("main")
            .or_else(|| app_clone.webview_windows().into_values().next().map(|w| w.as_ref().window().clone()));
        let res = match win {
            Some(w) => tm.switch_to_tab(&w, &tab_id),
            None => Err("Main window not found".to_string()),
        };
        let _ = tx.send(res);
    }).map_err(|e| e.to_string())?;

    rx.await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn native_close_tab(
    app: tauri::AppHandle,
    tab_id: String,
    tab_manager: State<'_, TabManager>,
) -> Result<(), String> {
    let tm = tab_manager.inner().clone();
    let app_clone = app.clone();
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.run_on_main_thread(move || {
        let win = app_clone.get_window("main")
            .or_else(|| app_clone.webview_windows().into_values().next().map(|w| w.as_ref().window().clone()));
        let res = match win {
            Some(w) => tm.close_tab(&w, &tab_id),
            None => Err("Main window not found".to_string()),
        };
        let _ = tx.send(res);
    }).map_err(|e| e.to_string())?;

    rx.await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn native_go_back(
    app: tauri::AppHandle,
    tab_id: String,
    tab_manager: State<'_, TabManager>,
) -> Result<(), String> {
    let tm = tab_manager.inner().clone();
    let app_clone = app.clone();
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.run_on_main_thread(move || {
        let win = app_clone.get_window("main")
            .or_else(|| app_clone.webview_windows().into_values().next().map(|w| w.as_ref().window().clone()));
        let res = match win {
            Some(w) => tm.go_back(&w, &tab_id),
            None => Err("Main window not found".to_string()),
        };
        let _ = tx.send(res);
    }).map_err(|e| e.to_string())?;

    rx.await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn native_go_forward(
    app: tauri::AppHandle,
    tab_id: String,
    tab_manager: State<'_, TabManager>,
) -> Result<(), String> {
    let tm = tab_manager.inner().clone();
    let app_clone = app.clone();
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.run_on_main_thread(move || {
        let win = app_clone.get_window("main")
            .or_else(|| app_clone.webview_windows().into_values().next().map(|w| w.as_ref().window().clone()));
        let res = match win {
            Some(w) => tm.go_forward(&w, &tab_id),
            None => Err("Main window not found".to_string()),
        };
        let _ = tx.send(res);
    }).map_err(|e| e.to_string())?;

    rx.await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn native_reload(
    app: tauri::AppHandle,
    tab_id: String,
    tab_manager: State<'_, TabManager>,
) -> Result<(), String> {
    let tm = tab_manager.inner().clone();
    let app_clone = app.clone();
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.run_on_main_thread(move || {
        let win = app_clone.get_window("main")
            .or_else(|| app_clone.webview_windows().into_values().next().map(|w| w.as_ref().window().clone()));
        let res = match win {
            Some(w) => tm.reload(&w, &tab_id),
            None => Err("Main window not found".to_string()),
        };
        let _ = tx.send(res);
    }).map_err(|e| e.to_string())?;

    rx.await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn native_close_all_tabs(
    app: tauri::AppHandle,
    tab_manager: State<'_, TabManager>,
) -> Result<(), String> {
    let tm = tab_manager.inner().clone();
    let app_clone = app.clone();
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.run_on_main_thread(move || {
        let win = app_clone.get_window("main")
            .or_else(|| app_clone.webview_windows().into_values().next().map(|w| w.as_ref().window().clone()));
        let res = match win {
            Some(w) => tm.close_all_tabs(&w),
            None => Err("Main window not found".to_string()),
        };
        let _ = tx.send(res);
    }).map_err(|e| e.to_string())?;

    rx.await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn native_update_top_bar_height(
    app: tauri::AppHandle,
    height: f64,
    tab_manager: State<'_, TabManager>,
) {
    if let Some(win) = app.get_window("main").or_else(|| app.windows().into_values().next().map(|w| w.clone())) {
        tab_manager.update_top_bar_height(&win, height);
    }
}

#[tauri::command]
pub fn native_minimize_window(app: tauri::AppHandle) {
    if let Some(win) = app.get_window("main").or_else(|| app.windows().into_values().next().map(|w| w.clone())) {
        let _ = win.minimize();
    }
}

#[tauri::command]
pub fn native_maximize_window(app: tauri::AppHandle) {
    if let Some(win) = app.get_window("main").or_else(|| app.windows().into_values().next().map(|w| w.clone())) {
        if let Ok(is_maximized) = win.is_maximized() {
            if is_maximized {
                let _ = win.unmaximize();
            } else {
                let _ = win.maximize();
            }
        }
    }
}

#[tauri::command]
pub fn native_close_window(app: tauri::AppHandle) {
    if let Some(win) = app.get_window("main").or_else(|| app.windows().into_values().next().map(|w| w.clone())) {
        let _ = win.close();
    }
}

#[tauri::command]
pub fn native_start_dragging(app: tauri::AppHandle) {
    if let Some(win) = app.get_window("main").or_else(|| app.windows().into_values().next().map(|w| w.clone())) {
        let _ = win.start_dragging();
    }
}

#[tauri::command]
pub fn native_title_changed(app: tauri::AppHandle, tab_id: String, title: String) {
    #[derive(serde::Serialize, Clone)]
    struct TitlePayload {
        tab_id: String,
        title: String,
    }
    use tauri::Emitter;
    let _ = app.emit("webview-title-changed", TitlePayload { tab_id, title });
}

#[tauri::command]
pub fn native_favicon_changed(app: tauri::AppHandle, tab_id: String, favicon: String) {
    #[derive(serde::Serialize, Clone)]
    struct FaviconPayload {
        tab_id: String,
        favicon: String,
    }
    use tauri::Emitter;
    let _ = app.emit("webview-favicon-changed", FaviconPayload { tab_id, favicon });
}

#[tauri::command]
pub fn native_context_menu(app: tauri::AppHandle, window: String, href: String, src: String, selection: String) {
    println!("Context menu: window={}, href={}, src={}, selection={}", window, href, src, selection);
}

#[tauri::command]
pub fn native_close_window_by_label(app: tauri::AppHandle, label: String) {
    if let Some(win) = app.get_window(&label) {
        let _ = win.close();
    }
}

#[tauri::command]
pub fn native_url_changed(app: tauri::AppHandle, tab_id: String, url: String) {
    #[derive(serde::Serialize, Clone)]
    #[allow(non_snake_case)]
    struct UrlPayload {
        tabId: String,
        url: String,
    }
    use tauri::Emitter;
    // Emit webview-loaded so frontend updates the URL and stops loading spinner
    let _ = app.emit("webview-loaded", UrlPayload { tabId: tab_id, url });
}

#[tauri::command]
pub fn native_set_zoom(
    app: tauri::AppHandle,
    tab_id: String,
    zoom_factor: f64,
    tab_manager: State<'_, TabManager>,
) -> Result<(), String> {
    if let Some(win) = app.get_window("main").or_else(|| app.windows().into_values().next().map(|w| w.clone())) {
        tab_manager.set_zoom(&win, &tab_id, zoom_factor)?;
    }
    Ok(())
}

#[tauri::command]
pub fn native_download_started(
    app: tauri::AppHandle,
    tab_id: String,
    url: String,
    filename: String,
) {
    #[derive(serde::Serialize, Clone)]
    #[allow(non_snake_case)]
    struct DownloadPayload {
        tabId: String,
        url: String,
        filename: String,
    }
    use tauri::Emitter;
    let _ = app.emit("webview-download-started", DownloadPayload {
        tabId: tab_id,
        url,
        filename,
    });
}

#[tauri::command]
pub fn native_open_path(path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = path;
        Ok(())
    }
}

#[tauri::command]
pub fn native_show_in_folder(path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg("/select,")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = path;
        Ok(())
    }
}

#[tauri::command]
pub fn native_get_downloads_dir() -> String {
    if let Ok(user_profile) = std::env::var("USERPROFILE") {
        let downloads = std::path::Path::new(&user_profile).join("Downloads");
        if downloads.exists() {
            return downloads.to_string_lossy().to_string();
        }
    }
    ".".to_string()
}
