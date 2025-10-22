//! MySQL数据库适配器
//!
//! 基于mysql_async库实现的MySQL数据库适配器，提供完整的CRUD操作支持

use crate::adapter::DatabaseAdapter;
use crate::pool::DatabaseConnection;
use crate::error::{QuickDbError, QuickDbResult};
use crate::types::{DataValue, QueryCondition, QueryConditionGroup, QueryOperator, QueryOptions, SortDirection, IdStrategy};
use crate::adapter::query_builder::SqlQueryBuilder;
use crate::table::{TableManager, TableSchema, ColumnType};
use crate::model::{FieldType, FieldDefinition, ModelMeta};
use crate::manager;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use rat_logger::{info, error, warn, debug};
use sqlx::{MySql, Pool, Row, Column, TypeInfo};
use sqlx::mysql::MySqlRow;
use serde_json::Value as JsonValue;
// 移除不存在的rat_logger::prelude导入

/// MySQL适配器
#[derive(Debug, Clone)]
pub struct MysqlAdapter {
    /// 适配器名称
    pub name: String,
    /// 表创建锁，防止重复创建表
    creation_locks: Arc<Mutex<HashMap<String, ()>>>,
}

impl MysqlAdapter {
    /// 创建新的MySQL适配器
    pub fn new() -> Self {
        Self {
            name: "MySQL".to_string(),
            creation_locks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 获取表创建锁
    async fn acquire_table_lock(&self, table: &str) -> tokio::sync::MutexGuard<'_, HashMap<String, ()>> {
        let mut locks = self.creation_locks.lock().await;
        if !locks.contains_key(table) {
            locks.insert(table.to_string(), ());
            debug!("🔒 获取表 {} 的创建锁", table);
        }
        locks
    }

    /// 释放表创建锁
    async fn release_table_lock(&self, table: &str, mut locks: tokio::sync::MutexGuard<'_, HashMap<String, ()>>) {
        locks.remove(table);
        debug!("🔓 释放表 {} 的创建锁", table);
    }
    
    /// 安全地读取整数字段，防止 byteorder 错误
    fn safe_read_integer(row: &MySqlRow, column_name: &str) -> QuickDbResult<DataValue> {
        // 尝试多种整数类型读取，按照从最常见到最不常见的顺序
        
        // 1. 尝试读取为 Option<i64>
        if let Ok(val) = row.try_get::<Option<i64>, _>(column_name) {
            return Ok(match val {
                Some(i) => {
                    // 如果是id字段且值很大，可能是雪花ID，转换为字符串保持跨数据库兼容性
                    if column_name == "id" && i > 1000000000000000000 {
                        DataValue::String(i.to_string())
                    } else {
                        DataValue::Int(i)
                    }
                },
                None => DataValue::Null,
            });
        }
        
        // 2. 尝试读取为 Option<i32>
        if let Ok(val) = row.try_get::<Option<i32>, _>(column_name) {
            return Ok(match val {
                Some(i) => DataValue::Int(i as i64),
                None => DataValue::Null,
            });
        }
        
        // 3. 尝试读取为 Option<u64>
        if let Ok(val) = row.try_get::<Option<u64>, _>(column_name) {
            return Ok(match val {
                Some(i) => {
                    if i <= i64::MAX as u64 {
                        DataValue::Int(i as i64)
                    } else {
                        // 如果超出 i64 范围，转为字符串
                        DataValue::String(i.to_string())
                    }
                },
                None => DataValue::Null,
            });
        }
        
        // 4. 尝试读取为 Option<u32>
        if let Ok(val) = row.try_get::<Option<u32>, _>(column_name) {
            return Ok(match val {
                Some(i) => DataValue::Int(i as i64),
                None => DataValue::Null,
            });
        }
        
        // 5. 最后尝试读取为字符串
        if let Ok(val) = row.try_get::<Option<String>, _>(column_name) {
            return Ok(match val {
                Some(s) => {
                    // 尝试解析为数字
                    if let Ok(i) = s.parse::<i64>() {
                        DataValue::Int(i)
                    } else {
                        DataValue::String(s)
                    }
                },
                None => DataValue::Null,
            });
        }
        
        // 如果所有尝试都失败，返回错误
        Err(QuickDbError::SerializationError {
            message: format!("无法读取整数字段 '{}' 的值，所有类型转换都失败", column_name),
        })
    }

    /// 安全读取浮点数，避免 byteorder 错误
    fn safe_read_float(row: &MySqlRow, column_name: &str) -> QuickDbResult<DataValue> {
        // 首先尝试读取 f32 (MySQL FLOAT 是 4 字节)
        if let Ok(val) = row.try_get::<Option<f32>, _>(column_name) {
            return Ok(match val {
                Some(f) => DataValue::Float(f as f64),
                None => DataValue::Null,
            });
        }
        
        // 然后尝试读取 f64 (MySQL DOUBLE 是 8 字节)
        if let Ok(val) = row.try_get::<Option<f64>, _>(column_name) {
            return Ok(match val {
                Some(f) => DataValue::Float(f),
                None => DataValue::Null,
            });
        }
        
        // 尝试以字符串读取并解析
        if let Ok(val) = row.try_get::<Option<String>, _>(column_name) {
            return Ok(match val {
                Some(s) => {
                    if let Ok(f) = s.parse::<f64>() {
                        DataValue::Float(f)
                    } else {
                        DataValue::String(s)
                    }
                },
                None => DataValue::Null,
            });
        }
        
        // 所有尝试都失败，返回错误
        Err(QuickDbError::SerializationError { message: format!("无法读取浮点数字段 '{}'", column_name) })
    }

    /// 安全读取JSON字段，处理MySQL中JSON的多种存储格式
    fn safe_read_json(row: &MySqlRow, column_name: &str) -> QuickDbResult<DataValue> {
        debug!("开始安全读取JSON字段: {}", column_name);

        // 1. 首先尝试直接解析为JsonValue（标准的JSON字段）
        let direct_json_result = row.try_get::<Option<JsonValue>, _>(column_name);
        debug!("直接解析JsonValue结果: {:?}", direct_json_result);

        if let Ok(value) = direct_json_result {
            debug!("成功直接解析为JsonValue: {:?}", value);
            return Ok(match value {
                Some(json) => DataValue::Json(json),
                None => DataValue::Null,
            });
        }

        // 2. 如果直接解析失败，尝试读取为字符串，然后解析为JSON
        let string_result = row.try_get::<Option<String>, _>(column_name);
        debug!("读取为字符串结果: {:?}", string_result);

        if let Ok(value) = string_result {
            match value {
                Some(s) => {
                    debug!("获取到字符串值，长度: {}, 前50字符: {}", s.len(), &s[..s.len().min(50)]);
                    // 检查是否是JSON字符串格式（以{或[开头）
                    if s.starts_with('{') || s.starts_with('[') {
                        debug!("检测到JSON格式字符串，尝试解析");
                        // 尝试解析为JSON值
                        match serde_json::from_str::<JsonValue>(&s) {
                            Ok(json_value) => {
                                debug!("JSON字符串解析成功: {:?}", json_value);
                                // 直接根据JSON类型转换为对应的DataValue
                                // 这样可以避免DataValue::Json包装，确保Object字段正确解析
                                match json_value {
                                    JsonValue::Object(obj) => {
                                        let data_object: HashMap<String, DataValue> = obj.into_iter()
                                            .map(|(k, v)| (k, crate::types::json_value_to_data_value(v)))
                                            .collect();
                                        debug!("转换为DataValue::Object，包含{}个字段", data_object.len());
                                        Ok(DataValue::Object(data_object))
                                    },
                                    JsonValue::Array(arr) => {
                                        let data_array: Vec<DataValue> = arr.into_iter()
                                            .map(|v| crate::types::json_value_to_data_value(v))
                                            .collect();
                                        debug!("转换为DataValue::Array，包含{}个元素", data_array.len());
                                        Ok(DataValue::Array(data_array))
                                    },
                                    _ => {
                                        debug!("转换为其他DataValue类型");
                                        Ok(crate::types::json_value_to_data_value(json_value))
                                    },
                                }
                            },
                            Err(e) => {
                                warn!("JSON字符串解析失败: {}，错误: {}", s, e);
                                // 解析失败，作为普通字符串处理
                                Ok(DataValue::String(s))
                            }
                        }
                    } else {
                        debug!("不是JSON格式字符串，返回DataValue::String");
                        // 不是JSON格式，作为普通字符串处理
                        Ok(DataValue::String(s))
                    }
                },
                None => {
                    debug!("字符串值为None，返回DataValue::Null");
                    Ok(DataValue::Null)
                },
            }
        } else {
            error!("所有读取方式都失败");
            Err(QuickDbError::SerializationError {
                message: format!("无法读取JSON字段 '{}' 的值，所有类型转换都失败", column_name)
            })
        }
    }

    /// 安全读取布尔值，避免 byteorder 错误
    fn safe_read_bool(row: &MySqlRow, column_name: &str) -> QuickDbResult<DataValue> {
        // 尝试以 bool 读取
        if let Ok(val) = row.try_get::<Option<bool>, _>(column_name) {
            return Ok(match val {
                Some(b) => DataValue::Bool(b),
                None => DataValue::Null,
            });
        }
        
        // 尝试以整数读取（MySQL 中 BOOLEAN 通常存储为 TINYINT）
        if let Ok(val) = row.try_get::<Option<i8>, _>(column_name) {
            return Ok(match val {
                Some(i) => DataValue::Bool(i != 0),
                None => DataValue::Null,
            });
        }
        
        // 尝试以字符串读取并解析
        if let Ok(val) = row.try_get::<Option<String>, _>(column_name) {
            return Ok(match val {
                Some(s) => {
                    match s.to_lowercase().as_str() {
                        "true" | "1" | "yes" | "on" => DataValue::Bool(true),
                        "false" | "0" | "no" | "off" => DataValue::Bool(false),
                        _ => DataValue::String(s),
                    }
                },
                None => DataValue::Null,
            });
        }
        
        // 所有尝试都失败，返回错误
        Err(QuickDbError::SerializationError { message: format!("无法读取布尔字段 '{}'", column_name) })
    }

    /// 将MySQL行转换为DataValue映射
    fn row_to_data_map(&self, row: &MySqlRow) -> QuickDbResult<HashMap<String, DataValue>> {
        let mut data_map = HashMap::new();

        for column in row.columns() {
            let column_name = column.name();
            let column_type = column.type_info().name();
            
            // 调试：输出列类型信息
            debug!("开始处理MySQL列 '{}' 的类型: '{}'", column_name, column_type);
              
            // 根据MySQL类型转换值
            let data_value = match column_type {
                "INT" | "BIGINT" | "SMALLINT" | "TINYINT" => {
                    debug!("准备读取整数字段: {}", column_name);
                    // 使用安全的整数读取方法，防止 byteorder 错误
                    match Self::safe_read_integer(row, column_name) {
                        Ok(value) => {
                            debug!("成功读取整数字段 {}: {:?}", column_name, value);
                            value
                        },
                        Err(e) => {
                            error!("读取整数字段 {} 时发生错误: {}", column_name, e);
                            DataValue::Null
                        }
                    }
                },
                // 处理UNSIGNED整数类型
                "INT UNSIGNED" | "BIGINT UNSIGNED" | "SMALLINT UNSIGNED" | "TINYINT UNSIGNED" => {
                    // 对于LAST_INSERT_ID()，MySQL返回的是unsigned long long，但sqlx可能会将其作为i64处理
                    // 我们应该优先尝试i64，因为MySQL的LAST_INSERT_ID()通常在合理范围内
                    
                    // 1. 首先尝试i64，因为MySQL的自增ID通常不会超过i64::MAX
                    if let Ok(val) = row.try_get::<Option<i64>, _>(column_name) {
                        match val {
                            Some(i) => {
                                // 如果i64为负数，这可能是类型转换错误，尝试u64
                                if i < 0 {
                                    if let Ok(u_val) = row.try_get::<Option<u64>, _>(column_name) {
                                        if let Some(u) = u_val {
                                            DataValue::Int(u as i64)
                                        } else {
                                            DataValue::Null
                                        }
                                    } else {
                                        DataValue::Null
                                    }
                                } else {
                                    DataValue::Int(i)
                                }
                            },
                            None => DataValue::Null,
                        }
                    }
                    // 2. 尝试u64
                    else if let Ok(val) = row.try_get::<Option<u64>, _>(column_name) {
                        match val {
                            Some(u) => {
                                if u <= i64::MAX as u64 {
                                    DataValue::Int(u as i64)
                                } else {
                                    DataValue::String(u.to_string())
                                }
                            },
                            None => DataValue::Null,
                        }
                    }
                    // 3. 尝试作为字符串读取，避免字节序问题
                    else if let Ok(val) = row.try_get::<Option<String>, _>(column_name) {
                        match val {
                            Some(s) => {
                                if let Ok(u) = s.parse::<u64>() {
                                    if u <= i64::MAX as u64 {
                                        DataValue::Int(u as i64)
                                    } else {
                                        DataValue::String(u.to_string())
                                    }
                                } else if let Ok(i) = s.parse::<i64>() {
                                    DataValue::Int(i)
                                } else {
                                    DataValue::String(s)
                                }
                            },
                            None => DataValue::Null,
                        }
                    } else {
                        warn!("无法读取无符号整数字段 '{}' 的值，类型: {}", column_name, column_type);
                        DataValue::Null
                    }
                },
                "FLOAT" | "DOUBLE" => {
                    debug!("准备读取浮点数字段: {}", column_name);
                    match Self::safe_read_float(row, column_name) {
                        Ok(value) => {
                            debug!("成功读取浮点数字段 {}: {:?}", column_name, value);
                            value
                        },
                        Err(e) => {
                            error!("读取浮点数字段 {} 时发生错误: {}", column_name, e);
                            DataValue::Null
                        }
                    }
                },
                "BOOLEAN" | "BOOL" => {
                    debug!("准备读取布尔字段: {}", column_name);
                    match Self::safe_read_bool(row, column_name) {
                        Ok(value) => {
                            debug!("成功读取布尔字段 {}: {:?}", column_name, value);
                            value
                        },
                        Err(e) => {
                            error!("读取布尔字段 {} 时发生错误: {}", column_name, e);
                            DataValue::Null
                        }
                    }
                },
                "CHAR" => {
                    debug!("准备读取字符串字段: {}", column_name);
                    if let Ok(value) = row.try_get::<Option<String>, _>(column_name) {
                        let result = match value {
                            Some(s) => DataValue::String(s),
                            None => DataValue::Null,
                        };
                        debug!("成功读取字符串字段 {}: {:?}", column_name, result);
                        result
                    } else {
                        error!("无法读取字符串字段: {}", column_name);
                        DataValue::Null
                    }
                },
                "JSON" | "LONGTEXT" | "TEXT" | "VARCHAR" => {
                    // 简化处理：所有文本类型都作为字符串读取
                    debug!("读取文本字段: {} (类型: {})", column_name, column_type);
                    if let Ok(value) = row.try_get::<Option<String>, _>(column_name) {
                        let result = match value {
                            Some(s) => DataValue::String(s),
                            None => DataValue::Null,
                        };
                        debug!("读取文本字段 {}: {:?}", column_name, result);
                        result
                    } else {
                        error!("无法读取文本字段: {}", column_name);
                        DataValue::Null
                    }
                },
                "BLOB" => {
                    // BLOB类型可能存储JSON数据，需要作为字节数组读取然后转换为字符串
                    debug!("读取BLOB字段: {} (类型: {})", column_name, column_type);
                    if let Ok(value) = row.try_get::<Option<Vec<u8>>, _>(column_name) {
                        let result = match value {
                            Some(bytes) => {
                                // 尝试将字节数组转换为UTF-8字符串
                                match String::from_utf8(bytes.clone()) {
                                    Ok(s) => DataValue::String(s),
                                    Err(e) => {
                                        warn!("BLOB字段UTF-8转换失败: {}, 使用base64编码", e);
                                        DataValue::String(base64::encode(&bytes))
                                    }
                                }
                            },
                            None => DataValue::Null,
                        };
                        debug!("读取BLOB字段 {}: {:?}", column_name, result);
                        result
                    } else {
                        error!("无法读取BLOB字段: {}", column_name);
                        DataValue::Null
                    }
                },
                "DATETIME" | "TIMESTAMP" => {
                    debug!("准备读取日期时间字段: {}", column_name);
                    if let Ok(value) = row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>(column_name) {
                        let result = match value {
                            Some(dt) => DataValue::DateTime(dt),
                            None => DataValue::Null,
                        };
                        debug!("成功读取日期时间字段 {}: {:?}", column_name, result);
                        result
                    } else {
                        error!("无法读取日期时间字段: {}", column_name);
                        DataValue::Null
                    }
                },
                _ => {
                debug!("处理未知类型字段: {} (类型: '{}', 长度: {})", column_name, column_type, column_type.len());
                // 对于未知类型，尝试作为字符串处理
                if let Ok(value) = row.try_get::<Option<String>, _>(column_name) {
                    let result = match value {
                        Some(s) => DataValue::String(s),
                        None => DataValue::Null,
                    };
                    debug!("成功读取未知类型字段 {}: {:?}", column_name, result);
                    result
                } else {
                    error!("无法读取未知类型字段: {}", column_name);
                    DataValue::Null
                }
            }
            };
            
            data_map.insert(column_name.to_string(), data_value);
        }
        
        Ok(data_map)
    }



    /// 执行查询并返回结果
    async fn execute_query(
        &self,
        pool: &Pool<MySql>,
        sql: &str,
        params: &[DataValue],
    ) -> QuickDbResult<Vec<DataValue>> {
        let mut query = sqlx::query(sql);
        
        // 绑定参数
        for param in params {
            query = match param {
                DataValue::String(s) => {
                    // 检查是否为JSON字符串，如果是则转换为对应的DataValue类型
                    let converted_value = crate::types::parse_json_string_to_data_value(s.clone());
                    match converted_value {
                        DataValue::Json(json_val) => {
                            query.bind(serde_json::to_string(&json_val).unwrap_or_default())
                        },
                        _ => query.bind(s)
                    }
                },
                DataValue::Int(i) => query.bind(*i),
                DataValue::Float(f) => query.bind(*f),
                DataValue::Bool(b) => query.bind(*b),
                DataValue::DateTime(dt) => query.bind(*dt),
                DataValue::Uuid(uuid) => query.bind(*uuid),
                DataValue::Json(json) => query.bind(json.to_string()),
                DataValue::Bytes(bytes) => query.bind(bytes.as_slice()),
                DataValue::Null => query.bind(Option::<String>::None),
                DataValue::Array(arr) => {
                    // 将DataValue数组转换为原始JSON数组
                    let json_values: Vec<serde_json::Value> = arr.iter()
                        .map(|v| v.to_json_value())
                        .collect();
                    query.bind(serde_json::to_string(&json_values).unwrap_or_default())
                },
                DataValue::Object(obj) => {
                    // 将DataValue对象转换为原始JSON对象
                    let json_map: serde_json::Map<String, serde_json::Value> = obj.iter()
                        .map(|(k, v)| (k.clone(), v.to_json_value()))
                        .collect();
                    query.bind(serde_json::to_string(&json_map).unwrap_or_default())
                },
            };
        }

        let rows = query.fetch_all(pool).await
            .map_err(|e| QuickDbError::QueryError {
                message: format!("执行MySQL查询失败: {}", e),
            })?;
        
        let mut results = Vec::new();
        for row in rows {
            // 使用 catch_unwind 捕获可能的 panic，防止连接池崩溃
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                self.row_to_data_map(&row)
            })) {
                Ok(Ok(data_map)) => {
                    results.push(DataValue::Object(data_map));
                },
                Ok(Err(e)) => {
                    error!("行数据转换失败: {}", e);
                    // 创建一个包含错误信息的对象，而不是跳过这一行
                    let mut error_map = HashMap::new();
                    error_map.insert("error".to_string(), DataValue::String(format!("数据转换失败: {}", e)));
                    results.push(DataValue::Object(error_map));
                },
                Err(panic_info) => {
                    error!("行数据转换时发生 panic: {:?}", panic_info);
                    // 创建一个包含 panic 信息的对象
                    let mut error_map = HashMap::new();
                    error_map.insert("error".to_string(), DataValue::String("数据转换时发生内部错误".to_string()));
                    results.push(DataValue::Object(error_map));
                }
            }
        }
        
        Ok(results)
    }

    /// 执行更新操作
    async fn execute_update(
        &self,
        pool: &Pool<MySql>,
        sql: &str,
        params: &[DataValue],
    ) -> QuickDbResult<u64> {
        let mut query = sqlx::query(sql);
        
        // 绑定参数
        for param in params {
            query = match param {
                DataValue::String(s) => {
                    // 检查是否为JSON字符串，如果是则转换为对应的DataValue类型
                    let converted_value = crate::types::parse_json_string_to_data_value(s.clone());
                    match converted_value {
                        DataValue::Json(json_val) => {
                            query.bind(serde_json::to_string(&json_val).unwrap_or_default())
                        },
                        _ => query.bind(s)
                    }
                },
                DataValue::Int(i) => query.bind(*i),
                DataValue::Float(f) => query.bind(*f),
                DataValue::Bool(b) => query.bind(*b),
                DataValue::DateTime(dt) => query.bind(*dt),
                DataValue::Uuid(uuid) => query.bind(*uuid),
                DataValue::Json(json) => query.bind(json.to_string()),
                DataValue::Bytes(bytes) => query.bind(bytes.as_slice()),
                DataValue::Null => query.bind(Option::<String>::None),
                DataValue::Array(arr) => {
                    // 将DataValue数组转换为原始JSON数组
                    let json_values: Vec<serde_json::Value> = arr.iter()
                        .map(|v| v.to_json_value())
                        .collect();
                    query.bind(serde_json::to_string(&json_values).unwrap_or_default())
                },
                DataValue::Object(obj) => {
                    // 将DataValue对象转换为原始JSON对象
                    let json_map: serde_json::Map<String, serde_json::Value> = obj.iter()
                        .map(|(k, v)| (k.clone(), v.to_json_value()))
                        .collect();
                    query.bind(serde_json::to_string(&json_map).unwrap_or_default())
                },
            };
        }

        let result = query.execute(pool).await
            .map_err(|e| QuickDbError::QueryError {
                message: format!("执行MySQL更新失败: {}", e),
            })?;
        
        Ok(result.rows_affected())
    }
}

