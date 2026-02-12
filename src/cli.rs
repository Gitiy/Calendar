//! 命令行参数定义
//!
//! 使用 `clap` 库定义和解析命令行参数。

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// 图片批量下载与处理程序
#[derive(Parser, Debug)]
#[command(
    name = "calendar",
    author = "Calendar Downloader",
    version = "0.1.0",
    about = "从指定起始日期开始批量下载图片，并修改 EXIF 信息和文件属性",
    long_about = "一个批量的图片下载工具，支持从指定日期开始下载每日图片，\
                  自动修改照片的 EXIF 信息和文件时间戳。"
)]
pub struct Cli {
    /// 配置文件路径 (默认: config.toml)
    #[arg(short = 'c', long, global = true, default_value = "config.toml")]
    pub config: PathBuf,

    /// 日志级别 (trace, debug, info, warn, error) (默认: info)
    #[arg(short = 'l', long, global = true, default_value = "info")]
    pub log_level: String,

    /// 子命令 (默认: run)
    #[command(subcommand)]
    pub command: Option<Command>,
}

/// 子命令
#[derive(Subcommand, Debug)]
pub enum Command {
    /// 从起始日期批量下载到当前或指定结束日期
    Run {
        /// 起始日期 (格式: YYYY-MM-DD)
        ///
        /// 如果不指定则使用配置文件中的起始日期
        #[arg(long)]
        start_date: Option<String>,

        /// 结束日期 (格式: YYYY-MM-DD)
        ///
        /// 如果不指定则使用当前日期
        #[arg(long)]
        end_date: Option<String>,

        /// 覆盖已存在的文件
        ///
        /// 默认情况下，已存在的文件会跳过下载，但仍然执行 EXIF 和文件属性更新
        #[arg(long, default_value_t = false)]
        overwrite: bool,

        /// 仅下载，不修改 EXIF 和文件属性
        ///
        /// 适用于只需要下载文件的场景
        #[arg(long, default_value_t = false)]
        download_only: bool,
    },

    /// 处理指定日期的单个或多个文件
    Process {
        /// 单个日期 (格式: YYYY-MM-DD)
        ///
        /// 如果需要处理多个日期，建议使用 --dates 参数
        #[arg(long)]
        date: Option<String>,

        /// 多个日期，使用逗号分隔或多次指定 (格式: YYYY-MM-DD,YYYY-MM-DD)
        ///
        /// 示例: --dates 2024-06-15,2024-06-20,2024-06-25
        /// 或: --dates 2024-06-15 --dates 2024-06-20
        #[arg(long, value_delimiter = ',', required_unless_present = "date")]
        dates: Option<Vec<String>>,

        /// 覆盖已存在的文件
        #[arg(long, default_value_t = false)]
        overwrite: bool,

        /// 仅修改 EXIF 和文件属性，不下载
        ///
        /// 适用于文件已存在但需要更新元数据的场景
        #[arg(long, default_value_t = false)]
        metadata_only: bool,
    },

    /// 配置文件验证
    Config {
        /// 验证配置文件是否正确
        #[arg(long, default_value_t = false)]
        validate: bool,
    },
}

impl Command {
    /// 获取日期列表
    pub fn get_dates(&self) -> Result<Vec<String>, AppError> {
        match self {
            Command::Run { .. } => {
                // run 命令的日期由 main.rs 根据 start_date 和 end_date 生成
                Ok(vec![])
            }
            Command::Config { .. } => {
                // config 命令不需要日期
                Ok(vec![])
            }
            Command::Process { date, dates, .. } => {
                let mut date_list = vec![];

                if let Some(d) = date {
                    date_list.push(d.clone());
                }

                if let Some(d) = dates {
                    date_list.extend(d.clone());
                }

                if date_list.is_empty() {
                    return Err(AppError::argument_error(
                        "必须指定 --date 或 --dates 参数",
                    ));
                }

                // 去重并验证日期格式
                date_list.sort();
                date_list.dedup();

                for d in &date_list {
                    // 验证日期格式
                    chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").map_err(|e| {
                        AppError::InvalidDate {
                            input: d.clone(),
                            details: e.to_string(),
                        }
                    })?;
                }

                Ok(date_list)
            }
        }
    }
}

