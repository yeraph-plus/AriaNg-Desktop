mod aria2;
mod commands;
mod constants;
mod setup;
mod tray;

use log::info;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    tauri::Builder::default()
        // Single instance plugin - must be registered first
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // When a second instance is launched, focus the existing window
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_dialog::init())
        // Setup hook - initialize aria2 and configure AriaNg
        .setup(|app| {
            info!("AriaNg App starting...");

            // Initialize aria2 manager and start sidecar
            if let Err(e) = setup::initialize(app) {
                log::error!("Setup failed: {}", e);
            }

            // Build system tray
            if let Err(e) = tray::build_tray(app.handle()) {
                log::error!("Failed to build tray: {}", e);
            }

            Ok(())
        })
        // Register Tauri commands
        .invoke_handler(tauri::generate_handler![
            commands::get_aria2_config,
            commands::get_aria2_status,
            commands::restart_aria2,
            commands::sync_window_title,
            commands::save_ariang_options,
        ])
        // Handle window close - minimize to tray instead of exiting
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // Hide window instead of closing (minimize to tray)
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            // Handle app exit - shutdown aria2 gracefully
            if let tauri::RunEvent::Exit = event {
                info!("Application exiting, shutting down aria2...");
                let manager = app.state::<aria2::manager::Aria2Manager>();
                if let Err(e) = manager.shutdown() {
                    log::error!("Failed to shutdown aria2: {}", e);
                }
            }
        });
}
