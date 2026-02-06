//! 配置文件管理
//!
//! 负责加载和解析 TOML 格式的配置文件，支持从配置文件和命令行参数合并配置。

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration as StdDuration;

use crate::cli::Command;
use crate::date_utils;
use crate::error::{AppError, Result};

/// 应用程序配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// 起始日期 (格式: YYYY-MM-DD)
    #[serde(with = "serde_date")]
    pub start_date: NaiveDate,

    /// 基础 URL，支持占位符：{year}、{month}、{day}（月份和日期支持 `:02` 格式化为两位）
    pub base_url: String,

    /// 输出目录
    pub output_dir: String,

    /// 文件名格式，支持占位符：{yyyy}、{yy}、{mm}、{dd}
    pub filename_format: String,

    /// 最大并发数（仅对 run 命令有效）
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent: usize,

    /// HTTP 请求时使用的 User-Agent
    #[serde(default = "default_user_agent")]
    pub user_agent: String,

    /// 下载超时时间（秒）
    #[serde(default = "default_timeout")]
    pub timeout: u64,

    /// 最大重试次数
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,

    /// 重试基础延迟（毫秒）
    #[serde(default = "default_retry_delay")]
    pub retry_delay_ms: u64,
}

/// 用于 serde 的日期序列化/反序列化模块
mod serde_date {
    use super::*;
    use chrono::NaiveDate;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(date: &NaiveDate, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = date.format("%Y-%m-%d").to_string();
        s.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<NaiveDate, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        date_utils::parse_date(&s).map_err(serde::de::Error::custom)
    }
}

/// 默认最大并发数
fn default_max_concurrent() -> usize {
    3
}

/// 默认 User-Agent
fn default_user_agent() -> String {
    "Mozilla/5.0".to_string()
}

/// 默认超时时间（秒）
fn default_timeout() -> u64 {
    30
}

/// 默认最大重试次数
fn default_max_retries() -> u32 {
    3
}

/// 默认重试延迟（毫秒）
fn default_retry_delay() -> u64 {
    1000
}

impl Config {
    /// 从 TOML 文件加载配置
    pub fn from_file(path: &Path) -> Result<Self> {
        tracing::info!("加载配置文件: {}", path.display());

        let content = std::fs::read_to_string(path).map_err(|e| {
            AppError::config_error(path, format!("无法读取配置文件: {}", e))
        })?;

        let config: Config = toml::from_str(&content).map_err(|e| {
            AppError::config_error(path, format!("TOML 解析失败: {}", e))
        })?;

        tracing::debug!("配置加载成功: {:?}", config);
        Ok(config)
    }

    /// 获取重试配置
    pub fn retry_config(&self) -> crate::downloader::RetryConfig {
        crate::downloader::RetryConfig {
            max_retries: self.max_retries,
            base_delay_ms: self.retry_delay_ms,
            max_delay_ms: 30000, // 最大等待 30 秒
            enabled: self.max_retries > 0,
        }
    }

    /// 合并命令行参数的默认值
    pub fn merge_cli_defaults(&self, command: &Command) -> ConfigWithDefaults {
        match command {
            Command::Run {
                start_date,
                end_date,
                overwrite,
                download_only,
            } => ConfigWithDefaults {
                start_date_override: start_date.clone(),
                end_date: end_date.clone(),
                overwrite: *overwrite,
                download_only: *download_only,
                metadata_only: false,
            },
            Command::Process {
                overwrite,
                metadata_only,
                ..
            } => ConfigWithDefaults {
                start_date_override: None,
                end_date: None,
                overwrite: *overwrite,
                download_only: false,
                metadata_only: *metadata_only,
            },
            Command::Config { .. } => ConfigWithDefaults {
                start_date_override: None,
                end_date: None,
                overwrite: false,
                download_only: false,
                metadata_only: false,
            },
        }
    }

    /// 获取有效的起始日期
    pub fn get_effective_start_date(&self, override_date: &Option<String>) -> Result<NaiveDate> {
        if let Some(date_str) = override_date {
            date_utils::parse_date(date_str)
        } else {
            Ok(self.start_date)
        }
    }

    /// 获取有效的结束日期
    pub fn get_effective_end_date(
        &self,
        override_date: &Option<String>,
    ) -> Result<Option<NaiveDate>> {
        override_date
            .as_ref()
            .map(|d| date_utils::parse_date(d))
            .transpose()
    }

    /// 获取超时时长
    pub fn timeout_duration(&self) -> StdDuration {
        StdDuration::from_secs(self.timeout)
    }

    /// 应用环境变量和用户特定配置
    pub fn apply_env_overrides(self) -> Self {
        // 从环境变量读取敏感配置
        let mut config = self;

        if let Ok(agent) = std::env::var("CALENDAR_USER_AGENT") {
            config.user_agent = agent;
            tracing::debug!("从环境变量覆盖 User-Agent");
        }

        if let Ok(timeout) = std::env::var("CALENDAR_TIMEOUT") {
            if let Ok(secs) = timeout.parse::<u64>() {
                config.timeout = secs;
                tracing::debug!("从环境变量覆盖超时时间: {} 秒", secs);
            }
        }

        config
    }

