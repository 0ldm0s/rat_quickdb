//! PostgreSQL适配器辅助工具函数

use crate::adapter::postgres::PostgresAdapter;
use crate::error::{QuickDbError, QuickDbResult};
use crate::types::DataValue;
use rat_logger::debug;
use serde_json::Value;
use sqlx::{Row, Column, TypeInfo};
use std::collections::HashMap;

/// 将PostgreSQL行转换为DataValue映射
pub(crate) fn row_to_data_map(
    adapter: &PostgresAdapter,
    row: &sqlx::postgres::PgRow,
) -> QuickDbResult<HashMap<String, DataValue>> {
    let mut map = HashMap::new();

    for column in row.columns() {
        let column_name = column.name();
        let type_name = column.type_info().name();

        // 根据PostgreSQL类型转换值
        let data_value = match type_name {
            "INT4" | "INT8" => {
                if let Ok(val) = row.try_get::<Option<i32>, _>(column_name) {
                    match val {
                        Some(i) => DataValue::Int(i as i64),
                        None => DataValue::Null,
                    }
                } else if let Ok(val) = row.try_get::<Option<i64>, _>(column_name) {
                    match val {
                        Some(i) => {
                            // 如果是id字段且值很大，可能是雪花ID，转换为字符串保持跨数据库兼容性
                            if column_name == "id" && i > 1000000000000000000 {
                                DataValue::String(i.to_string())
                            } else {
                                DataValue::Int(i)
                            }
                        },
                        None => DataValue::Null,
                    }
                } else {
                    DataValue::Null
                }
            },
            "FLOAT4" | "FLOAT8" => {
                if let Ok(val) = row.try_get::<Option<f32>, _>(column_name) {
                    match val {
                        Some(f) => DataValue::Float(f as f64),
                        None => DataValue::Null,
                    }
                } else if let Ok(val) = row.try_get::<Option<f64>, _>(column_name) {
                    match val {
                        Some(f) => DataValue::Float(f),
                        None => DataValue::Null,
                    }
                } else {
                    DataValue::Null
                }
            },
            "BOOL" => {
                if let Ok(val) = row.try_get::<Option<bool>, _>(column_name) {
                    match val {
                        Some(b) => DataValue::Bool(b),
                        None => DataValue::Null,
                    }
                } else {
                    DataValue::Null
                }
            },
            "TEXT" | "VARCHAR" | "CHAR" => {
                if let Ok(val) = row.try_get::<Option<String>, _>(column_name) {
                    match val {
                        Some(s) => DataValue::String(s),
                        None => DataValue::Null,
                    }
                } else {
                    DataValue::Null
                }
            },
            "UUID" => {
                if let Ok(val) = row.try_get::<Option<uuid::Uuid>, _>(column_name) {
                    match val {
                        Some(u) => {
                            // 将UUID转换为字符串以保持跨数据库兼容性
                            DataValue::String(u.to_string())
                        },
                        None => DataValue::Null,
                    }
                } else {
                    DataValue::Null
                }
            },
            "JSON" | "JSONB" => {
                // PostgreSQL原生支持JSONB，直接获取serde_json::Value
                // 无需像MySQL/SQLite那样解析JSON字符串
                if let Ok(val) = row.try_get::<Option<serde_json::Value>, _>(column_name) {
                    match val {
                        Some(json_val) => {
                            // 使用现有的转换函数，确保类型正确
                            crate::types::data_value::json_value_to_data_value(json_val)
                        },
                        None => DataValue::Null,
                    }
                } else {
                    DataValue::Null
                }
            },
            // 处理PostgreSQL数组类型（如 text[], integer[], bigint[] 等）
            type_name if type_name.ends_with("[]") => {
                // 尝试将PostgreSQL数组转换为Vec<String>，然后再转换为DataValue::Array
                if let Ok(val) = row.try_get::<Option<Vec<String>>, _>(column_name) {
                    match val {
                        Some(arr) => {
                            debug!("PostgreSQL数组字段 {} 转换为DataValue::Array，元素数量: {}", column_name, arr.len());
                            // 将字符串数组转换为DataValue数组
                            let data_array: Vec<DataValue> = arr.into_iter()
                                .map(DataValue::String)
                                .collect();
                            DataValue::Array(data_array)
                        },
                        None => DataValue::Null,
                    }
                } else {
                    // 如果字符串数组读取失败，尝试其他方法
                    debug!("PostgreSQL数组字段 {} 无法作为字符串数组读取，尝试作为JSON", column_name);
                    if let Ok(val) = row.try_get::<Option<serde_json::Value>, _>(column_name) {
                        match val {
                            Some(json_val) => {
                                debug!("PostgreSQL数组字段 {} 作为JSON处理: {:?}", column_name, json_val);
                                crate::types::data_value::json_value_to_data_value(json_val)
                            },
                            None => DataValue::Null,
                        }
                    } else {
                        debug!("PostgreSQL数组字段 {} 读取失败，设置为Null", column_name);
                        DataValue::Null
                    }
                }
            },
            "timestamp without time zone" | "TIMESTAMP" | "TIMESTAMPTZ" => {
                // 对于不带时区的时间戳，先尝试作为chrono::DateTime<chrono::Utc>，如果失败则尝试作为chrono::NaiveDateTime
                if let Ok(val) = row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>(column_name) {
                    match val {
                        Some(dt) => DataValue::DateTime(dt),
                        None => DataValue::Null,
                    }
                } else if let Ok(val) = row.try_get::<Option<chrono::NaiveDateTime>, _>(column_name) {
                    match val {
                        Some(ndt) => {
                            // 将NaiveDateTime转换为UTC时间
                            let utc_dt = ndt.and_utc();
                            DataValue::DateTime(utc_dt)
                        },
                        None => DataValue::Null,
                    }
                } else {
                    DataValue::Null
                }
            },
            _ => {
                // 对于未知类型，尝试作为字符串获取
                if let Ok(val) = row.try_get::<Option<String>, _>(column_name) {
                    match val {
                        Some(s) => DataValue::String(s),
                        None => DataValue::Null,
                    }
                } else {
                    DataValue::Null
                }
            }
        };

        map.insert(column_name.to_string(), data_value);
    }

    Ok(map)
}

