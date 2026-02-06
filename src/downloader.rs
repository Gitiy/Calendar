//! 下载器模块
//!
//! 负责从指定的 URL 下载图片，支持并发下载和错误重试。

use chrono::{Datelike, NaiveDate, TimeZone, Utc};
use futures::stream::{FuturesUnordered, StreamExt};
use reqwest::{
    header::{HeaderMap, USER_AGENT},
    Client, StatusCode,
};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

use crate::{
    build_year_path,
    config::Config,
    date_utils,
    error::{AppError, Result, RetryableError},
    exif,
    fileops,
    filename::FilenameFormatter,
    DownloadStats,
};

/// 下载重试配置
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// 最大重试次数
    pub max_retries: u32,
    /// 基础退避时间（毫秒）
    pub base_delay_ms: u64,
    /// 最大退避时间（毫秒）
    pub max_delay_ms: u64,
    /// 是否启用重试
    pub enabled: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay_ms: 1000,
            max_delay_ms: 30000,
            enabled: true,
        }
    }
}

/// 下载器
pub struct Downloader {
    /// HTTP 客户端
    client: Client,
    /// 文件名格式化器
    formatter: FilenameFormatter,
    /// 输出目录
    output_dir: String,
    /// 用户代理（保留字段，用于未来功能扩展）
    _user_agent: String,
    /// 重试配置
    retry_config: RetryConfig,
}

