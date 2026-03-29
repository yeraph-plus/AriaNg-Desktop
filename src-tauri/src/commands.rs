use serde::Serialize;
use tauri::{command, State, WebviewWindow};

use crate::aria2::manager::Aria2Manager;

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