/// 将PostgreSQL行转换为JSON值（保留用于向后兼容）
pub(crate) fn row_to_json(
    adapter: &PostgresAdapter,
    row: &sqlx::postgres::PgRow,
) -> QuickDbResult<Value> {
    let data_map = row_to_data_map(adapter, row)?;
    let mut json_map = serde_json::Map::new();

    for (key, value) in data_map {
        json_map.insert(key, value.to_json_value());
    }

    Ok(Value::Object(json_map))
}

/// 执行查询并返回结果
pub(crate) async fn execute_query(
    adapter: &PostgresAdapter,
    pool: &sqlx::Pool<sqlx::Postgres>,
    sql: &str,
    params: &[DataValue],
) -> QuickDbResult<Vec<DataValue>> {
    let mut query = sqlx::query(sql);

    // 绑定参数
    for param in params {
        query = match param {
            DataValue::String(s) => {
                // 尝试判断是否为UUID格式，如果是则转换为UUID类型
                match s.parse::<uuid::Uuid>() {
                    Ok(uuid) => query.bind(uuid), // 绑定为UUID类型
                    Err(_) => query.bind(s),       // 不是UUID格式，绑定为字符串
                }
            },
            DataValue::Int(i) => query.bind(*i),
            DataValue::Float(f) => query.bind(*f),
            DataValue::Bool(b) => query.bind(*b),
            DataValue::DateTime(dt) => query.bind(*dt),
            DataValue::Uuid(uuid) => query.bind(*uuid),
            DataValue::Json(json) => query.bind(json),
            DataValue::Bytes(bytes) => query.bind(bytes.as_slice()),
            DataValue::Null => query.bind(Option::<String>::None),
            DataValue::Array(arr) => {
                // 使用 to_json_value() 避免序列化时包含类型标签
                let json_array = DataValue::Array(arr.clone()).to_json_value();
                query.bind(json_array)
            },
            DataValue::Object(obj) => {
                // 使用 to_json_value() 避免序列化时包含类型标签
                let json_object = DataValue::Object(obj.clone()).to_json_value();
                query.bind(json_object)
            },
        };
    }

    let rows = query.fetch_all(pool)
        .await
        .map_err(|e| QuickDbError::QueryError {
            message: format!("执行PostgreSQL查询失败: {}", e),
        })?;

    let mut results = Vec::new();
    for row in rows {
        let data_map = row_to_data_map(adapter, &row)?;
        results.push(DataValue::Object(data_map));
    }

    Ok(results)
}

/// 执行更新操作
pub(crate) async fn execute_update(
    adapter: &PostgresAdapter,
    pool: &sqlx::Pool<sqlx::Postgres>,
    sql: &str,
    params: &[DataValue],
) -> QuickDbResult<u64> {
    let mut query = sqlx::query(sql);

    // 绑定参数
    for param in params {
        query = match param {
            DataValue::String(s) => {
                // 尝试判断是否为UUID格式，如果是则转换为UUID类型
                match s.parse::<uuid::Uuid>() {
                    Ok(uuid) => query.bind(uuid), // 绑定为UUID类型
                    Err(_) => query.bind(s),       // 不是UUID格式，绑定为字符串
                }
            },
            DataValue::Int(i) => query.bind(*i),
            DataValue::Float(f) => query.bind(*f),
            DataValue::Bool(b) => query.bind(*b),
            DataValue::DateTime(dt) => query.bind(*dt),
            DataValue::Uuid(uuid) => query.bind(*uuid),
            DataValue::Json(json) => query.bind(json),
            DataValue::Bytes(bytes) => query.bind(bytes.as_slice()),
            DataValue::Null => query.bind(Option::<String>::None),
            DataValue::Array(arr) => query.bind(serde_json::to_value(arr).unwrap_or_default()),
            DataValue::Object(obj) => query.bind(serde_json::to_value(obj).unwrap_or_default()),
        };
    }

    let result = query.execute(pool)
        .await
        .map_err(|e| QuickDbError::QueryError {
            message: format!("执行PostgreSQL更新失败: {}", e),
        })?;

    Ok(result.rows_affected())
}