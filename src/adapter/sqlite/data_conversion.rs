
  //! SQLite数据转换模块
//!
//! 提供基于字段元数据的数据库行到DataValue的转换功能

use crate::error::{QuickDbError, QuickDbResult};
use crate::types::*;
use crate::model::{FieldDefinition, FieldType};
use std::collections::HashMap;
use rat_logger::debug;
use sqlx::{Row, sqlite::SqliteRow, Column};

/// 将sqlx的行转换为DataValue映射（单表版本）
///
/// # 参数
/// - `row`: 数据库行数据
/// - `fields`: 字段名到字段定义的映射
///
/// # 返回
/// 字段名到DataValue的映射
pub fn row_to_data_map_with_metadata(
    row: &SqliteRow,
    fields: &HashMap<String, FieldDefinition>,
) -> QuickDbResult<HashMap<String, DataValue>> {
        let mut map = HashMap::new();

        for column in row.columns() {
            let column_name = column.name();

            // 严格使用字段元数据，不存在则报错
            let field_def = fields.get(column_name).ok_or_else(|| {
                QuickDbError::ValidationError {
                    field: column_name.to_string(),
                    message: format!("字段 '{}' 未在模型元数据中定义", column_name),
                }
            })?;

            // 根据字段类型进行转换
            let data_value = match &field_def.field_type {
                FieldType::DateTime => {
                    // 普通DateTime字段：转换为UTC的RFC3339字符串
                    let timestamp: i64 = row.try_get(column_name)
                        .map_err(|e| QuickDbError::QueryError {
                            message: format!("读取DateTime字段 '{}' 失败: {}", column_name, e),
                        })?;
                    let dt = chrono::DateTime::from_timestamp(timestamp, 0)
                        .ok_or_else(|| QuickDbError::QueryError {
                            message: format!("无效的时间戳: {}", timestamp),
                        })?;
                    // 返回UTC的RFC3339格式字符串
                    DataValue::String(dt.to_rfc3339())
                },
                FieldType::DateTimeWithTz { timezone_offset } => {
                    // 带时区的DateTime字段：转换为带时区的RFC3339字符串
                    let timestamp: i64 = row.try_get(column_name)
                        .map_err(|e| QuickDbError::QueryError {
                            message: format!("读取DateTimeWithTz字段 '{}' 失败: {}", column_name, e),
                        })?;
                    let utc_dt = chrono::DateTime::from_timestamp(timestamp, 0)
                        .ok_or_else(|| QuickDbError::QueryError {
                            message: format!("无效的时间戳: {}", timestamp),
                        })?;

                    // 转换为指定时区的本地时间
                    let local_dt = apply_timezone_offset_to_utc(utc_dt, timezone_offset)?;

                    // 返回带时区的RFC3339格式字符串
                    DataValue::String(local_dt.to_rfc3339())
                },
                FieldType::Boolean => {
                    let value: i64 = row.try_get(column_name)
                        .map_err(|e| QuickDbError::QueryError {
                            message: format!("读取Boolean字段 '{}' 失败: {}", column_name, e),
                        })?;
                    DataValue::Bool(value == 1)
                },
                FieldType::Integer { .. } | FieldType::BigInteger => {
                    let value: i64 = row.try_get(column_name)
                        .map_err(|e| QuickDbError::QueryError {
                            message: format!("读取整数字段 '{}' 失败: {}", column_name, e),
                        })?;
                    DataValue::Int(value)
                },
                FieldType::Float { .. } | FieldType::Double => {
                    let value: f64 = row.try_get(column_name)
                        .map_err(|e| QuickDbError::QueryError {
                            message: format!("读取浮点数字段 '{}' 失败: {}", column_name, e),
                        })?;
                    DataValue::Float(value)
                },
                FieldType::String { .. } | FieldType::Text => {
                    let value: String = row.try_get(column_name)
                        .map_err(|e| QuickDbError::QueryError {
                            message: format!("读取字符串字段 '{}' 失败: {}", column_name, e),
                        })?;
                    DataValue::String(value)
                },
                FieldType::Uuid => {
                    let value: String = row.try_get(column_name)
                        .map_err(|e| QuickDbError::QueryError {
                            message: format!("读取UUID字段 '{}' 失败: {}", column_name, e),
                        })?;
                    DataValue::String(value)
                },
                FieldType::Array { item_type, max_items: _, min_items: _ } => {
                    // Array字段：读取字符串数组JSON并根据字段定义转换类型
                    let json_string: String = row.try_get(column_name)
                        .map_err(|e| QuickDbError::QueryError {
                            message: format!("读取Array字段 '{}' 失败: {}", column_name, e),
                        })?;

                    // 解析为字符串数组
                    match serde_json::from_str::<Vec<String>>(&json_string) {
                        Ok(string_array) => {
                            // 将字符串数组转换回原始DataValue数组
                            let data_array: Vec<DataValue> = string_array.into_iter().map(|s| {
                                // 根据item_type进行类型转换
                                match &**item_type {
                                    FieldType::String { .. } => DataValue::String(s),
                                    FieldType::Integer { .. } => {
                                        match s.parse::<i64>() {
                                            Ok(i) => DataValue::Int(i),
                                            Err(_) => {
                                                debug!("Array字段 '{}' 整数转换失败: {}，保持字符串", column_name, s);
                                                DataValue::String(s)
                                            }
                                        }
                                    },
                                    FieldType::Float { .. } => {
                                        match s.parse::<f64>() {
                                            Ok(f) => DataValue::Float(f),
                                            Err(_) => {
                                                debug!("Array字段 '{}' 浮点数转换失败: {}，保持字符串", column_name, s);
                                                DataValue::String(s)
                                            }
                                        }
                                    },
                                    FieldType::Uuid => {
                                        match s.parse::<uuid::Uuid>() {
                                            Ok(uuid) => DataValue::Uuid(uuid),
                                            Err(_) => {
                                                debug!("Array字段 '{}' UUID转换失败: {}，保持字符串", column_name, s);
                                                DataValue::String(s)
                                            }
                                        }
                                    },
                                    _ => {
                                        debug!("Array字段 '{}' 不支持的item_type: {:?}，保持字符串", column_name, item_type);
                                        DataValue::String(s)
                                    }
                                }
                            }).collect();
                            DataValue::Array(data_array)
                        },
                        Err(e) => {
                            debug!("Array字段 '{}' JSON解析失败: {}，返回原始字符串", column_name, e);
                            DataValue::String(json_string)
                        }
                    }
                },
                FieldType::Json | FieldType::Object { .. } => {
                    let value: String = row.try_get(column_name)
                        .map_err(|e| QuickDbError::QueryError {
                            message: format!("读取JSON字段 '{}' 失败: {}", column_name, e),
                        })?;
                    crate::types::data_value::parse_json_string_to_data_value(value)
                },
                FieldType::Binary => {
                    let value: Vec<u8> = row.try_get(column_name)
                        .map_err(|e| QuickDbError::QueryError {
                            message: format!("读取二进制字段 '{}' 失败: {}", column_name, e),
                        })?;
                    DataValue::Bytes(value)
                },
                // 其他字段类型
                _ => {
                    return Err(QuickDbError::ValidationError {
                        field: column_name.to_string(),
                        message: format!("不支持的字段类型: {:?}", field_def.field_type),
                    });
                }
            };

            map.insert(column_name.to_string(), data_value);
        }
        
        Ok(map)
    }

