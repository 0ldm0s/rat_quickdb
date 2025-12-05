use chrono::{DateTime, FixedOffset, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// 通用数据值类型 - 支持跨数据库的数据表示
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum DataValue {
    /// 空值
    Null,
    /// 布尔值
    Bool(bool),
    /// 整数
    Int(i64),
    /// 无符号整数
    UInt(u64),
    /// 浮点数
    Float(f64),
    /// 字符串
    String(String),
    /// 字节数组
    Bytes(Vec<u8>),
    /// 日期时间
    DateTime(DateTime<FixedOffset>),
    /// UTC日期时间
    DateTimeUTC(DateTime<Utc>),
    /// UUID
    Uuid(Uuid),
    /// JSON 对象
    Json(serde_json::Value),
    /// 数组
    Array(Vec<DataValue>),
    /// 对象/文档
    Object(HashMap<String, DataValue>),
}

impl std::fmt::Display for DataValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataValue::Null => write!(f, "null"),
            DataValue::Bool(b) => write!(f, "{}", b),
            DataValue::Int(i) => write!(f, "{}", i),
            DataValue::UInt(u) => write!(f, "{}", u),
            DataValue::Float(fl) => write!(f, "{}", fl),
            DataValue::String(s) => write!(f, "{}", s),
            DataValue::Bytes(bytes) => write!(f, "[{} bytes]", bytes.len()),
            DataValue::DateTime(dt) => write!(f, "{}", dt.to_rfc3339()),
            DataValue::DateTimeUTC(dt) => write!(f, "{}", dt.to_rfc3339()),
            DataValue::Uuid(uuid) => write!(f, "{}", uuid),
            DataValue::Json(json) => write!(f, "{}", json),
            DataValue::Array(arr) => {
                let json_str = serde_json::to_string(arr).unwrap_or_default();
                write!(f, "{}", json_str)
            }
            DataValue::Object(obj) => {
                let json_str = serde_json::to_string(obj).unwrap_or_default();
                write!(f, "{}", json_str)
            }
        }
    }
}

impl std::fmt::Debug for DataValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Debug trait 和 Display 保持一致，显示实际值而不是类型构造函数
        write!(f, "{}", self)
    }
}

