//! PostgreSQL JSON处理器
//!
//! 专门处理PostgreSQL数据库的JSON到DataValue转换，包括datetime字段解析

use crate::types::DataValue;
use serde_json::Value;
use std::collections::HashMap;
use super::DatabaseJsonProcessor;

/// PostgreSQL JSON处理器
pub struct PostgresJsonProcessor;

impl PostgresJsonProcessor {
    pub fn new() -> Self {
        Self
    }
}

impl DatabaseJsonProcessor for PostgresJsonProcessor {
    fn convert_json_to_data_map(
        &self,
        json_obj: &serde_json::Map<String, Value>,
        table_name: &str,
        db_alias: &str,
    ) -> Result<HashMap<String, DataValue>, String> {
        let mut data_map = HashMap::new();

        // 获取模型元数据
        let model_meta = crate::manager::get_model(table_name)
            .ok_or_else(|| format!("未找到表'{}'的模型元数据", table_name))?;

        
        for (field_name, json_value) in json_obj {
            // 获取字段定义
            let field_def = model_meta.fields.get(field_name)
                .ok_or_else(|| format!("字段'{}'未在表'{}'的模型中定义", field_name, table_name))?;

            
            // 根据字段定义类型进行转换
            let data_value = self.convert_field_value(field_name, json_value, field_def)?;
            data_map.insert(field_name.clone(), data_value);
        }

        Ok(data_map)
    }

    fn get_database_type(&self) -> crate::types::DatabaseType {
        crate::types::DatabaseType::PostgreSQL
    }
}

impl PostgresJsonProcessor {
    /// 转换单个字段值
    fn convert_field_value(
        &self,
        field_name: &str,
        json_value: &Value,
        field_def: &crate::model::FieldDefinition,
    ) -> Result<DataValue, String> {
        // 检查字段是否为DateTime类型
        let is_datetime = matches!(field_def.field_type,
            crate::model::FieldType::DateTime |
            crate::model::FieldType::Date |
            crate::model::FieldType::Time
        );

        match json_value {
            Value::Null => Ok(DataValue::Null),
            Value::String(s) => {
                if is_datetime {
                                        match self.parse_datetime_string(s) {
                        Some(dt) => Ok(DataValue::DateTime(dt)),
                        None => Err(format!("datetime字段'{}'格式错误: {}, 必须使用有效的ISO 8601格式", field_name, s))
                    }
                } else if self.is_uuid_field(field_name, &s) {
                                        match self.parse_uuid_string(s) {
                        Some(uuid) => Ok(DataValue::Uuid(uuid)),
                        None => Err(format!("UUID字段'{}'格式错误: {}", field_name, s))
                    }
                } else {
                    Ok(DataValue::String(s.clone()))
                }
            },
            Value::Bool(b) => Ok(DataValue::Bool(*b)),
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Ok(DataValue::Int(i))
                } else if let Some(f) = n.as_f64() {
                    Ok(DataValue::Float(f))
                } else {
                    Err(format!("字段'{}'的数字格式不支持: {:?}", field_name, n))
                }
            },
            Value::Array(arr) => {
                let data_array: Vec<DataValue> = arr.iter()
                    .map(|v| self.convert_field_value("", v, &crate::model::FieldDefinition {
                        field_type: crate::model::FieldType::String { max_length: None, min_length: None, regex: None },
                        required: false,
                        default: None,
                        unique: false,
                        indexed: false,
                        description: None,
                        validator: None,
                        sqlite_compatibility: false,
                    }))
                    .collect::<Result<Vec<_>, String>>()?;
                Ok(DataValue::Array(data_array))
            },
            Value::Object(obj) => {
                let data_object: HashMap<String, DataValue> = obj.iter()
                    .map(|(k, v)| {
                        self.convert_field_value(k, v, &crate::model::FieldDefinition {
                            field_type: crate::model::FieldType::String { max_length: None, min_length: None, regex: None },
                            required: false,
                            default: None,
                            unique: false,
                            indexed: false,
                            description: None,
                            validator: None,
                            sqlite_compatibility: false,
                        }).map(|val| (k.clone(), val))
                    })
                    .collect::<Result<HashMap<String, DataValue>, String>>()?;
                Ok(DataValue::Object(data_object))
            }
        }
    }

    /// 解析datetime字符串
    fn parse_datetime_string(&self, s: &str) -> Option<chrono::DateTime<chrono::Utc>> {
        use chrono::{DateTime, Utc, NaiveDateTime};

        if s.is_empty() {
            return None;
        }

        // 1. ISO 8601格式带时区
        if s.contains('T') && (s.contains('+') || s.contains('-') || s.contains('Z')) {
            match DateTime::parse_from_rfc3339(s) {
                Ok(dt) => return Some(dt.with_timezone(&Utc)),
                Err(_) => {},
            }
        }

        // 2. ISO 8601格式无时区 (假定为UTC)
        if s.contains('T') {
            match NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f") {
                Ok(ndt) => return Some(DateTime::from_naive_utc_and_offset(ndt, Utc)),
                Err(_) => {
                    match NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
                        Ok(ndt) => return Some(DateTime::from_naive_utc_and_offset(ndt, Utc)),
                        Err(_) => {},
                    }
                }
            }
        }

        // 3. MySQL格式
        if s.len() == 19 && s.contains(' ') {
            match NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
                Ok(ndt) => return Some(DateTime::from_naive_utc_and_offset(ndt, Utc)),
                Err(_) => {},
            }
        }

                None
    }

    /// 检查字段是否为UUID字段
    fn is_uuid_field(&self, field_name: &str, value: &str) -> bool {
        // 简单检测：字段名为"id"或包含"id"，且值符合UUID格式
        (field_name == "id" || field_name.contains("id")) && self.looks_like_uuid(value)
    }

    /// 检查字符串是否看起来像UUID
    fn looks_like_uuid(&self, s: &str) -> bool {
        // UUID格式：8-4-4-4-12个十六进制字符
        let uuid_pattern = regex::Regex::new(r"^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$").unwrap();
        !s.is_empty() && uuid_pattern.is_match(s)
    }

    /// 解析UUID字符串
    fn parse_uuid_string(&self, s: &str) -> Option<uuid::Uuid> {
        if s.is_empty() {
            return None;
        }

        match uuid::Uuid::parse_str(s) {
            Ok(uuid) => Some(uuid),
            Err(_) => {
                                None
            }
        }
    }
}