#[async_trait]
impl DatabaseAdapter for MysqlAdapter {
    async fn create(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        data: &HashMap<String, DataValue>,
        id_strategy: &IdStrategy,
    ) -> QuickDbResult<DataValue> {
        if let DatabaseConnection::MySQL(pool) = connection {
            // 自动建表逻辑：检查表是否存在，如果不存在则创建
            if !self.table_exists(connection, table).await? {
                // 获取表创建锁，防止重复创建
                let _lock = self.acquire_table_lock(table).await;
                // 再次检查表是否存在（双重检查锁定模式）
                if !self.table_exists(connection, table).await? {
                    // 尝试从模型管理器获取预定义的元数据
                    if let Some(model_meta) = manager::get_model(table) {
                        debug!("表 {} 不存在，使用预定义模型元数据创建", table);

                        // 使用模型元数据创建表
                        self.create_table(connection, table, &model_meta.fields, id_strategy).await?;
                        debug!("✅ 使用模型元数据创建MySQL表 '{}' 成功", table);
                        // 等待100ms确保数据库事务完全提交
                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                        debug!("⏱️ 等待100ms确保表 '{}' 创建完成", table);
                    } else {
                        return Err(QuickDbError::ValidationError {
                            field: "table_creation".to_string(),
                            message: format!("表 '{}' 不存在，且没有预定义的模型元数据。请先定义模型并使用 define_model! 宏明确指定字段类型。", table),
                        });
                    }
                } else {
                    debug!("表 {} 已存在，跳过创建", table);
                }
                // 锁会在这里自动释放（当 _lock 超出作用域时）
            }
            
            let (sql, params) = SqlQueryBuilder::new()
                .database_type(crate::types::DatabaseType::MySQL)
                .insert(data.clone())
                .from(table)
                .build()?;

            debug!("生成的INSERT SQL: {}", sql);
            debug!("绑定参数: {:?}", params);

            // 使用事务确保插入和获取ID在同一个连接中
            let mut tx = pool.begin().await
                .map_err(|e| QuickDbError::QueryError {
                    message: format!("开始事务失败: {}", e),
                })?;
            
            let affected_rows = {
                let mut query = sqlx::query(&sql);
                // 绑定参数
                for param in &params {
                    query = match param {
                        DataValue::String(s) => query.bind(s),
                        DataValue::Int(i) => query.bind(*i),
                        DataValue::Float(f) => query.bind(*f),
                        DataValue::Bool(b) => query.bind(*b),
                        DataValue::DateTime(dt) => query.bind(*dt),
                        DataValue::Uuid(uuid) => query.bind(*uuid),
                        DataValue::Json(json) => query.bind(json.to_string()),
                        DataValue::Bytes(bytes) => query.bind(bytes.as_slice()),
                        DataValue::Null => query.bind(Option::<String>::None),
                        DataValue::Array(arr) => {
                            let json_values: Vec<serde_json::Value> = arr.iter()
                                .map(|v| v.to_json_value())
                                .collect();
                            query.bind(serde_json::to_string(&json_values).unwrap_or_default())
                        },
                        DataValue::Object(obj) => {
                            let json_map: serde_json::Map<String, serde_json::Value> = obj.iter()
                                .map(|(k, v)| (k.clone(), v.to_json_value()))
                                .collect();
                            query.bind(serde_json::to_string(&json_map).unwrap_or_default())
                        },
                    };
                }
                
                let execute_result = query.execute(&mut *tx).await;
                match execute_result {
                    Ok(result) => {
                        let rows = result.rows_affected();
                        debug!("✅ SQL执行成功，影响的行数: {}", rows);
                        rows
                    },
                    Err(e) => {
                        debug!("❌ SQL执行失败: {}", e);
                        return Err(QuickDbError::QueryError {
                            message: format!("执行插入失败: {}", e),
                        });
                    }
                }
            };

            debug!("插入操作最终影响的行数: {}", affected_rows);

            // 根据ID策略获取返回的ID
            let id_value = match id_strategy {
                IdStrategy::AutoIncrement => {
                    // AutoIncrement策略：获取MySQL自动生成的ID
                    let last_id_row = sqlx::query("SELECT LAST_INSERT_ID()")
                        .fetch_one(&mut *tx)
                        .await
                        .map_err(|e| QuickDbError::QueryError {
                            message: format!("获取LAST_INSERT_ID失败: {}", e),
                        })?;

                    let last_id: u64 = last_id_row.try_get(0)
                        .map_err(|e| QuickDbError::QueryError {
                            message: format!("解析LAST_INSERT_ID失败: {}", e),
                        })?;

                    debug!("在事务中获取到的LAST_INSERT_ID: {}", last_id);
                    DataValue::Int(last_id as i64)
                },
                _ => {
                    // 其他策略：使用数据中的ID字段
                    if let Some(id_data) = data.get("id") {
                        debug!("使用数据中的ID字段: {:?}", id_data);
                        id_data.clone()
                    } else {
                        debug!("数据中没有ID字段，返回默认值0");
                        DataValue::Int(0)
                    }
                }
            };

            // 提交事务
            let commit_result = tx.commit().await;
            match commit_result {
                Ok(_) => debug!("✅ 事务提交成功"),
                Err(e) => {
                    debug!("❌ 事务提交失败: {}", e);
                    return Err(QuickDbError::QueryError {
                        message: format!("提交事务失败: {}", e),
                    });
                }
            }

            // 构造返回的DataValue
            let mut result_map = std::collections::HashMap::new();

            result_map.insert("id".to_string(), id_value.clone());
            result_map.insert("affected_rows".to_string(), DataValue::Int(affected_rows as i64));

            debug!("最终返回的DataValue: {:?}", DataValue::Object(result_map.clone()));
            Ok(DataValue::Object(result_map))
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望MySQL连接".to_string(),
            })
        }
    }

    async fn find_by_id(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        id: &DataValue,
    ) -> QuickDbResult<Option<DataValue>> {
        if let DatabaseConnection::MySQL(pool) = connection {
            let condition = QueryCondition {
                field: "id".to_string(),
                operator: QueryOperator::Eq,
                value: id.clone(),
            };
            
            let (sql, params) = SqlQueryBuilder::new()
                .database_type(crate::types::DatabaseType::MySQL)
                .select(&["*"])
                .from(table)
                .where_condition(condition)
                .limit(1)
                .build()?;
            
            let results = self.execute_query(pool, &sql, &params).await?;
            Ok(results.into_iter().next())
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望MySQL连接".to_string(),
            })
        }
    }

    async fn find(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
        options: &QueryOptions,
    ) -> QuickDbResult<Vec<DataValue>> {
        // 将简单条件转换为条件组合（AND逻辑）
        let condition_groups = if conditions.is_empty() {
            vec![]
        } else {
            let group_conditions = conditions.iter()
                .map(|c| QueryConditionGroup::Single(c.clone()))
                .collect();
            vec![QueryConditionGroup::Group {
                operator: crate::types::LogicalOperator::And,
                conditions: group_conditions,
            }]
        };
        
        // 统一使用 find_with_groups 实现
        self.find_with_groups(connection, table, &condition_groups, options).await
    }

    async fn find_with_groups(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        condition_groups: &[QueryConditionGroup],
        options: &QueryOptions,
    ) -> QuickDbResult<Vec<DataValue>> {
        if let DatabaseConnection::MySQL(pool) = connection {
            let mut builder = SqlQueryBuilder::new()
                .database_type(crate::types::DatabaseType::MySQL)
                .select(&["*"])
                .from(table)
                .where_condition_groups(condition_groups);
            
            // 添加排序
            for sort_field in &options.sort {
                builder = builder.order_by(&sort_field.field, sort_field.direction.clone());
            }
            
            // 添加分页
            if let Some(pagination) = &options.pagination {
                builder = builder.limit(pagination.limit).offset(pagination.skip);
            }
            
            let (sql, params) = builder.build()?;
            
            debug!("执行MySQL条件组合查询: {}", sql);
            
            self.execute_query(pool, &sql, &params).await
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望MySQL连接".to_string(),
            })
        }
    }

    async fn update(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
        data: &HashMap<String, DataValue>,
    ) -> QuickDbResult<u64> {
        if let DatabaseConnection::MySQL(pool) = connection {
            let (sql, params) = SqlQueryBuilder::new()
                .database_type(crate::types::DatabaseType::MySQL)
                .update(data.clone())
                .from(table)
                .where_conditions(conditions)
                .build()?;
            
            self.execute_update(pool, &sql, &params).await
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望MySQL连接".to_string(),
            })
        }
    }

    async fn update_by_id(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        id: &DataValue,
        data: &HashMap<String, DataValue>,
    ) -> QuickDbResult<bool> {
        if let DatabaseConnection::MySQL(pool) = connection {
            let condition = QueryCondition {
                field: "id".to_string(),
                operator: QueryOperator::Eq,
                value: id.clone(),
            };
            
            let (sql, params) = SqlQueryBuilder::new()
                .database_type(crate::types::DatabaseType::MySQL)
                .update(data.clone())
                .from(table)
                .where_condition(condition)
                .build()?;
            
            let affected_rows = self.execute_update(pool, &sql, &params).await?;
            Ok(affected_rows > 0)
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望MySQL连接".to_string(),
            })
        }
    }

    async fn update_with_operations(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
        operations: &[crate::types::UpdateOperation],
    ) -> QuickDbResult<u64> {
        if let DatabaseConnection::MySQL(pool) = connection {
            let mut set_clauses = Vec::new();
            let mut params = Vec::new();

            for operation in operations {
                match &operation.operation {
                    crate::types::UpdateOperator::Set => {
                        set_clauses.push(format!("{} = ?", operation.field));
                        params.push(operation.value.clone());
                    }
                    crate::types::UpdateOperator::Increment => {
                        set_clauses.push(format!("{} = {} + ?", operation.field, operation.field));
                        params.push(operation.value.clone());
                    }
                    crate::types::UpdateOperator::Decrement => {
                        set_clauses.push(format!("{} = {} - ?", operation.field, operation.field));
                        params.push(operation.value.clone());
                    }
                    crate::types::UpdateOperator::Multiply => {
                        set_clauses.push(format!("{} = {} * ?", operation.field, operation.field));
                        params.push(operation.value.clone());
                    }
                    crate::types::UpdateOperator::Divide => {
                        set_clauses.push(format!("{} = {} / ?", operation.field, operation.field));
                        params.push(operation.value.clone());
                    }
                    crate::types::UpdateOperator::PercentIncrease => {
                        set_clauses.push(format!("{} = {} * (1.0 + ?/100.0)", operation.field, operation.field));
                        params.push(operation.value.clone());
                    }
                    crate::types::UpdateOperator::PercentDecrease => {
                        set_clauses.push(format!("{} = {} * (1.0 - ?/100.0)", operation.field, operation.field));
                        params.push(operation.value.clone());
                    }
                }
            }

            if set_clauses.is_empty() {
                return Err(QuickDbError::ValidationError {
                    field: "operations".to_string(),
                    message: "更新操作不能为空".to_string(),
                });
            }

            let mut sql = format!("UPDATE {} SET {}", table, set_clauses.join(", "));

            // 添加WHERE条件
            if !conditions.is_empty() {
                let (where_clause, mut where_params) = SqlQueryBuilder::new()
                    .database_type(crate::types::DatabaseType::MySQL)
                    .build_where_clause(conditions)?;

                sql.push_str(&format!(" WHERE {}", where_clause));
                params.extend(where_params);
            }

            debug!("执行MySQL操作更新: {}", sql);

            self.execute_update(pool, &sql, &params).await
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望MySQL连接".to_string(),
            })
        }
    }

    async fn delete(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
    ) -> QuickDbResult<u64> {
        if let DatabaseConnection::MySQL(pool) = connection {
            let (sql, params) = SqlQueryBuilder::new()
                .database_type(crate::types::DatabaseType::MySQL)
                .delete()
                .from(table)
                .where_conditions(conditions)
                .build()?;
            
            self.execute_update(pool, &sql, &params).await
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望MySQL连接".to_string(),
            })
        }
    }

    async fn delete_by_id(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        id: &DataValue,
    ) -> QuickDbResult<bool> {
        if let DatabaseConnection::MySQL(pool) = connection {
            let condition = QueryCondition {
                field: "id".to_string(),
                operator: QueryOperator::Eq,
                value: id.clone(),
            };
            
            let (sql, params) = SqlQueryBuilder::new()
                .database_type(crate::types::DatabaseType::MySQL)
                .delete()
                .from(table)
                .where_condition(condition)
                .build()?;
            
            let affected_rows = self.execute_update(pool, &sql, &params).await?;
            Ok(affected_rows > 0)
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望MySQL连接".to_string(),
            })
        }
    }

    async fn count(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
    ) -> QuickDbResult<u64> {
        if let DatabaseConnection::MySQL(pool) = connection {
            let (sql, params) = SqlQueryBuilder::new()
                .database_type(crate::types::DatabaseType::MySQL)
                .select(&["COUNT(*) as count"])
                .from(table)
                .where_conditions(conditions)
                .build()?;
            
            let results = self.execute_query(pool, &sql, &params).await?;
            
            if let Some(result) = results.first() {
                if let DataValue::Object(map) = result {
                    if let Some(DataValue::Int(count)) = map.get("count") {
                        return Ok(*count as u64);
                    }
                }
            }
            
            Ok(0)
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望MySQL连接".to_string(),
            })
        }
    }

    async fn exists(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
    ) -> QuickDbResult<bool> {
        let count = self.count(connection, table, conditions).await?;
        Ok(count > 0)
    }

    async fn create_table(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        fields: &HashMap<String, FieldDefinition>,
        id_strategy: &IdStrategy,
    ) -> QuickDbResult<()> {
        if let DatabaseConnection::MySQL(pool) = connection {
            let mut field_definitions = Vec::new();
            
            // 统一处理id字段，根据ID策略决定类型和属性
            let id_definition = match id_strategy {
                IdStrategy::AutoIncrement => "id BIGINT AUTO_INCREMENT PRIMARY KEY".to_string(),
                IdStrategy::ObjectId => "id VARCHAR(255) PRIMARY KEY".to_string(), // ObjectId存储为字符串
                IdStrategy::Uuid => "id VARCHAR(36) PRIMARY KEY".to_string(),
                IdStrategy::Snowflake { .. } => "id BIGINT PRIMARY KEY".to_string(),
                IdStrategy::Custom(_) => "id VARCHAR(255) PRIMARY KEY".to_string(), // 自定义ID使用字符串
            };
            field_definitions.push(id_definition);

            for (name, field_definition) in fields {
                // 跳过id字段，因为已经根据策略处理过了
                if name == "id" {
                    continue;
                }

                // 非id字段的正常处理
                let sql_type = match &field_definition.field_type {
                    FieldType::String { max_length, .. } => {
                        if let Some(max_len) = max_length {
                            format!("VARCHAR({})", max_len)
                        } else {
                            // 对于没有指定长度的字符串字段，使用合理的默认长度
                            "VARCHAR(1000)".to_string()
                        }
                    },
                    FieldType::Integer { .. } => "INT".to_string(),
                    FieldType::BigInteger => "BIGINT".to_string(),
                    FieldType::Float { .. } => "FLOAT".to_string(),
                    FieldType::Double => "DOUBLE".to_string(),
                    FieldType::Text => "TEXT".to_string(),
                    FieldType::Boolean => "BOOLEAN".to_string(),
                    FieldType::DateTime => "DATETIME".to_string(),
                    FieldType::Date => "DATE".to_string(),
                    FieldType::Time => "TIME".to_string(),
                    FieldType::Uuid => "VARCHAR(36)".to_string(),
                    FieldType::Json => "JSON".to_string(),
                    FieldType::Binary => "BLOB".to_string(),
                    FieldType::Decimal { precision, scale } => format!("DECIMAL({},{})", precision, scale),
                    FieldType::Array { .. } => "JSON".to_string(),
                    FieldType::Object { .. } => "JSON".to_string(),
                    FieldType::Reference { .. } => "VARCHAR(255)".to_string(),
                };

                // 添加NULL或NOT NULL约束
                let null_constraint = if field_definition.required {
                    "NOT NULL"
                } else {
                    "NULL"
                };
                field_definitions.push(format!("{} {} {}", name, sql_type, null_constraint));
            }
            
            let sql = format!(
                "CREATE TABLE IF NOT EXISTS {} ({})",
                table,
                field_definitions.join(", ")
            );
            
            self.execute_update(pool, &sql, &[]).await?;
            
            Ok(())
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望MySQL连接".to_string(),
            })
        }
    }

    async fn create_index(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        index_name: &str,
        fields: &[String],
        unique: bool,
    ) -> QuickDbResult<()> {
        if let DatabaseConnection::MySQL(pool) = connection {
            let unique_clause = if unique { "UNIQUE " } else { "" };
            let sql = format!(
                "CREATE {}INDEX {} ON {} ({})",
                unique_clause,
                index_name,
                table,
                fields.join(", ")
            );
            
            self.execute_update(pool, &sql, &[]).await?;
            
            Ok(())
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望MySQL连接".to_string(),
            })
        }
    }

    async fn table_exists(
        &self,
        connection: &DatabaseConnection,
        table: &str,
    ) -> QuickDbResult<bool> {
        if let DatabaseConnection::MySQL(pool) = connection {
            let sql = "SELECT TABLE_NAME FROM INFORMATION_SCHEMA.TABLES WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = ?";
            let params = vec![DataValue::String(table.to_string())];
            let results = self.execute_query(pool, sql, &params).await?;
            
            Ok(!results.is_empty())
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望MySQL连接".to_string(),
            })
        }
    }

    async fn drop_table(
        &self,
        connection: &DatabaseConnection,
        table: &str,
    ) -> QuickDbResult<()> {
        if let DatabaseConnection::MySQL(pool) = connection {
            let sql = format!("DROP TABLE IF EXISTS {}", table);

            debug!("执行MySQL删除表SQL: {}", sql);

            self.execute_update(pool, &sql, &[]).await?;

            debug!("成功删除MySQL表: {}", table);
            Ok(())
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望MySQL连接".to_string(),
            })
        }
    }

    async fn get_server_version(
        &self,
        connection: &DatabaseConnection,
    ) -> QuickDbResult<String> {
        if let DatabaseConnection::MySQL(pool) = connection {
            let sql = "SELECT VERSION()";

            debug!("执行MySQL版本查询SQL: {}", sql);

            let results = self.execute_query(pool, sql, &[]).await?;

            if let Some(result) = results.first() {
                match result {
                    DataValue::Object(obj) => {
                        // MySQL适配器返回的是Object包装的结果，需要提取版本信息
                        if let Some((_, DataValue::String(version))) = obj.iter().next() {
                            debug!("成功获取MySQL版本: {}", version);
                            Ok(version.clone())
                        } else {
                            Err(QuickDbError::QueryError {
                                message: "MySQL版本查询返回的对象中没有找到字符串版本信息".to_string(),
                            })
                        }
                    },
                    DataValue::String(version) => {
                        // 兼容直接返回字符串的情况
                        debug!("成功获取MySQL版本: {}", version);
                        Ok(version.clone())
                    },
                    _ => {
                        debug!("MySQL版本查询返回了意外的数据类型: {:?}", result);
                        Err(QuickDbError::QueryError {
                            message: "MySQL版本查询返回了非字符串结果".to_string(),
                        })
                    },
                }
            } else {
                Err(QuickDbError::QueryError {
                    message: "MySQL版本查询返回了空结果".to_string(),
                })
            }
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望MySQL连接".to_string(),
            })
        }
    }
}

impl Default for MysqlAdapter {
    fn default() -> Self {
        Self::new()
    }
}