impl DataValue {
    /// 获取数据类型名称
    pub fn type_name(&self) -> &'static str {
        match self {
            DataValue::Null => "null",
            DataValue::Bool(_) => "boolean",
            DataValue::Int(_) => "integer",
            DataValue::UInt(_) => "unsigned_integer",
            DataValue::Float(_) => "float",
            DataValue::String(_) => "string",
            DataValue::Bytes(_) => "bytes",
            DataValue::DateTime(_) => "datetime",
            DataValue::DateTimeUTC(_) => "datetime",
            DataValue::Uuid(_) => "uuid",
            DataValue::Json(_) => "json",
            DataValue::Array(_) => "array",
            DataValue::Object(_) => "object",
        }
    }

    /// 判断是否为空值
    pub fn is_null(&self) -> bool {
        matches!(self, DataValue::Null)
    }

    /// 转换为 JSON 字符串
    pub fn to_json_string(&self) -> Result<String, crate::error::QuickDbError> {
        serde_json::to_string(self).map_err(|e| {
            crate::quick_error!(serialization, format!("DataValue 转换为 JSON 失败: {}", e))
        })
    }

    /// 从 JSON 字符串解析
    pub fn from_json_string(json: &str) -> Result<Self, crate::error::QuickDbError> {
        serde_json::from_str(json).map_err(|e| {
            crate::quick_error!(serialization, format!("JSON 解析为 DataValue 失败: {}", e))
        })
    }

    /// 转换为 JSON 值
    pub fn to_json_value(&self) -> serde_json::Value {
        match self {
            DataValue::Null => serde_json::Value::Null,
            DataValue::Bool(b) => serde_json::Value::Bool(*b),
            DataValue::Int(i) => serde_json::Value::Number(serde_json::Number::from(*i)),
            DataValue::UInt(u) => serde_json::Value::Number(serde_json::Number::from(*u)),
            DataValue::Float(f) => {
                serde_json::Number::from_f64(*f)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null)
            },
            DataValue::String(s) => serde_json::Value::String(s.clone()),
            DataValue::Bytes(b) => {
                // 将字节数组转换为 base64 字符串
                serde_json::Value::String(base64::encode(b))
            }
            DataValue::DateTime(dt) => serde_json::Value::String(dt.to_rfc3339()),
            DataValue::DateTimeUTC(dt) => serde_json::Value::String(dt.to_rfc3339()),
            DataValue::Uuid(u) => serde_json::Value::String(u.to_string()),
            DataValue::Json(j) => {
                // 对于 JSON 值，需要检查是否包含带类型标签的数组或对象
                match j {
                    serde_json::Value::Array(arr) => {
                        // 检查数组元素是否是带类型标签的对象，如果是则提取原始值
                        let cleaned_array: Vec<serde_json::Value> = arr
                            .iter()
                            .map(|item| {
                                if let serde_json::Value::Object(obj) = item {
                                    // 检查是否是单键对象（类型标签格式）
                                    if obj.len() == 1 {
                                        let (key, value) = obj.iter().next().unwrap();
                                        match key.as_str() {
                                            "String" | "Int" | "Float" | "Bool" | "Null"
                                            | "Bytes" | "DateTime" | "Uuid" => value.clone(),
                                            _ => item.clone(),
                                        }
                                    } else {
                                        item.clone()
                                    }
                                } else {
                                    item.clone()
                                }
                            })
                            .collect();
                        serde_json::Value::Array(cleaned_array)
                    }
                    _ => j.clone(),
                }
            }
            DataValue::Array(arr) => {
                let json_array: Vec<serde_json::Value> = arr
                    .iter()
                    .map(|item| {
                        // 对于数组元素，直接提取原始值，避免带类型标签的序列化
                        match item {
                            DataValue::String(s) => serde_json::Value::String(s.clone()),
                        DataValue::Int(i) => serde_json::Value::Number(serde_json::Number::from(*i)),
                            DataValue::UInt(u) => serde_json::Value::Number(serde_json::Number::from(*u)),
                            DataValue::Float(f) => {
                                serde_json::Number::from_f64(*f)
                                    .map(serde_json::Value::Number)
                                    .unwrap_or(serde_json::Value::Null)
                            },
                            DataValue::Bool(b) => serde_json::Value::Bool(*b),
                            DataValue::Null => serde_json::Value::Null,
                            DataValue::Bytes(b) => serde_json::Value::String(base64::encode(b)),
                            DataValue::DateTime(dt) => serde_json::Value::String(dt.to_rfc3339()),
                            DataValue::DateTimeUTC(dt) => {
                                serde_json::Value::String(dt.to_rfc3339())
                            }
                            DataValue::Uuid(u) => serde_json::Value::String(u.to_string()),
                            DataValue::Json(j) => j.clone(),
                            // 对于复杂类型，仍然递归调用
                            _ => item.to_json_value(),
                        }
                    })
                    .collect();
                serde_json::Value::Array(json_array)
            }
            DataValue::Object(obj) => {
                let json_object: serde_json::Map<String, serde_json::Value> = obj
                    .iter()
                    .map(|(k, v)| (k.clone(), v.to_json_value()))
                    .collect();
                serde_json::Value::Object(json_object)
            }
        }
    }

    /// 从 JSON 值解析
    pub fn from_json_value(value: serde_json::Value) -> Self {
        serde_json::from_value(value).unwrap_or(DataValue::Null)
    }

    /// 转换为 JSON（兼容旧代码）
    pub fn to_json(&self) -> serde_json::Value {
        self.to_json_value()
    }

    /// 从 JSON 解析（兼容旧代码）
    pub fn from_json(value: serde_json::Value) -> Self {
        Self::from_json_value(value)
    }

    /// 直接反序列化为指定类型
    pub fn deserialize_to<T>(&self) -> Result<T, crate::error::QuickDbError>
    where
        T: serde::de::DeserializeOwned,
    {
        serde_json::from_value(serde_json::to_value(self)?).map_err(|e| {
            crate::quick_error!(serialization, format!("DataValue 反序列化失败: {}", e))
        })
    }

    /// 期望Object类型，如果不是则返回错误
    pub fn expect_object(self) -> Result<HashMap<String, DataValue>, crate::error::QuickDbError> {
        match self {
            DataValue::Object(map) => Ok(map),
            other => Err(crate::quick_error!(
                validation,
                "data_type",
                format!("期望Object类型，但收到: {}", other.type_name())
            )),
        }
    }
}
impl From<bool> for DataValue {
    fn from(value: bool) -> Self {
        DataValue::Bool(value)
    }
}

impl From<i32> for DataValue {
    fn from(value: i32) -> Self {
        DataValue::Int(value as i64)
    }
}

impl From<i64> for DataValue {
    fn from(value: i64) -> Self {
        DataValue::Int(value)
    }
}


impl From<f32> for DataValue {
    fn from(value: f32) -> Self {
        DataValue::Float(value as f64)
    }
}

impl From<f64> for DataValue {
    fn from(value: f64) -> Self {
        DataValue::Float(value)
    }
}