use crate::error::{AppError, Result};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_default_values() {
        let cli = Cli::try_parse_from(["calendar"]).unwrap();
        assert_eq!(cli.config, PathBuf::from("config.toml"));
        assert_eq!(cli.log_level, "info");
    }

    #[test]
    fn test_cli_config_option() {
        let cli = Cli::try_parse_from(["calendar", "-c", "my-config.toml"]).unwrap();
        assert_eq!(cli.config, PathBuf::from("my-config.toml"));
    }

    #[test]
    fn test_cli_run_command() {
        let cli = Cli::try_parse_from(["calendar", "run"]).unwrap();
        assert!(matches!(cli.command, Command::Run { .. }));
    }

    #[test]
    fn test_cli_run_with_dates() {
        let cli = Cli::try_parse_from([
            "calendar",
            "run",
            "--start-date",
            "2024-06-01",
            "--end-date",
            "2024-06-15",
        ])
        .unwrap();
        if let Command::Run {
            start_date,
            end_date,
            ..
        } = cli.command
        {
            assert_eq!(start_date, Some("2024-06-01".to_string()));
            assert_eq!(end_date, Some("2024-06-15".to_string()));
        } else {
            panic!("Expected Run command");
        }
    }

    #[test]
    fn test_cli_process_command_single_date() {
        let cli = Cli::try_parse_from(["calendar", "process", "--date", "2024-06-15"]).unwrap();
        assert!(matches!(cli.command, Command::Process { .. }));
        let dates = cli.command.get_dates().unwrap();
        assert_eq!(dates, vec!["2024-06-15"]);
    }

    #[test]
    fn test_cli_process_command_multiple_dates() {
        let cli = Cli::try_parse_from([
            "calendar",
            "process",
            "--dates",
            "2024-06-15,2024-06-20,2024-06-25",
        ])
        .unwrap();
        let dates = cli.command.get_dates().unwrap();
        assert_eq!(dates.len(), 3);
        assert!(dates.contains(&"2024-06-15".to_string()));
        assert!(dates.contains(&"2024-06-20".to_string()));
        assert!(dates.contains(&"2024-06-25".to_string()));
    }

    #[test]
    fn test_cli_process_command_multiple_specifications() {
        let cli = Cli::try_parse_from([
            "calendar",
            "process",
            "--dates",
            "2024-06-15",
            "--dates",
            "2024-06-20",
        ])
        .unwrap();
        let dates = cli.command.get_dates().unwrap();
        assert_eq!(dates.len(), 2);
        assert!(dates.contains(&"2024-06-15".to_string()));
        assert!(dates.contains(&"2024-06-20".to_string()));
    }

    #[test]
    fn test_cli_process_requires_date_or_dates() {
        let result = Cli::try_parse_from(["calendar", "process"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_cli_invalid_date_format() {
        let cli = Cli::try_parse_from(["calendar", "process", "--date", "invalid"]).unwrap();
        let result = cli.command.get_dates();
        assert!(result.is_err());
    }

    #[test]
    fn test_cli_overwrite_flag() {
        let cli = Cli::try_parse_from(["calendar", "run", "--overwrite"]).unwrap();
        if let Command::Run { overwrite, .. } = cli.command {
            assert!(overwrite);
        } else {
            panic!("Expected Run command");
        }
    }

    #[test]
    fn test_cli_log_level() {
        let cli = Cli::try_parse_from(["calendar", "-l", "debug", "run"]).unwrap();
        assert_eq!(cli.log_level, "debug");
    }
}
