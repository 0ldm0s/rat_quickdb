//! MySQL数据转换模块
//!
//! 提供基于字段元数据的数据库行到DataValue的转换功能

use crate::error::{QuickDbError, QuickDbResult};
use crate::types::*;
use crate::model::{FieldDefinition, FieldType};
use std::collections::HashMap;
use rat_logger::debug;
use sqlx::{Row, mysql::MySqlRow, Column};

/// 将sqlx的行转换为DataValue映射（单表版本）
///
/// # 参数
/// - `row`: 数据库行数据
/// - `fields`: 字段名到字段定义的映射
///
/// # 返回
/// 字段名到DataValue的映射
pub fn row_to_data_map_with_metadata(
    row: &MySqlRow,
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
                // 普通DateTime字段：MySQL存储为DATETIME，直接读取
                if let Ok(value) = row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>(column_name) {
                    match value {
                        Some(dt) => {
                            println!("🔍 MySQL DateTime字段: {} -> {}", column_name, dt);
                            // MySQL存储的是UTC时间，直接返回UTC时间
                            DataValue::DateTime(dt.with_timezone(&chrono::FixedOffset::east(0)))
                        },
                        None => DataValue::Null,
                    }
                } else {
                    return Err(QuickDbError::QueryError {
                        message: format!("读取DateTime字段 '{}' 失败", column_name),
                    });
                }
            },
            FieldType::DateTimeWithTz { timezone_offset } => {
                // 带时区的DateTime字段：MySQL存储为DATETIME，需要应用时区转换
                if let Ok(value) = row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>(column_name) {
                    match value {
                        Some(utc_dt) => {
                            println!("🔍 MySQL DateTimeWithTz字段: {} = {} 时区: {}", column_name, utc_dt, timezone_offset);
                            // MySQL存储的是UTC时间，需要转换为指定时区的本地时间
                            let local_dt = apply_timezone_offset_to_utc(utc_dt, timezone_offset)?;
                            println!("🔍 MySQL时区转换结果: {} -> {}", utc_dt, local_dt);
                            DataValue::DateTime(local_dt)
                        },
                        None => DataValue::Null,
                    }
                } else {
                    return Err(QuickDbError::QueryError {
                        message: format!("读取DateTimeWithTz字段 '{}' 失败", column_name),
                    });
                }
            },
            FieldType::Boolean => {
                // MySQL BOOLEAN通常是TINYINT(1)
                if let Ok(value) = row.try_get::<Option<i32>, _>(column_name) {
                    match value {
                        Some(v) => DataValue::Bool(v != 0),
                        None => DataValue::Null,
                    }
                } else {
                    return Err(QuickDbError::QueryError {
                        message: format!("读取Boolean字段 '{}' 失败", column_name),
                    });
                }
            },
            FieldType::Integer { .. } | FieldType::BigInteger => {
                if let Ok(value) = row.try_get::<Option<i64>, _>(column_name) {
                    match value {
                        Some(v) => DataValue::Int(v),
                        None => DataValue::Null,
                    }
                } else {
                    return Err(QuickDbError::QueryError {
                        message: format!("读取整数字段 '{}' 失败", column_name),
                    });
                }
            },
            FieldType::Float { .. } | FieldType::Double => {
                if let Ok(value) = row.try_get::<Option<f64>, _>(column_name) {
                    match value {
                        Some(v) => DataValue::Float(v),
                        None => DataValue::Null,
                    }
                } else {
                    return Err(QuickDbError::QueryError {
                        message: format!("读取浮点数字段 '{}' 失败", column_name),
                    });
                }
            },
            FieldType::String { .. } | FieldType::Text | FieldType::Uuid => {
                if let Ok(value) = row.try_get::<Option<String>, _>(column_name) {
                    match value {
                        Some(v) => DataValue::String(v),
                        None => DataValue::Null,
                    }
                } else {
                    return Err(QuickDbError::QueryError {
                        message: format!("读取字符串字段 '{}' 失败", column_name),
                    });
                }
            },
            FieldType::Json | FieldType::Array { .. } | FieldType::Object { .. } => {
                if let Ok(value) = row.try_get::<Option<String>, _>(column_name) {
                    match value {
                        Some(v) => {
                            // 解析JSON字符串为DataValue
                            crate::types::data_value::parse_json_string_to_data_value(v)
                        },
                        None => DataValue::Null,
                    }
                } else {
                    return Err(QuickDbError::QueryError {
                        message: format!("读取JSON字段 '{}' 失败", column_name),
                    });
                }
            },
            FieldType::Binary => {
                if let Ok(value) = row.try_get::<Option<Vec<u8>>, _>(column_name) {
                    match value {
                        Some(v) => DataValue::Bytes(v),
                        None => DataValue::Null,
                    }
                } else {
                    return Err(QuickDbError::QueryError {
                        message: format!("读取二进制字段 '{}' 失败", column_name),
                    });
                }
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
    let offset_seconds = crate::utils::timezone::parse_timezone_offset_to_seconds(timezone_offset)?;

    // 检查时区偏移是否在有效范围内（-23:59 到 +23:59）
    if offset_seconds < -86399 || offset_seconds > 86399 {
        return Err(QuickDbError::ValidationError {
            field: "timezone_offset".to_string(),
            message: format!("时区偏移超出有效范围: {}, 允许范围: -23:59 到 +23:59", timezone_offset),
        });
    }

    Ok(utc_dt.with_timezone(&chrono::FixedOffset::east(offset_seconds)))
}

