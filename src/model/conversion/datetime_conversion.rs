//! DateTime字段转换工具
//!
//! 处理String到DateTime的转换，支持时区偏移

use crate::error::{QuickDbError, QuickDbResult};
use crate::types::DataValue;
use crate::model::conversion::ToDataValue;

/// 将String输入转换为DateTimeWithTz字段的DataValue
///
/// 这个函数专门用于DateTimeWithTz字段的智能转换：
/// - 如果输入是DateTime<Utc>，直接使用
/// - 如果输入是String，尝试解析为RFC3339格式并转换为DateTime<Utc>
/// - 转换失败时提供清晰的错误信息
pub fn convert_string_to_datetime_with_tz<T: std::fmt::Debug + ToDataValue>(
    value: &T,
    timezone_offset: &str,
) -> QuickDbResult<DataValue> {
    // 首先尝试直接转换为DataValue，看看是否已经是DateTime类型
    let data_value = value.to_data_value();

    match data_value {
        DataValue::DateTime(dt) => {
            // 对于DateTimeWithTz字段，需要应用时区转换
            apply_timezone_to_datetime(dt, timezone_offset)
        },
        DataValue::String(s) => {
            // 如果是字符串，尝试解析为DateTime
            parse_string_to_datetime_with_tz(&s, timezone_offset)
        },
        DataValue::Null => {
            // Null值（对应Option::None），这是合法的
            Ok(DataValue::Null)
        },
        _ => {
            // 其他类型，返回错误
            Err(QuickDbError::ValidationError {
                field: "DateTimeWithTz字段".to_string(),
                message: format!(
                    "不支持的数据类型 {:?}，期望DateTime<Utc>或RFC3339格式的字符串",
                    std::any::type_name_of_val(value)
                ),
            })
        }
    }
}

/// 将UTC DateTime应用时区偏移
///
/// 对于DateTimeWithTz字段，将UTC时间转换为指定时区的本地时间
fn apply_timezone_to_datetime(
    utc_dt: chrono::DateTime<chrono::FixedOffset>,
    timezone_offset: &str,
) -> QuickDbResult<DataValue> {
    // 解析时区偏移
    let tz_offset = parse_timezone_offset(timezone_offset)?;

    // 解析目标时区偏移
    let target_offset_seconds = tz_offset;
    let target_tz = chrono::FixedOffset::east(target_offset_seconds);

    // 转换为目标时区
    let target_dt = utc_dt.with_timezone(&target_tz);
    Ok(DataValue::DateTime(target_dt))
}

/// 解析字符串为DateTime，应用时区偏移
fn parse_string_to_datetime_with_tz(
    datetime_str: &str,
    timezone_offset: &str,
) -> QuickDbResult<DataValue> {
    // 首先尝试直接解析为RFC3339格式（包含时区信息）
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(datetime_str) {
        // 保持FixedOffset格式
        return Ok(DataValue::DateTime(dt.with_timezone(&chrono::FixedOffset::east(0))));
    }

    // 如果RFC3339解析失败，尝试解析为本地时间格式并应用指定的时区偏移
    let local_datetime_formats = [
        "%Y-%m-%d %H:%M:%S",      // 2024-01-15 14:30:00
        "%Y-%m-%d %H:%M:%S%.3f",  // 2024-01-15 14:30:00.123
        "%Y-%m-%d %H:%M",         // 2024-01-15 14:30
        "%Y-%m-%dT%H:%M:%S",      // 2024-01-15T14:30:00
        "%Y-%m-%dT%H:%M:%S%.3f",  // 2024-01-15T14:30:00.123
        "%Y-%m-%dT%H:%M",         // 2024-01-15T14:30
        "%Y/%m/%d %H:%M:%S",      // 2024/01/15 14:30:00
        "%Y/%m/%d %H:%M:%S%.3f",  // 2024/01/15 14:30:00.123
        "%Y/%m/%d %H:%M",         // 2024/01/15 14:30
    ];

    for format in &local_datetime_formats {
        if let Ok(naive_dt) = chrono::NaiveDateTime::parse_from_str(datetime_str, format) {
            // 成功解析为本地时间，现在需要应用时区偏移

            // 解析时区偏移
            let tz_offset = parse_timezone_offset(timezone_offset)?;

            // 创建带时区的DateTime
            if let Some(offset) = chrono::FixedOffset::west_opt(tz_offset) {
                let aware_dt = naive_dt.and_local_timezone(offset)
                    .single()
                    .ok_or_else(|| QuickDbError::ValidationError {
                        field: "DateTimeWithTz字段".to_string(),
                        message: format!(
                            "时间 '{}' 在时区 '{}' 下存在歧义（夏令时等）",
                            datetime_str, timezone_offset
                        ),
                    })?;

                // 保持FixedOffset格式
                return Ok(DataValue::DateTime(aware_dt.with_timezone(&chrono::FixedOffset::east(0))));
            } else {
                return Err(QuickDbError::ValidationError {
                    field: "DateTimeWithTz字段".to_string(),
                    message: format!("无效的时区偏移: {}", timezone_offset),
                });
            }
        }
    }

    // 所有解析尝试都失败
    Err(QuickDbError::ValidationError {
        field: "DateTimeWithTz字段".to_string(),
        message: format!(
            "无法解析日期时间字符串 '{}'。支持的格式：\n\
            1. RFC3339格式（推荐）：2024-01-15T14:30:00+08:00\n\
            2. 本地时间格式：2024-01-15 14:30:00\n\
            3. 其他常见格式：2024-01-15T14:30:00、2024/01/15 14:30:00等",
            datetime_str
        ),
    })
}