    /// 保存配置到文件
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        tracing::info!("保存配置文件: {}", path.display());

        let toml_content = toml::to_string_pretty(self).map_err(|e| {
            AppError::config_error(path, format!("TOML 序列化失败: {}", e))
        })?;

        std::fs::write(path, toml_content).map_err(|e| {
            AppError::config_error(path, format!("写入配置文件失败: {}", e))
        })?;

        tracing::debug!("配置文件保存成功: {}", path.display());
        Ok(())
    }

    /// 更新起始日期并保存到文件
    pub fn update_start_date(&mut self, new_date: NaiveDate, path: &Path) -> Result<()> {
        tracing::info!("更新起始日期: {} -> {}", self.start_date, new_date);
        self.start_date = new_date;
        self.save_to_file(path)?;
        Ok(())
    }
}

/// 带有命令行参数默认值的配置
#[derive(Debug, Clone)]
pub struct ConfigWithDefaults {
    pub start_date_override: Option<String>,
    pub end_date: Option<String>,
    pub overwrite: bool,
    pub download_only: bool,
    pub metadata_only: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Cli;
    use chrono::Datelike;
    use clap::Parser;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::tempdir;

    fn create_test_config(contents: &str) -> PathBuf {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.toml");
        fs::write(&config_path, contents).unwrap();
        config_path
    }

    #[test]
    fn test_parse_config() {
        let contents = r#"
start_date = "2024-01-01"
base_url = "https://example.com/images/{year}/{month:02}/{day:02}.jpg"
output_dir = "./images"
filename_format = "{yyyy}{mm}{dd}.jpg"
max_concurrent = 5
user_agent = "TestAgent/1.0"
timeout = 60
"#;
        let config_path = create_test_config(contents);
        let config = Config::from_file(&config_path).unwrap();

        assert_eq!(config.start_date.year(), 2024);
        assert_eq!(config.max_concurrent, 5);
        assert_eq!(config.user_agent, "TestAgent/1.0");
        assert_eq!(config.timeout, 60);
    }

    #[test]
    fn test_default_values() {
        let contents = r#"
start_date = "2024-01-01"
base_url = "https://example.com/images/{year}/{month:02}/{day:02}.jpg"
output_dir = "./images"
filename_format = "{yyyy}{mm}{dd}.jpg"
"#;
        let config_path = create_test_config(contents);
        let config = Config::from_file(&config_path).unwrap();

        assert_eq!(config.max_concurrent, 3);
        assert_eq!(config.user_agent, "Mozilla/5.0");
        assert_eq!(config.timeout, 30);
    }

    #[test]
    fn test_invalid_date_format() {
        let contents = r#"
start_date = "invalid-date"
base_url = "https://example.com/images/{year}/{month:02}/{day:02}.jpg"
output_dir = "./images"
filename_format = "{yyyy}{mm}{dd}.jpg"
"#;
        let config_path = create_test_config(contents);
        let result = Config::from_file(&config_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_required_field() {
        let contents = r#"
base_url = "https://example.com/images/{year}/{month:02}/{day:02}.jpg"
output_dir = "./images"
"#;
        let config_path = create_test_config(contents);
        let result = Config::from_file(&config_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_merge_cli_defaults() {
        let cli = Cli::try_parse_from([
            "calendar",
            "run",
            "--start-date",
            "2024-06-01",
            "--end-date",
            "2024-06-30",
            "--overwrite",
        ])
        .unwrap();

        let contents = r#"
start_date = "2024-01-01"
base_url = "https://example.com/images/{year}/{month:02}/{day:02}.jpg"
output_dir = "./images"
filename_format = "{yyyy}{mm}{dd}.jpg"
"#;
        let config_path = create_test_config(contents);
        let config = Config::from_file(&config_path).unwrap();

        let defaults = config.merge_cli_defaults(&cli.command);

        assert_eq!(
            defaults.start_date_override,
            Some("2024-06-01".to_string())
        );
        assert_eq!(defaults.end_date, Some("2024-06-30".to_string()));
        assert!(defaults.overwrite);
    }

    #[test]
    fn test_apply_env_overrides() {
        std::env::set_var("CALENDAR_USER_AGENT", "EnvAgent/2.0");
        std::env::set_var("CALENDAR_TIMEOUT", "120");

        let contents = r#"
start_date = "2024-01-01"
base_url = "https://example.com/images/{year}/{month:02}/{day:02}.jpg"
output_dir = "./images"
filename_format = "{yyyy}{mm}{dd}.jpg"
max_concurrent = 3
user_agent = "OriginalAgent/1.0"
timeout = 30
"#;
        let config_path = create_test_config(contents);
        let config = Config::from_file(&config_path).unwrap();
        let config = config.apply_env_overrides();

        assert_eq!(config.user_agent, "EnvAgent/2.0");
        assert_eq!(config.timeout, 120);
        assert_eq!(config.max_concurrent, 3); // 保持原值

        std::env::remove_var("CALENDAR_USER_AGENT");
        std::env::remove_var("CALENDAR_TIMEOUT");
    }
}
