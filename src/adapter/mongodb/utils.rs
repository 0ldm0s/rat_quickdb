//! MongoDB工具函数模块
//!
//! 包含BSON数据转换和数据库连接相关的工具函数

use crate::adapter::mongodb::MongoAdapter;
use crate::types::*;
use crate::error::{QuickDbError, QuickDbResult};
use mongodb::{Collection, Database};
use mongodb::bson::{doc, Bson, Document};
use std::collections::HashMap;
use rat_logger::debug;

/// 将DataValue转换为BSON值
pub(crate) fn data_value_to_bson(adapter: &MongoAdapter, value: &DataValue) -> Bson {
    match value {
        DataValue::String(s) => Bson::String(s.clone()),
        DataValue::Int(i) => Bson::Int64(*i),
        DataValue::Float(f) => Bson::Double(*f),
        DataValue::Bool(b) => Bson::Boolean(*b),
        DataValue::DateTime(dt) => {
            // 将DateTime<FixedOffset>转换为DateTime<Utc>，然后转换为MongoDB BSON DateTime
            let utc_dt = chrono::DateTime::<chrono::Utc>::from(*dt);
            Bson::DateTime(mongodb::bson::DateTime::from_system_time(utc_dt.into()))
        },
        DataValue::DateTimeUTC(dt) => {
            // DateTime<Utc>直接转换为MongoDB BSON DateTime
            Bson::DateTime(mongodb::bson::DateTime::from_system_time(dt.clone().into()))
        },
        DataValue::Uuid(uuid) => Bson::String(uuid.to_string()),
        DataValue::Json(json) => {
            // 尝试将JSON转换为BSON文档
            if let Ok(doc) = mongodb::bson::to_document(json) {
                Bson::Document(doc)
            } else {
                Bson::String(json.to_string())
            }
        },
        DataValue::Array(arr) => {
            let bson_array: Vec<Bson> = arr.iter()
                .map(|v| data_value_to_bson(adapter, v))
                .collect();
            Bson::Array(bson_array)
        },
        DataValue::Object(obj) => {
            let mut bson_doc = Document::new();
            for (key, value) in obj {
                let bson_value = data_value_to_bson(adapter, value);
                bson_doc.insert(key, bson_value);
            }
            Bson::Document(bson_doc)
        },
        DataValue::Null => Bson::Null,
        DataValue::Bytes(bytes) => Bson::Binary(mongodb::bson::Binary { bytes: bytes.clone(), subtype: mongodb::bson::spec::BinarySubtype::Generic }),
    }
}

/// 将Document转换为HashMap<String, DataValue>
pub(crate) fn document_to_data_map(adapter: &MongoAdapter, doc: &Document) -> QuickDbResult<HashMap<String, DataValue>> {
    let mut result = HashMap::new();

    for (key, bson_value) in doc {
        if let Ok(data_value) = bson_to_data_value(adapter, bson_value) {
            // 将MongoDB的_id字段映射回id字段
            let mapped_key = if key == "_id" {
                "id".to_string()
            } else {
                key.to_string()
            };
            result.insert(mapped_key, data_value);
        }
    }

    Ok(result)
}

/// 将Document转换为DataValue
pub(crate) fn document_to_data_value(adapter: &MongoAdapter, doc: &Document) -> QuickDbResult<DataValue> {
    let map = document_to_data_map(adapter, doc)?;
    Ok(DataValue::Object(map))
}

