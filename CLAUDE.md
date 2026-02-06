# CLAUDE.md

**使用中文回复、编写文档、注释**

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目概述

基于 Rust 的批量图片下载器，从可配置的 URL 模式下载每日图片，然后修改文件时间戳以匹配日期。使用 Tokio 异步运行时，支持可配置的并发数。

## 构建与运行

```bash
# Release 构建
cargo build --release

# Debug 构建
cargo build

# 运行
cargo run -- run                              # 从配置文件起始日期批量下载到当前日期
cargo run -- run --start-date 2024-06-01 --end-date 2024-06-15  # 指定日期范围
cargo run -- process --dates 2024-06-15,2024-06-20  # 处理指定日期
cargo run -- process --date 2024-06-15              # 处理单个日期

# 测试
cargo test
cargo test filename::tests  # 运行特定模块测试

# 代码检查
cargo check
```

## CLI 全局选项

- `-c, --config <PATH>`: 配置文件路径 (默认: `config.toml`)
- `-l, --log-level <LEVEL>`: 日志级别 (trace, debug, info, warn, error)

## 架构

```
src/
├── lib.rs              # 公共类型 (DownloadStats, ProcessResult, date_utils)
├── main.rs             # 程序入口，子命令调度
├── cli.rs              # clap CLI 定义 (Run/Process 子命令)
├── config.rs           # TOML 配置加载，环境变量覆盖
├── downloader.rs       # 异步批量下载，信号量控制并发，重试机制
├── exif.rs             # EXIF DateTimeOriginal 标记 (当前为日志记录)
├── fileops.rs          # 跨平台文件时间戳操作 (filetime)
├── filename.rs         # 日期占位符格式解析器
└── error.rs            # AppError, RetryableError 枚举 (thiserror)
```

### 核心类型

| 类型 | 位置 | 职责 |
|------|------|------|
| `Downloader` | downloader.rs | HTTP 客户端管理，批量下载调度，重试逻辑 |
| `FilenameFormatter` | filename.rs | `{yyyy}`, `{mm:02}` 等占位符解析 |
| `Config` | config.rs | TOML 配置，`merge_cli_defaults()` 合并 CLI 参数 |
| `ConfigWithDefaults` | config.rs | CLI 参数默认值传递 |
| `AppError` | error.rs | 结构化错误 (NetworkError, HttpError, FileError 等) |
| `RetryableError` | error.rs | 可重试错误分类 (超时, 429, 5xx 等) |
| `RetryConfig` | downloader.rs | 重试策略配置 (次数, 退避时间) |

### 子命令

**Run**: 批量下载模式下，通过 `Semaphore` 控制并发数，下载所有日期范围内的图片。

**Process**: 处理指定日期（单进程），`--metadata-only` 跳过下载仅更新元数据。

### 配置与环境变量

```toml
start_date = "2015-02-18"
base_url = "http://img.owspace.com/Public/uploads/Download/{year}/{month:02}{day:02}.jpg"
output_dir = "/path/to/output"
filename_format = "owspace_{yyyy}{mm}{dd}.jpg"
max_concurrent = 16
timeout = 30
max_retries = 3      # 最大重试次数 (默认: 3, 0 表示禁用重试)
retry_delay_ms = 1000 # 重试基础延迟 (毫秒, 默认: 1000)
```

环境变量覆盖:
- `CALENDAR_USER_AGENT`: 覆盖 User-Agent
- `CALENDAR_TIMEOUT`: 覆盖超时时间（秒）

**重试机制**: 下载失败时自动重试，使用指数退避策略：
- 429 (Too Many Requests): 初始等待 5 秒
- 5xx 服务器错误: 初始等待 2 秒
- 连接超时/DNS 失败: 初始等待 1 秒
- 后续重试等待时间翻倍，上限 30 秒

失败下载日期保存至 `{output_dir}/failed_downloads.txt`，可使用 `cargo run -- process --dates <失败日期>` 重试。

## 占位符

| 占位符 | 说明 | 示例 (2024-06-15) |
|--------|------|-------------------|
| `{yyyy}` / `{year}` | 四位年份 | 2024 |
| `{yy}` | 两位年份 | 24 |
| `{mm}` / `{month:02}` | 两位月份 | 06 |
| `{m}` / `{month}` | 不补位月份 | 6 |
| `{dd}` / `{day:02}` | 两位日期 | 15 |
| `{d}` / `{day}` | 不补位日期 | 15 |

## 扩展

**添加占位符**: 修改 [filename.rs](src/filename.rs) 中的 `format()` 和 `format_variable_width_placeholders()`。

**添加错误类型**: 在 [error.rs](src/error.rs) 的 `AppError` 枚举中添加变体。

**EXIF 修改**: 当前 [exif.rs](src/exif.rs) 仅记录日志。需修改时，可调用外部 `exiftool` 工具实现实际写入。
