//! MySQL JSON处理器
//!
//! 处理MySQL数据库的JSON到DataValue转换，MySQL接受datetime字符串格式

use crate::types::DataValue;
use serde_json::Value;
use std::collections::HashMap;
use super::DatabaseJsonProcessor;

/// MySQL JSON处理器
pub struct MysqlJsonProcessor;

impl MysqlJsonProcessor {
    pub fn new() -> Self {
        Self
    }
}

impl DatabaseJsonProcessor for MysqlJsonProcessor {
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

            
            // MySQL接受datetime字符串，直接使用标准转换
            let data_value = self.convert_standard_field_value(json_value)?;
            data_map.insert(field_name.clone(), data_value);
        }

        Ok(data_map)
    }

    fn get_database_type(&self) -> crate::types::DatabaseType {
        crate::types::DatabaseType::MySQL
    }
}

impl MysqlJsonProcessor {
    /// 标准字段值转换（MySQL不需要特殊的datetime处理）
    fn convert_standard_field_value(&self, json_value: &Value) -> Result<DataValue, String> {
        match json_value {
            Value::Null => Ok(DataValue::Null),
            Value::String(s) => Ok(DataValue::String(s.clone())),
            Value::Bool(b) => Ok(DataValue::Bool(*b)),
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Ok(DataValue::Int(i))
                } else if let Some(f) = n.as_f64() {
                    Ok(DataValue::Float(f))
                } else {
                    Err(format!("数字格式不支持: {:?}", n))
                }
            },
            Value::Array(arr) => {
                let data_array: Vec<DataValue> = arr.iter()
                    .map(|v| self.convert_standard_field_value(v))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(DataValue::Array(data_array))
            },
            Value::Object(obj) => {
                let data_object: HashMap<String, DataValue> = obj.iter()
                    .map(|(k, v)| {
                        self.convert_standard_field_value(v).map(|val| (k.clone(), val))
                    })
                    .collect::<Result<HashMap<String, DataValue>, String>>()?;
                Ok(DataValue::Object(data_object))
            }
        }
    }
}