/// 将BSON转换为JSON Value
pub(crate) fn bson_to_json_value(adapter: &MongoAdapter, bson: &Bson) -> QuickDbResult<serde_json::Value> {
    let json_value = match bson {
        Bson::String(s) => serde_json::Value::String(s.clone()),
        Bson::Int64(i) => serde_json::Value::Number(serde_json::Number::from(*i)),
        Bson::Int32(i) => serde_json::Value::Number(serde_json::Number::from(*i)),
        Bson::Double(d) => serde_json::Value::Number(serde_json::Number::from_f64(*d).unwrap_or(serde_json::Number::from(0))),
        Bson::Boolean(b) => serde_json::Value::Bool(*b),
        Bson::DateTime(dt) => serde_json::Value::String(dt.to_string()),
        Bson::ObjectId(oid) => serde_json::Value::String(oid.to_hex()),
        Bson::Null => serde_json::Value::Null,
        Bson::Array(arr) => {
            let json_array: Vec<serde_json::Value> = arr.iter()
                .map(|v| bson_to_json_value(adapter, v))
                .collect::<Result<Vec<_>, _>>()?;
            serde_json::Value::Array(json_array)
        }
        Bson::Document(doc) => {
            let json_obj: serde_json::Map<String, serde_json::Value> = doc.iter()
                .map(|(k, v)| (k.to_string(), bson_to_json_value(adapter, v).ok().unwrap_or(serde_json::Value::Null)))
                .collect();
            serde_json::Value::Object(json_obj)
        }
        Bson::Binary(bin) => serde_json::Value::String(format!("Binary({})", bin.bytes.len())),
        Bson::RegularExpression(regex) => serde_json::Value::String(format!("Regex({}, {})", regex.pattern, regex.options)),
        Bson::JavaScriptCode(code) => serde_json::Value::String(format!("Code({})", code)),
        Bson::JavaScriptCodeWithScope(code_with_scope) => serde_json::Value::String(format!("CodeWithScope({})", code_with_scope.code)),
        Bson::Undefined => serde_json::Value::String("undefined".to_string()),
        Bson::MaxKey => serde_json::Value::String("maxKey".to_string()),
        Bson::MinKey => serde_json::Value::String("minKey".to_string()),
        Bson::Timestamp(timestamp) => serde_json::Value::String(format!("Timestamp({})", timestamp.time)),
        Bson::DbPointer(db_pointer) => serde_json::Value::String(format!("DbPointer({:?})", db_pointer)),
        Bson::Symbol(symbol) => serde_json::Value::String(symbol.to_string()),
        Bson::Decimal128(decimal) => serde_json::Value::String(decimal.to_string()),
    };

    Ok(json_value)
}

/// 将BSON转换为DataValue
pub(crate) fn bson_to_data_value(adapter: &MongoAdapter, bson: &Bson) -> QuickDbResult<DataValue> {
    match bson {
        Bson::String(s) => Ok(DataValue::String(s.clone())),
        Bson::Int64(i) => Ok(DataValue::Int(*i)),
        Bson::Int32(i) => Ok(DataValue::Int(*i as i64)),
        Bson::Double(d) => Ok(DataValue::Float(*d)),
        Bson::Boolean(b) => Ok(DataValue::Bool(*b)),
        Bson::DateTime(dt) => {
            let utc_dt = chrono::DateTime::<chrono::Utc>::from(dt.to_system_time());
            let fixed_dt = utc_dt.with_timezone(&chrono::FixedOffset::east(0));
            Ok(DataValue::DateTime(fixed_dt))
        },
        Bson::ObjectId(oid) => Ok(DataValue::String(oid.to_hex())),
        Bson::Null => Ok(DataValue::Null),
        Bson::Array(arr) => {
            let data_array: Vec<DataValue> = arr.iter()
                .map(|v| bson_to_data_value(adapter, v))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(DataValue::Array(data_array))
        }
        Bson::Document(doc) => {
            let map = document_to_data_map(adapter, doc)?;
            Ok(DataValue::Object(map))
        }
        Bson::Binary(bin) => Ok(DataValue::Bytes(bin.bytes.clone())),
        Bson::Undefined => Ok(DataValue::Null),
        _ => {
            // 对于其他类型，转换为JSON字符串再解析
            if let Ok(json_value) = bson_to_json_value(adapter, bson) {
                Ok(DataValue::Json(json_value))
            } else {
                // 转换失败时返回字符串表示
                Ok(DataValue::String(format!("{:?}", bson)))
            }
        }
    }
}

/// 构建更新文档
pub(crate) fn build_update_document(adapter: &MongoAdapter, data: &HashMap<String, DataValue>) -> Document {
    let mut update_doc = Document::new();
    let mut set_doc = Document::new();

    // 映射字段名（id -> _id）
    let mapped_data = map_data_fields(adapter, data);
    for (key, value) in &mapped_data {
        if key != "_id" { // MongoDB的_id字段不能更新
            set_doc.insert(key, data_value_to_bson(adapter, value));
        }
    }

    update_doc.insert("$set", set_doc);
    update_doc
}

/// 获取MongoDB集合
pub(crate) fn get_collection(adapter: &MongoAdapter, db: &mongodb::Database, table: &str) -> Collection<Document> {
    db.collection(table)
}

/// 映射字段名（适配MongoDB命名约定）
pub(crate) fn map_field_name(adapter: &MongoAdapter, field_name: &str) -> String {
    // 这里可以实现字段名映射逻辑
    field_name.to_string()
}

/// 映射数据字段（适配MongoDB字段命名）
pub(crate) fn map_data_fields(adapter: &MongoAdapter, data: &HashMap<String, DataValue>) -> HashMap<String, DataValue> {
    let mut mapped_data = HashMap::new();

    for (key, value) in data {
        let mapped_key = if key == "id" {
            "_id" // 将id映射为_id
        } else {
            key.as_str()
        };
        mapped_data.insert(mapped_key.to_string(), value.clone());
    }

    mapped_data
}