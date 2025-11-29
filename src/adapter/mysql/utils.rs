    //! MySQL适配器辅助工具方法

use crate::adapter::MysqlAdapter;
use crate::error::{QuickDbError, QuickDbResult};
use crate::types::{DataValue, QueryCondition, QueryConditionGroup, LogicalOperator, QueryOperator};
use crate::adapter::mysql::query_builder::SqlQueryBuilder;
use async_trait::async_trait;
use rat_logger::{debug, warn, error};
use std::collections::HashMap;
use sqlx::{MySql, Pool, Row, Column, TypeInfo};
use sqlx::mysql::MySqlRow;
use serde_json::Value as JsonValue;

impl MysqlAdapter {
    /// 安全地读取整数字段，防止 byteorder 错误
    pub fn safe_read_integer(row: &MySqlRow, column_name: &str) -> QuickDbResult<DataValue> {
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
    pub fn safe_read_float(row: &MySqlRow, column_name: &str) -> QuickDbResult<DataValue> {
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
    pub fn safe_read_json(row: &MySqlRow, column_name: &str) -> QuickDbResult<DataValue> {
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
                                            .map(|(k, v)| (k, crate::types::data_value::json_value_to_data_value(v)))
                                            .collect();
                                        debug!("转换为DataValue::Object，包含{}个字段", data_object.len());
                                        Ok(DataValue::Object(data_object))
                                    },
                                    JsonValue::Array(arr) => {
                                        let data_array: Vec<DataValue> = arr.into_iter()
                                            .map(|v| crate::types::data_value::json_value_to_data_value(v))
                                            .collect();
                                        debug!("转换为DataValue::Array，包含{}个元素", data_array.len());
                                        Ok(DataValue::Array(data_array))
                                    },
                                    _ => {
                                        debug!("转换为其他DataValue类型");
                                        Ok(crate::types::data_value::json_value_to_data_value(json_value))
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
    pub fn safe_read_bool(row: &MySqlRow, column_name: &str) -> QuickDbResult<DataValue> {
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
    pub fn row_to_data_map(&self, row: &MySqlRow) -> QuickDbResult<HashMap<String, DataValue>> {
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
                            Some(dt) => DataValue::DateTime(dt.with_timezone(&chrono::FixedOffset::east(0))),
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
    pub async fn execute_query(
        &self,
        pool: &Pool<MySql>,
        sql: &str,
        params: &[DataValue],
    ) -> QuickDbResult<Vec<DataValue>> {
        let mut query = sqlx::query::<sqlx::MySql>(sql);
        
        // 绑定参数
        for param in params {
            query = match param {
                DataValue::String(s) => {
                    // 检查是否为JSON字符串，如果是则转换为对应的DataValue类型
                    let converted_value = crate::types::data_value::parse_json_string_to_data_value(s.clone());
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
                DataValue::DateTime(dt) => query.bind(dt.naive_utc().and_utc()),
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
    pub async fn execute_update(
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
                    let converted_value = crate::types::data_value::parse_json_string_to_data_value(s.clone());
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
                DataValue::DateTime(dt) => query.bind(dt.naive_utc().and_utc()),
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
