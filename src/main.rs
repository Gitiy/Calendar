//! 主程序入口
//!
//! 负责解析命令行参数、加载配置、执行下载任务和显示结果。

use chrono::NaiveDate;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use calendar::cli::{Cli, Command};
use calendar::config::Config;
use calendar::date_utils;
use calendar::downloader::Downloader;
use calendar::{AppError, Result};

use clap::Parser;

/// 设置日志记录
fn setup_tracing(log_level: &str) {
    let level_filter = match log_level {
        "trace" => tracing::Level::TRACE,
        "debug" => tracing::Level::DEBUG,
        "info" => tracing::Level::INFO,
        "warn" => tracing::Level::WARN,
        "error" => tracing::Level::ERROR,
        _ => tracing::Level::INFO,
    };

    tracing_subscriber::fmt()
        .with_max_level(level_filter)
        .with_target(false)
        .without_time()
        .init();
}

/// 保存失败下载日期到文件
fn save_failed_downloads(
    failed_dates: &[String],
    output_dir: &Path,
) -> Result<std::path::PathBuf> {
    let log_path = output_dir.join("failed_downloads.txt");

    let mut file = File::create(&log_path)
        .map_err(|e: std::io::Error| AppError::file_error(&log_path, e.to_string()))?;

    for date in failed_dates {
        writeln!(file, "{}", date)
            .map_err(|e| AppError::file_error(&log_path, e.to_string()))?;
    }

    Ok(log_path)
}

/// 执行 run 命令（批量下载）
async fn run_command(
    config_path: &Path,
    config: &Config,
    cli_defaults: calendar::config::ConfigWithDefaults,
) -> Result<()> {
    tracing::info!("执行 run 命令");

    // 获取有效的起始和结束日期
    let start_date = config.get_effective_start_date(&cli_defaults.start_date_override)?;
    let end_date = match config.get_effective_end_date(&cli_defaults.end_date)? {
        Some(d) => d,
        None => date_utils::today(),
    };

    tracing::info!(
        "日期范围: {} 到 {}",
        date_utils::format_date(&start_date),
        date_utils::format_date(&end_date)
    );

    // 生成日期列表
    let dates = date_utils::date_range(start_date, end_date);
    tracing::info!("待处理日期数量: {}", dates.len());

    // 创建下载器（使用重试配置）
    let retry_config = config.retry_config();
    tracing::info!(
        "重试配置: max_retries={}, base_delay={}ms",
        retry_config.max_retries,
        retry_config.base_delay_ms
    );
    let downloader = Downloader::with_retry_config(config, retry_config)?;

    // 执行批量下载
    let stats = downloader
        .download_batch(
            &config.base_url,
            &dates,
            config.max_concurrent,
            cli_defaults.overwrite,
            cli_defaults.download_only,
        )
        .await;

    // 打印统计结果
    println!("\n========== 下载统计 ==========");
    println!("总数量:     {}", stats.total);
    println!("成功:       {}", stats.succeeded);
    println!("失败:       {}", stats.failed);
    println!("跳过:       {}", stats.skipped);
    println!("成功率:     {:.1}%", stats.success_rate());

    // 保存失败的日期
    if !stats.failed_dates.is_empty() {
        let log_path = save_failed_downloads(&stats.failed_dates, Path::new(&config.output_dir))?;
        println!("\n失败的日期已保存到: {}", log_path.display());
        println!("可使用以下命令重新处理:");
        println!(
            "  cargo run -- process --dates {}",
            stats.failed_dates.join(",")
        );
    }

    // 如果有成功下载，更新配置文件中的 start_date
    if let Some(latest_date) = stats.latest_success_date() {
        // 只在用户未通过命令行指定 start_date 时才更新
        if cli_defaults.start_date_override.is_none() && latest_date > config.start_date {
            println!("\n更新配置文件中的起始日期: {} -> {}",
                date_utils::format_date(&config.start_date),
                date_utils::format_date(&latest_date)
            );

            // 创建可变配置副本并更新
            let mut config_clone = config.clone();
            config_clone.update_start_date(latest_date, config_path)?;
            println!("配置文件已更新: {}", config_path.display());
        }
    }

    Ok(())
}

/// 执行 process 命令（处理指定日期）
async fn process_command(
    config: &Config,
    cli_defaults: calendar::config::ConfigWithDefaults,
    dates: &[String],
) -> Result<()> {
    tracing::info!("执行 process 命令，处理 {} 个日期", dates.len());

    // 解析日期列表
    let parsed_dates: Result<Vec<NaiveDate>> = dates
        .iter()
        .map(|d| date_utils::parse_date(d))
        .collect();

    let parsed_dates = parsed_dates?;

    // 创建下载器（使用重试配置）
    let retry_config = config.retry_config();
    let downloader = Downloader::with_retry_config(config, retry_config)?;

    // 执行处理
    let stats = downloader
        .process_dates(
            &config.base_url,
            &parsed_dates,
            cli_defaults.overwrite,
            cli_defaults.metadata_only,
        )
        .await;

    // 打印统计结果
    println!("\n========== 处理统计 ==========");
    println!("总数量:     {}", stats.total);
    println!("成功:       {}", stats.succeeded);
    println!("失败:       {}", stats.failed);
    println!("跳过:       {}", stats.skipped);
    println!("成功率:     {:.1}%", stats.success_rate());

    // 保存失败的日期
    if !stats.failed_dates.is_empty() {
        let log_path = save_failed_downloads(&stats.failed_dates, Path::new(&config.output_dir))?;
        println!("\n失败的日期已保存到: {}", log_path.display());
        println!("可使用以下命令重新处理:");
        println!(
            "  cargo run -- process --dates {}",
            stats.failed_dates.join(",")
        );
    }

    Ok(())
}

/// 主函数
#[tokio::main]
async fn main() -> Result<()> {
    // 解析命令行参数
    let cli = Cli::parse();

    // 设置日志
    setup_tracing(&cli.log_level);

    tracing::info!("Calendar 图片下载器启动");
    tracing::debug!("日志级别: {}", cli.log_level);

    // 加载配置文件
    let config_path = cli.config.as_path();
    let config = Config::from_file(config_path)?.apply_env_overrides();

    tracing::info!(
        "配置加载完成: start_date={}, max_concurrent={}",
        date_utils::format_date(&config.start_date),
        config.max_concurrent
    );

    // 根据子命令执行相应操作
    match &cli.command {
        Command::Run {
            start_date: _,
            end_date: _,
            overwrite: _,
            download_only: _,
        } => {
            let cli_defaults = config.merge_cli_defaults(&cli.command);
            run_command(config_path, &config, cli_defaults).await?;
        }
        Command::Process {
            date: _,
            dates: _,
            overwrite: _,
            metadata_only: _,
        } => {
            let dates = cli.command.get_dates()?;
            let cli_defaults = config.merge_cli_defaults(&cli.command);
            process_command(&config, cli_defaults, &dates).await?;
        }
    }

    tracing::info!("程序执行完成");
    Ok(())
}
