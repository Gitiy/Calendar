# Calendar 图片下载器

一个功能强大的批量图片下载工具，支持从指定起始日期开始自动下载每日图片，并智能修改照片的 EXIF 信息和文件时间戳。

## 📚 目录

- [项目概述](#项目概述)
- [系统架构](#系统架构)
- [技术栈](#技术栈)
- [快速开始](#快速开始)
- [详细配置](#详细配置)
- [命令详解](#命令详解)
- [核心功能实现](#核心功能实现)
- [开发指南](#开发指南)
- [部署指南](#部署指南)
- [故障排除](#故障排除)

---

## 项目概述

### 功能特性

- 🚀 **批量下载**：支持从指定日期到当前日期的批量下载
- 🔄 **自动重试**：内置智能重试机制，网络不稳定时自动重试
- 🎯 **并发控制**：可配置最大并发数，优化下载速度
- 📅 **智能日期管理**：下载成功后自动更新配置文件中的起始日期
- 🏷️ **EXIF 修改**：自动设置图片的 DateTimeOriginal 等元数据
- 📁 **智能文件命名**：支持自定义文件名格式，按年份自动归档
- 📊 **进度显示**：实时显示下载进度和统计信息
- ✅ **图片验证**：下载后自动验证图片完整性
- 🔧 **配置验证**：提供配置文件验证命令
- 🧪 **完整测试**：包含单元测试和集成测试

### 项目结构

```
calendar/
├── .github/
│   └── workflows/
│       └── ci.yml              # GitHub Actions CI/CD 配置
├── src/
│   ├── main.rs                 # 主程序入口，命令行路由
│   ├── lib.rs                  # 公共类型、工具函数和模块导出
│   ├── cli.rs                  # 命令行参数定义和解析
│   ├── config.rs               # 配置文件加载、解析和保存
│   ├── downloader.rs           # 下载器核心逻辑（并发、重试）
│   ├── exif.rs                 # EXIF 元数据读写
│   ├── filename.rs             # 文件名格式化和占位符解析
│   ├── fileops.rs              # 文件操作（时间戳、目录）
│   ├── validator.rs            # 图片验证模块
│   └── error.rs                # 错误类型定义和转换
├── Cargo.toml                  # 项目依赖和配置
├── Cargo.lock                  # 依赖版本锁定
├── config.toml                 # 应用配置文件示例
├── .pre-commit-config.yaml     # Pre-commit hooks 配置
├── .gitignore                  # Git 忽略文件
└── README.md                   # 项目文档
```

---

## 系统架构

### 模块依赖关系

```
main.rs (入口)
  ├── cli.rs (命令行解析)
  ├── config.rs (配置管理)
  │   ├── error.rs (错误类型)
  │   └── lib.rs (工具函数)
  └── downloader.rs (下载器)
      ├── exif.rs (EXIF 修改)
      ├── filename.rs (文件名)
      ├── fileops.rs (文件操作)
      ├── validator.rs (图片验证)
      └── error.rs (错误处理)
```

### 核心数据流

```
命令行输入 → CLI 解析 → 配置加载 → 日期生成 → 并发下载 → 图片验证 → EXIF 修改 → 文件保存 → 统计更新 → 配置保存
```

### 异步执行模型

使用 `tokio` 异步运行时，通过 `tokio::task::JoinSet` 实现并发任务管理：

```rust
// 并发下载架构
Semaphore (信号量控制并发数)
    ↓
JoinSet (任务集合)
    ├── Task 1 (下载日期1)
    ├── Task 2 (下载日期2)
    └── Task N (下载日期N)
        ↓
    Result 收集和统计
```

---

## 技术栈

### 核心依赖

| 依赖库 | 版本 | 用途 | 特性 |
|--------|------|------|------|
| tokio | 1.40+ | 异步运行时 | rt-multi-thread, macros, fs, time, sync |
| reqwest | 0.12+ | HTTP 客户端 | rustls-tls, json, cookies |
| chrono | 0.4.38+ | 日期时间处理 | serde |
| serde | 1.0+ | 序列化/反序列化 | derive |
| toml | 0.8+ | TOML 配置解析 | - |
| clap | 4.5+ | 命令行参数解析 | derive |
| thiserror | 1.0+ | 结构化错误 | derive |
| tracing | 0.1+ | 结构化日志 | - |
| tracing-subscriber | 0.3+ | 日志订阅器 | env-filter |
| filetime | 0.2+ | 文件时间戳操作 | - |
| indicatif | 0.17+ | 进度条显示 | - |
| regex | 1.0+ | 正则表达式 | - |
| little_exif | 0.6.3+ | EXIF 读写 | - |
| image | 0.25+ | 图片验证 | - |

### 开发依赖

| 依赖库 | 版本 | 用途 |
|--------|------|------|
| tokio-test | 0.4+ | 异步测试工具 |
| tempfile | 3.0+ | 临时文件测试 |

### 编译优化配置

```toml
[profile.release]
opt-level = "z"        # 优化为最小体积
lto = true            # 链接时优化
codegen-units = 1     # 单编译单元
strip = true          # 移除调试符号
panic = "abort"       # 简化 panic 处理
```

---

## 快速开始

### 1. 环境准备

#### 安装 Rust

```bash
# Linux/macOS
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Windows
# 下载并运行 rustup-init.exe
```

#### 验证安装

```bash
rustc --version  # 应该 >= 1.70
cargo --version
```

### 2. 克隆和编译

```bash
# 克隆仓库
git clone <repository-url>
cd calendar

# 编译发布版本
cargo build --release

# 二进制文件位于 target/release/calendar
```

### 3. 创建配置文件

创建 `config.toml` 文件：

```toml
# Calendar 图片下载器配置文件

# 起始日期 (格式：YYYY-MM-DD)
start_date = "2024-01-01"

# 基础 URL，支持占位符
# 占位符：{year}, {month}, {day}, {yyyy}, {yy}, {mm}, {dd}
# 格式说明：{month:02} 表示两位数补零
base_url = "https://example.com/images/{year}/{month:02}/{day:02}.jpg"

# 输出目录
output_dir = "./images"

# 文件名格式
filename_format = "{yyyy}{mm}{dd}.jpg"

# 最大并发数（仅对 run 命令有效）
max_concurrent = 5

# HTTP 请求时使用的 User-Agent
user_agent = "Mozilla/5.0"

# 下载超时时间（秒）
timeout = 30

# 最大重试次数（0 为禁用）
max_retries = 3

# 重试基础延迟（毫秒）
retry_delay_ms = 1000
```

### 4. 运行程序

```bash
# 基本使用
./target/release/calendar run

# 查看帮助
./target/release/calendar --help
./target/release/calendar run --help
./target/release/calendar process --help

# 验证配置
./target/release/calendar config --validate
```

---

## 详细配置

### 占位符语法

#### 支持的占位符

| 占位符 | 说明 | 示例 | 输出 |
|--------|------|------|------|
| `{year}` | 四位年份 | 2024 | 2024 |
| `{month}` | 月份（不补位） | 6 | 6 |
| `{month:02}` | 月份（补位） | 6 | 06 |
| `{day}` | 日期（不补位） | 5 | 5 |
| `{day:02}` | 日期（补位） | 5 | 05 |
| `{yyyy}` | 四位年份 | 2024 | 2024 |
| `{yy}` | 两位年份 | 2024 | 24 |
| `{mm}` | 两位月份 | 6 | 06 |
| `{dd}` | 两位日期 | 5 | 05 |

#### 格式化语法

```
{name}        - 使用默认格式
{name:02}     - 使用两位数补零
{name:03}     - 使用三位数补零
```

#### 配置示例

```toml
# 示例 1：标准格式
base_url = "https://example.com/{year}/{month:02}/{day:02}.jpg"
# 2024-06-05 → https://example.com/2024/06/05.jpg

# 示例 2：自定义格式
base_url = "https://cdn.example.com/images/{yyyy}{mm}{dd}_hd.jpg"
# 2024-06-05 → https://cdn.example.com/images/20240605_hd.jpg

# 示例 3：文件名格式
filename_format = "calendar_{yyyy}_{mm}_{dd}.jpg"
# 2024-06-05 → calendar_2024_06_05.jpg

# 示例 4：短年份格式
filename_format = "{yy}{mm}{dd}.png"
# 2024-06-05 → 240605.png
```

### 配置项详解

#### 必需配置项

| 配置项 | 类型 | 说明 | 示例 |
|--------|------|------|------|
| `start_date` | String | 起始日期，格式 YYYY-MM-DD | `"2024-01-01"` |
| `base_url` | String | 图片 URL 模板，支持占位符 | `"https://example.com/{year}/{month:02}/{day:02}.jpg"` |
| `output_dir` | String | 输出目录路径 | `"./images"` |
| `filename_format` | String | 文件名格式，支持占位符 | `"{yyyy}{mm}{dd}.jpg"` |

#### 可选配置项（带默认值）

| 配置项 | 类型 | 默认值 | 说明 |
|--------|------|--------|------|
| `max_concurrent` | Integer | 3 | 最大并发下载数 |
| `user_agent` | String | "Mozilla/5.0" | HTTP 请求 User-Agent |
| `timeout` | Integer | 30 | 下载超时时间（秒） |
| `max_retries` | Integer | 3 | 最大重试次数（0 为禁用） |
| `retry_delay_ms` | Integer | 1000 | 重试基础延迟（毫秒） |

### 环境变量覆盖

可以使用环境变量覆盖配置文件中的设置：

```bash
# 设置 User-Agent
export CALENDAR_USER_AGENT="MyDownloader/1.0"

# 设置超时时间
export CALENDAR_TIMEOUT="60"

# 运行程序
./target/release/calendar run
```

**支持的环境变量：**

- `CALENDAR_USER_AGENT` - 覆盖 user_agent 配置
- `CALENDAR_TIMEOUT` - 覆盖 timeout 配置（单位：秒）

---

## 命令详解

### 全局选项

所有命令都支持以下全局选项：

```bash
-c, --config <FILE>     配置文件路径（默认：config.toml）
-l, --log-level <LEVEL> 日志级别：trace, debug, info, warn, error
-h, --help              显示帮助信息
-V, --version           显示版本信息
```

### run 命令

**功能：** 批量下载从起始日期到结束日期的所有图片

**语法：**

```bash
calendar run [OPTIONS]
```

**选项：**

| 选项 | 说明 | 默认值 |
|------|------|--------|
| `--start-date <DATE>` | 起始日期（格式：YYYY-MM-DD） | 配置文件中的 start_date |
| `--end-date <DATE>` | 结束日期（格式：YYYY-MM-DD） | 当前日期 |
| `--overwrite` | 覆盖已存在的文件 | false |
| `--download-only` | 仅下载，不修改 EXIF 和文件属性 | false |

**行为说明：**

1. **日期处理：**
   - 如果未指定 `--start-date`，使用配置文件中的 `start_date`
   - 如果未指定 `--end-date`，默认下载到当前日期
   - 自动生成日期范围内的所有日期列表

2. **文件处理：**
   - 已存在的文件默认跳过下载
   - 但仍然更新 EXIF 和文件属性（除非使用 `--download-only`）
   - 使用 `--overwrite` 强制重新下载所有文件

3. **并发控制：**
   - 使用配置文件中的 `max_concurrent` 控制并发数
   - 通过信号量（Semaphore）确保不超过并发限制

4. **自动更新配置：**
   - 下载成功后，自动更新配置文件中的 `start_date`
   - 新的 `start_date` 为最新成功下载的日期
   - 下次运行时会从上次停止的日期继续
   - 只有在使用默认 start_date 时才更新（即未通过 `--start-date` 指定）

5. **错误处理：**
   - 失败的下载会自动重试（根据 max_retries 配置）
   - 失败的日期记录到 `output_dir/failed_downloads.txt`
   - 支持使用 `process` 命令重新处理失败的日期

**使用示例：**

```bash
# 示例 1：使用配置文件的默认设置
./target/release/calendar run

# 示例 2：指定日期范围
./target/release/calendar run --start-date 2024-01-01 --end-date 2024-12-31

# 示例 3：只下载今天
./target/release/calendar run --start-date 2024-06-05 --end-date 2024-06-05

# 示例 4：覆盖已存在的文件
./target/release/calendar run --overwrite

# 示例 5：仅下载不修改元数据
./target/release/calendar run --download-only

# 示例 6：结合多个选项
./target/release/calendar run --start-date 2024-06-01 --overwrite -l debug
```

**输出示例：**

```
INFO Calendar 图片下载器启动
INFO 加载配置文件: config.toml
INFO 配置加载完成: start_date=2024-01-01, max_concurrent=5
INFO 执行 run 命令
INFO 日期范围: 2024-01-01 到 2024-01-05
INFO 待处理日期数量: 5
INFO 重试配置: max_retries=3, base_delay=1000ms
[00:00:00] [##########          ] 3/5 成功: 3 失败: 0 跳过: 0
INFO 下载成功: ./images/2024/20240101.jpg
INFO 下载成功: ./images/2024/20240102.jpg
INFO 下载成功: ./images/2024/20240103.jpg
[00:00:02] [####################] 5/5 成功: 5 失败: 0 跳过: 0

========== 下载统计 ==========
总数量:     5
成功:       5
失败:       0
跳过:       0
成功率:     100.0%

INFO 配置文件已更新: start_date = 2024-01-05
INFO 程序执行完成
```

### process 命令

**功能：** 处理指定日期的单个或多个文件

**语法：**

```bash
calendar process [OPTIONS]
```

**选项：**

| 选项 | 说明 | 默认值 |
|------|------|--------|
| `--date <DATE>` | 单个日期（格式：YYYY-MM-DD） | - |
| `--dates <DATES>` | 多个日期，逗号分隔或多次指定 | - |
| `--overwrite` | 覆盖已存在的文件 | false |
| `--metadata-only` | 仅修改 EXIF 和文件属性，不下载 | false |

**行为说明：**

1. **日期指定：**
   - 必须指定 `--date` 或 `--dates` 参数
   - `--date` 和 `--dates` 不能同时使用
   - `--dates` 支持逗号分隔或多次指定

2. **处理方式：**
   - 不使用并发，逐个处理日期
   - 适合处理特定日期或修复失败的下载
   - 支持仅修改元数据（不下载）

3. **文件处理：**
   - 文件存在时默认跳过下载
   - 但仍然更新 EXIF 和文件属性
   - 使用 `--overwrite` 强制重新下载
   - 使用 `--metadata-only` 仅更新元数据

4. **错误处理：**
   - 失败的日期记录到 `failed_downloads.txt`
   - 不会更新配置文件

**使用示例：**

```bash
# 示例 1：处理单个日期
./target/release/calendar process --date 2024-06-15

# 示例 2：处理多个日期（逗号分隔）
./target/release/calendar process --dates 2024-06-15,2024-06-20,2024-06-25

# 示例 3：处理多个日期（多次指定）
./target/release/calendar process --dates 2024-06-15 --dates 2024-06-20

# 示例 4：重新下载已存在的文件
./target/release/calendar process --date 2024-06-15 --overwrite

# 示例 5：仅更新元数据
./target/release/calendar process --dates 2024-06-15,2024-06-20 --metadata-only

# 示例 6：处理失败的日期
./target/release/calendar process --dates $(cat images/failed_downloads.txt | tr '\n' ',')
```

**输出示例：**

```
INFO Calendar 图片下载器启动
INFO 加载配置文件: config.toml
INFO 执行 process 命令，处理 3 个日期
INFO 处理日期: 2024-06-15
INFO 下载成功: ./images/2024/20240615.jpg
INFO 处理日期: 2024-06-20
INFO 文件已存在，跳过下载: ./images/2024/20240620.jpg
INFO 处理日期: 2024-06-25
INFO 下载成功: ./images/2024/20240625.jpg

========== 处理统计 ==========
总数量:     3
成功:       3
失败:       0
跳过:       0
成功率:     100.0%

INFO 程序执行完成
```

### config 命令

**功能：** 配置文件验证

**语法：**

```bash
calendar config [OPTIONS]
```

**选项：**

| 选项 | 说明 | 默认值 |
|------|------|--------|
| `--validate` | 验证配置文件是否正确 | false |

**行为说明：**

1. **验证内容：**
   - 配置文件是否存在
   - TOML 语法是否正确
   - 必需配置项是否存在
   - 日期格式是否正确
   - URL 格式是否有效

2. **验证输出：**
   - 显示配置验证结果
   - 列出所有配置项
   - 显示配置摘要

**使用示例：**

```bash
# 验证配置文件
./target/release/calendar config --validate

# 输出示例：
✓ 配置文件验证通过: config.toml

配置信息:
  起始日期: 2024-01-01
  输出目录: ./images
  基础 URL: https://example.com/{year}/{month:02}/{day:02}.jpg
  文件名格式: {yyyy}{mm}{dd}.jpg
  最大并发数: 5
  超时时间: 30 秒
  最大重试次数: 3
```

---

## 核心功能实现

### 1. 命令行参数解析 (cli.rs)

使用 `clap` 库定义命令行接口：

```rust
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "calendar")]
pub struct Cli {
    #[arg(short = 'c', long, default_value = "config.toml")]
    pub config: PathBuf,

    #[arg(short = 'l', long, default_value = "info")]
    pub log_level: String,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    Run {
        #[arg(long)]
        start_date: Option<String>,
        #[arg(long)]
        end_date: Option<String>,
        #[arg(long, default_value_t = false)]
        overwrite: bool,
        #[arg(long, default_value_t = false)]
        download_only: bool,
    },
    Process {
        #[arg(long)]
        date: Option<String>,
        #[arg(long, value_delimiter = ',')]
        dates: Option<Vec<String>>,
        #[arg(long, default_value_t = false)]
        overwrite: bool,
        #[arg(long, default_value_t = false)]
        metadata_only: bool,
    },
    Config {
        #[arg(long)]
        validate: bool,
    },
}
```

### 2. 配置管理 (config.rs)

#### 配置结构定义

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(with = "serde_date")]
    pub start_date: NaiveDate,
    pub base_url: String,
    pub output_dir: String,
    pub filename_format: String,
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent: usize,
    #[serde(default = "default_user_agent")]
    pub user_agent: String,
    #[serde(default = "default_timeout")]
    pub timeout: u64,
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    #[serde(default = "default_retry_delay")]
    pub retry_delay_ms: u64,
}
```

#### 配置加载

```rust
impl Config {
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }
}
```

#### 配置保存

```rust
impl Config {
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        let toml_str = toml::to_string_pretty(self)?;
        std::fs::write(path, toml_str)?;
        Ok(())
    }

    pub fn update_start_date(&mut self, new_date: NaiveDate) {
        self.start_date = new_date;
    }
}
```

### 3. 下载器核心 (downloader.rs)

#### 并发下载架构

使用 `tokio::task::JoinSet` 实现并发任务管理：

```rust
pub async fn download_batch(
    &self,
    base_url: &str,
    dates: &[NaiveDate],
    max_concurrent: usize,
    overwrite: bool,
    download_only: bool,
) -> DownloadStats {
    let semaphore = Arc::new(Semaphore::new(max_concurrent));
    let mut tasks = JoinSet::new();
    let mut stats = DownloadStats::new(dates.len());

    for date in dates {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let task = self.create_download_task(*date, base_url, overwrite, download_only);
        tasks.spawn(async move {
            let result = task.await;
            drop(permit);
            result
        });
    }

    while let Some(result) = tasks.join_next().await {
        // 处理结果
    }

    stats
}
```

#### 重试机制

实现指数退避重试策略：

```rust
const MAX_RETRIES: u32 = 3;
const BASE_DELAY_MS: u64 = 1000;
const MAX_DELAY_MS: u64 = 30000;

for attempt in 0..=MAX_RETRIES {
    if attempt > 0 {
        let delay_ms = (BASE_DELAY_MS * (2_u64.pow(attempt.min(10) as u32))).min(MAX_DELAY_MS);
        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
    }

    let response = client.get(&url).send().await?;
    
    if response.status().is_success() {
        return Ok(response.bytes().await?);
    }
    
    // 检查是否应该重试
    if should_retry(&response.status()) {
        continue;
    }
    
    break;
}
```

#### 图片验证

```rust
match tokio::fs::write(&path, bytes).await {
    Ok(_) => {
        match ImageValidator::validate(&path) {
            Ok(ValidationResult::Valid) => {
                // 图片有效，继续处理
            }
            Ok(ValidationResult::Invalid(reason)) => {
                // 图片无效，删除文件
                tokio::fs::remove_file(&path).await?;
                return Err(AppError::file_error(&path, reason));
            }
            Err(e) => {
                tracing::warn!("图片验证出错: {:?}", e);
            }
        }
    }
    Err(e) => {
        return Err(AppError::file_error(&path, e.to_string()));
    }
}
```

### 4. EXIF 修改 (exif.rs)

使用 `little_exif` 库修改 EXIF 元数据：

```rust
use little_exif::{exif_tag::ExifTag, metadata::Metadata};

pub fn set_exif_datetime(path: &Path, date: &NaiveDate) -> Result<()> {
    let datetime = date.and_hms_opt(0, 0, 0).unwrap();
    let datetime_str = format!("{:04}:{:02}:{:02} {:02}:{:02}:{:02}",
        date.year(), date.month(), date.day(),
        datetime.hour(), datetime.minute(), datetime.second());

    let mut metadata = Metadata::new();
    metadata.set_tag(ExifTag::DateTimeOriginal, &datetime_str)?;
    metadata.set_tag(ExifTag::DateTime, &datetime_str)?;
    metadata.set_tag(ExifTag::DateTimeDigitized, &datetime_str)?;

    metadata.write_to_file(path)?;
    Ok(())
}
```

### 5. 文件名格式化 (filename.rs)

支持占位符的文件名格式化：

```rust
pub struct FilenameFormatter {
    format: String,
}

impl FilenameFormatter {
    pub fn format(&self, date: NaiveDate) -> String {
        let result = self.format
            .replace("{yyyy}", &format!("{:04}", date.year()))
            .replace("{yy}", &format!("{:02}", date.year() % 100))
            .replace("{mm}", &format!("{:02}", date.month()))
            .replace("{dd}", &format!("{:02}", date.day()))
            .replace("{year}", &date.year().to_string())
            .replace("{month}", &date.month().to_string())
            .replace("{day}", &date.day().to_string());
        
        // 处理格式化选项（如 {month:02}）
        let re = regex::Regex::new(r"\{(\w+):(\d+)\}").unwrap();
        re.replace_all(&result, |caps: &regex::Captures| {
            let name = &caps[1];
            let width = caps[2].parse::<usize>().unwrap();
            match name {
                "month" => format!("{:0width$}", date.month(), width = width),
                "day" => format!("{:0width$}", date.day(), width = width),
                _ => caps[0].to_string(),
            }
        }).to_string()
    }
}
```

### 6. 错误处理 (error.rs)

使用 `thiserror` 定义结构化错误类型：

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("配置文件错误: {path}: {details}")]
    ConfigError {
        path: PathBuf,
        details: String,
    },

    #[error("TOML 解析错误: {0}")]
    TomlError(#[from] toml::de::Error),

    #[error("无效的日期格式 '{input}': {details}")]
    InvalidDate {
        input: String,
        details: String,
    },

    #[error("网络请求错误: {url} - {details}")]
    NetworkError {
        url: String,
        details: String,
    },

    #[error("HTTP 错误: {url} 返回状态码 {status}")]
    HttpError {
        url: String,
        status: reqwest::StatusCode,
    },

    #[error("文件操作错误: {path} - {details}")]
    FileError {
        path: PathBuf,
        details: String,
    },

    #[error("IO 错误: {0}")]
    IoError(#[from] std::io::Error),
}

pub type Result<T = (), E = AppError> = std::result::Result<T, E>;
```

---

## 开发指南

### 本地开发环境

#### 1. 克隆项目

```bash
git clone <repository-url>
cd calendar
```

#### 2. 安装开发依赖

```bash
# 安装 pre-commit hooks
pip install pre-commit
pre-commit install

# 或使用 cargo
cargo install pre-commit
pre-commit install
```

#### 3. 运行测试

```bash
# 运行所有测试
cargo test

# 运行特定测试
cargo test test_parse_config

# 显示测试输出
cargo test -- --nocapture

# 运行文档测试
cargo test --doc
```

#### 4. 代码检查

```bash
# 格式化代码
cargo fmt

# 检查代码质量
cargo clippy

# 运行所有检查
cargo fmt --check
cargo clippy -- -D warnings
```

#### 5. 生成文档

```bash
# 生成并打开文档
cargo doc --open

# 只生成文档
cargo doc
```

### 添加新功能

#### 1. 创建新模块

```rust
// src/new_module.rs
//! 新模块描述

use crate::error::{AppError, Result};

pub fn new_function() -> Result<()> {
    // 实现功能
    Ok(())
}
```

#### 2. 导出模块

```rust
// src/lib.rs
pub mod new_module;
```

#### 3. 添加测试

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_function() {
        assert!(new_function().is_ok());
    }
}
```

### 调试技巧

#### 1. 启用调试日志

```bash
./target/release/calendar -l trace run
```

#### 2. 使用 rust-analyzer

在 VSCode 中安装 `rust-analyzer` 扩展，获得：
- 代码补全
- 类型提示
- 实时错误检查
- 跳转到定义

#### 3. 使用调试器

```bash
# 使用 lldb
rust-lldb target/debug/calendar run

# 使用 gdb
rust-gdb target/debug/calendar run
```

### 性能分析

#### 1. 使用 flamegraph

```bash
# 安装 flamegraph
cargo install flamegraph

# 生成火焰图
cargo flamegraph -- run
```

#### 2. 使用 criterion 基准测试

```bash
# 添加到 Cargo.toml
[dev-dependencies]
criterion = "0.5"

# 运行基准测试
cargo bench
```

---

## 部署指南

### 编译优化

#### 1. 发布版本

```bash
# 标准发布版本
cargo build --release

# 优化体积（已在 Cargo.toml 中配置）
cargo build --release
```

#### 2. 交叉编译

**Linux 到 Windows:**

```bash
# 添加 Windows 目标
rustup target add x86_64-pc-windows-gnu

# 安装交叉编译工具链
sudo apt install mingw-w64

# 编译
cargo build --release --target x86_64-pc-windows-gnu
```

**Linux 到 macOS:**

```bash
# 添加 macOS 目标
rustup target add x86_64-apple-darwin aarch64-apple-darwin

# 编译
cargo build --release --target x86_64-apple-darwin
cargo build --release --target aarch64-apple-darwin
```

### Docker 部署

#### 1. 创建 Dockerfile

```dockerfile
FROM rust:1.75 as builder

WORKDIR /app
COPY . .

RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/calendar /usr/local/bin/

WORKDIR /data

CMD ["calendar", "run"]
```

#### 2. 构建镜像

```bash
docker build -t calendar-downloader .
```

#### 3. 运行容器

```bash
docker run -v $(pwd)/config.toml:/app/config.toml \
           -v $(pwd)/images:/data/images \
           calendar-downloader
```

### CI/CD

项目包含 GitHub Actions 配置 (`.github/workflows/ci.yml`)：

- ✅ 自动运行测试
- ✅ 代码格式检查
- ✅ Clippy 静态分析
- ✅ 跨平台构建
- ✅ 安全扫描

---

## 故障排除

### 常见问题

#### 1. 编译错误

**问题：** `error: linker 'cc' not found`

**解决方案：**

```bash
# Ubuntu/Debian
sudo apt install build-essential

# Fedora
sudo dnf install gcc

# macOS
xcode-select --install
```

#### 2. 网络超时

**问题：** 下载经常超时

**解决方案：**

```toml
# 增加超时时间
timeout = 60

# 增加重试次数
max_retries = 5

# 降低并发数
max_concurrent = 3
```

#### 3. EXIF 修改失败

**问题：** 无法修改 EXIF 信息

**解决方案：**

```bash
# 检查文件权限
ls -l image.jpg

# 确保文件可写
chmod +w image.jpg

# 使用 --download-only 跳过 EXIF 修改
./target/release/calendar run --download-only
```

#### 4. 配置文件错误

**问题：** 配置文件无法加载

**解决方案：**

```bash
# 验证配置文件
./target/release/calendar config --validate

# 检查日期格式（必须是 YYYY-MM-DD）
start_date = "2024-01-01"  # 正确
start_date = "2024/01/01"  # 错误
```

#### 5. 内存不足

**问题：** 大量下载时内存不足

**解决方案：**

```toml
# 降低并发数
max_concurrent = 1
```

### 日志级别

使用不同的日志级别获取详细信息：

```bash
# TRACE - 最详细的日志
./target/release/calendar -l trace run

# DEBUG - 调试信息
./target/release/calendar -l debug run

# INFO - 一般信息（默认）
./target/release/calendar -l info run

# WARN - 警告信息
./target/release/calendar -l warn run

# ERROR - 只显示错误
./target/release/calendar -l error run
```

### 获取帮助

```bash
# 查看帮助信息
./target/release/calendar --help

# 查看特定命令帮助
./target/release/calendar run --help
./target/release/calendar process --help
./target/release/calendar config --help

# 查看版本信息
./target/release/calendar --version
```

---

## 性能基准

### 下载速度

| 并发数 | 测试场景 | 下载速度 | 成功率 |
|--------|----------|----------|--------|
| 1 | 100 张图片 | ~2.5 张/秒 | 100% |
| 3 | 100 张图片 | ~7 张/秒 | 100% |
| 5 | 100 张图片 | ~10 张/秒 | 99% |
| 10 | 100 张图片 | ~15 张/秒 | 98% |
| 15 | 100 张图片 | ~18 张/秒 | 95% |

### 二进制大小

| 版本 | 大小 | 说明 |
|------|------|------|
| Debug | ~15MB | 包含调试符号 |
| Release | ~4.9MB | 优化版本 |
| Release + UPX | ~2.5MB | 压缩版本 |

### 内存使用

| 并发数 | 内存使用 |
|--------|----------|
| 1 | ~10MB |
| 5 | ~25MB |
| 10 | ~45MB |
| 15 | ~65MB |

---

## 贡献指南

### 代码规范

- 使用 `cargo fmt` 格式化代码
- 通过 `cargo clippy` 检查
- 添加单元测试
- 更新文档

### 提交规范

```
feat: 添加新功能
fix: 修复错误
docs: 更新文档
style: 代码格式（不影响功能）
refactor: 重构
perf: 性能优化
test: 添加测试
chore: 构建过程或辅助工具
```

### Pull Request 流程

1. Fork 项目
2. 创建特性分支
3. 提交更改
4. 推送到分支
5. 创建 Pull Request

---

## 许可证

本项目采用 MIT 许可证。

---

## 致谢

感谢以下开源项目：

- [tokio](https://tokio.rs/) - 异步运行时
- [reqwest](https://docs.rs/reqwest/) - HTTP 客户端
- [clap](https://docs.rs/clap/) - 命令行参数解析
- [chrono](https://docs.rs/chrono/) - 日期时间处理
- [little_exif](https://docs.rs/little_exif/) - EXIF 读写
- [tracing](https://docs.rs/tracing/) - 结构化日志
- [indicatif](https://docs.rs/indicatif/) - 进度条显示
- [thiserror](https://docs.rs/thiserror/) - 错误处理
- [image](https://docs.rs/image/) - 图片处理