impl From<String> for DataValue {
    fn from(value: String) -> Self {
        DataValue::String(value)
    }
}

impl From<&str> for DataValue {
    fn from(value: &str) -> Self {
        DataValue::String(value.to_string())
    }
}

impl From<Vec<u8>> for DataValue {
    fn from(value: Vec<u8>) -> Self {
        DataValue::Bytes(value)
    }
}

impl From<DateTime<Utc>> for DataValue {
    fn from(value: DateTime<Utc>) -> Self {
        DataValue::DateTime(value.with_timezone(&FixedOffset::east(0)))
    }
}

impl From<DateTime<FixedOffset>> for DataValue {
    fn from(value: DateTime<FixedOffset>) -> Self {
        DataValue::DateTime(value)
    }
}

impl From<Uuid> for DataValue {
    fn from(value: Uuid) -> Self {
        DataValue::Uuid(value)
    }
}

impl From<serde_json::Value> for DataValue {
    fn from(value: serde_json::Value) -> Self {
        DataValue::Json(value)
    }
}

/// 将 serde_json::Value 正确转换为对应的 DataValue 类型
/// 而不是简单包装为 DataValue::Json
pub fn json_value_to_data_value(value: serde_json::Value) -> DataValue {
    match value {
        serde_json::Value::Null => DataValue::Null,
        serde_json::Value::Bool(b) => DataValue::Bool(b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                DataValue::Int(i)
            } else if let Some(u) = n.as_u64() {
                DataValue::UInt(u)
            } else if let Some(f) = n.as_f64() {
                DataValue::Float(f)
            } else {
                DataValue::Json(serde_json::Value::Number(n))
            }
        }
        serde_json::Value::String(s) => DataValue::String(s),
        serde_json::Value::Array(arr) => {
            // 递归转换数组元素为DataValue
            let data_array: Vec<DataValue> =
                arr.into_iter().map(json_value_to_data_value).collect();
            DataValue::Array(data_array)
        }
        serde_json::Value::Object(obj) => {
            // 递归转换对象为HashMap<String, DataValue>
            let data_object: HashMap<String, DataValue> = obj
                .into_iter()
                .map(|(k, v)| (k, json_value_to_data_value(v)))
                .collect();
            DataValue::Object(data_object)
        }
    }
}

/// SQL适配器通用的JSON字符串检测和反序列化方法
/// 基于SQLite成功的修复方案，用于处理存储为JSON字符串的数组和对象字段
///
/// # 参数
/// * `value` - 可能包含JSON字符串的字符串值
///
/// # 返回值
/// * 如果字符串以'['或'{'开头且能成功解析为JSON，返回对应的DataValue::Array或DataValue::Object
/// * 否则返回DataValue::String
pub fn parse_json_string_to_data_value(value: String) -> DataValue {
    // 检查字符串是否以JSON数组或对象标识符开头
    if value.starts_with('[') || value.starts_with('{') {
        // 尝试解析为JSON
        match serde_json::from_str::<serde_json::Value>(&value) {
            Ok(json_value) => {
                // 根据JSON类型返回对应的DataValue
                json_value_to_data_value(json_value)
            }
            Err(_) => {
                // 解析失败，作为普通字符串处理
                DataValue::String(value)
            }
        }
    } else {
        // 不是JSON格式，直接返回字符串
        DataValue::String(value)
    }
}

