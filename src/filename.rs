//! 文件名格式化器
//!
//! 支持将占位符替换为实际日期值，生成文件名和 URL。
//!
//! 支持的占位符格式：
//! - `{yyyy}` 或 `{year}` → 四位年份 (如: 2024)
//! - `{yy}` → 两位年份 (如: 24)
//! - `{mm}` 或 `{month:02}` → 两位月份 (如: 01, 06, 12)
//! - `{m}` 或 `{month}` → 不补位的月份 (如: 1, 6, 12)
//! - `{dd}` 或 `{day:02}` → 两位日期 (如: 01, 15, 31)
//! - `{d}` 或 `{day}` → 不补位的日期 (如: 1, 15, 31)

use chrono::{Datelike, NaiveDate};
use regex::Regex;

use crate::error::{AppError, Result};

/// 文件名格式化器
#[derive(Debug, Clone)]
pub struct FilenameFormatter {
    /// 格式字符串
    format: String,
    /// 占位符正则表达式
    placeholder_regex: Regex,
}

impl FilenameFormatter {
    /// 创建新的格式化器
    pub fn new(format: &str) -> Result<Self> {
        // 验证格式字符串
        if format.is_empty() {
            return Err(AppError::FilenameFormatError {
                format: format.to_string(),
                details: "格式字符串不能为空".to_string(),
            });
        }

        // 构建匹配占位符的正则表达式
        // 匹配类似 {year}、{month:02}、{dd} 等模式
        let regex_str = r"\{([^}]+)\}";
        let placeholder_regex = Regex::new(regex_str).map_err(|e| AppError::RegexError(e))?;

        Ok(Self {
            format: format.to_string(),
            placeholder_regex,
        })
    }

    /// 格式化日期为文件名
    ///
    /// # 示例
    /// ```
    /// # use chrono::NaiveDate;
    /// # use calendar::filename::FilenameFormatter;
    /// let formatter = FilenameFormatter::new("{yyyy}{mm}{dd}.jpg").unwrap();
    /// let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    /// assert_eq!(formatter.format(&date), "20240615.jpg");
    /// ```
    pub fn format(&self, date: &NaiveDate) -> String {
        let mut result = self.format.clone();

        // 处理常见的占位符格式
        // 必须按照特定顺序处理，避免部分替换

        // {yyyy} -> 四位年份
        result = result.replace("{yyyy}", &date.year().to_string());
        result = result.replace("{year}", &date.year().to_string());

        // {yy} -> 两位年份
        let two_digit_year = (date.year() % 100).abs();
        result = result.replace("{yy}", &format!("{:02}", two_digit_year));

        // {mm} -> 两位月份
        result = result.replace("{mm}", &format!("{:02}", date.month()));

        // {m} -> 不补位的月份
        result = result.replace("{m}", &date.month().to_string());
        result = result.replace("{month}", &date.month().to_string());

        // {dd} -> 两位日期
        result = result.replace("{dd}", &format!("{:02}", date.day()));

        // {d} -> 不补位的日期
        result = result.replace("{d}", &date.day().to_string());
        result = result.replace("{day}", &date.day().to_string());

        // 处理带格式化修饰符的占位符 (如 {month:02}, {day:02})
        result = self.format_variable_width_placeholders(&result, date);

        result
    }

    /// 格式化日期为 URL
    ///
    /// 与 `format` 类似，但针对 URL 使用场景进行优化
    pub fn format_url(&self, date: &NaiveDate) -> String {
        self.format(date)
    }

    /// 处理带宽度修饰符的占位符
    ///
    /// 支持格式：{name:02}、{name:03} 等
    fn format_variable_width_placeholders(&self, format_str: &str, date: &NaiveDate) -> String {
        let mut result = format_str.to_string();

        // 查找所有符合 {name:width} 模式的占位符
        let captures = self.placeholder_regex.captures_iter(format_str);

        for cap in captures {
            let full_match = cap.get(0).unwrap().as_str();
            let placeholder = cap.get(1).unwrap().as_str();

            // 检查是否包含 :width 格式
            if let Some(colon_pos) = placeholder.find(':') {
                let name = &placeholder[..colon_pos];
                let width_str = &placeholder[colon_pos + 1..];

                // 解析宽度值
                if let Ok(width) = width_str.parse::<usize>() {
                    let value = match name {
                        "year" => date.year().to_string(),
                        "month" => format!("{:0width$}", date.month(), width = width),
                        "day" => format!("{:0width$}", date.day(), width = width),
                        _ => full_match.to_string(),
                    };

                    // 替换完整匹配
                    result = result.replace(full_match, &value);
                }
            }
        }

        result
    }

