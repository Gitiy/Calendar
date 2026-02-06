//! EXIF 信息修改
//!
//! 使用 `little_exif` 库读取和修改图片的 EXIF 数据。
//! 主要功能是将 `DateTimeOriginal` 等日期字段设置为指定日期。

use chrono::{NaiveDate, NaiveDateTime};
use std::path::Path as StdPath;

use little_exif::metadata::Metadata;
use little_exif::exif_tag::ExifTag;

use crate::Result;

/// 检查文件是否支持 EXIF
pub fn supports_exif(path: &StdPath) -> bool {
    // 通过扩展名判断
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        let ext_lower = ext.to_lowercase();
        matches!(
            ext_lower.as_str(),
            "jpg" | "jpeg" | "tif" | "tiff" | "png" | "heic" | "heif"
        )
    } else {
        false
    }
}

/// 设置图片的 EXIF DateTimeOriginal 字段
///
/// 使用 `little_exif` 库将图片的 `DateTimeOriginal` 等日期字段设置为指定日期。
/// 注意：此实现会创建新的 EXIF 数据并追加到文件，原始 EXIF 数据会被保留。
pub fn set_exif_datetime(path: &StdPath, date: &NaiveDateTime) -> Result<()> {
    // 检查文件是否支持 EXIF
    if !supports_exif(path) {
        tracing::debug!("文件类型不支持 EXIF: {:?}", path);
        return Ok(());
    }

    // 格式化日期时间字符串 (EXIF 标准格式: "YYYY:MM:DD HH:MM:SS")
    let datetime_str = date.format("%Y:%m:%d %H:%M:%S").to_string();

    tracing::info!(
        "设置 EXIF 时间: {:?} -> {}",
        path,
        datetime_str
    );

    // 读取 EXIF 元数据并设置新标签
    let mut metadata = Metadata::new_from_path(path).unwrap_or_else(|_| Metadata::new());
    metadata.set_tag(ExifTag::DateTimeOriginal(datetime_str.clone()));
    metadata.set_tag(ExifTag::CreateDate(datetime_str.clone()));
    metadata.set_tag(ExifTag::ModifyDate(datetime_str.clone()));
    metadata.set_tag(ExifTag::Artist("OWSPACE".to_string()));
    metadata.set_tag(ExifTag::ImageDescription(date.format("%Y-%m-%d").to_string()));
        

    // 写入 EXIF 数据到文件
    metadata.write_to_file(path).map_err(|e| {
        crate::AppError::file_error(
            path,
            format!("写入 EXIF 数据失败: {}", e),
        )
    })?;

    tracing::debug!("EXIF 日期设置成功: {:?}", path);
    Ok(())
}

/// 获取图片的 EXIF DateTimeOriginal 字段
pub fn get_exif_datetime(path: &StdPath) -> Result<Option<NaiveDate>> {
    tracing::debug!("获取 EXIF 时间: {:?}", path);

    // 检查文件是否存在且支持 EXIF
    if !supports_exif(path) || !path.exists() {
        return Ok(None);
    }

    // 从文件读取 EXIF 元数据
    let metadata = Metadata::new_from_path(path).map_err(|e| {
        crate::AppError::file_error(
            path,
            format!("读取 EXIF 数据失败: {}", e),
        )
    })?;

    // 尝试获取 DateTimeOriginal
    // get_tag 返回迭代器，使用 next() 获取第一个匹配项
    if let Some(datetime_tag) = metadata.get_tag(&ExifTag::DateTimeOriginal(String::new())).next() {
        // 通过模式匹配获取 DateTimeOriginal 中的值
        match datetime_tag {
            ExifTag::DateTimeOriginal(datetime_str) => {
                tracing::debug!("原始 EXIF DateTimeOriginal: {}", datetime_str);
                if let Some(date) = parse_exif_datetime(datetime_str) {
                    return Ok(Some(date));
                }
            }
            _ => {}
        }
    }

    Ok(None)
}

/// 解析 EXIF 日期时间字符串
///
/// EXIF 标准格式为 `YYYY:MM:DD HH:MM:SS`
fn parse_exif_datetime(datetime_str: &str) -> Option<NaiveDate> {
    // 尝试标准 EXIF 格式: "YYYY:MM:DD HH:MM:SS"
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(datetime_str, "%Y:%m:%d %H:%M:%S") {
        return Some(dt.date());
    }

    // 尝试替代格式: "YYYY-MM-DD HH:MM:SS"
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(datetime_str, "%Y-%m-%d %H:%M:%S") {
        return Some(dt.date());
    }

    tracing::warn!("无法解析 EXIF 日期时间: {}", datetime_str);
    None
}

#[cfg(test)]
mod tests {

    use little_exif::metadata;

    use super::*;

    #[test]
    fn test_supports_exif_jpg() {
        assert!(supports_exif(StdPath::new("test.jpg")));
        assert!(supports_exif(StdPath::new("test.jpeg")));
        assert!(supports_exif(StdPath::new("photo.JPG")));
    }

    #[test]
    fn test_supports_exif_tiff() {
        assert!(supports_exif(StdPath::new("test.tif")));
        assert!(supports_exif(StdPath::new("test.tiff")));
    }

    #[test]
    fn test_supports_exif_png() {
        assert!(supports_exif(StdPath::new("test.png")));
    }

    #[test]
    fn test_supports_exif_no_extension() {
        assert!(!supports_exif(StdPath::new("test")));
    }

    #[test]
    fn test_supports_exif_unsupported_format() {
        assert!(!supports_exif(StdPath::new("test.txt")));
        assert!(!supports_exif(StdPath::new("test.pdf")));
    }

    #[test]
    fn test_parse_exif_datetime() {
        let p = StdPath::new("/mnt/d/WorkSpace/copilot/calendar/owspace_20150218.jpg");
        let date=NaiveDate::from_ymd_opt(2015, 2, 18).unwrap().and_hms_opt(8, 0, 0).unwrap();
        println!("{}", date.format("%Y:%m:%d  %H:%M:%S").to_string());
        let mut metadata = metadata::Metadata::new_from_path(p).unwrap();
        metadata.set_tag(ExifTag::DateTimeOriginal(date.format("%Y:%m:%d %H:%M:%S").to_string()));
        metadata.set_tag(ExifTag::CreateDate(date.format("%Y:%m:%d %H:%M:%S").to_string()));
        metadata.set_tag(ExifTag::ModifyDate(date.format("%Y:%m:%d %H:%M:%S").to_string()));
        metadata.set_tag(ExifTag::Artist("OWSPACE".to_string()));
        metadata.set_tag(ExifTag::ImageDescription(date.format("%Y-%m-%d").to_string()));
        metadata.write_to_file(p).unwrap();

    }
}
