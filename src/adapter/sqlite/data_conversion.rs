
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
                FieldType::DateTime | FieldType::DateTimeWithTz { .. } => {
                    // DateTime字段：存储为时间戳，需要转换回DateTime
                    let timestamp: i64 = row.try_get(column_name)
                        .map_err(|e| QuickDbError::QueryError {
                            message: format!("读取DateTime字段 '{}' 失败: {}", column_name, e),
                        })?;
                    DataValue::DateTime(
                        chrono::DateTime::from_timestamp(timestamp, 0)
                            .ok_or_else(|| QuickDbError::QueryError {
                                message: format!("无效的时间戳: {}", timestamp),
                            })?
                    )
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
                FieldType::Json | FieldType::Array { .. } | FieldType::Object { .. } => {
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

