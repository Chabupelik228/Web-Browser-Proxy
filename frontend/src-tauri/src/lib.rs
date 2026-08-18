// frontend/src-tauri/src/lib.rs
pub mod browser;
pub mod commands;
pub mod security;
pub mod tunnel;

use browser::TabManager;
use commands::*;
use tauri::Manager;
use tunnel::TunnelManager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Загружаем переменные окружения
    dotenvy::from_path("../.env").expect("FATAL: .env file is missing in frontend/");
    
    let proxy_port = std::env::var("VITE_LOCAL_PROXY_PORT").expect("FATAL: VITE_LOCAL_PROXY_PORT is not set in .env");
    let api_domain = std::env::var("VITE_API_DOMAIN").expect("FATAL: VITE_API_DOMAIN is not set in .env");

    let proxy_args = format!(
        "--proxy-server=http://127.0.0.1:{} --proxy-bypass-list=127.0.0.1,localhost,tauri.localhost,{} --disable-features=AutofillServerCommunication,PasswordManager --disable-save-password-bubble --disable-single-click-autofill",
        proxy_port, api_domain
    );

    // Единые аргументы для ВСЕХ WebView2 инстансов
    std::env::set_var(
        "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS",
        proxy_args,
    );

    let tunnel_manager = TunnelManager::new();
    let tab_manager = TabManager::new();

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
            encrypt_session_data,
            decrypt_session_data,
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
            native_context_menu,
            native_url_changed,
            native_set_zoom,
            native_download_started,
            native_open_path,
            native_show_in_folder,
            native_get_downloads_dir,
        ])
        .on_window_event(|window, event| {
            match event {
                tauri::WindowEvent::CloseRequested { .. } => {
                    std::process::exit(0);
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
