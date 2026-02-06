//! 应用程序错误类型定义
//!
//! 使用 `thiserror` 为应用程序定义结构化的错误类型，确保所有错误都能被正确处理。

use std::path::PathBuf;
use thiserror::Error;
use reqwest::header::InvalidHeaderValue;

/// 应用程序 Result 类型
pub type Result<T = (), E = AppError> = std::result::Result<T, E>;

/// 可重试的错误类型
#[derive(Debug, Clone, PartialEq)]
pub enum RetryableError {
    /// 连接超时
    ConnectionTimeout,
    /// DNS 解析失败
    DnsFailed,
    /// 连接被拒绝
    ConnectionRefused,
    /// 服务器不可达
    ConnectionFailed,
    /// 读写超时
    ReadTimeout,
    /// 写入超时
    WriteTimeout,
    /// TLS 握手失败
    TlsFailed,
    /// HTTP 429 Too Many Requests
    TooManyRequests,
    /// 服务器内部错误 (5xx)
    ServerError(reqwest::StatusCode),
    ///  декоди失败（可能是临时数据问题）
    DecodingFailed(String),
    /// 未知但可能可重试的错误
    Unknown(String),
}

impl RetryableError {
    /// 判断错误是否应该重试
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::ConnectionTimeout | Self::DnsFailed | Self::ConnectionRefused
            | Self::ConnectionFailed | Self::ReadTimeout | Self::WriteTimeout | Self::TlsFailed
            | Self::TooManyRequests | Self::ServerError(_) | Self::DecodingFailed(_) => true,
            Self::Unknown(_) => false,
        }
    }

    /// 从错误消息推断可重试错误类型
    pub fn from_error_message(err_msg: &str, status: Option<reqwest::StatusCode>) -> Self {
        // 首先检查 HTTP 状态码
        if let Some(status) = status {
            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                return Self::TooManyRequests;
            }
            if status.is_server_error() {
                return Self::ServerError(status);
            }
        }

        let err_lower = err_msg.to_lowercase();

        if err_lower.contains("connection timed out") {
            Self::ConnectionTimeout
        } else if err_lower.contains("timed out") {
            Self::ReadTimeout
        } else if err_lower.contains("dns")
            || err_lower.contains("name or service not known")
            || err_lower.contains("no address associated with name")
        {
            Self::DnsFailed
        } else if err_lower.contains("connection refused") {
            Self::ConnectionRefused
        } else if err_lower.contains("network is unreachable")
            || err_lower.contains("connection reset")
            || err_lower.contains("broken pipe")
            || err_lower.contains("connection closed")
        {
            Self::ConnectionFailed
        } else if err_lower.contains("tls") || err_lower.contains("ssl")
            || err_lower.contains("certificate")
        {
            Self::TlsFailed
        } else if err_lower.contains("decode")
            || err_lower.contains("utf")
            || err_lower.contains("invalid utf")
            || err_lower.contains("stream")
        {
            Self::DecodingFailed(err_msg.to_string())
        } else {
            Self::Unknown(err_msg.to_string())
        }
    }

    /// 获取建议的等待时间（毫秒）
    pub fn suggested_delay_ms(&self) -> u64 {
        match self {
            Self::TooManyRequests => 5000, // 429 建议等待 5 秒
            Self::ServerError(_) => 2000, // 5xx 建议等待 2 秒
            Self::ConnectionTimeout => 1000,
            Self::DnsFailed => 2000,
            Self::ConnectionRefused => 2000,
            Self::ConnectionFailed => 2000,
            Self::ReadTimeout => 1000,
            Self::WriteTimeout => 1000,
            Self::TlsFailed => 3000,
            Self::DecodingFailed(_) => 1000,
            Self::Unknown(_) => 0,
        }
    }
}

/// 应用程序错误类型
#[derive(Error, Debug)]
pub enum AppError {
    /// 配置文件加载错误
    #[error("配置文件错误: {path}: {details}")]
    ConfigError {
        path: PathBuf,
        details: String,
    },

