// frontend/src-tauri/src/commands.rs
use crate::browser::TabManager;
use crate::security::crypto::{decrypt_string, encrypt_string, EncryptedPayload};
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

/// Zero-Knowledge шифрование данных (куки + вкладки) перед отправкой на VPS
#[tauri::command]
pub fn encrypt_session_data(data_json: String, password: String) -> Result<EncryptedPayload, String> {
    encrypt_string(&data_json, &password)
}

/// Zero-Knowledge расшифровка данных сессии после скачивания с VPS
#[tauri::command]
pub fn decrypt_session_data(
    payload_b64: String,
    iv_b64: String,
    password: String,
) -> Result<String, String> {
    decrypt_string(&payload_b64, &iv_b64, &password)
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
