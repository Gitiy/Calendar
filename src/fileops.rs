//! 文件属性操作
//!
//! 使用 `filetime` 库实现跨平台的文件时间戳设置功能。
//! 支持修改文件的创建时间和最后修改时间。

use chrono::{DateTime, Utc};
use std::fs;
use std::path::Path;

use crate::error::{AppError, Result};

/// 设置文件的时间戳（创建时间和修改时间）
///
/// # 参数
/// - `path`: 文件路径
/// - `datetime`: 目标日期时间（UTC）
///
/// # 跨平台说明
/// - **Windows**: 设置创建时间 和修改时间
/// - **Unix/Linux**: 设置修改时间 和访问时间（无法设置创建时间）
///
/// # 示例
/// ```
/// # use chrono::{TimeZone, Utc};
/// # use std::path::Path;
/// # use calendar::fileops::set_file_timestamps;
/// // let datetime = Utc.with_ymd_and_hms(2024, 6, 15, 0, 0, 0).unwrap();
/// // set_file_timestamps(Path::new("photo.jpg"), datetime)?;
/// ```
pub fn set_file_timestamps(path: &Path, datetime: DateTime<Utc>) -> Result<()> {
    tracing::debug!(
        "设置文件时间戳: {:?} -> {}",
        path,
        datetime.format("%Y-%m-%d %H:%M:%S")
    );

    // 确保文件存在
    if !path.exists() {
        return Err(AppError::file_error(path, "文件不存在".to_string()));
    }

    // 将 DateTime<Utc> 转换为 FileTime
    let filetime = datetime_to_filetime(&datetime);

    // 设置文件时间戳
    #[cfg(target_os = "windows")]
    {
        // Windows: 设置创建时间和修改时间
        filetime::set_file_times(path, filetime, filetime)
            .map_err(|e| AppError::file_error(path, e.to_string()))?;
        tracing::debug!("已设置 Windows 文件时间戳（创建时间和修改时间）");
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        // Unix: 设置修改时间和访问时间
        filetime::set_file_times(path, filetime, filetime)
            .map_err(|e| AppError::file_error(path, e.to_string()))?;
        tracing::debug!("已设置 Unix 文件时间戳（修改时间和访问时间）");
    }

    // 获取更新后的时间戳进行验证
    let updated_metadata = fs::metadata(path)
        .map_err(|e| AppError::file_error(path, e.to_string()))?;

    #[cfg(target_os = "windows")]
    let updated_mtime = {
        use std::os::windows::fs::MetadataExt;
        let creation = updated_metadata.creation_time();
        let modified = updated_metadata.last_write_time();
        (creation, modified)
    };

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    let updated_mtime = {
        use std::os::unix::fs::MetadataExt;
        let mtime = updated_metadata.mtime();
        mtime
    };

    tracing::trace!("更新后的文件时间戳: {:?}", updated_mtime);

    Ok(())
}

/// 仅设置文件的最后修改时间
///
/// # 参数
/// - `path`: 文件路径
/// - `datetime`: 目标日期时间（UTC）
pub fn set_file_mtime(path: &Path, datetime: DateTime<Utc>) -> Result<()> {
    tracing::debug!(
        "设置文件修改时间: {:?} -> {}",
        path,
        datetime.format("%Y-%m-%d %H:%M:%S")
    );

    if !path.exists() {
        return Err(AppError::file_error(path, "文件不存在".to_string()));
    }

    let filetime = datetime_to_filetime(&datetime);

    // 仅设置修改时间
    filetime::set_file_mtime(path, filetime)
        .map_err(|e| AppError::file_error(path, e.to_string()))?;

    Ok(())
}

/// 仅设置文件的访问时间
///
/// # 参数
/// - `path`: 文件路径
/// - `datetime`: 目标日期时间（UTC）
pub fn set_file_atime(path: &Path, datetime: DateTime<Utc>) -> Result<()> {
    tracing::debug!(
        "设置文件访问时间: {:?} -> {}",
        path,
        datetime.format("%Y-%m-%d %H:%M:%S")
    );

    if !path.exists() {
        return Err(AppError::file_error(path, "文件不存在".to_string()));
    }

    let filetime = datetime_to_filetime(&datetime);

    // 仅设置访问时间
    filetime::set_file_atime(path, filetime)
        .map_err(|e| AppError::file_error(path, e.to_string()))?;

    Ok(())
}

/// 将 DateTime<Utc> 转换为 filetime::FileTime
fn datetime_to_filetime(datetime: &DateTime<Utc>) -> filetime::FileTime {
    // 获取 Unix 时间戳（秒）
    let timestamp = datetime.timestamp();

    // 获取纳秒部分
    let nsec = datetime.timestamp_subsec_nanos() as u32;

    filetime::FileTime::from_unix_time(timestamp, nsec)
}

/// 获取文件的修改时间
///
/// # 参数
/// - `path`: 文件路径
///
/// # 返回
/// 返回 Option<DateTime<Utc>>，如果获取失败则为 None
pub fn get_file_mtime(path: &Path) -> Result<Option<DateTime<Utc>>> {
    if !path.exists() {
        return Ok(None);
    }

    let metadata = fs::metadata(path)
        .map_err(|e| AppError::file_error(path, e.to_string()))?;

    #[cfg(target_os = "windows")]
    let mtime = {
        use std::os::windows::fs::MetadataExt;
        let win_time = metadata.last_write_time();
        windows_time_to_datetime(win_time)
    };

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    let mtime = {
        use std::os::unix::fs::MetadataExt;
        let timestamp = metadata.mtime();
        let nsec = metadata.mtime_nsec() as u32;
        Some(DateTime::from_timestamp(timestamp, nsec).unwrap())
    };

    Ok(mtime)
}

