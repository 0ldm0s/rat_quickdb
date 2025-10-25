use serde::{Deserialize, Serializer, Deserializer};
use std::collections::HashMap;
use crate::types::data_value::DataValue;

/// 序列化辅助模块
pub mod hashmap_datavalue {
    use super::*;

    pub fn serialize<S>(map: &HashMap<String, DataValue>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeMap;
        let mut ser_map = serializer.serialize_map(Some(map.len()))?;
        for (key, value) in map {
            // 激进处理：检测到 DataValue::Null 就直接输出 JSON null，不进行后续转换
            let json_value = match value {
                DataValue::Null => serde_json::Value::Null,
                _ => value.to_json_value(),
            };
            ser_map.serialize_entry(key, &json_value)?;
        }
        ser_map.end()
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<HashMap<String, DataValue>, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;

        // 尝试反序列化为Map或String
        let value: serde_json::Value = serde_json::Value::deserialize(deserializer)?;

        let json_map = match value {
            // 如果是对象，直接使用
            serde_json::Value::Object(map) => {
                map.into_iter().collect::<HashMap<String, serde_json::Value>>()
            },
            // 如果是字符串，尝试解析为JSON
            serde_json::Value::String(s) => {
                serde_json::from_str::<HashMap<String, serde_json::Value>>(&s)
                    .map_err(|e| D::Error::custom(format!("无法解析JSON字符串: {}", e)))?
            },
            _ => return Err(D::Error::custom("期望JSON对象或JSON字符串")),
        };

        let mut result = HashMap::new();
        for (key, value) in json_map {
            result.insert(key, crate::types::data_value::json_value_to_data_value(value));
        }
        Ok(result)
    }
}