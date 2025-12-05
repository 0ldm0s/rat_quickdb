//! PostgreSQL 专用工具模块
//!
//! 提供PostgreSQL数据库的特殊处理工具，包括JSONB查询、类型转换等功能

use crate::error::{QuickDbError, QuickDbResult};
use crate::types::DataValue;
use std::collections::HashMap;

/// PostgreSQL JSON字段查询策略选择和SQL生成
/// 根据查询值的类型和内容，选择合适的JSON查询策略
///
/// # 参数
/// * `field_name` - 字段名
/// * `value` - 查询值
/// * `placeholder` - SQL占位符
///
/// # 返回值
/// * `Ok((String, DataValue))` - (SQL条件片段, 参数值)
/// * `Err(QuickDbError)` - 查询值无效
pub fn build_json_query_condition(
    field_name: &str,
    value: &DataValue,
    placeholder: &str,
) -> QuickDbResult<(String, DataValue)> {
    #[cfg(debug_assertions)]
    {
        rat_logger::debug!("PostgreSQL JSON查询策略分析:");
        rat_logger::debug!("  字段名: {}", field_name);
        rat_logger::debug!("  查询值: {:?}", value);
        rat_logger::debug!("  值类型: {:?}", std::mem::discriminant(value));
    }

    match value {
        DataValue::String(s) => {
            let trimmed = s.trim_start();

            // 检查是否是精确JSON匹配
            if (trimmed.starts_with('{') && trimmed.ends_with('}'))
                || (trimmed.starts_with('[') && trimmed.ends_with(']'))
            {
                // 验证JSON格式
                match serde_json::from_str::<serde_json::Value>(s) {
                    Ok(json_val) => {
                        #[cfg(debug_assertions)]
                        rat_logger::debug!("  策略: 精确JSON匹配，使用 @> 操作符");

                        // 使用JSONB包含操作符进行精确匹配
                        Ok((
                            format!("{} @> {}", field_name, placeholder),
                            DataValue::Json(json_val),
                        ))
                    }
                    Err(e) => {
                        #[cfg(debug_assertions)]
                        rat_logger::debug!("  JSON格式无效: {}，回退到文本搜索", e);

                        // JSON格式无效，回退到文本搜索
                        Ok((
                            format!("{}::text ILIKE {}", field_name, placeholder),
                            DataValue::String(format!("%{}%", s)),
                        ))
                    }
                }
            } else {
                // 普通字符串：使用文本搜索策略（用户期望的行为）
                #[cfg(debug_assertions)]
                rat_logger::debug!("  策略: 文本搜索，使用 ::text ILIKE");

                Ok((
                    format!("{}::text ILIKE {}", field_name, placeholder),
                    DataValue::String(format!("%{}%", s)),
                ))
            }
        }

        // 数值类型：使用JSONB精确匹配
        DataValue::Int(i) => {
            #[cfg(debug_assertions)]
            rat_logger::debug!("  策略: 数值精确匹配，使用 @> 操作符");

            let json_val = serde_json::Value::Number(serde_json::Number::from(*i));
            Ok((
                format!("{} @> ?", field_name),
                DataValue::Json(json_val)
            ))
        }

        DataValue::UInt(u) => {
            #[cfg(debug_assertions)]
            rat_logger::debug!("  策略: 无符号数值精确匹配，使用 @> 操作符");

            let json_val = serde_json::Value::Number(serde_json::Number::from(*u));
            Ok((
                format!("{} @> ?", field_name),
                DataValue::Json(json_val)
            ))
        }

        DataValue::Float(f) => {
            #[cfg(debug_assertions)]
            rat_logger::debug!("  策略: 浮点数精确匹配，使用 @> 操作符");

            let json_val = serde_json::Number::from_f64(*f)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null);
            Ok((
                format!("{} @> {}", field_name, placeholder),
                DataValue::Json(json_val),
            ))
        }

        // 布尔类型：使用JSONB精确匹配
        DataValue::Bool(b) => {
            #[cfg(debug_assertions)]
            rat_logger::debug!("  策略: 布尔值精确匹配，使用 @> 操作符");

            Ok((
                format!("{} @> {}", field_name, placeholder),
                DataValue::Json(serde_json::Value::Bool(*b)),
            ))
        }

        // Null值：检查JSON字段是否为null
        DataValue::Null => {
            #[cfg(debug_assertions)]
            rat_logger::debug!("  策略: Null值检查");

            Ok((format!("{} IS NULL", field_name), DataValue::Null))
        }

        // JSON类型：直接使用精确匹配
        DataValue::Json(json_val) => {
            #[cfg(debug_assertions)]
            rat_logger::debug!("  策略: JSON值精确匹配，使用 @> 操作符");

            Ok((
                format!("{} @> {}", field_name, placeholder),
                DataValue::Json(json_val.clone()),
            ))
        }

        // 数组类型：转换为JSON数组进行匹配
        DataValue::Array(arr) => {
            #[cfg(debug_assertions)]
            rat_logger::debug!("  策略: 数组精确匹配，转换为JSON");

            let json_array: Vec<serde_json::Value> = arr
                .iter()
                .map(|item| match item {
                    DataValue::String(s) => serde_json::Value::String(s.clone()),
                    DataValue::Int(i) => serde_json::Value::Number(serde_json::Number::from(*i)),
                    DataValue::UInt(u) => serde_json::Value::Number(serde_json::Number::from(*u)),
                    DataValue::Float(f) => serde_json::Number::from_f64(*f)
                        .map(serde_json::Value::Number)
                        .unwrap_or(serde_json::Value::Null),
                    DataValue::Bool(b) => serde_json::Value::Bool(*b),
                    DataValue::Null => serde_json::Value::Null,
                    _ => serde_json::Value::String(item.to_string()),
                })
                .collect();

            Ok((
                format!("{} @> {}", field_name, placeholder),
                DataValue::Json(serde_json::Value::Array(json_array)),
            ))
        }

        // 其他类型：转换为字符串进行文本搜索
        _ => {
            #[cfg(debug_assertions)]
            rat_logger::debug!("  策略: 其他类型，转换为文本搜索");

            let text_value = value.to_string();
            Ok((
                format!("{}::text ILIKE {}", field_name, placeholder),
                DataValue::String(format!("%{}%", text_value)),
            ))
        }
    }
}