/// 将 Windows FILETIME 转换为 DateTime<Utc>
#[cfg(target_os = "windows")]
fn windows_time_to_datetime(win_time: u64) -> Option<DateTime<Utc>> {
    const WINDOWS_TICK: i64 = 10_000_000; // 100 纳秒
    const SEC_TO_UNIX_EPOCH: i64 = 116_444_73600; // 1601-01-01 到 1970-01-01 的秒数

    let timestamp = (win_time as i64 / WINDOWS_TICK) - SEC_TO_UNIX_EPOCH;
    DateTime::from_timestamp(timestamp, 0)
}

/// 创建目录（如果不存在）
///
/// # 参数
/// - `path`: 目录路径
pub fn ensure_dir_exists(path: &Path) -> Result<()> {
    if !path.exists() {
        fs::create_dir_all(path)
            .map_err(|e| AppError::file_error(path, e.to_string()))?;
        tracing::debug!("创建目录: {}", path.display());
    }
    Ok(())
}

/// 检查文件是否存在
///
/// # 参数
/// - `path`: 文件路径
///
/// # 返回
/// 返回 true 如果文件存在
pub fn file_exists(path: &Path) -> bool {
    path.exists() && path.is_file()
}

/// 获取文件大小
///
/// # 参数
/// - `path`: 文件路径
///
/// # 返回
/// 返回文件大小（字节），如果文件不存在则返回 None
pub fn get_file_size(path: &Path) -> Result<Option<u64>> {
    if !path.exists() {
        return Ok(None);
    }

    let metadata = fs::metadata(path)
        .map_err(|e| AppError::file_error(path, e.to_string()))?;

    Ok(Some(metadata.len()))
}

/// 删除文件
///
/// # 参数
/// - `path`: 文件路径
pub fn delete_file(path: &Path) -> Result<()> {
    if path.exists() {
        fs::remove_file(path)
            .map_err(|e| AppError::file_error(path, e.to_string()))?;
        tracing::debug!("删除文件: {}", path.display());
    }
    Ok(())
}

/// 复制文件
///
/// # 参数
/// - `src`: 源文件路径
/// - `dst`: 目标文件路径
pub fn copy_file(src: &Path, dst: &Path) -> Result<()> {
    ensure_dir_exists(dst.parent().unwrap_or(Path::new(".")))?;
    fs::copy(src, dst)
        .map_err(|e| AppError::file_error(dst, e.to_string()))?;
    tracing::debug!("复制文件: {} -> {}", src.display(), dst.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use tempfile::NamedTempFile;

    #[test]
    fn test_datetime_to_filetime() {
        let datetime = Utc
            .with_ymd_and_hms(2024, 6, 15, 12, 30, 45)
            .unwrap();
        let filetime = datetime_to_filetime(&datetime);

        assert_ne!(filetime.unix_seconds(), 0);
        assert_eq!(filetime_nanoseconds(filetime), datetime.timestamp_subsec_nanos());
    }

    fn filetime_nanoseconds(ft: filetime::FileTime) -> u32 {
        const NANOS_PER_SEC: u32 = 1_000_000_000;
        ((ft.nanoseconds() as u32) * NANOS_PER_SEC) / 1_000_000_000
    }

    #[test]
    fn test_set_file_timestamps() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        // 创建一个测试文件
        fs::write(path, b"test content").unwrap();

        let datetime = Utc
            .with_ymd_and_hms(2024, 6, 15, 0, 0, 0)
            .unwrap();

        let result = set_file_timestamps(path, datetime);
        assert!(result.is_ok());
    }

    #[test]
    fn test_set_file_timestamps_nonexistent() {
        let path = Path::new("/nonexistent/file.jpg");
        let datetime = Utc
            .with_ymd_and_hms(2024, 6, 15, 0, 0, 0)
            .unwrap();

        let result = set_file_timestamps(path, datetime);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_file_mtime_nonexistent() {
        let path = Path::new("/nonexistent/file.jpg");
        let result = get_file_mtime(path).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_file_exists() {
        let temp_file = NamedTempFile::new().unwrap();
        assert!(file_exists(temp_file.path()));
        assert!(!file_exists(Path::new("/nonexistent/file")));
    }

    #[test]
    fn test_get_file_size() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();
        let content = b"Hello, World!";

        fs::write(path, content).unwrap();

        let size = get_file_size(path).unwrap().unwrap();
        assert_eq!(size, content.len() as u64);
    }

    #[test]
    fn test_ensure_dir_exists() {
        let temp_dir = tempfile::tempdir().unwrap();
        let new_dir = temp_dir.path().join("nested").join("dir");

        assert!(!new_dir.exists());
        let result = ensure_dir_exists(&new_dir);
        assert!(result.is_ok());
        assert!(new_dir.exists());
    }

    #[test]
    fn test_copy_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let src = temp_dir.path().join("source.txt");
        let dst = temp_dir.path().join("subdir").join("dest.txt");

        fs::write(&src, b"test content").unwrap();

        let result = copy_file(&src, &dst);
        assert!(result.is_ok());
        assert!(dst.exists());
        assert_eq!(fs::read_to_string(&dst).unwrap(), "test content");
    }
}