    /// TOML 解析错误
    #[error("TOML 解析错误: {0}")]
    TomlError(#[from] toml::de::Error),

    /// 日期解析错误
    #[error("无效的日期格式 '{input}': {details}")]
    InvalidDate {
        input: String,
        details: String,
    },

    /// 网络请求错误
    #[error("网络请求错误: {url} - {details}")]
    NetworkError {
        url: String,
        details: String,
    },

    /// HTTP 状态码错误
    #[error("HTTP 错误: {url} 返回状态码 {status}")]
    HttpError {
        url: String,
        status: reqwest::StatusCode,
    },

    /// 文件操作错误
    #[error("文件操作错误: {path} - {details}")]
    FileError {
        path: PathBuf,
        details: String,
    },

    /// IO 错误
    #[error("IO 错误: {0}")]
    IoError(#[from] std::io::Error),

    /// EXIF 修改错误
    #[error("EXIF 修改错误: {path} - {details}")]
    ExifError {
        path: PathBuf,
        details: String,
    },

    /// 文件名格式错误
    #[error("文件名格式错误: {format} - {details}")]
    FilenameFormatError {
        format: String,
        details: String,
    },

    /// 正则表达式错误
    #[error("正则表达式错误: {0}")]
    RegexError(#[from] regex::Error),

    /// URL 构建错误
    #[error("URL 构建错误: {template} - {details}")]
    UrlBuildError {
        template: String,
        details: String,
    },

    /// 参数错误
    #[error("参数错误: {0}")]
    ArgumentError(String),

    /// HTTP 头部错误
    #[error("HTTP 头部错误: {0}")]
    HeaderError(String),
}

impl From<InvalidHeaderValue> for AppError {
    fn from(err: InvalidHeaderValue) -> Self {
        Self::HeaderError(err.to_string())
    }
}

impl AppError {
    /// 创建配置文件错误
    pub fn config_error(path: impl Into<PathBuf>, details: impl Into<String>) -> Self {
        Self::ConfigError {
            path: path.into(),
            details: details.into(),
        }
    }

    /// 创建网络请求错误
    pub fn network_error(url: impl Into<String>, details: impl Into<String>) -> Self {
        Self::NetworkError {
            url: url.into(),
            details: details.into(),
        }
    }

    /// 创建 HTTP 错误
    pub fn http_error(url: impl Into<String>, status: reqwest::StatusCode) -> Self {
        Self::HttpError {
            url: url.into(),
            status,
        }
    }

    /// 创建文件操作错误
    pub fn file_error(path: impl Into<PathBuf>, details: impl Into<String>) -> Self {
        Self::FileError {
            path: path.into(),
            details: details.into(),
        }
    }

    /// 创建 EXIF 错误
    pub fn exif_error(path: impl Into<PathBuf>, details: impl Into<String>) -> Self {
        Self::ExifError {
            path: path.into(),
            details: details.into(),
        }
    }

    /// 创建 URL 构建错误
    pub fn url_build_error(template: impl Into<String>, details: impl Into<String>) -> Self {
        Self::UrlBuildError {
            template: template.into(),
            details: details.into(),
        }
    }

    /// 创建参数错误
    pub fn argument_error(msg: impl Into<String>) -> Self {
        Self::ArgumentError(msg.into())
    }
}

impl From<reqwest::Error> for AppError {
    fn from(err: reqwest::Error) -> Self {
        let url = err.url().map(|u| u.to_string()).unwrap_or_else(|| "<unknown>".to_string());
        if let Some(status) = err.status() {
            Self::HttpError { url, status }
        } else {
            Self::NetworkError {
                url,
                details: err.to_string(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = AppError::config_error("config.toml", "missing field");
        assert!(err.to_string().contains("config.toml"));
        assert!(err.to_string().contains("missing field"));
    }

    #[test]
    fn test_network_error() {
        let err = AppError::network_error("https://example.com", "connection refused");
        assert!(matches!(err, AppError::NetworkError { .. }));
    }
}