/// SQL适配器通用的可选字符串JSON检测和反序列化方法
/// 处理数据库中可能为NULL的字符串字段
///
/// # 参数
/// * `value` - 可能为None的字符串值
///
/// # 返回值
/// * 如果值为None，返回DataValue::Null
/// * 如果字符串以'['或'{'开头且能成功解析为JSON，返回对应的DataValue::Array或DataValue::Object
/// * 否则返回DataValue::String
pub fn parse_optional_json_string_to_data_value(value: Option<String>) -> DataValue {
    match value {
        Some(s) => parse_json_string_to_data_value(s),
        None => DataValue::Null,
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
pub fn convert_to_postgresql_jsonb_value(
    value: &DataValue,
) -> crate::error::QuickDbResult<DataValue> {
    const MAX_JSONB_LENGTH: usize = 1024 * 1024; // 1MB限制

    match value {
        // 字符串值：根据用户友好原则，支持文本搜索和精确匹配两种模式
        DataValue::String(s) => {
            // 检查长度
            if s.len() > MAX_JSONB_LENGTH {
                return Err(crate::quick_error!(
                    validation,
                    "jsonb_value_too_long",
                    format!("JSONB查询值过长，最大允许{}字节", MAX_JSONB_LENGTH)
                ));
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
        DataValue::UInt(u) => Ok(DataValue::String(u.to_string())),
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
                return Err(crate::quick_error!(
                    validation,
                    "jsonb_array_too_large",
                    "JSONB查询数组元素过多，最大允许1000个元素"
                ));
            }

            match serde_json::to_string(arr) {
                Ok(json_str) => {
                    if json_str.len() > MAX_JSONB_LENGTH {
                        return Err(crate::quick_error!(
                            validation,
                            "jsonb_array_too_long",
                            format!("JSONB查询数组过长，最大允许{}字节", MAX_JSONB_LENGTH)
                        ));
                    }
                    Ok(DataValue::String(json_str))
                }
                Err(e) => Err(crate::quick_error!(
                    serialization,
                    format!("数组序列化为JSON失败: {}", e)
                )),
            }
        }

        // 对象类型：序列化为JSON对象
        DataValue::Object(obj) => {
            // 检查对象大小
            if obj.len() > 1000 {
                // 限制对象字段数量
                return Err(crate::quick_error!(
                    validation,
                    "jsonb_object_too_large",
                    "JSONB查询对象字段过多，最大允许1000个字段"
                ));
            }

            match serde_json::to_string(obj) {
                Ok(json_str) => {
                    if json_str.len() > MAX_JSONB_LENGTH {
                        return Err(crate::quick_error!(
                            validation,
                            "jsonb_object_too_long",
                            format!("JSONB查询对象过长，最大允许{}字节", MAX_JSONB_LENGTH)
                        ));
                    }
                    Ok(DataValue::String(json_str))
                }
                Err(e) => Err(crate::quick_error!(
                    serialization,
                    format!("对象序列化为JSON失败: {}", e)
                )),
            }
        }

        // JSON类型：直接序列化
        DataValue::Json(json_val) => match serde_json::to_string(json_val) {
            Ok(json_str) => {
                if json_str.len() > MAX_JSONB_LENGTH {
                    return Err(crate::quick_error!(
                        validation,
                        "jsonb_value_too_long",
                        format!("JSONB查询值过长，最大允许{}字节", MAX_JSONB_LENGTH)
                    ));
                }
                Ok(DataValue::String(json_str))
            }
            Err(e) => Err(crate::quick_error!(
                serialization,
                format!("JSON值序列化失败: {}", e)
            )),
        },

        // 日期时间：转换为ISO8601字符串
        DataValue::DateTime(dt) => Ok(DataValue::String(dt.to_rfc3339())),
        DataValue::DateTimeUTC(dt) => Ok(DataValue::String(dt.to_rfc3339())),

        // UUID：直接转换为字符串
        DataValue::Uuid(u) => Ok(DataValue::String(u.to_string())),

        // 二进制数据：拒绝用于JSONB查询
        DataValue::Bytes(bytes) => {
            if bytes.len() > 1024 {
                // 1KB限制
                return Err(crate::quick_error!(
                    validation,
                    "jsonb_bytes_too_large",
                    "二进制数据过大，不能用于JSONB查询"
                ));
            }
            // 转换为base64字符串
            Ok(DataValue::String(format!("\"{}\"", base64::encode(bytes))))
        }
    }
}

// 为常用的Vec类型提供From实现，直接序列化为JSON字符串
impl From<Vec<String>> for DataValue {
    fn from(value: Vec<String>) -> Self {
        match serde_json::to_string(&value) {
            Ok(json_str) => DataValue::String(json_str),
            Err(_) => DataValue::Array(vec![]), // 序列化失败时返回空数组
        }
    }
}

impl From<Vec<i32>> for DataValue {
    fn from(value: Vec<i32>) -> Self {
        match serde_json::to_string(&value) {
            Ok(json_str) => DataValue::String(json_str),
            Err(_) => DataValue::Array(vec![]), // 序列化失败时返回空数组
        }
    }
}

impl From<Vec<i64>> for DataValue {
    fn from(value: Vec<i64>) -> Self {
        match serde_json::to_string(&value) {
            Ok(json_str) => DataValue::String(json_str),
            Err(_) => DataValue::Array(vec![]), // 序列化失败时返回空数组
        }
    }
}

impl From<Vec<f64>> for DataValue {
    fn from(value: Vec<f64>) -> Self {
        match serde_json::to_string(&value) {
            Ok(json_str) => DataValue::String(json_str),
            Err(_) => DataValue::Array(vec![]), // 序列化失败时返回空数组
        }
    }
}

impl<T> From<Option<T>> for DataValue
where
    T: Into<DataValue>,
{
    fn from(value: Option<T>) -> Self {
        match value {
            Some(v) => v.into(),
            None => DataValue::Null,
        }
    }
}