/// 将UTC时间戳转换为本地时间字符串
///
/// # 参数
/// - `local_datetime_str`: 本地时间字符串，格式 "YYYY-MM-DD HH:MM:SS"
/// - `timezone_offset`: 时区偏移，格式 "+08:00", "-05:00"
///
/// # 返回
/// Unix时间戳
pub fn convert_local_to_timestamp(
    local_datetime_str: &str,
    timezone_offset: &str,
) -> QuickDbResult<i64> {
    // 解析本地时间
    let naive_dt = chrono::NaiveDateTime::parse_from_str(local_datetime_str, "%Y-%m-%d %H:%M:%S")
        .map_err(|e| QuickDbError::ValidationError {
            field: "datetime".to_string(),
            message: format!("无效的日期时间格式 '{}': {}", local_datetime_str, e),
        })?;

    // 将本地时间视为UTC时间（应用时区偏移前的状态）
    let utc_dt = naive_dt.and_utc();

    // 解析时区偏移
    let offset_seconds = parse_timezone_offset_to_seconds(timezone_offset)?;

    // 本地时间 = UTC时间 + 时区偏移
    // UTC时间 = 本地时间 - 时区偏移
    let adjusted_dt = utc_dt - chrono::Duration::seconds(offset_seconds as i64);

    Ok(adjusted_dt.timestamp())
}

