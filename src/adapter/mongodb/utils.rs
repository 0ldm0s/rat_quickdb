//! MongoDB工具函数模块

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
            DataValue::DateTime(dt) => Bson::DateTime(mongodb::bson::DateTime::from_system_time(dt.clone().into())),
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
                let mut doc = Document::new();
                for (key, value) in obj {
                    doc.insert(key, data_value_to_bson(adapter, value));
                }
                Bson::Document(doc)
            },
            // Reference类型不存在，移除此行
            DataValue::Null => Bson::Null,
            DataValue::Bytes(bytes) => Bson::Binary(mongodb::bson::Binary {
                subtype: mongodb::bson::spec::BinarySubtype::Generic,
                bytes: bytes.clone(),
            }),
        }
    }

    /// 将BSON文档转换为DataValue映射（不包装在Object中）
    pub(crate) fn document_to_data_map(adapter: &MongoAdapter, doc: &Document) -> QuickDbResult<HashMap<String, DataValue>> {
        let mut data_map = HashMap::new();

        for (key, value) in doc {
            let mut data_value = bson_to_data_value(adapter, value)?;

            // 特殊处理_id字段，映射为id并进行类型转换
            if key == "_id" {
                match &data_value {
                    DataValue::String(s) => {
                        // 检查是否是雪花ID（19位数字的字符串）
                        if s.len() == 19 && s.chars().all(|c| c.is_ascii_digit()) {
                            // 雪花ID：在查询结果中保持字符串格式以维持跨数据库兼容性
                            data_value = DataValue::String(s.clone());
                        } else {
                            // 其他ID格式保持原样
                        }
                    },
                    _ => {}
                }
                // 将_id映射为id
                data_map.insert("id".to_string(), data_value);
            } else {
                // 保持原始字段名
                data_map.insert(key.clone(), data_value);
            }
        }

        Ok(data_map)
    }

    /// 将BSON文档转换为DataValue（保持兼容性）
    pub(crate) fn document_to_data_value(adapter: &MongoAdapter, doc: &Document) -> QuickDbResult<DataValue> {
        let data_map = document_to_data_map(adapter, doc)?;
        Ok(DataValue::Object(data_map))
    }
    
    /// 将BSON值转换为JSON Value
    pub(crate) fn bson_to_json_value(adapter: &MongoAdapter, bson: &Bson) -> QuickDbResult<serde_json::Value> {
        match bson {
            Bson::ObjectId(oid) => Ok(serde_json::Value::String(oid.to_hex())),
            Bson::String(s) => Ok(serde_json::Value::String(s.clone())),
            Bson::Int32(i) => Ok(serde_json::Value::Number(serde_json::Number::from(*i))),
            Bson::Int64(i) => Ok(serde_json::Value::Number(serde_json::Number::from(*i))),
            Bson::Double(f) => {
                if let Some(num) = serde_json::Number::from_f64(*f) {
                    Ok(serde_json::Value::Number(num))
                } else {
                    Ok(serde_json::Value::String(f.to_string()))
                }
            },
            Bson::Boolean(b) => Ok(serde_json::Value::Bool(*b)),
            Bson::Null => Ok(serde_json::Value::Null),
            Bson::Array(arr) => {
                let mut json_arr = Vec::new();
                for item in arr {
                    json_arr.push(bson_to_json_value(adapter, item)?);
                }
                Ok(serde_json::Value::Array(json_arr))
            },
            Bson::Document(doc) => {
                let mut json_map = serde_json::Map::new();
                for (key, value) in doc {
                    json_map.insert(key.clone(), bson_to_json_value(adapter, value)?);
                }
                Ok(serde_json::Value::Object(json_map))
            },
            Bson::DateTime(dt) => {
                // 将BSON DateTime转换为ISO 8601字符串
                let system_time: std::time::SystemTime = dt.clone().into();
                let datetime = chrono::DateTime::<chrono::Utc>::from(system_time);
                Ok(serde_json::Value::String(datetime.to_rfc3339()))
            },
            Bson::Binary(bin) => Ok(serde_json::Value::String(base64::encode(&bin.bytes))),
            Bson::Decimal128(dec) => Ok(serde_json::Value::String(dec.to_string())),
            _ => Ok(serde_json::Value::String(format!("{:?}", bson))),
        }
    }
    
    /// 将BSON值转换为DataValue，正确处理ObjectId
    pub(crate) fn bson_to_data_value(adapter: &MongoAdapter, bson: &Bson) -> QuickDbResult<DataValue> {
        match bson {
            Bson::ObjectId(oid) => Ok(DataValue::String(oid.to_hex())),
            Bson::String(s) => Ok(DataValue::String(s.clone())),
            Bson::Int32(i) => Ok(DataValue::Int(*i as i64)),
            Bson::Int64(i) => {
                // 检查是否可能是雪花ID，保持跨数据库兼容性
                if *i > 1000000000000000000 {
                    Ok(DataValue::String(i.to_string()))
                } else {
                    Ok(DataValue::Int(*i))
                }
            },
            Bson::Double(f) => Ok(DataValue::Float(*f)),
            Bson::Boolean(b) => Ok(DataValue::Bool(*b)),
            Bson::Null => Ok(DataValue::Null),
            Bson::Array(arr) => {
                let mut data_arr = Vec::new();
                for item in arr {
                    data_arr.push(bson_to_data_value(adapter, item)?);
                }
                Ok(DataValue::Array(data_arr))
            },
            Bson::Document(doc) => {
                let mut data_map = HashMap::new();
                for (key, value) in doc {
                    let data_value = bson_to_data_value(adapter, value)?;
                    data_map.insert(key.clone(), data_value);
                }
                Ok(DataValue::Object(data_map))
            },
            Bson::DateTime(dt) => {
                // 将BSON DateTime转换为chrono::DateTime
                let system_time: std::time::SystemTime = dt.clone().into();
                let datetime = chrono::DateTime::<chrono::Utc>::from(system_time);
                Ok(DataValue::DateTime(datetime))
            },
            Bson::Binary(bin) => Ok(DataValue::Bytes(bin.bytes.clone())),
            Bson::Decimal128(dec) => Ok(DataValue::String(dec.to_string())),
            _ => {
                // 对于其他BSON类型，转换为字符串
                Ok(DataValue::String(bson.to_string()))
            }
        }
    }

    /// 构建MongoDB查询文档
    pub(crate) fn build_query_document(adapter: &MongoAdapter, conditions: &[QueryCondition]) -> QuickDbResult<Document> {
        debug!("[MongoDB] 开始构建查询文档，条件数量: {}", conditions.len());
        let mut query_doc = Document::new();

        for (index, condition) in conditions.iter().enumerate() {
            let field_name = map_field_name(adapter, &condition.field);

            // 特殊处理_id字段的ObjectId格式
            let bson_value = if field_name == "_id" {
                if let DataValue::String(id_str) = &condition.value {
                    // 处理ObjectId格式：ObjectId("xxx") 或直接是ObjectId字符串
                    let actual_id = if id_str.starts_with("ObjectId(\"") && id_str.ends_with("\")") {
                        // 提取ObjectId字符串部分
                        &id_str[10..id_str.len()-2]
                    } else {
                        id_str
                    };

                    // 尝试解析为ObjectId
                    if let Ok(object_id) = mongodb::bson::oid::ObjectId::parse_str(actual_id) {
                        Bson::ObjectId(object_id)
                    } else {
                        Bson::String(actual_id.to_string())
                    }
                } else {
                    data_value_to_bson(adapter, &condition.value)
                }
            } else {
                data_value_to_bson(adapter, &condition.value)
            };

            debug!("[MongoDB] 条件[{}]: 字段='{}' -> '{}', 操作符={:?}, 原始值={:?}, BSON值={:?}",
                   index, condition.field, field_name, condition.operator, condition.value, bson_value);

            match condition.operator {
                QueryOperator::Eq => {
                    debug!("[MongoDB] 处理Eq操作符: {} = {:?}", field_name, bson_value);
                    query_doc.insert(field_name, bson_value);
                },
                QueryOperator::Ne => {
                    query_doc.insert(field_name, doc! { "$ne": bson_value });
                },
                QueryOperator::Gt => {
                    query_doc.insert(field_name, doc! { "$gt": bson_value });
                },
                QueryOperator::Gte => {
                    query_doc.insert(field_name, doc! { "$gte": bson_value });
                },
                QueryOperator::Lt => {
                    query_doc.insert(field_name, doc! { "$lt": bson_value });
                },
                QueryOperator::Lte => {
                    query_doc.insert(field_name, doc! { "$lte": bson_value });
                },
                QueryOperator::Contains => {
                    if let Bson::String(s) = bson_value {
                        let regex_doc = doc! { "$regex": s.clone(), "$options": "i" };
                        debug!("[MongoDB] 处理Contains操作符(字符串): {} = {:?}", field_name, regex_doc);
                        query_doc.insert(field_name, regex_doc);
                    } else {
                        let in_doc = doc! { "$in": [bson_value.clone()] };
                        debug!("[MongoDB] 处理Contains操作符(非字符串): {} = {:?}", field_name, in_doc);
                        query_doc.insert(field_name, in_doc);
                    }
                },
                QueryOperator::StartsWith => {
                    if let Bson::String(s) = bson_value {
                        query_doc.insert(field_name, doc! { "$regex": format!("^{}", regex::escape(&s)), "$options": "i" });
                    }
                },
                QueryOperator::EndsWith => {
                    if let Bson::String(s) = bson_value {
                        query_doc.insert(field_name, doc! { "$regex": format!("{}$", regex::escape(&s)), "$options": "i" });
                    }
                },
                QueryOperator::In => {
                    if let Bson::Array(arr) = bson_value {
                        let in_doc = doc! { "$in": arr.clone() };
                        debug!("[MongoDB] 处理In操作符: {} = {:?}", field_name, in_doc);
                        query_doc.insert(field_name, in_doc);
                    } else {
                        debug!("[MongoDB] In操作符期望数组类型，但得到: {:?}", bson_value);
                    }
                },
                QueryOperator::NotIn => {
                    if let Bson::Array(arr) = bson_value {
                        query_doc.insert(field_name, doc! { "$nin": arr });
                    }
                },
                QueryOperator::Regex => {
                    if let Bson::String(s) = bson_value {
                        query_doc.insert(field_name, doc! { "$regex": s, "$options": "i" });
                    }
                },
                QueryOperator::Exists => {
                    if let Bson::Boolean(exists) = bson_value {
                        query_doc.insert(field_name, doc! { "$exists": exists });
                    }
                },
                QueryOperator::IsNull => {
                    query_doc.insert(field_name, Bson::Null);
                },
                QueryOperator::IsNotNull => {
                    query_doc.insert(field_name, doc! { "$ne": Bson::Null });
                },
            }
        }
        
        debug!("[MongoDB] 最终查询文档: {:?}", query_doc);
        Ok(query_doc)
    }

    /// 构建条件组合查询文档
    pub(crate) fn build_condition_groups_document(adapter: &MongoAdapter, condition_groups: &[QueryConditionGroup]) -> QuickDbResult<Document> {
        debug!("[MongoDB] 开始构建条件组查询文档，组数量: {}", condition_groups.len());
        if condition_groups.is_empty() {
            debug!("[MongoDB] 条件组为空，返回空文档");
            return Ok(Document::new());
        }
        
        if condition_groups.len() == 1 {
            // 单个条件组，直接构建
            debug!("[MongoDB] 单个条件组，直接构建");
            let group = &condition_groups[0];
            return build_single_condition_group_document(adapter, group);
        }
        
        // 多个条件组，使用 $and 连接
        debug!("[MongoDB] 多个条件组，使用$and连接");
        let mut group_docs = Vec::new();
        for (index, group) in condition_groups.iter().enumerate() {
            debug!("[MongoDB] 处理条件组[{}]: {:?}", index, group);
            let group_doc = build_single_condition_group_document(adapter, group)?;
            debug!("[MongoDB] 条件组[{}]生成的文档: {:?}", index, group_doc);
            if !group_doc.is_empty() {
                group_docs.push(group_doc);
            }
        }
        
        let final_doc = if group_docs.is_empty() {
            debug!("[MongoDB] 所有条件组都为空，返回空文档");
            Document::new()
        } else if group_docs.len() == 1 {
            debug!("[MongoDB] 只有一个有效条件组，直接返回");
            group_docs.into_iter().next().unwrap()
        } else {
            debug!("[MongoDB] 多个有效条件组，使用$and连接");
            doc! { "$and": group_docs }
        };
        
        debug!("[MongoDB] 条件组最终文档: {:?}", final_doc);
        Ok(final_doc)
    }
    
    /// 构建单个条件组文档
    pub(crate) fn build_single_condition_group_document(adapter: &MongoAdapter, group: &QueryConditionGroup) -> QuickDbResult<Document> {
        debug!("[MongoDB] 构建单个条件组文档: {:?}", group);
        match group {
            QueryConditionGroup::Single(condition) => {
                debug!("[MongoDB] 处理单个条件: {:?}", condition);
                build_query_document(adapter, &[condition.clone()])
             },
            QueryConditionGroup::Group { conditions, operator } => {
                debug!("[MongoDB] 处理条件组，操作符: {:?}, 条件数量: {}", operator, conditions.len());
                if conditions.is_empty() {
                    debug!("[MongoDB] 条件组为空");
                    return Ok(Document::new());
                }
                
                if conditions.len() == 1 {
                     // 单个条件组，递归处理
                     debug!("[MongoDB] 条件组只有一个条件，递归处理");
                     return build_single_condition_group_document(adapter, &conditions[0]);
                 }
                
                // 多个条件组，根据逻辑操作符连接
                debug!("[MongoDB] 处理多个条件，使用{:?}操作符", operator);
                 let condition_docs: Result<Vec<Document>, QuickDbError> = conditions
                     .iter()
                     .enumerate()
                     .map(|(i, condition_group)| {
                         debug!("[MongoDB] 处理子条件[{}]: {:?}", i, condition_group);
                         let doc = build_single_condition_group_document(adapter, condition_group)?;
                         debug!("[MongoDB] 子条件[{}]生成文档: {:?}", i, doc);
                         Ok(doc)
                     })
                     .collect();
                
                let condition_docs = condition_docs?;
                let non_empty_docs: Vec<Document> = condition_docs.into_iter()
                    .filter(|doc| !doc.is_empty())
                    .collect();
                
                debug!("[MongoDB] 有效文档数量: {}", non_empty_docs.len());
                
                if non_empty_docs.is_empty() {
                    debug!("[MongoDB] 没有有效文档");
                    return Ok(Document::new());
                }
                
                if non_empty_docs.len() == 1 {
                    debug!("[MongoDB] 只有一个有效文档，直接返回");
                    return Ok(non_empty_docs.into_iter().next().unwrap());
                }
                
                let result_doc = match operator {
                    LogicalOperator::And => {
                        debug!("[MongoDB] 使用$and连接文档");
                        doc! { "$and": non_empty_docs }
                    },
                    LogicalOperator::Or => {
                        debug!("[MongoDB] 使用$or连接文档");
                        doc! { "$or": non_empty_docs }
                    },
                };
                
                debug!("[MongoDB] 条件组最终结果: {:?}", result_doc);
                Ok(result_doc)
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
        
        if !set_doc.is_empty() {
            update_doc.insert("$set", set_doc);
        }
        
        update_doc
    }

    /// 获取集合引用
    pub(crate) fn get_collection(adapter: &MongoAdapter, db: &mongodb::Database, table: &str) -> Collection<Document> {
        db.collection::<Document>(table)
    }
    
    /// 将用户字段名映射到MongoDB字段名（id -> _id）
    pub(crate) fn map_field_name(adapter: &MongoAdapter, field_name: &str) -> String {
        if field_name == "id" {
            "_id".to_string()
        } else {
            field_name.to_string()
        }
    }
    
    /// 将数据映射中的id字段转换为_id字段
    pub(crate) fn map_data_fields(adapter: &MongoAdapter, data: &HashMap<String, DataValue>) -> HashMap<String, DataValue> {
        let mut mapped_data = HashMap::new();

        // 首先处理_id字段（如果存在且不为空）
        if let Some(_id_value) = data.get("_id") {
            if let DataValue::String(s) = _id_value {
                if !s.is_empty() {
                    mapped_data.insert("_id".to_string(), _id_value.clone());
                }
            } else {
                mapped_data.insert("_id".to_string(), _id_value.clone());
            }
        }

        // 然后处理其他字段，避免覆盖_id字段
        for (key, value) in data {
            if key != "_id" { // 跳过_id字段，避免覆盖
                let mapped_key = map_field_name(adapter, key);
                if mapped_key != "_id" { // 确保不会映射到_id
                    mapped_data.insert(mapped_key, value.clone());
                }
            }
        }

        mapped_data
    }

