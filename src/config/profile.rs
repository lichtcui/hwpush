use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::cli::CliError;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    #[serde(default)]
    pub push: PushConfig,

    #[serde(default)]
    pub defaults: DefaultsConfig,

    #[serde(default)]
    pub storage: StorageConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PushConfig {
    pub service_url: String,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    #[serde(default = "default_retry")]
    pub retry_count: u32,
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DefaultsConfig {
    #[serde(default = "default_result")]
    pub result: String,
    #[serde(default = "default_source")]
    pub source: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StorageConfig {
    pub history_db_path: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            push: PushConfig::default(),
            defaults: DefaultsConfig::default(),
            storage: StorageConfig::default(),
        }
    }
}

impl Default for PushConfig {
    fn default() -> Self {
        Self {
            service_url: "https://hiboard-claw-drcn.ai.dbankcloud.cn/distribution/message/cloud/claw/msg/upload".into(),
            timeout_secs: 30,
            retry_count: 3,
            dry_run: false,
        }
    }
}

impl Default for DefaultsConfig {
    fn default() -> Self {
        Self {
            result: "任务已完成".into(),
            source: "OpenClaw".into(),
        }
    }
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            history_db_path: Self::default_db_path().to_string_lossy().into(),
        }
    }
}

impl StorageConfig {
    fn default_db_path() -> PathBuf {
        let base = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("~/.local/share"));
        base.join("hwpush").join("history.db")
    }
}

pub fn default_config_path() -> PathBuf {
    let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("~/.config"));
    base.join("hwpush").join("config.toml")
}

pub fn default_template_dir() -> PathBuf {
    let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("~/.config"));
    base.join("hwpush").join("templates")
}

pub fn default_storage_dir() -> PathBuf {
    let base = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("~/.local/share"));
    base.join("hwpush")
}

pub fn load() -> Result<Config, CliError> {
    let path = default_config_path();
    if !path.exists() {
        return Ok(Config::default());
    }
    let content = std::fs::read_to_string(&path)
        .map_err(|e| CliError::Config(format!("读取配置文件失败: {e}")))?;
    let cfg: Config = toml::from_str(&content)
        .map_err(|e| CliError::Config(format!("解析配置文件失败: {e}")))?;
    Ok(cfg)
}

pub fn save(cfg: &Config) -> Result<(), CliError> {
    let path = default_config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| CliError::Config(format!("创建配置目录失败: {e}")))?;
    }
    let content = toml::to_string_pretty(cfg)
        .map_err(|e| CliError::Config(format!("序列化配置失败: {e}")))?;
    std::fs::write(&path, content)
        .map_err(|e| CliError::Config(format!("写入配置文件失败: {e}")))?;
    Ok(())
}

pub fn set_value(cfg: &mut Config, key: &str, value: &str) {
    match key {
        "push.service_url" => cfg.push.service_url = value.into(),
        "push.timeout_secs" => cfg.push.timeout_secs = value.parse().unwrap_or(30),
        "push.retry_count" => cfg.push.retry_count = value.parse().unwrap_or(3),
        "push.dry_run" => cfg.push.dry_run = value == "true",
        "defaults.result" => cfg.defaults.result = value.into(),
        "defaults.source" => cfg.defaults.source = value.into(),
        "storage.history_db_path" => cfg.storage.history_db_path = value.into(),
        _ => eprintln!("未知配置项: {key}"),
    }
}

fn default_timeout() -> u64 {
    30
}

fn default_retry() -> u32 {
    3
}

fn default_result() -> String {
    "任务已完成".into()
}

fn default_source() -> String {
    "OpenClaw".into()
}