/// 将RFC3339格式字符串转换为本地时间字符串
///
/// # 参数
/// - `rfc3339_str`: RFC3339格式字符串，如 "2024-06-15T12:00:00+08:00"
///
/// # 返回
/// 本地时间字符串，格式 "YYYY-MM-DD HH:MM:SS"
pub fn convert_rfc3339_to_local(rfc3339_str: &str) -> QuickDbResult<String> {
    let dt = chrono::DateTime::parse_from_rfc3339(rfc3339_str)
        .map_err(|e| QuickDbError::ValidationError {
            field: "datetime".to_string(),
            message: format!("无效的RFC3339格式 '{}': {}", rfc3339_str, e),
        })?;

    Ok(dt.format("%Y-%m-%d %H:%M:%S").to_string())
}

/// 对UTC时间应用时区偏移，返回本地时间
///
/// # 参数
/// - `utc_dt`: UTC时间
/// - `timezone_offset`: 时区偏移，格式 "+08:00", "-05:00"
///
/// # 返回
/// 本地时间（带时区信息）
fn apply_timezone_offset_to_utc(
    utc_dt: chrono::DateTime<chrono::Utc>,
    timezone_offset: &str,
) -> QuickDbResult<chrono::DateTime<chrono::FixedOffset>> {
    let offset_seconds = parse_timezone_offset_to_seconds(timezone_offset)?;

    // 检查时区偏移是否在有效范围内（-23:59 到 +23:59）
    if offset_seconds < -86399 || offset_seconds > 86399 {
        return Err(QuickDbError::ValidationError {
            field: "timezone_offset".to_string(),
            message: format!("时区偏移超出有效范围: {}, 允许范围: -23:59 到 +23:59", timezone_offset),
        });
    }

    Ok(utc_dt.with_timezone(&chrono::FixedOffset::east(offset_seconds)))
}

/// 将时区偏移字符串转换为秒数
///
/// # 参数
/// - `timezone_offset`: 时区偏移，格式 "+08:00", "-05:00"
///
/// # 返回
/// 秒数
fn parse_timezone_offset_to_seconds(timezone_offset: &str) -> QuickDbResult<i32> {
    if timezone_offset.len() != 6 {
        return Err(QuickDbError::ValidationError {
            field: "timezone_offset".to_string(),
            message: format!("无效的时区偏移格式: '{}', 期望格式: +HH:MM", timezone_offset),
        });
    }

    let sign = if timezone_offset.starts_with('+') { 1 } else { -1 };
    let hours: i32 = timezone_offset[1..3].parse()
        .map_err(|_| QuickDbError::ValidationError {
            field: "timezone_offset".to_string(),
            message: format!("无效的小时格式: '{}'", &timezone_offset[1..3]),
        })?;
    let minutes: i32 = timezone_offset[4..6].parse()
        .map_err(|_| QuickDbError::ValidationError {
            field: "timezone_offset".to_string(),
            message: format!("无效的分钟格式: '{}'", &timezone_offset[4..6]),
        })?;

    let total_seconds = sign * (hours * 3600 + minutes * 60);
    Ok(total_seconds)
}