impl Downloader {
    /// 创建新的下载器
    ///
    /// # 参数
    /// - `config`: 配置
    pub fn new(config: &Config) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, config.user_agent.parse()?);

        let client = Client::builder()
            .timeout(config.timeout_duration())
            .connect_timeout(Duration::from_secs(30))
            .default_headers(headers)
            // 配置连接池：限制最大连接数以避免服务器过载
            .pool_max_idle_per_host(8)
            .pool_idle_timeout(Duration::from_secs(90))
            .build()?;

        let formatter = FilenameFormatter::new(&config.filename_format)?;

        Ok(Self {
            client,
            formatter,
            output_dir: config.output_dir.clone(),
            _user_agent: config.user_agent.clone(),
            retry_config: RetryConfig::default(),
        })
    }

    /// 使用自定义重试配置创建下载器
    pub fn with_retry_config(config: &Config, retry_config: RetryConfig) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, config.user_agent.parse()?);

        let client = Client::builder()
            .timeout(config.timeout_duration())
            .connect_timeout(Duration::from_secs(30))
            .default_headers(headers)
            // 配置连接池：限制最大连接数以避免服务器过载
            .pool_max_idle_per_host(8)
            .pool_idle_timeout(Duration::from_secs(90))
            .build()?;

        let formatter = FilenameFormatter::new(&config.filename_format)?;

        Ok(Self {
            client,
            formatter,
            output_dir: config.output_dir.clone(),
            _user_agent: config.user_agent.clone(),
            retry_config,
        })
    }

    /// 计算指数退避延迟时间
    fn calculate_delay(&self, attempt: u32, base_delay: u64, max_delay: u64) -> u64 {
        let delay = base_delay * (2_u64.pow(attempt.min(10) as u32));
        delay.min(max_delay)
    }

    /// 睡眠指定毫秒数
    async fn sleep_ms(ms: u64) {
        tokio::time::sleep(Duration::from_millis(ms)).await
    }

    /// 获取给定日期的 URL
    fn build_url(&self, base_url: &str, date: &NaiveDate) -> String {
        let url_formatter =
            FilenameFormatter::new(base_url).unwrap_or_else(|_| self.formatter.clone());
        url_formatter.format_url(date)
    }

    /// 构建文件路径
    fn build_path(&self, date: &NaiveDate) -> std::path::PathBuf {
        let filename = self.formatter.format(date);
        let year_dir = build_year_path(Path::new(&self.output_dir), date.year());
        year_dir.join(&filename)
    }

    /// 下载单个日期的图片
    ///
    /// # 参数
    /// - `base_url`: 基础 URL 模板
    /// - `date`: 下载日期
    /// - `overwrite`: 是否覆盖已存在的文件
    /// - `download_only`: 是否仅下载（不修改 EXIF 和文件属性）
    ///
    /// # 返回
    /// 返回下载结果和文件路径
    pub async fn download(
        &self,
        base_url: &str,
        date: &NaiveDate,
        overwrite: bool,
        download_only: bool,
    ) -> Result<(std::path::PathBuf, bool)> {
        self.download_with_retry(base_url, date, overwrite, download_only)
            .await
    }

    /// 带重试的下载实现
    async fn download_with_retry(
        &self,
        base_url: &str,
        date: &NaiveDate,
        overwrite: bool,
        download_only: bool,
    ) -> Result<(std::path::PathBuf, bool)> {
        let url = self.build_url(base_url, date);
        let path = self.build_path(date);
        let date_str = date_utils::format_date(date);

        tracing::debug!("处理日期: {} -> {:?}", date_str, path);

        // 检查文件是否已存在
        if path.exists() && !overwrite {
            tracing::debug!("文件已存在，跳过下载: {:?}", path);

            // 即使文件已存在，也要更新 EXIF 和文件属性（除非 --download-only）
            if !download_only {
                let datetime = date.and_hms_opt(0, 0, 0).unwrap();
                let datetime_utc = Utc.from_utc_datetime(&datetime);

                // 更新 EXIF
                if let Err(e) = exif::set_exif_datetime(&path, &datetime) {
                    tracing::warn!("更新 EXIF 失败: {:?}: {}", path, e);
                }

                // 更新文件时间戳
                if let Err(e) = fileops::set_file_timestamps(&path, datetime_utc) {
                    tracing::warn!("更新文件时间戳失败: {:?}: {}", path, e);
                }
            }

            return Ok((path, true)); // true 表示已存在
        }

        // 如果文件不存在，创建目录
        if let Some(parent) = path.parent() {
            fileops::ensure_dir_exists(parent)?;
        }

        // 如果重试已禁用，直接下载
        if !self.retry_config.enabled {
            return self.execute_download(&url, &path, date, download_only).await;
        }

        // 带重试的下载
        let mut last_error: Option<AppError> = None;
        let max_retries = self.retry_config.max_retries;

        for attempt in 0..=max_retries {
            match self.execute_download(&url, &path, date, download_only).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    let retryable = self
                        .classify_error(&e)
                        .map(|re| re.is_retryable())
                        .unwrap_or(false);

                    if retryable && attempt < max_retries {
                        let base_delay = self.retry_config.base_delay_ms;
                        let max_delay = self.retry_config.max_delay_ms;
                        let delay = self.calculate_delay(attempt, base_delay, max_delay);

                        // 检查是否有建议的延迟时间
                        if let Some(re) = self.classify_error(&e) {
                            let suggested = re.suggested_delay_ms();
                            if suggested > delay {
                                // 使用建议的延迟时间和指数退避的较大者
                            }
                        }

                        tracing::warn!(
                            "下载失败 (尝试 {}/{}): {} - {}ms 后重试",
                            attempt + 1,
                            max_retries + 1,
                            url,
                            delay
                        );
                        Self::sleep_ms(delay).await;
                        last_error = Some(e);
                    } else {
                        // 不可重试错误或已达最大重试次数
                        tracing::error!("下载失败: {} - {}", url, e);
                        return Err(e);
                    }
                }
            }
        }

        // 所有重试都失败
        if let Some(e) = last_error {
            tracing::error!("下载失败，已重试 {} 次: {}", max_retries + 1, e);
            Err(e)
        } else {
            unreachable!()
        }
    }

    /// 对错误进行分类
    fn classify_error(&self, error: &AppError) -> Option<RetryableError> {
        match error {
            AppError::NetworkError { url: _, details } => {
                Some(RetryableError::from_error_message(details, None))
            }
            AppError::HttpError { url: _, status } => {
                if *status == StatusCode::TOO_MANY_REQUESTS {
                    Some(RetryableError::TooManyRequests)
                } else if status.is_server_error() {
                    Some(RetryableError::ServerError(*status))
                } else {
                    Some(RetryableError::Unknown(format!(
                        "HTTP {}",
                        status
                    )))
                }
            }
            _ => None,
        }
    }

    /// 执行实际下载（无重试）
    async fn execute_download(
        &self,
        url: &str,
        path: &std::path::PathBuf,
        date: &NaiveDate,
        download_only: bool,
    ) -> Result<(std::path::PathBuf, bool)> {
        tracing::debug!("开始下载: {}", url);

        let response = match self.client.get(url).send().await {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!("请求失败: {} - {}", url, e);
                return Err(AppError::NetworkError {
                    url: url.to_string(),
                    details: e.to_string(),
                });
            }
        };

        // 检查响应状态码
        if !response.status().is_success() {
            if response.status() == StatusCode::NOT_FOUND {
                return Err(AppError::HttpError {
                    url: url.to_string(),
                    status: StatusCode::NOT_FOUND,
                });
            }
            tracing::warn!("HTTP 错误 {}: {}", response.status(), url);
            return Err(AppError::HttpError {
                url: url.to_string(),
                status: response.status(),
            });
        }

        // 读取响应体
        let bytes = match response.bytes().await {
            Ok(b) => b,
            Err(e) => {
                tracing::warn!("读取响应体失败: {} - {}", url, e);
                return Err(AppError::NetworkError {
                    url: url.to_string(),
                    details: format!("读取响应体失败: {}", e),
                });
            }
        };

        // 写入文件
        tokio::fs::write(path, bytes)
            .await
            .map_err(|e| AppError::file_error(path, e.to_string()))?;

        tracing::info!("下载成功: {:?}", path);

        // 更新 EXIF 和文件属性（除非 --download-only）
        if !download_only {
            let datetime = date.and_hms_opt(0, 0, 0).unwrap();
            let datetime_utc = Utc.from_utc_datetime(&datetime);

            // 更新 EXIF
            if let Err(e) = exif::set_exif_datetime(path, &datetime) {
                tracing::warn!("更新 EXIF 失败: {:?}: {}", path, e);
            }

            // 更新文件时间戳
            if let Err(e) = fileops::set_file_timestamps(path, datetime_utc) {
                tracing::warn!("更新文件时间戳失败: {:?}: {}", path, e);
            }
        }

        Ok((path.clone(), false)) // false 表示新下载
    }

    /// 批量下载多个日期的图片
    ///
    /// # 参数
    /// - `base_url`: 基础 URL 模板
    /// - `dates`: 日期列表
    /// - `max_concurrent`: 最大并发数
    /// - `overwrite`: 是否覆盖已存在的文件
    /// - `download_only`: 是否仅下载（不修改 EXIF 和文件属性）
    ///
    /// # 返回
    /// 返回下载统计信息
    pub async fn download_batch(
        &self,
        base_url: &str,
        dates: &[NaiveDate],
        max_concurrent: usize,
        overwrite: bool,
        download_only: bool,
    ) -> DownloadStats {
        let semaphore = Arc::new(Semaphore::new(max_concurrent));
        let mut tasks = FuturesUnordered::new();

        let mut stats = DownloadStats::new(dates.len());

        // 创建进度条
        let progress = indicatif::ProgressBar::new(dates.len() as u64);
        progress.set_style(
            indicatif::ProgressStyle::default_bar()
                .template(
                    "[{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} \
                     成功: {green} 失败: {red} 跳过: {yellow}",
                )
                .unwrap()
                .progress_chars("##-"),
        );

        for date in dates {
            let permit = semaphore.clone().acquire_owned().await;
            if permit.is_err() {
                tracing::error!("未能获取信号量许可");
                break;
            }

            let formatter = self.formatter.clone();
            let url = self.build_url(base_url, date);
            let client = self.client.clone();
            let output_dir = self.output_dir.clone();
            let date_clone = *date;
            let progress = progress.clone();

            tasks.push(tokio::spawn(async move {
                let date_str = date_utils::format_date(&date_clone);
                let filename = formatter.format(&date_clone);
                let year_dir = build_year_path(Path::new(&output_dir), date_clone.year());
                let path = year_dir.join(&filename);

                // permit 在此作用域结束时自动释放，确保整个下载过程都受信号量控制

                // 检查文件是否已存在
                if path.exists() && !overwrite {
                    tracing::debug!("文件已存在，跳过下载: {:?}", path);

                    let datetime =
                        date_clone.and_hms_opt(0, 0, 0).unwrap();
                    let datetime_utc = Utc.from_utc_datetime(&datetime);

                    if !download_only {
                        // 更新 EXIF
                        if let Err(e) = exif::set_exif_datetime(&path, &datetime) {
                            tracing::warn!("更新 EXIF 失败: {:?}: {}", path, e);
                        }

                        // 更新文件时间戳
                        if let Err(e) = fileops::set_file_timestamps(&path, datetime_utc) {
                            tracing::warn!("更新文件时间戳失败: {:?}: {}", path, e);
                        }
                    }

                    progress.inc(1);
                    progress.set_message(format!("跳过: {}", date_str));
                    return (date_str, Ok((path, true)));
                }

                // 创建目录
                if let Some(parent) = path.parent() {
                    let _ = fileops::ensure_dir_exists(parent);
                }

                // 下载文件（带重试）
                const MAX_RETRIES: u32 = 3;
                const BASE_DELAY_MS: u64 = 1000;
                const MAX_DELAY_MS: u64 = 30000;

                let download_result = async {
                    for attempt in 0..=MAX_RETRIES {
                        // 检查是否需要重试（不是第一次尝试）
                        if attempt > 0 {
                            let delay_ms = (BASE_DELAY_MS * (2_u64.pow(attempt.min(10) as u32)))
                                .min(MAX_DELAY_MS);
                            // 检查是否是 decoding 错误，增加额外延迟
                            if attempt == 1 {
                                tokio::time::sleep(Duration::from_millis(2000)).await;
                            } else {
                                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                            }
                            tracing::warn!(
                                "重试下载 (尝试 {}/{}): {}",
                                attempt + 1,
                                MAX_RETRIES + 1,
                                url
                            );
                        }

                        // 发送请求
                        let response = match client.get(&url).send().await {
                            Ok(r) => r,
                            Err(e) => {
                                // 只有最后一次才记录错误
                                if attempt == MAX_RETRIES {
                                    tracing::error!("下载失败: {}: {}", date_str, e);
                                    return Err(AppError::NetworkError {
                                        url: url.clone(),
                                        details: e.to_string(),
                                    });
                                }
                                continue;
                            }
                        };

                        // 检查响应状态码
                        if !response.status().is_success() {
                            // 404 不重试
                            if response.status() == StatusCode::NOT_FOUND {
                                tracing::error!("资源不存在: {}", url);
                                return Err(AppError::HttpError {
                                    url: url.clone(),
                                    status: response.status(),
                                });
                            }

                            // 只有最后一次才记录错误
                            if attempt == MAX_RETRIES {
                                tracing::error!(
                                    "HTTP 错误: {} 返回状态码 {}",
                                    url,
                                    response.status()
                                );
                                return Err(AppError::HttpError {
                                    url: url.clone(),
                                    status: response.status(),
                                });
                            }
                            continue;
                        }

                        // 读取响应体
                        match response.bytes().await {
                            Ok(b) => {
                                // 验证是否为空响应
                                if b.is_empty() {
                                    if attempt == MAX_RETRIES {
                                        tracing::error!("服务器返回空响应: {}", url);
                                        return Err(AppError::NetworkError {
                                            url: url.clone(),
                                            details: "服务器返回空响应".to_string(),
                                        });
                                    }
                                    continue;
                                }
                                return Ok(b);
                            }
                            Err(e) => {
                                let err_msg = e.to_string().to_lowercase();
                                // decoding 错误可重试
                                let is_retryable = err_msg.contains("decode")
                                    || err_msg.contains("stream")
                                    || err_msg.contains("connection")
                                    || err_msg.contains("timeout");

                                if !is_retryable || attempt == MAX_RETRIES {
                                    tracing::error!("读取响应体失败: {}: {}", date_str, e);
                                    return Err(AppError::NetworkError {
                                        url: url.clone(),
                                        details: e.to_string(),
                                    });
                                }
                                continue;
                            }
                        }
                    }

                    unreachable!()
                }.await;

                // 处理下载结果
                let bytes = match download_result {
                    Ok(b) => b,
                    Err(e) => {
                        progress.inc(1);
                        progress.set_message(format!("失败: {}", date_str));
                        return (date_str, Err(e));
                    }
                };

                // 写入文件
                match tokio::fs::write(&path, bytes).await {
                    Ok(_) => {
                        tracing::info!("下载成功: {:?}", path);

                        let datetime =
                            date_clone.and_hms_opt(0, 0, 0).unwrap();
                        let datetime_utc = Utc.from_utc_datetime(&datetime);

                        if !download_only {
                            // 更新 EXIF
                            if let Err(e) = exif::set_exif_datetime(&path, &datetime) {
                                tracing::warn!("更新 EXIF 失败: {:?}: {}", path, e);
                            }

                            // 更新文件时间戳
                            if let Err(e) = fileops::set_file_timestamps(&path, datetime_utc) {
                                tracing::warn!("更新文件时间戳失败: {:?}: {}", path, e);
                            }
                        }

                        progress.inc(1);
                        progress.set_message(format!("成功: {}", date_str));
                        
                        drop(permit);

                        (date_str, Ok((path, false)))
                    }
                    Err(e) => {
                        progress.inc(1);
                        progress.set_message(format!("失败: {}", date_str));
                        tracing::error!("写入文件失败: {:?}: {}", path, e);
                        (
                            date_str,
                            Err(AppError::file_error(&path, e.to_string())),
                        )
                    }
                }
            }));
        }

        // 等待所有任务完成
        while let Some(result) = tasks.next().await {
            match result {
                Ok((date_str, result)) => match result {
                    Ok((_, existed)) => {
                        if existed {
                            stats.record_skip();
                        } else {
                            stats.record_success();
                        }
                    }
                    Err(_) => {
                        stats.record_failure(&date_str);
                    }
                },
                Err(e) => {
                    tracing::error!("任务执行失败: {}", e);
                }
            }
        }

        progress.finish_with_message("完成");
        stats
    }

    /// 处理指定日期的文件（process 命令）
    ///
    /// # 参数
    /// - `base_url`: 基础 URL 模板
    /// - `dates`: 日期列表
    /// - `overwrite`: 是否覆盖已存在的文件
    /// - `metadata_only`: 是否仅修改元数据（不下载）
    ///
    /// # 返回
    /// 返回下载统计信息
    pub async fn process_dates(
        &self,
        base_url: &str,
        dates: &[NaiveDate],
        overwrite: bool,
        metadata_only: bool,
    ) -> DownloadStats {
        let download_only = false; // process 命令默认需要修改元数据

        self.download_batch(
            base_url,
            dates,
            1, // process 命令不使用并发
            overwrite,
            if metadata_only { true } else { download_only },
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use std::path::PathBuf;

    #[test]
    fn test_build_url() {
        let config = Config {
            start_date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            base_url: "https://example.com/{year}/{month:02}/{day:02}.jpg".to_string(),
            output_dir: "./images".to_string(),
            filename_format: "{yyyy}{mm}{dd}.jpg".to_string(),
            max_concurrent: 3,
            user_agent: "Test".to_string(),
            timeout: 30,
            max_retries: 3,
            retry_delay_ms: 1000,
        };

        let downloader = Downloader::new(&config).unwrap();
        let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();

        let url = downloader.build_url(&config.base_url, &date);
        assert_eq!(url, "https://example.com/2024/06/15.jpg");
    }

    #[test]
    fn test_build_path() {
        let config = Config {
            start_date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            base_url: "https://example.com/{year}/{month:02}/{day:02}.jpg".to_string(),
            output_dir: "/tmp/images".to_string(),
            filename_format: "{yyyy}{mm}{dd}.jpg".to_string(),
            max_concurrent: 3,
            user_agent: "Test".to_string(),
            timeout: 30,
            max_retries: 3,
            retry_delay_ms: 1000,
        };

        let downloader = Downloader::new(&config).unwrap();
        let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();

        let path = downloader.build_path(&date);
        assert_eq!(path, PathBuf::from("/tmp/images/2024/20240615.jpg"));
    }
}
