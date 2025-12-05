//! 模型数据转换模块
//!
//! 提供从DataValue映射到模型实例的直接转换功能

use crate::error::{QuickDbError, QuickDbResult};
use crate::types::DataValue;
use std::collections::HashMap;
use crate::debug_log;

/// 从DataValue映射直接创建模型实例
///
/// 直接从HashMap<String, DataValue>转换为模型实例，避免JSON中转
/// 这是高效的数据转换方法，消除了不必要的序列化开销
pub fn create_model_from_data_map<T>(
    data_map: &HashMap<String, DataValue>,
) -> QuickDbResult<T>
where
    T: serde::de::DeserializeOwned,
{
    // 创建一个自定义的反序列化器，直接从DataValue读取数据
    let deserializer = DataValueDeserializer::new(data_map);

    T::deserialize(deserializer).map_err(|e| QuickDbError::SerializationError {
        message: format!("无法从DataValue映射创建模型实例: {}", e),
    })
}

/// 从DataValue映射创建模型实例（带调试信息）
///
/// 在开发时使用，提供更详细的错误信息
pub fn create_model_from_data_map_with_debug<T>(
    data_map: &HashMap<String, DataValue>,
) -> QuickDbResult<T>
where
    T: serde::de::DeserializeOwned,
{
    
    let result = create_model_from_data_map::<T>(data_map);

    
    result
}

/// DataValue反序列化器
///
/// 实现serde::de::Deserializer trait，直接从DataValue读取数据
struct DataValueDeserializer<'a> {
    data_map: &'a HashMap<String, DataValue>,
    current_key: Option<String>,
}

impl<'a> DataValueDeserializer<'a> {
    fn new(data_map: &'a HashMap<String, DataValue>) -> Self {
        Self {
            data_map,
            current_key: None,
        }
    }

    fn get_current_value(&self) -> Option<&'a DataValue> {
        match &self.current_key {
            Some(key) => self.data_map.get(key),
            None => None,
        }
    }
}

impl<'a, 'de> serde::de::Deserializer<'de> for DataValueDeserializer<'a> {
    type Error = serde_json::Error;

    fn deserialize_struct<V>(
        mut self,
        _name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        // 为结构体创建字段访问器
        visitor.visit_map(DataValueStructDeserializer::new(&self.data_map, fields))
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_map(DataValueMapDeserializer::new(&self.data_map))
    }

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_map(DataValueMapDeserializer::new(&self.data_map))
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_some(self)
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes
        byte_buf unit unit_struct seq tuple tuple_struct enum identifier ignored_any
        newtype_struct
    }
}

/// 用于结构体字段访问的反序列化器
struct DataValueStructDeserializer<'a> {
    data_map: &'a HashMap<String, DataValue>,
    fields: &'static [&'static str],
    current_index: usize,
}

