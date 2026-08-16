// frontend/src-tauri/src/lib.rs
pub mod browser;
pub mod commands;
pub mod security;
pub mod tunnel;

use browser::TabManager;
use commands::*;
use tunnel::TunnelManager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Единые аргументы для ВСЕХ WebView2 инстансов (устраняет ошибку 0x8007139F)
    std::env::set_var(
        "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS",
        "--proxy-server=http://127.0.0.1:11338 --proxy-bypass-list=127.0.0.1,localhost,tauri.localhost,web.chabupelik.su",
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
        ])
        .on_window_event(|_window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                std::process::exit(0);
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
