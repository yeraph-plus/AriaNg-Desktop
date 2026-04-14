use serde::Serialize;
use tauri::{command, AppHandle, Manager, State, WebviewWindow};

use crate::aria2::manager::Aria2Manager;
use crate::constants::ARIANG_OPTIONS_FILE_NAME;

#[derive(Debug, Serialize)]
pub struct Aria2RpcConfig {
    pub port: u16,
    pub secret: String,
}

#[derive(Debug, Serialize)]
pub struct Aria2Status {
    pub running: bool,
    pub port: u16,
}

/// Get the aria2 RPC connection configuration (port + secret).
/// Called by the frontend to configure AriaNg's connection settings.
#[command]
pub fn get_aria2_config(manager: State<'_, Aria2Manager>) -> Result<Aria2RpcConfig, String> {
    Ok(Aria2RpcConfig {
        port: manager.get_port(),
        secret: manager.get_secret(),
    })
}

/// Get the current status of the aria2 process.
#[command]
pub fn get_aria2_status(manager: State<'_, Aria2Manager>) -> Result<Aria2Status, String> {
    Ok(Aria2Status {
        running: manager.is_running(),
        port: manager.get_port(),
    })
}

/// Restart the aria2 process.
#[command]
pub fn restart_aria2(manager: State<'_, Aria2Manager>) -> Result<(), String> {
    manager.restart()
}

/// Sync the window title with AriaNg's document.title.
#[command]
pub fn sync_window_title(window: WebviewWindow, title: String) -> Result<(), String> {
    window.set_title(&title).map_err(|e| e.to_string())
}

/// Persist AriaNg.Options JSON string to disk.
/// Called periodically by injected JS when localStorage options change.
#[command]
pub fn save_ariang_options(app: AppHandle, json: String) -> Result<(), String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    std::fs::create_dir_all(&app_data_dir)
        .map_err(|e| format!("Failed to create config dir: {}", e))?;

    let path = app_data_dir.join(ARIANG_OPTIONS_FILE_NAME);
    std::fs::write(&path, &json)
        .map_err(|e| format!("Failed to write AriaNg options: {}", e))?;

    log::debug!("AriaNg options persisted to {:?}", path);
    Ok(())
}