impl<'a> DataValueStructDeserializer<'a> {
    fn new(data_map: &'a HashMap<String, DataValue>, fields: &'static [&'static str]) -> Self {
        Self {
            data_map,
            fields,
            current_index: 0,
        }
    }
}

impl<'a, 'de> serde::de::MapAccess<'de> for DataValueStructDeserializer<'a> {
    type Error = serde_json::Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: serde::de::DeserializeSeed<'de>,
    {
        if self.current_index < self.fields.len() {
            let field_name = self.fields[self.current_index];
            let key_deserializer = serde::de::value::StrDeserializer::new(field_name);
            seed.deserialize(key_deserializer).map(Some)
        } else {
            Ok(None)
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        if self.current_index < self.fields.len() {
            let field_name = self.fields[self.current_index];
            self.current_index += 1;

            if let Some(data_value) = self.data_map.get(field_name) {
                let deserializer = DataValueSingleDeserializer::new(data_value);
                seed.deserialize(deserializer)
            } else {
                // 字段不存在时返回错误，让调用方处理
                Err(serde::de::Error::custom(format!("字段 '{}' 不存在", field_name)))
            }
        } else {
            Err(serde::de::Error::custom("字段访问越界"))
        }
    }
}

/// 用于Map访问的反序列化器
struct DataValueMapDeserializer<'a> {
    data: &'a HashMap<String, DataValue>,
    keys: std::vec::IntoIter<String>,
}

impl<'a> DataValueMapDeserializer<'a> {
    fn new(data: &'a HashMap<String, DataValue>) -> Self {
        Self {
            data,
            keys: data.keys().cloned().collect::<Vec<_>>().into_iter(),
        }
    }
}

impl<'a, 'de> serde::de::MapAccess<'de> for DataValueMapDeserializer<'a> {
    type Error = serde_json::Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: serde::de::DeserializeSeed<'de>,
    {
        match self.keys.next() {
            Some(key) => {
                let key_deserializer = serde::de::value::StrDeserializer::new(&key);
                seed.deserialize(key_deserializer).map(Some)
            },
            None => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        // 检查最后一个处理的键
        let key_count = self.keys.as_slice().len();
        let total_keys = self.data.len();

        if total_keys > 0 && key_count < total_keys {
            // 通过重新构建key迭代器来获取当前key
            let all_keys: Vec<String> = self.data.keys().cloned().collect();
            if let Some(current_key) = all_keys.get(total_keys - key_count - 1) {
                if let Some(data_value) = self.data.get(current_key) {
                    let deserializer = DataValueSingleDeserializer::new(data_value);
                    seed.deserialize(deserializer)
                } else {
                    Err(serde::de::Error::custom("数据值不存在"))
                }
            } else {
                Err(serde::de::Error::custom("键访问错误"))
            }
        } else {
            Err(serde::de::Error::custom("键访问错误"))
        }
    }
}

/// 单个DataValue的反序列化器
struct DataValueSingleDeserializer<'a> {
    data_value: &'a DataValue,
}

impl<'a> DataValueSingleDeserializer<'a> {
    fn new(data_value: &'a DataValue) -> Self {
        Self { data_value }
    }
}

impl<'a, 'de> serde::de::Deserializer<'de> for DataValueSingleDeserializer<'a> {
    type Error = serde_json::Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.data_value {
            DataValue::Null => visitor.visit_unit(),
            DataValue::Bool(b) => visitor.visit_bool(*b),
            DataValue::Int(i) => visitor.visit_i64(*i),
            DataValue::UInt(u) => visitor.visit_u64(*u),
            DataValue::Float(f) => visitor.visit_f64(*f),
            DataValue::String(s) => visitor.visit_str(s),
            DataValue::Array(arr) => {
                let deserializer = DataValueArrayDeserializer::new(arr);
                visitor.visit_seq(deserializer)
            },
            DataValue::Object(obj) => {
                let deserializer = DataValueMapDeserializer::new(obj);
                visitor.visit_map(deserializer)
            },
            DataValue::Bytes(bytes) => {
                let base64_str = base64::encode(bytes);
                visitor.visit_str(&base64_str)
            },
            DataValue::DateTime(dt) => visitor.visit_str(&dt.to_rfc3339()),
            DataValue::DateTimeUTC(dt) => visitor.visit_str(&dt.to_rfc3339()),
            DataValue::Uuid(u) => visitor.visit_str(&u.to_string()),
            DataValue::Json(json) => {
                // 将JSON对象序列化为字符串，让开发者用户自行解析
                let json_str = serde_json::to_string(json).unwrap_or_else(|_| "{}".to_string());
                visitor.visit_str(&json_str)
            },
        }
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.data_value {
            DataValue::Null => visitor.visit_none(),
            _ => visitor.visit_some(self),
        }
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes
        byte_buf unit unit_struct seq map tuple tuple_struct enum
        ignored_any identifier struct newtype_struct
    }
}

/// DataValue数组的反序列化器
struct DataValueArrayDeserializer<'a> {
    array: &'a Vec<DataValue>,
    current_index: usize,
}

impl<'a> DataValueArrayDeserializer<'a> {
    fn new(array: &'a Vec<DataValue>) -> Self {
        Self {
            array,
            current_index: 0,
        }
    }
}

impl<'a, 'de> serde::de::SeqAccess<'de> for DataValueArrayDeserializer<'a> {
    type Error = serde_json::Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: serde::de::DeserializeSeed<'de>,
    {
        if self.current_index < self.array.len() {
            let data_value = &self.array[self.current_index];
            self.current_index += 1;

            let deserializer = DataValueSingleDeserializer::new(data_value);
            seed.deserialize(deserializer).map(Some)
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestModel {
        id: String,
        name: String,
        age: i32,
        active: bool,
    }

    #[test]
    fn test_direct_model_creation() {
        let mut data_map = HashMap::new();
        data_map.insert("id".to_string(), DataValue::String("test-123".to_string()));
        data_map.insert("name".to_string(), DataValue::String("测试".to_string()));
        data_map.insert("age".to_string(), DataValue::Int(25));
        data_map.insert("active".to_string(), DataValue::Bool(true));

        let model: TestModel = create_model_from_data_map::<TestModel>(&data_map).unwrap();

        assert_eq!(model.id, "test-123");
        assert_eq!(model.name, "测试");
        assert_eq!(model.age, 25);
        assert_eq!(model.active, true);
    }
}