/// 解析时区偏移字符串为秒数
///
/// # 参数
/// * `timezone_offset` - 时区偏移字符串，格式如 "+08:00"、"-05:30"
///
/// # 返回值
/// * `Ok(i32)` - 东经为负数，西经为正数（与chrono::FixedOffset::west_opt一致）
/// * `Err(QuickDbError)` - 格式错误时返回详细错误信息
pub fn parse_timezone_offset(timezone_offset: &str) -> QuickDbResult<i32> {
    // 验证时区偏移格式
    static TZ_REGEX: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let regex = TZ_REGEX.get_or_init(|| {
        regex::Regex::new(r"^([+-])(\d{2}):(\d{2})$").unwrap()
    });

    if !regex.is_match(timezone_offset) {
        return Err(QuickDbError::ValidationError {
            field: "时区偏移".to_string(),
            message: format!(
                "无效的时区偏移格式: '{}'。期望格式: [+/-]HH:MM，例如: +08:00、-05:30",
                timezone_offset
            ),
        });
    }

    let caps = regex.captures(timezone_offset).unwrap();
    let sign = caps.get(1).unwrap().as_str();
    let hours: i32 = caps.get(2).unwrap().as_str().parse()
        .map_err(|_| QuickDbError::ValidationError {
            field: "时区偏移".to_string(),
            message: format!("无效的小时数: {}", caps.get(2).unwrap().as_str()),
        })?;
    let minutes: i32 = caps.get(3).unwrap().as_str().parse()
        .map_err(|_| QuickDbError::ValidationError {
            field: "时区偏移".to_string(),
            message: format!("无效的分钟数: {}", caps.get(3).unwrap().as_str()),
        })?;

    // 验证范围
    if hours > 23 || minutes > 59 {
        return Err(QuickDbError::ValidationError {
            field: "时区偏移".to_string(),
            message: format!(
                "时区偏移超出范围: {}{}:{}.小时范围: 0-23，分钟范围: 0-59",
                sign, hours, minutes
            ),
        });
    }

    let total_seconds = hours * 3600 + minutes * 60;

    // 根据符号确定偏移方向
    // 注意：chrono::FixedOffset::west_opt使用西偏移（东经为负数）
    match sign {
        "+" => Ok(-total_seconds), // 东经，转换为负数
        "-" => Ok(total_seconds),  // 西经，保持正数
        _ => unreachable!(), // 正则表达式已经确保只有+或-
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_timezone_offset() {
        assert_eq!(parse_timezone_offset("+08:00").unwrap(), -8 * 3600);
        assert_eq!(parse_timezone_offset("-05:00").unwrap(), 5 * 3600);
        assert_eq!(parse_timezone_offset("+05:30").unwrap(), -(5 * 3600 + 30 * 60));
        assert_eq!(parse_timezone_offset("-09:30").unwrap(), 9 * 3600 + 30 * 60);

        // 测试无效格式
        assert!(parse_timezone_offset("08:00").is_err()); // 缺少符号
        assert!(parse_timezone_offset("+8:00").is_err()); // 小时数不足两位
        assert!(parse_timezone_offset("+08:0").is_err()); // 分钟数不足两位
        assert!(parse_timezone_offset("+24:00").is_err()); // 小时数超出范围
        assert!(parse_timezone_offset("+08:60").is_err()); // 分钟数超出范围
    }
}