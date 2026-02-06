// 公共类型和错误定义
mod error;

// 模块导出
pub mod cli;
pub mod config;
pub mod downloader;
pub mod exif;
pub mod filename;
pub mod fileops;

// 重新导出常用类型
pub use error::{AppError, Result, RetryableError};

use chrono::{NaiveDate, Utc};
use std::path::{Path, PathBuf};

/// 下载统计信息
#[derive(Debug, Default, Clone)]
pub struct DownloadStats {
    pub total: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub skipped: usize,
    pub failed_dates: Vec<String>,
    pub succeeded_dates: Vec<String>,
}

impl DownloadStats {
    pub fn new(total: usize) -> Self {
        Self {
            total,
            ..Default::default()
        }
    }

    pub fn record_success(&mut self) {
        self.succeeded += 1;
    }

    pub fn record_success_with_date(&mut self, date: &str) {
        self.succeeded += 1;
        self.succeeded_dates.push(date.to_string());
    }

    pub fn record_failure(&mut self, date: &str) {
        self.failed += 1;
        self.failed_dates.push(date.to_string());
    }

    pub fn record_skip(&mut self) {
        self.skipped += 1;
    }

    pub fn success_rate(&self) -> f64 {
        if self.total == 0 {
            return 0.0;
        }
        (self.succeeded as f64 / self.total as f64) * 100.0
    }

    /// 获取最新成功下载的日期
    pub fn latest_success_date(&self) -> Option<NaiveDate> {
        if self.succeeded_dates.is_empty() {
            return None;
        }
        // 找出最大的日期
        self.succeeded_dates
            .iter()
            .filter_map(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
            .max()
    }
}

/// 文件处理结果
#[derive(Debug, Clone)]
pub enum ProcessResult {
    Downloaded(PathBuf),
    AlreadyExists(PathBuf),
    Failed(String),
}

impl ProcessResult {
    pub fn is_success(&self) -> bool {
        matches!(self, ProcessResult::Downloaded(_) | ProcessResult::AlreadyExists(_))
    }

    pub fn path(&self) -> Option<&PathBuf> {
        match self {
            ProcessResult::Downloaded(p) | ProcessResult::AlreadyExists(p) => Some(p),
            ProcessResult::Failed(_) => None,
        }
    }
}

/// 日期处理辅助函数
pub mod date_utils {
    use super::*;
    use chrono::NaiveDate;

    /// 解析日期字符串 (格式: YYYY-MM-DD)
    pub fn parse_date(date_str: &str) -> Result<NaiveDate> {
        NaiveDate::parse_from_str(date_str, "%Y-%m-%d").map_err(|e| AppError::InvalidDate {
            input: date_str.to_string(),
            details: e.to_string(),
        })
    }

    /// 格式化日期为 YYYY-MM-DD
    pub fn format_date(date: &NaiveDate) -> String {
        date.format("%Y-%m-%d").to_string()
    }

    /// 获取当前日期
    pub fn today() -> NaiveDate {
        Utc::now().date_naive()
    }

    /// 生成交间范围的所有日期
    pub fn date_range(start: NaiveDate, end: NaiveDate) -> Vec<NaiveDate> {
        let mut dates = Vec::new();
        let mut current = start;
        while current <= end {
            dates.push(current);
            current = current.succ_opt().unwrap();
        }
        dates
    }
}

/// 构建年份目录路径
pub fn build_year_path(base_dir: &Path, year: i32) -> PathBuf {
    let year_dir = base_dir.join(year.to_string());
    std::fs::create_dir_all(&year_dir).unwrap_or_else(|e| {
        tracing::warn!("Failed to create directory {:?}: {}", year_dir, e);
    });
    year_dir
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    #[test]
    fn test_parse_date_valid() {
        let result = date_utils::parse_date("2024-06-15");
        assert!(result.is_ok());
        let date = result.unwrap();
        assert_eq!(date.year(), 2024);
        assert_eq!(date.month(), 6);
        assert_eq!(date.day(), 15);
    }

    #[test]
    fn test_parse_date_invalid() {
        let result = date_utils::parse_date("2024-13-01");
        assert!(result.is_err());
    }

    #[test]
    fn test_date_range() {
        let start = date_utils::parse_date("2024-06-01").unwrap();
        let end = date_utils::parse_date("2024-06-03").unwrap();
        let dates = date_utils::date_range(start, end);
        assert_eq!(dates.len(), 3);
        assert_eq!(dates[0].day(), 1);
        assert_eq!(dates[1].day(), 2);
        assert_eq!(dates[2].day(), 3);
    }

    #[test]
    fn test_download_stats() {
        let mut stats = DownloadStats::new(5);
        stats.record_success();
        stats.record_success();
        stats.record_failure("2024-06-01");
        stats.record_skip();

        assert_eq!(stats.total, 5);
        assert_eq!(stats.succeeded, 2);
        assert_eq!(stats.failed, 1);
        assert_eq!(stats.skipped, 1);
        assert_eq!(stats.success_rate(), 40.0);
    }
}
