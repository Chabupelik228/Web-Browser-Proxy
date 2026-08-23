pub mod browser;
pub mod commands;
pub mod security;
pub mod tunnel;

use browser::TabManager;
use commands::*;
use tauri::Manager;
use tunnel::TunnelManager;

#[tauri::command]
fn force_exit() {
    std::process::exit(0);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Полное удаление всех данных (cookies, localStorage, cache) ДО запуска WebView2,
    // чтобы избежать блокировки файлов и ERR_CACHE_READ_FAILURE
    if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
        let path = std::path::Path::new(&local_app_data).join("com.google.chrome");
        let _ = std::fs::remove_dir_all(&path);
    }
    if let Ok(app_data) = std::env::var("APPDATA") {
        let path = std::path::Path::new(&app_data).join("com.google.chrome");
        let _ = std::fs::remove_dir_all(&path);
    }

    // WebView2 не может работать без папки данных на диске, поэтому перенаправляем её
    // в %TEMP%\ShellCache — в LocalAppData/APPDATA приложение ничего не создаёт.
    // Чистим её от прошлого запуска ДО инициализации WebView2.
    let webview_data_dir = std::env::temp_dir().join("ShellCache");
    let _ = std::fs::remove_dir_all(&webview_data_dir);
    std::env::set_var(
        "WEBVIEW2_USER_DATA_FOLDER",
        &webview_data_dir,
    );

    let _ = dotenvy::from_path("../.env");
    let _ = dotenvy::from_path(".env");
    
    let proxy_port = option_env!("VITE_LOCAL_PROXY_PORT").unwrap_or("11338").to_string();
    let api_domain = option_env!("VITE_API_DOMAIN").unwrap_or("").to_string();
    let proxy_args = format!(
        "--proxy-server=127.0.0.1:{} --proxy-bypass-list=127.0.0.1,localhost,tauri.localhost,ipc.localhost,{} --disk-cache-size=1 --media-cache-size=1 --disable-http-cache --disable-features=AutofillServerCommunication,PasswordManager,UserAgentClientHint --disable-save-password-bubble --disable-single-click-autofill --enforce-webrtc-ip-permission-check --force-webrtc-ip-handling-policy=disable_non_proxied_udp --disable-quic --user-agent=\"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36\" --disable-blink-features=AutomationControlled",
        proxy_port, api_domain
    );

    std::env::set_var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS", proxy_args);

    let tunnel_manager = TunnelManager::new();
    let tab_manager = TabManager::new();
    
    // Запускаем глобальный прокси-сервер на старте
    let port_num = proxy_port.parse::<u16>().unwrap_or(11338);
    let tm_clone = tunnel_manager.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(e) = tm_clone.init_global_proxy(port_num).await {
            log::error!("Не удалось запустить локальный прокси: {}", e);
        }
    });

    tauri::Builder::default()
        .manage(tunnel_manager)
        .manage(tab_manager)
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_device_id,
            start_tunnel,
            stop_tunnel,
            get_tunnel_status,
            native_clear_browsing_data,
            native_navigate_tab,
            native_switch_tab,
            native_close_tab,
            native_go_back,
            native_go_forward,
            native_reload,
            native_close_all_tabs,
            native_update_top_bar_height,
            native_minimize_window,
            native_maximize_window,
            native_close_window,
            native_close_window_by_label,
            native_start_dragging,
            native_title_changed,
            native_favicon_changed,
            native_set_window_title,
            native_context_menu,
            native_url_changed,
            native_set_zoom,
            native_download_started,
            native_open_path,
            native_show_in_folder,
            native_get_downloads_dir,
            self_destruct,
            force_exit,
        ])
        .on_window_event(|window, event| {
            match event {
                tauri::WindowEvent::CloseRequested { .. } => {
                    // Let Javascript intercept the close event to send the CLOSE network log
                    // std::process::exit(0);
                }
                tauri::WindowEvent::Moved(_) | tauri::WindowEvent::Resized(_) => {
                    let app = window.app_handle();
                    let win = app.get_window("main").or_else(|| app.windows().into_values().next());
                    if let Some(main_win) = win {
                        let tab_mgr = app.state::<TabManager>();
                        tab_mgr.sync_main_window_geometry(&main_win);
                    }
                }
                _ => {}
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}









