use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::constants::ARIA_NG_CONFIG_FILE_NAME;

/// AriaNg 配置结构
/// 对应 AriaNg 存储在 localStorage 中的 AriaNg.Options 格式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AriaNgConfig {
    /// RPC 主机地址
    #[serde(default = "default_rpc_host")]
    pub rpc_host: String,
    
    /// RPC 端口
    #[serde(default = "default_rpc_port")]
    pub rpc_port: String,
    
    /// RPC 协议 (ws/wss/http/https)
    #[serde(default = "default_protocol")]
    pub protocol: String,
    
    /// RPC 密钥 (secret token)
    #[serde(default)]
    pub rpc_secret: String,
    
    /// 是否使用默认 RPC 设置
    #[serde(default)]
    pub rpc_default: bool,
    
    /// 语言设置
    #[serde(default = "default_language")]
    pub language: String,
    
    /// 主题 (light/dark)
    #[serde(default = "default_theme")]
    pub theme: String,
    
    /// 每秒下载速度
    #[serde(default = "default_download_rate")]
    pub max_download_limit: String,
    
    /// 每秒上传速度
    #[serde(default = "default_upload_rate")]
    pub max_upload_limit: String,
    
    /// 同时下载任务数
    #[serde(default = "default_max_concurrent_downloads")]
    pub max_concurrent_downloads: i32,
    
    /// 单服务器最大连接数
    #[serde(default = "default_max_connection_per_server")]
    pub max_connection_per_server: i32,
    
    /// 最小分片大小
    #[serde(default = "default_min_split_size")]
    pub min_split_size: String,
    
    /// 分片数
    #[serde(default = "default_split")]
    pub split: i32,
    
    /// 其他 AriaNg 选项（用于扩展）
    #[serde(flatten)]
    pub extra_options: HashMap<String, serde_json::Value>,
}

fn default_rpc_host() -> String {
    "127.0.0.1".to_string()
}

fn default_rpc_port() -> String {
    "6800".to_string()
}

fn default_protocol() -> String {
    "ws".to_string()
}

fn default_language() -> String {
    "zh_Hans".to_string()
}

fn default_theme() -> String {
    "light".to_string()
}

fn default_download_rate() -> String {
    "0".to_string()
}

fn default_upload_rate() -> String {
    "0".to_string()
}

fn default_max_concurrent_downloads() -> i32 {
    5
}

fn default_max_connection_per_server() -> i32 {
    16
}

fn default_min_split_size() -> String {
    "20M".to_string()
}

fn default_split() -> i32 {
    16
}

impl Default for AriaNgConfig {
    fn default() -> Self {
        Self {
            rpc_host: default_rpc_host(),
            rpc_port: default_rpc_port(),
            protocol: default_protocol(),
            rpc_secret: String::new(),
            rpc_default: false,
            language: default_language(),
            theme: default_theme(),
            max_download_limit: default_download_rate(),
            max_upload_limit: default_upload_rate(),
            max_concurrent_downloads: default_max_concurrent_downloads(),
            max_connection_per_server: default_max_connection_per_server(),
            min_split_size: default_min_split_size(),
            split: default_split(),
            extra_options: HashMap::new(),
        }
    }
}

impl AriaNgConfig {
    /// 从磁盘加载配置，如果不存在则创建默认配置
    pub fn load_or_create(app_data_dir: &Path) -> Result<Self, String> {
        let config_path = app_data_dir.join(ARIA_NG_CONFIG_FILE_NAME);

        if config_path.exists() {
            let content = fs::read_to_string(&config_path)
                .map_err(|e| format!("Failed to read AriaNg config: {}", e))?;
            let config: AriaNgConfig = serde_json::from_str(&content)
                .map_err(|e| format!("Failed to parse AriaNg config: {}", e))?;
            Ok(config)
        } else {
            let config = AriaNgConfig::default();
            config.save(app_data_dir)?;
            Ok(config)
        }
    }

    /// 保存配置到磁盘
    pub fn save(&self, app_data_dir: &Path) -> Result<(), String> {
        fs::create_dir_all(app_data_dir)
            .map_err(|e| format!("Failed to create config dir: {}", e))?;

        let config_path = app_data_dir.join(ARIA_NG_CONFIG_FILE_NAME);
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize AriaNg config: {}", e))?;
        fs::write(&config_path, content)
            .map_err(|e| format!("Failed to write AriaNg config: {}", e))?;
        Ok(())
    }

    /// 更新 RPC 配置（由应用设置）
    pub fn update_rpc_settings(&mut self, host: &str, port: u16, protocol: &str) {
        self.rpc_host = host.to_string();
        self.rpc_port = port.to_string();
        self.protocol = protocol.to_string();
    }

    /// 转换为 localStorage 存储的 JSON 字符串
    pub fn to_localstorage_json(&self) -> Result<String, String> {
        serde_json::to_string(self)
            .map_err(|e| format!("Failed to serialize to localStorage format: {}", e))
    }

    /// 从 localStorage 的 JSON 字符串解析
    pub fn from_localstorage_json(json: &str) -> Result<Self, String> {
        serde_json::from_str(json)
            .map_err(|e| format!("Failed to parse localStorage config: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AriaNgConfig::default();
        assert_eq!(config.rpc_host, "127.0.0.1");
        assert_eq!(config.rpc_port, "6800");
        assert_eq!(config.protocol, "ws");
    }

    #[test]
    fn test_serialization() {
        let config = AriaNgConfig::default();
        let json = config.to_localstorage_json().unwrap();
        assert!(json.contains("rpc_host"));
        assert!(json.contains("127.0.0.1"));
    }
}