    /// 获取格式字符串
    pub fn format_str(&self) -> &str {
        &self.format
    }
}

impl TryFrom<&str> for FilenameFormatter {
    type Error = AppError;

    fn try_from(format: &str) -> Result<Self> {
        Self::new(format)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_date(year: i32, month: u32, day: u32) -> NaiveDate {
        chrono::NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    #[test]
    fn test_basic_format_yyyy_mm_dd() {
        let formatter = FilenameFormatter::new("{yyyy}{mm}{dd}.jpg").unwrap();
        let date = test_date(2024, 6, 15);
        assert_eq!(formatter.format(&date), "20240615.jpg");
    }

    #[test]
    fn test_basic_format_year_month_day() {
        let formatter = FilenameFormatter::new("{year}_{month}_{day}.png").unwrap();
        let date = test_date(2024, 6, 5);
        assert_eq!(formatter.format(&date), "2024_6_5.png");
    }

    #[test]
    fn test_two_digit_year() {
        let formatter = FilenameFormatter::new("{yy}{mm}{dd}.jpg").unwrap();
        let date = test_date(2024, 6, 15);
        assert_eq!(formatter.format(&date), "240615.jpg");

        let date2 = test_date(1999, 12, 31);
        assert_eq!(formatter.format(&date2), "991231.jpg");
    }

    #[test]
    fn test_with_zero_padding_modifier() {
        let formatter = FilenameFormatter::new("{year}_{month:02}_{day:02}.png").unwrap();
        let date = test_date(2024, 6, 5);
        assert_eq!(formatter.format(&date), "2024_06_05.png");
    }

    #[test]
    fn test_mixed_placeholders() {
        let formatter = FilenameFormatter::new("{yy}/{mm}/{dd}.jpg").unwrap();
        let date = test_date(2024, 1, 1);
        assert_eq!(formatter.format(&date), "24/01/01.jpg");
    }

    #[test]
    fn test_without_zero_padding() {
        let formatter = FilenameFormatter::new("{y}{m}{d}.jpg").unwrap();
        let date = test_date(2024, 12, 31);
        // {y} 会保持 {y} 不变，因为没有这个占位符定义
        assert_eq!(formatter.format(&date), "{y}1231.jpg");
    }

    #[test]
    fn test_single_and_double_digit_dates() {
        let formatter = FilenameFormatter::new("{yyyy}-{mm}-{dd}.jpg").unwrap();

        // 单数字月和日
        let date1 = test_date(2024, 1, 5);
        assert_eq!(formatter.format(&date1), "2024-01-05.jpg");

        // 双数字月和日
        let date2 = test_date(2024, 12, 31);
        assert_eq!(formatter.format(&date2), "2024-12-31.jpg");
    }

    #[test]
    fn test_url_formatting() {
        let formatter =
            FilenameFormatter::new("https://example.com/{year}/{month:02}/{day:02}.jpg")
                .unwrap();
        let date = test_date(2024, 6, 5);
        assert_eq!(
            formatter.format_url(&date),
            "https://example.com/2024/06/05.jpg"
        );
    }

    #[test]
    fn test_empty_format_string() {
        let result = FilenameFormatter::new("");
        assert!(result.is_err());
        if let Err(AppError::FilenameFormatError { format, .. }) = result {
            assert_eq!(format, "");
        } else {
            panic!("Expected FilenameFormatError");
        }
    }

    #[test]
    fn test_format_str() {
        let formatter = FilenameFormatter::new("{yyyy}{mm}{dd}.jpg").unwrap();
        assert_eq!(formatter.format_str(), "{yyyy}{mm}{dd}.jpg");
    }

    #[test]
    fn test_try_from() {
        let formatter: Result<FilenameFormatter> = "{yyyy}{mm}{dd}.jpg".try_into();
        assert!(formatter.is_ok());

        let date = test_date(2024, 6, 15);
        assert_eq!(formatter.unwrap().format(&date), "20240615.jpg");
    }

    #[test]
    fn test_with_prefix_and_suffix() {
        let formatter = FilenameFormatter::new("photo_{yyyy}{mm}{dd}.jpg").unwrap();
        let date = test_date(2024, 6, 15);
        assert_eq!(formatter.format(&date), "photo_20240615.jpg");
    }

    #[test]
    fn test_three_digit_width() {
        let formatter = FilenameFormatter::new("{day:03}.jpg").unwrap();
        let date = test_date(2024, 6, 5);
        assert_eq!(formatter.format(&date), "005.jpg");
    }
}