/// PostgreSQL JSONB查询值转换和验证函数
/// 将普通DataValue转换为适合PostgreSQL JSONB查询的格式
///
/// 根据用户友好的原则：
/// - 对于字符串查询，返回适合文本搜索的格式
/// - 对于其他类型，返回精确匹配的JSON格式
///
/// # 参数
/// * `value` - 原始查询值
///
/// # 返回值
/// * `Ok(DataValue)` - 转换后适合JSONB查询的值
/// * `Err(QuickDbError)` - 值不适合JSONB查询（如二进制数据过大或不支持的类型）
pub fn convert_to_jsonb_value(value: &DataValue) -> QuickDbResult<DataValue> {
    const MAX_JSONB_LENGTH: usize = 1024 * 1024; // 1MB限制

    match value {
        // 字符串值：根据用户友好原则，支持文本搜索和精确匹配两种模式
        DataValue::String(s) => {
            // 检查长度
            if s.len() > MAX_JSONB_LENGTH {
                return Err(QuickDbError::ValidationError {
                    field: "jsonb_value".to_string(),
                    message: format!("JSONB查询值过长，最大允许{}字节", MAX_JSONB_LENGTH),
                });
            }

            // 检查是否已经是有效的JSON字符串（精确匹配模式）
            let trimmed = s.trim_start();
            if (trimmed.starts_with('{') && trimmed.ends_with('}'))
                || (trimmed.starts_with('[') && trimmed.ends_with(']'))
            {
                // 已经是JSON格式，验证是否有效
                match serde_json::from_str::<serde_json::Value>(s) {
                    Ok(_) => {
                        // 有效的JSON，返回精确匹配模式
                        #[cfg(debug_assertions)]
                        rat_logger::debug!("  检测到有效JSON字符串，使用精确匹配模式");
                        Ok(DataValue::String(s.clone()))
                    }
                    Err(_) => {
                        // 看起来像JSON但无效，作为普通字符串处理
                        #[cfg(debug_assertions)]
                        rat_logger::debug!("  JSON字符串格式无效，使用文本搜索模式");
                        Ok(DataValue::String(format!("%{}%", s)))
                    }
                }
            } else {
                // 普通字符串：使用文本搜索模式（用户期望的行为）
                #[cfg(debug_assertions)]
                rat_logger::debug!("  检测到普通字符串，使用文本搜索模式: '%{}%'", s);

                // 返回适合ILIKE查询的模式，这样用户可以搜索JSON中的任何文本内容
                Ok(DataValue::String(format!("%{}%", s)))
            }
        }

        // 数字类型：直接转换为JSON数字格式
        DataValue::Int(i) => Ok(DataValue::String(i.to_string())),
        DataValue::Float(f) => Ok(DataValue::String(f.to_string())),

        // 布尔值：转换为JSON布尔格式
        DataValue::Bool(b) => Ok(DataValue::String(b.to_string())),

        // Null值：转换为JSON null
        DataValue::Null => Ok(DataValue::String("null".to_string())),

        // 数组类型：序列化为JSON数组
        DataValue::Array(arr) => {
            // 检查数组大小
            if arr.len() > 1000 {
                // 限制数组元素数量
                return Err(QuickDbError::ValidationError {
                    field: "jsonb_array".to_string(),
                    message: "JSONB查询数组元素过多，最大允许1000个元素".to_string(),
                });
            }

            match serde_json::to_string(arr) {
                Ok(json_str) => {
                    if json_str.len() > MAX_JSONB_LENGTH {
                        return Err(QuickDbError::ValidationError {
                            field: "jsonb_array".to_string(),
                            message: format!("JSONB查询数组过长，最大允许{}字节", MAX_JSONB_LENGTH),
                        });
                    }
                    Ok(DataValue::String(json_str))
                }
                Err(e) => Err(QuickDbError::SerializationError {
                    message: format!("数组序列化为JSON失败: {}", e),
                }),
            }
        }

        // 对象类型：序列化为JSON对象
        DataValue::Object(obj) => {
            // 检查对象大小
            if obj.len() > 1000 {
                // 限制对象字段数量
                return Err(QuickDbError::ValidationError {
                    field: "jsonb_object".to_string(),
                    message: "JSONB查询对象字段过多，最大允许1000个字段".to_string(),
                });
            }

            match serde_json::to_string(obj) {
                Ok(json_str) => {
                    if json_str.len() > MAX_JSONB_LENGTH {
                        return Err(QuickDbError::ValidationError {
                            field: "jsonb_object".to_string(),
                            message: format!("JSONB查询对象过长，最大允许{}字节", MAX_JSONB_LENGTH),
                        });
                    }
                    Ok(DataValue::String(json_str))
                }
                Err(e) => Err(QuickDbError::SerializationError {
                    message: format!("对象序列化为JSON失败: {}", e),
                }),
            }
        }

        // JSON类型：直接序列化
        DataValue::Json(json_val) => match serde_json::to_string(json_val) {
            Ok(json_str) => {
                if json_str.len() > MAX_JSONB_LENGTH {
                    return Err(QuickDbError::ValidationError {
                        field: "jsonb_value".to_string(),
                        message: format!("JSONB查询值过长，最大允许{}字节", MAX_JSONB_LENGTH),
                    });
                }
                Ok(DataValue::String(json_str))
            }
            Err(e) => Err(QuickDbError::SerializationError {
                message: format!("JSON值序列化失败: {}", e),
            }),
        },

        // 数值类型：转换为JSON数字
        DataValue::Int(i) => {
            let json_val = serde_json::Value::Number(serde_json::Number::from(*i));
            Ok(DataValue::String(json_val.to_string()))
        }

        DataValue::UInt(u) => {
            let json_val = serde_json::Value::Number(serde_json::Number::from(*u));
            Ok(DataValue::String(json_val.to_string()))
        }

        DataValue::Float(f) => {
            let json_val = serde_json::Number::from_f64(*f)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null);
            Ok(DataValue::String(json_val.to_string()))
        }

        // 日期时间：转换为ISO8601字符串
        DataValue::DateTime(dt) => Ok(DataValue::String(dt.to_rfc3339())),
        DataValue::DateTimeUTC(dt) => Ok(DataValue::String(dt.to_rfc3339())),

        // UUID：直接转换为字符串
        DataValue::Uuid(u) => Ok(DataValue::String(u.to_string())),

        // 二进制数据：拒绝用于JSONB查询
        DataValue::Bytes(bytes) => {
            if bytes.len() > 1024 {
                // 1KB限制
                return Err(QuickDbError::ValidationError {
                    field: "jsonb_bytes".to_string(),
                    message: "二进制数据过大，不能用于JSONB查询".to_string(),
                });
            }
            // 转换为base64字符串
            Ok(DataValue::String(format!("\"{}\"", base64::encode(bytes))))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_json_query_condition_string_search() {
        let (sql, param) =
            build_json_query_condition("profile", &DataValue::String("Rust".to_string()), "$1")
                .unwrap();
        assert_eq!(sql, "profile::text ILIKE $1");
        assert_eq!(param, DataValue::String("%Rust%".to_string()));
    }

    #[test]
    fn test_json_query_condition_exact_match() {
        let json_value = json!({"theme": "dark"});
        let (sql, param) =
            build_json_query_condition("settings", &DataValue::Json(json_value.clone()), "$1")
                .unwrap();
        assert_eq!(sql, "settings @> $1");
        assert_eq!(param, DataValue::Json(json_value));
    }

    #[test]
    fn test_json_query_condition_number() {
        let (sql, param) = build_json_query_condition("age", &DataValue::Int(25), "$1").unwrap();
        assert_eq!(sql, "age @> $1");
        assert_eq!(param, DataValue::Json(json!(25)));
    }

    #[test]
    fn test_convert_to_jsonb_value_string() {
        let result = convert_to_jsonb_value(&DataValue::String("Rust".to_string())).unwrap();
        assert_eq!(result, DataValue::String("%Rust%".to_string()));
    }

    #[test]
    fn test_convert_to_jsonb_value_json_string() {
        let json_str = "{\"theme\": \"dark\"}";
        let result = convert_to_jsonb_value(&DataValue::String(json_str.to_string())).unwrap();
        assert_eq!(result, DataValue::String(json_str.to_string()));
    }
}
