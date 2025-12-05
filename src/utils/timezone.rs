//! 时区相关的工具函数

use crate::QuickDbError;
use crate::QuickDbResult;
use crate::types::DataValue;
use chrono::{DateTime, FixedOffset, Utc};

macro_rules! debug_log {
    ($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        rat_logger::debug!($($arg)*);
    };
}

/// 将时区偏移字符串转换为秒数
///
/// # 参数
/// * `timezone_offset` - 时区偏移，格式 "+08:00", "-05:00"
pub fn parse_timezone_offset_to_seconds(timezone_offset: &str) -> crate::error::QuickDbResult<i32> {
    if timezone_offset.len() != 6 {
        return Err(crate::quick_error!(
            validation,
            "timezone_offset",
            format!(
                "无效的时区偏移格式: '{}', 期望格式: +HH:MM",
                timezone_offset
            )
        ));
    }

    let sign = if timezone_offset.starts_with('+') {
        1
    } else {
        -1
    };
    let hours: i32 = timezone_offset[1..3].parse().map_err(|_| {
        crate::quick_error!(
            validation,
            "timezone_offset",
            format!("无效的小时格式: '{}'", &timezone_offset[1..3])
        )
    })?;
    let minutes: i32 = timezone_offset[4..6].parse().map_err(|_| {
        crate::quick_error!(
            validation,
            "timezone_offset",
            format!("无效的分钟格式: '{}'", &timezone_offset[4..6])
        )
    })?;

    let total_seconds = sign * (hours * 3600 + minutes * 60);
    Ok(total_seconds)
}

/// 将UTC时间转换为指定时区的时间
///
/// # 参数
/// * `utc_dt` - UTC时间
/// * `timezone_offset` - 时区偏移，格式 "+08:00", "-05:00"
pub fn utc_to_timezone(
    utc_dt: DateTime<Utc>,
    timezone_offset: &str,
) -> crate::error::QuickDbResult<DateTime<FixedOffset>> {
    let offset_seconds = parse_timezone_offset_to_seconds(timezone_offset)?;
    Ok(utc_dt.with_timezone(&FixedOffset::east(offset_seconds)))
}

pub fn process_data_fields_from_metadata(
    mut data_map: std::collections::HashMap<String, DataValue>,
    fields: &std::collections::HashMap<String, crate::model::FieldDefinition>,
) -> std::collections::HashMap<String, DataValue> {
    for (field_name, field_def) in fields {
        if let Some(current_value) = data_map.get::<str>(field_name) {
            let converted_value = match current_value {
                // 处理字符串类型的JSON数据
                DataValue::String(json_str)
                    if json_str.starts_with('[') || json_str.starts_with('{') =>
                {
                    // 尝试解析JSON
                    match serde_json::from_str::<serde_json::Value>(json_str.as_str()) {
                        Ok(json_value) => {
                            let converted =
                                crate::types::data_value::json_value_to_data_value(json_value);
                            debug_log!(
                                "字段 {} JSON转换成功: {:?} -> {:?}",
                                field_name,
                                json_str,
                                converted
                            );
                            Some(converted)
                        }
                        Err(e) => {
                            debug_log!(
                                "字段 {} JSON解析失败，保持原字符串: {} (错误: {})",
                                field_name,
                                json_str,
                                e
                            );
                            None // 解析失败，保持原字符串值
                        }
                    }
                }
                // 处理布尔字段的整数转换（SQLite等数据库的兼容性）
                DataValue::Int(int_val)
                    if matches!(field_def.field_type, crate::model::FieldType::Boolean) =>
                {
                    if *int_val == 0 || *int_val == 1 {
                        debug_log!(
                            "字段 {} 整数转布尔: {} -> {}",
                            field_name,
                            int_val,
                            *int_val == 1
                        );
                        Some(DataValue::Bool(*int_val == 1))
                    } else {
                        debug_log!(
                            "字段 {} 整数值超出布尔范围: {}，保持原值",
                            field_name,
                            int_val
                        );
                        None
                    }
                }
                // 处理DateTimeWithTz字段的String类型转换
                DataValue::String(s)
                    if matches!(
                        field_def.field_type,
                        crate::model::FieldType::DateTimeWithTz { .. }
                    ) =>
                {
                    match chrono::DateTime::parse_from_rfc3339(s) {
                        Ok(dt) => {
                            rat_logger::debug!(
                                "字段 {} String转DateTime: {} -> {}",
                                field_name,
                                s,
                                dt
                            );
                            Some(DataValue::DateTime(
                                dt.with_timezone(&chrono::FixedOffset::east(0)),
                            ))
                        }
                        Err(e) => {
                            rat_logger::debug!(
                                "字段 {} String转DateTime失败: {} (错误: {})",
                                field_name,
                                s,
                                e
                            );
                            None
                        }
                    }
                }
                // 处理DateTimeWithTz字段的DateTimeUTC类型转换
                DataValue::DateTimeUTC(dt)
                    if matches!(
                        field_def.field_type,
                        crate::model::FieldType::DateTimeWithTz { .. }
                    ) =>
                {
                    rat_logger::debug!("字段 {} DateTimeUTC转DateTime: {}", field_name, dt);
                    Some(DataValue::DateTime(
                        dt.with_timezone(&chrono::FixedOffset::east(0)),
                    ))
                }
                // 处理DateTimeWithTz字段的时区转换
                DataValue::DateTime(dt)
                    if matches!(
                        field_def.field_type,
                        crate::model::FieldType::DateTimeWithTz { .. }
                    ) =>
                {
                    if let crate::model::FieldType::DateTimeWithTz { timezone_offset } =
                        &field_def.field_type
                    {
                        debug_log!(
                            "字段 {} DateTimeWithTz时区转换: {} -> 时区 {}",
                            field_name,
                            dt,
                            timezone_offset
                        );
                        // 应用时区偏移转换
                        match apply_timezone_offset_to_datetime(*dt, timezone_offset) {
                            Ok(local_dt) => {
                                debug_log!(
                                    "字段 {} 时区转换成功: {} -> {}",
                                    field_name,
                                    dt,
                                    local_dt
                                );
                                Some(DataValue::DateTime(local_dt))
                            }
                            Err(e) => {
                                debug_log!(
                                    "字段 {} 时区转换失败: {} (错误: {})",
                                    field_name,
                                    dt,
                                    e
                                );
                                None // 转换失败，保持原值
                            }
                        }
                    } else {
                        None
                    }
                }
                _ => None, // 其他类型保持不变
            };

            // 如果有转换结果，更新数据映射
            if let Some(converted) = converted_value {
                data_map.insert(field_name.clone(), converted);
            }
        }
    }
    data_map
}

fn apply_timezone_offset_to_datetime(
    utc_dt: chrono::DateTime<chrono::FixedOffset>,
    timezone_offset: &str,
) -> QuickDbResult<chrono::DateTime<chrono::FixedOffset>> {
    let offset_seconds = crate::utils::timezone::parse_timezone_offset_to_seconds(timezone_offset)?;

    // 检查时区偏移是否在有效范围内（-23:59 到 +23:59）
    if offset_seconds < -86399 || offset_seconds > 86399 {
        return Err(QuickDbError::ValidationError {
            field: "timezone_offset".to_string(),
            message: format!(
                "时区偏移超出有效范围: {}, 允许范围: -23:59 到 +23:59",
                timezone_offset
            ),
        });
    }

    // 将UTC时间转换为本地时间
    let utc_chrono = utc_dt.with_timezone(&chrono::Utc);
    let local_dt = utc_chrono.with_timezone(&chrono::FixedOffset::east(offset_seconds));

    Ok(local_dt)
}
