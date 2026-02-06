//! 图片验证模块
//!
//! 用于验证下载的图片是否完整和有效。

use std::path::Path;
use crate::error::{AppError, Result};

/// 图片验证结果
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationResult {
    /// 图片有效
    Valid,
    /// 图片无效
    Invalid(String),
}

/// 图片验证器
pub struct ImageValidator;

impl ImageValidator {
    /// 验证图片文件
    ///
    /// # 参数
    /// - `path`: 图片文件路径
    ///
    /// # 返回
    /// 返回验证结果
    pub fn validate(path: &Path) -> Result<ValidationResult> {
        // 检查文件是否存在
        if !path.exists() {
            return Ok(ValidationResult::Invalid("文件不存在".to_string()));
        }

        // 检查文件大小
        let metadata = std::fs::metadata(path)
            .map_err(|e| AppError::file_error(path, e.to_string()))?;

        if metadata.len() == 0 {
            return Ok(ValidationResult::Invalid("文件为空".to_string()));
        }

        // 检查文件扩展名
        if let Some(ext) = path.extension() {
            let ext_lower = ext.to_string_lossy().to_lowercase();
            let valid_extensions = ["jpg", "jpeg", "png", "gif", "webp", "bmp", "tiff", "tif"];
            if !valid_extensions.contains(&ext_lower.as_str()) {
                return Ok(ValidationResult::Invalid(format!("不支持的文件格式: {}", ext_lower)));
            }
        }

        // 检查文件大小是否合理（至少 1KB，最大 50MB）
        let file_size = metadata.len();
        if file_size < 1024 {
            return Ok(ValidationResult::Invalid("文件太小，可能已损坏".to_string()));
        }
        if file_size > 50 * 1024 * 1024 {
            return Ok(ValidationResult::Invalid("文件过大".to_string()));
        }

        Ok(ValidationResult::Valid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[test]
    fn test_validate_nonexistent() {
        let result = ImageValidator::validate(Path::new("/nonexistent.jpg"));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ValidationResult::Invalid("文件不存在".to_string()));
    }

    #[test]
    fn test_validate_empty_file() {
        let temp_file = NamedTempFile::new().unwrap();
        let result = ImageValidator::validate(temp_file.path());
        assert!(result.is_ok());
        assert!(result.unwrap() == ValidationResult::Invalid("文件为空".to_string()));
    }

    #[test]
    fn test_validate_too_small_file() {
        let temp_file = NamedTempFile::with_suffix(".jpg").unwrap();
        write!(temp_file, "small").unwrap();
        let result = ImageValidator::validate(temp_file.path());
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), ValidationResult::Invalid(_)));
    }

    #[test]
    fn test_validate_valid_size_file() {
        let temp_file = NamedTempFile::with_suffix(".jpg").unwrap();
        let data = vec![0u8; 2048]; // 2KB
        std::fs::write(temp_file.path(), data).unwrap();
        let result = ImageValidator::validate(temp_file.path());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ValidationResult::Valid);
    }
}