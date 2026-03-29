use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::constants::{CONFIG_FILE_NAME, DEFAULT_RPC_PORT, SESSION_FILE_NAME};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Aria2Config {
    pub rpc_port: u16,
    pub rpc_secret: String,
    pub download_dir: String,
    pub max_concurrent_downloads: u32,
}

impl Default for Aria2Config {
    fn default() -> Self {
        let download_dir = dirs_default_download();
        Self {
            rpc_port: DEFAULT_RPC_PORT,
            rpc_secret: String::new(),
            download_dir,
            max_concurrent_downloads: 5,
        }
    }
}

impl Aria2Config {
    /// Load config from disk, or create a new default config if none exists.
    pub fn load_or_create(app_data_dir: &Path) -> Result<Self, String> {
        let config_path = app_data_dir.join(CONFIG_FILE_NAME);

        if config_path.exists() {
            let content = fs::read_to_string(&config_path)
                .map_err(|e| format!("Failed to read config: {}", e))?;
            let config: Aria2Config = serde_json::from_str(&content)
                .map_err(|e| format!("Failed to parse config: {}", e))?;
            Ok(config)
        } else {
            let config = Aria2Config::default();
            config.save(app_data_dir)?;
            Ok(config)
        }
    }

    /// Save config to disk.
    pub fn save(&self, app_data_dir: &Path) -> Result<(), String> {
        fs::create_dir_all(app_data_dir)
            .map_err(|e| format!("Failed to create config dir: {}", e))?;

        let config_path = app_data_dir.join(CONFIG_FILE_NAME);
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;
        fs::write(&config_path, content)
            .map_err(|e| format!("Failed to write config: {}", e))?;
        Ok(())
    }

    /// Ensure the aria2 session file exists (aria2 requires it for --save-session).
    pub fn ensure_session_file(app_data_dir: &Path) -> Result<PathBuf, String> {
        let session_path = app_data_dir.join(SESSION_FILE_NAME);
        if !session_path.exists() {
            fs::write(&session_path, "")
                .map_err(|e| format!("Failed to create session file: {}", e))?;
        }
        Ok(session_path)
    }

    /// Convert config to aria2c command-line arguments.
    pub fn to_aria2_args(&self, session_path: &Path) -> Vec<String> {
        let mut args = vec![
            "--enable-rpc=true".to_string(),
            format!("--rpc-listen-port={}", self.rpc_port),
            "--rpc-listen-all=false".to_string(),
            "--rpc-allow-origin-all=true".to_string(),
            format!("--dir={}", self.download_dir),
            format!("--max-concurrent-downloads={}", self.max_concurrent_downloads),
            format!("--save-session={}", session_path.display()),
            format!("--input-file={}", session_path.display()),
            "--save-session-interval=30".to_string(),
            "--auto-save-interval=30".to_string(),
            "--continue=true".to_string(),
            "--max-connection-per-server=16".to_string(),
            "--min-split-size=1M".to_string(),
            "--split=16".to_string(),
            "--check-certificate=true".to_string(),
        ];

        // Only pass --rpc-secret if user has configured one
        if !self.rpc_secret.is_empty() {
            args.push(format!("--rpc-secret={}", self.rpc_secret));
        }

        args
    }
}

/// Get the default download directory for the current platform.
fn dirs_default_download() -> String {
    if let Some(home) = home_dir() {
        let downloads = home.join("Downloads");
        if downloads.exists() {
            return downloads.to_string_lossy().to_string();
        }
        return home.to_string_lossy().to_string();
    }
    ".".to_string()
}

/// Cross-platform home directory detection.
fn home_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var("USERPROFILE").ok().map(PathBuf::from)
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("HOME").ok().map(PathBuf::from)
    }
}
