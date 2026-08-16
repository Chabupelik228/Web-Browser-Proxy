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
    // Единые аргументы для ВСЕХ WebView2 инстансов (прокси + полное отключение автозаполнения, но БЕЗ incognito, чтобы куки сохранялись локально)
    std::env::set_var(
        "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS",
        "--proxy-server=http://127.0.0.1:11338 --proxy-bypass-list=127.0.0.1,localhost,tauri.localhost,web.chabupelik.su --disable-features=AutofillServerCommunication,PasswordManager --disable-save-password-bubble --disable-single-click-autofill",
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
