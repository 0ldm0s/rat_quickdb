
//! SQLite适配器辅助方法模块

use crate::adapter::SqliteAdapter;
use crate::error::{QuickDbError, QuickDbResult};
use crate::pool::DatabaseConnection;
use crate::types::*;
use rat_logger::debug;
use sqlx::{Column, Row, sqlite::SqliteRow};
use std::collections::HashMap;

impl SqliteAdapter {
    /// 将sqlx的行转换为DataValue映射
    pub(crate) fn row_to_data_map(
        &self,
        row: &SqliteRow,
    ) -> QuickDbResult<HashMap<String, DataValue>> {
        let mut map = HashMap::new();

        for column in row.columns() {
            let column_name = column.name();

            // 尝试获取不同类型的值
            let data_value = if let Ok(value) = row.try_get::<Option<String>, _>(column_name) {
                // 使用通用的JSON字符串检测和反序列化方法
                match value {
                    Some(s) => crate::types::data_value::parse_json_string_to_data_value(s),
                    None => DataValue::Null,
                }
            } else if let Ok(value) = row.try_get::<Option<i64>, _>(column_name) {
                match value {
                    Some(i) => {
                        // 检查是否可能是boolean值（SQLite中boolean存储为0或1）
                        // 只对已知的boolean字段进行转换，避免误判其他integer字段
                        if matches!(
                            column_name,
                            "is_active"
                                | "active"
                                | "enabled"
                                | "disabled"
                                | "verified"
                                | "is_admin"
                                | "is_deleted"
                        ) && (i == 0 || i == 1)
                        {
                            DataValue::Bool(i == 1)
                        } else if column_name == "id" && i > 1000000000000000000 {
                            // 如果是id字段且值很大，可能是雪花ID，转换为字符串保持跨数据库兼容性
                            DataValue::String(i.to_string())
                        } else {
                            DataValue::Int(i)
                        }
                    }
                    None => DataValue::Null,
                }
            } else if let Ok(value) = row.try_get::<Option<f64>, _>(column_name) {
                match value {
                    Some(f) => DataValue::Float(f),
                    None => DataValue::Null,
                }
            } else if let Ok(value) = row.try_get::<Option<bool>, _>(column_name) {
                match value {
                    Some(b) => DataValue::Bool(b),
                    None => DataValue::Null,
                }
            } else if let Ok(value) = row.try_get::<Option<Vec<u8>>, _>(column_name) {
                match value {
                    Some(bytes) => DataValue::Bytes(bytes),
                    None => DataValue::Null,
                }
            } else {
                DataValue::Null
            };

            map.insert(column_name.to_string(), data_value);
        }

        Ok(map)
    }

    /// 执行更新操作
    pub(crate) async fn execute_update(
        &self,
        pool: &sqlx::Pool<sqlx::Sqlite>,
        sql: &str,
        params: &[DataValue],
    ) -> QuickDbResult<u64> {
        let mut query = sqlx::query(sql);

        // 绑定参数
        for param in params {
            query = match param {
                DataValue::String(s) => {
                    // SQLite中字符串直接绑定
                    query.bind(s)
                }
                DataValue::Int(i) => query.bind(*i),
                DataValue::UInt(u) => {
                    // SQLite 不支持 u64 编码，转换为 i64 或字符串
                    if *u <= i64::MAX as u64 {
                        query.bind(*u as i64)
                    } else {
                        query.bind(u.to_string())
                    }
                }
                DataValue::Float(f) => query.bind(*f),
                DataValue::Bool(b) => query.bind(i32::from(*b)), // SQLite使用整数表示布尔值
                DataValue::DateTime(dt) => query.bind(dt.timestamp()),
                DataValue::DateTimeUTC(dt) => query.bind(dt.timestamp()),
                DataValue::Uuid(uuid) => query.bind(uuid.to_string()),
                DataValue::Json(json) => query.bind(json.to_string()),
                DataValue::Bytes(bytes) => query.bind(bytes.as_slice()),
                DataValue::Null => query.bind(Option::<String>::None),
                DataValue::Array(arr) => {
                    // Array字段只支持简单类型：String、Int、Float、Uuid
                    let string_array: Result<Vec<String>, QuickDbError> = arr.iter().map(|item| {
                        Ok(match item {
                            DataValue::String(s) => s.clone(),
                            DataValue::Int(i) => i.to_string(),
                            DataValue::Float(f) => f.to_string(),
                            DataValue::Uuid(uuid) => uuid.to_string(),
                            _ => {
                                return Err(QuickDbError::ValidationError {
                                    field: "array_field".to_string(),
                                    message: format!("Array字段不支持该类型: {:?}，只支持String、Int、Float、Uuid类型", item),
                                });
                            }
                        })
                    }).collect();
                    let string_array = string_array?;
                    query.bind(serde_json::to_string(&string_array).unwrap_or_default())
                }
                DataValue::Object(obj) => {
                    query.bind(serde_json::to_string(obj).unwrap_or_default())
                }
            };
        }

        debug!("执行SQLite更新SQL: {}", sql);

        let result = query
            .execute(pool)
            .await
            .map_err(|e| QuickDbError::QueryError {
                message: format!("SQLite更新失败: {}", e),
            })?;

        Ok(result.rows_affected())
    }
}
