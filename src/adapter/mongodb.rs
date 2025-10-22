//! MongoDB数据库适配器
//! 
//! 使用mongodb库实现真实的MongoDB文档操作

use super::DatabaseAdapter;
use crate::error::{QuickDbError, QuickDbResult};
use crate::types::{*, IdStrategy};
use crate::pool::DatabaseConnection;
use crate::table::{TableManager, TableSchema, ColumnType};
use crate::model::{FieldType, FieldDefinition};
use async_trait::async_trait;

use std::collections::HashMap;
use std::sync::Arc;
use mongodb::{bson::{doc, Document, Bson}, Collection};
use rat_logger::{info, error, warn, debug};

/// MongoDB适配器
pub struct MongoAdapter {
    /// 表创建锁，防止并发创建同一个表
    creation_locks: Arc<tokio::sync::Mutex<HashMap<String, ()>>>,
}

impl MongoAdapter {
    /// 创建新的MongoDB适配器实例
    pub fn new() -> Self {
        Self {
            creation_locks: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
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
    fn release_table_lock(&self, table: &str, mut locks: tokio::sync::MutexGuard<'_, HashMap<String, ()>>) {
        locks.remove(table);
        debug!("🔓 释放表 {} 的创建锁", table);
    }
}

impl Default for MongoAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl MongoAdapter {
    /// 将DataValue转换为BSON值
    fn data_value_to_bson(&self, value: &DataValue) -> Bson {
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
                    .map(|v| self.data_value_to_bson(v))
                    .collect();
                Bson::Array(bson_array)
            },
            DataValue::Object(obj) => {
                let mut doc = Document::new();
                for (key, value) in obj {
                    doc.insert(key, self.data_value_to_bson(value));
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
    fn document_to_data_map(&self, doc: &Document) -> QuickDbResult<HashMap<String, DataValue>> {
        let mut data_map = HashMap::new();

        for (key, value) in doc {
            let mut data_value = self.bson_to_data_value(value)?;

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
    fn document_to_data_value(&self, doc: &Document) -> QuickDbResult<DataValue> {
        let data_map = self.document_to_data_map(doc)?;
        Ok(DataValue::Object(data_map))
    }
    
    /// 将BSON值转换为JSON Value
    fn bson_to_json_value(&self, bson: &Bson) -> QuickDbResult<serde_json::Value> {
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
                    json_arr.push(self.bson_to_json_value(item)?);
                }
                Ok(serde_json::Value::Array(json_arr))
            },
            Bson::Document(doc) => {
                let mut json_map = serde_json::Map::new();
                for (key, value) in doc {
                    json_map.insert(key.clone(), self.bson_to_json_value(value)?);
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
    fn bson_to_data_value(&self, bson: &Bson) -> QuickDbResult<DataValue> {
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
                    data_arr.push(self.bson_to_data_value(item)?);
                }
                Ok(DataValue::Array(data_arr))
            },
            Bson::Document(doc) => {
                let mut data_map = HashMap::new();
                for (key, value) in doc {
                    let data_value = self.bson_to_data_value(value)?;
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
    fn build_query_document(&self, conditions: &[QueryCondition]) -> QuickDbResult<Document> {
        debug!("[MongoDB] 开始构建查询文档，条件数量: {}", conditions.len());
        let mut query_doc = Document::new();

        for (index, condition) in conditions.iter().enumerate() {
            let field_name = self.map_field_name(&condition.field);

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
                    self.data_value_to_bson(&condition.value)
                }
            } else {
                self.data_value_to_bson(&condition.value)
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
    fn build_condition_groups_document(&self, condition_groups: &[QueryConditionGroup]) -> QuickDbResult<Document> {
        debug!("[MongoDB] 开始构建条件组查询文档，组数量: {}", condition_groups.len());
        if condition_groups.is_empty() {
            debug!("[MongoDB] 条件组为空，返回空文档");
            return Ok(Document::new());
        }
        
        if condition_groups.len() == 1 {
            // 单个条件组，直接构建
            debug!("[MongoDB] 单个条件组，直接构建");
            let group = &condition_groups[0];
            return self.build_single_condition_group_document(group);
        }
        
        // 多个条件组，使用 $and 连接
        debug!("[MongoDB] 多个条件组，使用$and连接");
        let mut group_docs = Vec::new();
        for (index, group) in condition_groups.iter().enumerate() {
            debug!("[MongoDB] 处理条件组[{}]: {:?}", index, group);
            let group_doc = self.build_single_condition_group_document(group)?;
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
    fn build_single_condition_group_document(&self, group: &QueryConditionGroup) -> QuickDbResult<Document> {
        debug!("[MongoDB] 构建单个条件组文档: {:?}", group);
        match group {
            QueryConditionGroup::Single(condition) => {
                debug!("[MongoDB] 处理单个条件: {:?}", condition);
                self.build_query_document(&[condition.clone()])
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
                     return self.build_single_condition_group_document(&conditions[0]);
                 }
                
                // 多个条件组，根据逻辑操作符连接
                debug!("[MongoDB] 处理多个条件，使用{:?}操作符", operator);
                 let condition_docs: Result<Vec<Document>, QuickDbError> = conditions
                     .iter()
                     .enumerate()
                     .map(|(i, condition_group)| {
                         debug!("[MongoDB] 处理子条件[{}]: {:?}", i, condition_group);
                         let doc = self.build_single_condition_group_document(condition_group)?;
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
    fn build_update_document(&self, data: &HashMap<String, DataValue>) -> Document {
        let mut update_doc = Document::new();
        let mut set_doc = Document::new();
        
        // 映射字段名（id -> _id）
        let mapped_data = self.map_data_fields(data);
        for (key, value) in &mapped_data {
            if key != "_id" { // MongoDB的_id字段不能更新
                set_doc.insert(key, self.data_value_to_bson(value));
            }
        }
        
        if !set_doc.is_empty() {
            update_doc.insert("$set", set_doc);
        }
        
        update_doc
    }

    /// 获取集合引用
    fn get_collection(&self, db: &mongodb::Database, table: &str) -> Collection<Document> {
        db.collection::<Document>(table)
    }
    
    /// 将用户字段名映射到MongoDB字段名（id -> _id）
    fn map_field_name(&self, field_name: &str) -> String {
        if field_name == "id" {
            "_id".to_string()
        } else {
            field_name.to_string()
        }
    }
    
    /// 将数据映射中的id字段转换为_id字段
    fn map_data_fields(&self, data: &HashMap<String, DataValue>) -> HashMap<String, DataValue> {
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
                let mapped_key = self.map_field_name(key);
                if mapped_key != "_id" { // 确保不会映射到_id
                    mapped_data.insert(mapped_key, value.clone());
                }
            }
        }

        mapped_data
    }
}

#[async_trait]
impl DatabaseAdapter for MongoAdapter {
    async fn create(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        data: &HashMap<String, DataValue>,
        id_strategy: &IdStrategy,
    ) -> QuickDbResult<DataValue> {
        if let DatabaseConnection::MongoDB(db) = connection {
            // 调试：打印原始接收到的数据
            debug!("🔍 MongoDB适配器原始接收到的数据: {:?}", data);
            // 自动建表逻辑：检查集合是否存在，如果不存在则创建
            if !self.table_exists(connection, table).await? {
                // 获取表创建锁，防止并发创建
                let _lock = self.acquire_table_lock(table).await;

                // 双重检查：再次确认集合不存在
                if !self.table_exists(connection, table).await? {
                    // 尝试从模型管理器获取预定义的元数据
                    if let Some(model_meta) = crate::manager::get_model(table) {
                        debug!("集合 {} 不存在，使用预定义模型元数据创建", table);

                        // MongoDB不需要预创建表结构，集合是无模式的
                        debug!("✅ MongoDB集合 '{}' 不存在，使用无模式设计，将根据数据推断结构", table);
                    } else {
                        return Err(QuickDbError::ValidationError {
                            field: "collection_creation".to_string(),
                            message: format!("集合 '{}' 不存在，且没有预定义的模型元数据。MongoDB使用无模式设计，但建议先定义模型。", table),
                        });
                    }

                    // 等待一小段时间确保数据库事务完成
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
            }
            
            let collection = self.get_collection(db, table);
            
            // 映射字段名（id -> _id）并处理ID策略
            let mut mapped_data = self.map_data_fields(data);

            // 调试：打印接收到的数据
            debug!("🔍 MongoDB适配器接收到的数据: {:?}", mapped_data);

            // 根据ID策略处理ID字段
            if mapped_data.contains_key("_id") {
                let strategy = id_strategy;
                match strategy {
                    IdStrategy::AutoIncrement | IdStrategy::ObjectId => {
                        // 对于这些策略，移除空的ID字段，让MongoDB自动生成
                        if let Some(DataValue::String(s)) = mapped_data.get("_id") {
                            if s.is_empty() {
                                mapped_data.remove("_id");
                            }
                        }
                    },
                    IdStrategy::Snowflake { .. } | IdStrategy::Uuid => {
                        // 对于雪花和UUID策略，移除空的ID字段，让ODM层生成的ID生效
                        if let Some(DataValue::String(s)) = mapped_data.get("_id") {
                            if s.is_empty() {
                                mapped_data.remove("_id");
                            }
                        }
                    },
                    _ => {
                        // 其他策略保留ID字段
                    }
                }
            } else {
                // 没有ID字段，检查策略是否需要ID
                let strategy = id_strategy;
                match strategy {
                    IdStrategy::Snowflake { .. } | IdStrategy::Uuid => {
                        return Err(QuickDbError::ValidationError {
                            field: "_id".to_string(),
                            message: format!("使用{:?}策略时必须提供ID字段", strategy),
                        });
                    },
                    _ => {} // 其他策略不需要ID字段
                }
            }

            let mut doc = Document::new();
            for (key, value) in &mapped_data {
                doc.insert(key, self.data_value_to_bson(value));
            }

            debug!("执行MongoDB插入到集合 {}: {:?}", table, doc);
            
            let result = collection.insert_one(doc, None)
                .await
                .map_err(|e| QuickDbError::QueryError {
                    message: format!("MongoDB插入失败: {}", e),
                })?;
            
            let mut result_map = HashMap::new();

            // 检查是否有ODM层生成的ID，如果有则使用它，否则使用MongoDB生成的ID
            if let Some(id_value) = mapped_data.get("_id") {
                if let DataValue::String(id_str) = id_value {
                    if !id_str.is_empty() {
                        // 使用ODM层生成的ID
                        result_map.insert("id".to_string(), DataValue::String(id_str.clone()));
                        Ok(DataValue::Object(result_map))
                    } else {
                        // 使用MongoDB生成的ID，确保转换为纯字符串格式
                        let id_str = match result.inserted_id {
                            mongodb::bson::Bson::ObjectId(oid) => oid.to_hex(),
                            _ => result.inserted_id.to_string(),
                        };
                        result_map.insert("id".to_string(), DataValue::String(id_str));
                        Ok(DataValue::Object(result_map))
                    }
                } else {
                    // 使用MongoDB生成的ID，确保转换为纯字符串格式
                    let id_str = match result.inserted_id {
                        mongodb::bson::Bson::ObjectId(oid) => oid.to_hex(),
                        _ => result.inserted_id.to_string(),
                    };
                    result_map.insert("id".to_string(), DataValue::String(id_str));
                    Ok(DataValue::Object(result_map))
                }
            } else {
                // 使用MongoDB生成的ID，确保转换为纯字符串格式
                let id_str = match result.inserted_id {
                    mongodb::bson::Bson::ObjectId(oid) => oid.to_hex(),
                    _ => result.inserted_id.to_string(),
                };
                result_map.insert("id".to_string(), DataValue::String(id_str));
                Ok(DataValue::Object(result_map))
            }
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望MongoDB连接".to_string(),
            })
        }
    }

    async fn find_by_id(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        id: &DataValue,
    ) -> QuickDbResult<Option<DataValue>> {
        if let DatabaseConnection::MongoDB(db) = connection {
            let collection = self.get_collection(db, table);
            
            let query = match id {
                DataValue::String(id_str) => {
                    // 处理ObjectId格式：ObjectId("xxx") 或直接是ObjectId字符串
                    let actual_id = if id_str.starts_with("ObjectId(\"") && id_str.ends_with("\")") {
                        // 提取ObjectId字符串部分
                        &id_str[10..id_str.len()-2]
                    } else {
                        id_str
                    };

                    // 尝试解析为ObjectId，如果失败则作为字符串查询
                    if let Ok(object_id) = mongodb::bson::oid::ObjectId::parse_str(actual_id) {
                        doc! { "_id": object_id }
                    } else {
                        doc! { "_id": actual_id }
                    }
                },
                _ => doc! { "_id": self.data_value_to_bson(id) }
            };
            
            debug!("执行MongoDB根据ID查询: {:?}", query);
            
            let result = collection.find_one(query, None)
                .await
                .map_err(|e| QuickDbError::QueryError {
                    message: format!("MongoDB查询失败: {}", e),
                })?;
            
            if let Some(doc) = result {
                let data_map = self.document_to_data_map(&doc)?;
                // 直接返回Object，避免双重包装
                Ok(Some(DataValue::Object(data_map)))
            } else {
                Ok(None)
            }
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望MongoDB连接".to_string(),
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
                operator: LogicalOperator::And,
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
        if let DatabaseConnection::MongoDB(db) = connection {
            let collection = self.get_collection(db, table);
            
            let query = self.build_condition_groups_document(condition_groups)?;
            
            debug!("执行MongoDB条件组合查询: {:?}", query);
            
            let mut find_options = mongodb::options::FindOptions::default();
            
            // 添加排序
            if !options.sort.is_empty() {
                let mut sort_doc = Document::new();
                for sort_field in &options.sort {
                    let sort_value = match sort_field.direction {
                        SortDirection::Asc => 1,
                        SortDirection::Desc => -1,
                    };
                    sort_doc.insert(&sort_field.field, sort_value);
                }
                find_options.sort = Some(sort_doc);
            }
            
            // 添加分页
            if let Some(pagination) = &options.pagination {
                find_options.limit = Some(pagination.limit as i64);
                find_options.skip = Some(pagination.skip);
            }
            
            let mut cursor = collection.find(query, find_options)
                .await
                .map_err(|e| QuickDbError::QueryError {
                    message: format!("MongoDB条件组合查询失败: {}", e),
                })?;
            
            let mut results = Vec::new();
            while cursor.advance().await.map_err(|e| QuickDbError::QueryError {
                message: format!("MongoDB游标遍历失败: {}", e),
            })? {
                let doc = cursor.deserialize_current().map_err(|e| QuickDbError::QueryError {
                    message: format!("MongoDB文档反序列化失败: {}", e),
                })?;
                let data_map = self.document_to_data_map(&doc)?;
                // 直接返回Object，避免双重包装
                results.push(DataValue::Object(data_map));
            }
            
            Ok(results)
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望MongoDB连接".to_string(),
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
        if let DatabaseConnection::MongoDB(db) = connection {
            let collection = self.get_collection(db, table);
            
            let query = self.build_query_document(conditions)?;
            let update = self.build_update_document(data);
            
            debug!("执行MongoDB更新: 查询={:?}, 更新={:?}", query, update);
            
            let result = collection.update_many(query, update, None)
                .await
                .map_err(|e| QuickDbError::QueryError {
                    message: format!("MongoDB更新失败: {}", e),
                })?;
            
            Ok(result.modified_count)
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望MongoDB连接".to_string(),
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
        let conditions = vec![QueryCondition {
            field: "_id".to_string(), // MongoDB使用_id作为主键
            operator: QueryOperator::Eq,
            value: id.clone(),
        }];
        
        let affected = self.update(connection, table, &conditions, data).await?;
        Ok(affected > 0)
    }

    async fn update_with_operations(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
        operations: &[crate::types::UpdateOperation],
    ) -> QuickDbResult<u64> {
        if let DatabaseConnection::MongoDB(db) = connection {
            let collection = self.get_collection(db, table);

            let query = self.build_query_document(conditions)?;
            let mut update_doc = Document::new();

            let mut set_doc = Document::new();
            let mut inc_doc = Document::new();

            for operation in operations {
                match &operation.operation {
                    crate::types::UpdateOperator::Set => {
                        let bson_value = self.data_value_to_bson(&operation.value);
                        set_doc.insert(&operation.field, bson_value);
                    }
                    crate::types::UpdateOperator::Increment => {
                        let bson_value = self.data_value_to_bson(&operation.value);
                        inc_doc.insert(&operation.field, bson_value);
                    }
                    crate::types::UpdateOperator::Decrement => {
                        // 对于减少操作，使用负数的inc操作
                        let neg_value = match &operation.value {
                            crate::types::DataValue::Int(i) => crate::types::DataValue::Int(-i),
                            crate::types::DataValue::Float(f) => crate::types::DataValue::Float(-f),
                            _ => return Err(QuickDbError::ValidationError {
                                field: operation.field.clone(),
                                message: "Decrement操作只支持数值类型".to_string(),
                            }),
                        };
                        let bson_value = self.data_value_to_bson(&neg_value);
                        inc_doc.insert(&operation.field, bson_value);
                    }
                    crate::types::UpdateOperator::Multiply => {
                        // MongoDB使用$multiply操作符
                        let bson_value = self.data_value_to_bson(&operation.value);
                        if !set_doc.contains_key("$mul") {
                            set_doc.insert("$mul", Document::new());
                        }
                        let mul_doc = set_doc.get_mut("$mul").unwrap().as_document_mut().unwrap();
                        mul_doc.insert(&operation.field, bson_value);
                    }
                    crate::types::UpdateOperator::Divide => {
                        // MongoDB不支持直接除法，但可以使用乘法配合小数
                        let divisor = match &operation.value {
                            crate::types::DataValue::Int(i) => 1.0 / *i as f64,
                            crate::types::DataValue::Float(f) => 1.0 / f,
                            _ => return Err(QuickDbError::ValidationError {
                                field: operation.field.clone(),
                                message: "Divide操作只支持数值类型".to_string(),
                            }),
                        };
                        let bson_value = self.data_value_to_bson(&crate::types::DataValue::Float(divisor));
                        if !set_doc.contains_key("$mul") {
                            set_doc.insert("$mul", Document::new());
                        }
                        let mul_doc = set_doc.get_mut("$mul").unwrap().as_document_mut().unwrap();
                        mul_doc.insert(&operation.field, bson_value);
                    }
                    crate::types::UpdateOperator::PercentIncrease => {
                        // 百分比增加：转换为乘法 (1 + percentage/100)
                        let percentage = match &operation.value {
                            crate::types::DataValue::Float(f) => *f,
                            crate::types::DataValue::Int(i) => *i as f64,
                            _ => return Err(QuickDbError::ValidationError {
                                field: operation.field.clone(),
                                message: "PercentIncrease操作只支持数值类型".to_string(),
                            }),
                        };
                        let multiplier = 1.0 + percentage / 100.0;
                        let bson_value = self.data_value_to_bson(&crate::types::DataValue::Float(multiplier));
                        if !set_doc.contains_key("$mul") {
                            set_doc.insert("$mul", Document::new());
                        }
                        let mul_doc = set_doc.get_mut("$mul").unwrap().as_document_mut().unwrap();
                        mul_doc.insert(&operation.field, bson_value);
                    }
                    crate::types::UpdateOperator::PercentDecrease => {
                        // 百分比减少：转换为乘法 (1 - percentage/100)
                        let percentage = match &operation.value {
                            crate::types::DataValue::Float(f) => *f,
                            crate::types::DataValue::Int(i) => *i as f64,
                            _ => return Err(QuickDbError::ValidationError {
                                field: operation.field.clone(),
                                message: "PercentDecrease操作只支持数值类型".to_string(),
                            }),
                        };
                        let multiplier = 1.0 - percentage / 100.0;
                        let bson_value = self.data_value_to_bson(&crate::types::DataValue::Float(multiplier));
                        if !set_doc.contains_key("$mul") {
                            set_doc.insert("$mul", Document::new());
                        }
                        let mul_doc = set_doc.get_mut("$mul").unwrap().as_document_mut().unwrap();
                        mul_doc.insert(&operation.field, bson_value);
                    }
                }
            }

            if !set_doc.is_empty() {
                // 将$mul操作从set_doc中分离出来
                let mut mul_doc = Document::new();
                if let Some(bson_value) = set_doc.remove("$mul") {
                    update_doc.insert("$mul", bson_value);
                }
                update_doc.insert("$set", set_doc);
            }

            if !inc_doc.is_empty() {
                update_doc.insert("$inc", inc_doc);
            }

            if update_doc.is_empty() {
                return Err(QuickDbError::ValidationError {
                    field: "operations".to_string(),
                    message: "更新操作不能为空".to_string(),
                });
            }

            debug!("执行MongoDB操作更新: query={:?}, update={:?}", query, update_doc);

            let result = collection.update_many(query, update_doc, None)
                .await
                .map_err(|e| QuickDbError::QueryError {
                    message: format!("MongoDB更新失败: {}", e),
                })?;

            Ok(result.modified_count)
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望MongoDB连接".to_string(),
            })
        }
    }

    async fn delete(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
    ) -> QuickDbResult<u64> {
        if let DatabaseConnection::MongoDB(db) = connection {
            let collection = self.get_collection(db, table);
            
            let query = self.build_query_document(conditions)?;
            
            debug!("执行MongoDB删除: {:?}", query);
            
            let result = collection.delete_many(query, None)
                .await
                .map_err(|e| QuickDbError::QueryError {
                    message: format!("MongoDB删除失败: {}", e),
                })?;
            
            Ok(result.deleted_count)
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望MongoDB连接".to_string(),
            })
        }
    }

    async fn delete_by_id(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        id: &DataValue,
    ) -> QuickDbResult<bool> {
        let conditions = vec![QueryCondition {
            field: "_id".to_string(), // MongoDB使用_id作为主键
            operator: QueryOperator::Eq,
            value: id.clone(),
        }];
        
        let affected = self.delete(connection, table, &conditions).await?;
        Ok(affected > 0)
    }

    async fn count(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
    ) -> QuickDbResult<u64> {
        if let DatabaseConnection::MongoDB(db) = connection {
            let collection = self.get_collection(db, table);
            
            let query = self.build_query_document(conditions)?;
            
            debug!("执行MongoDB计数: {:?}", query);
            
            let count = collection.count_documents(query, None)
                .await
                .map_err(|e| QuickDbError::QueryError {
                    message: format!("MongoDB计数失败: {}", e),
                })?;
            
            Ok(count)
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望MongoDB连接".to_string(),
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
        _fields: &HashMap<String, FieldDefinition>,
        _id_strategy: &IdStrategy,
    ) -> QuickDbResult<()> {
        if let DatabaseConnection::MongoDB(db) = connection {
            // MongoDB是无模式的，集合会在第一次插入时自动创建
            // 这里我们可以创建集合并设置一些选项
            let options = mongodb::options::CreateCollectionOptions::default();
            
            debug!("创建MongoDB集合: {}", table);
            
            match db.create_collection(table, options).await {
                Ok(_) => {},
                Err(e) => {
                    // 如果集合已存在，忽略错误
                    if !e.to_string().contains("already exists") {
                        return Err(QuickDbError::QueryError {
                            message: format!("创建MongoDB集合失败: {}", e),
                        });
                    }
                }
            }
            
            Ok(())
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望MongoDB连接".to_string(),
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
        if let DatabaseConnection::MongoDB(db) = connection {
            let collection = self.get_collection(db, table);
            
            let mut index_doc = Document::new();
            for field in fields {
                index_doc.insert(field, 1); // 1表示升序索引
            }
            
            let mut index_options = mongodb::options::IndexOptions::default();
            index_options.name = Some(index_name.to_string());
            index_options.unique = Some(unique);
            
            let index_model = mongodb::IndexModel::builder()
                .keys(index_doc)
                .options(index_options)
                .build();
            
            debug!("创建MongoDB索引: {} 在集合 {}", index_name, table);
            
            collection.create_index(index_model, None)
                .await
                .map_err(|e| QuickDbError::QueryError {
                    message: format!("创建MongoDB索引失败: {}", e),
                })?;
            
            Ok(())
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望MongoDB连接".to_string(),
            })
        }
    }

    async fn table_exists(
        &self,
        connection: &DatabaseConnection,
        table: &str,
    ) -> QuickDbResult<bool> {
        if let DatabaseConnection::MongoDB(db) = connection {
            let collection_names = db.list_collection_names(None)
                .await
                .map_err(|e| QuickDbError::QueryError {
                    message: format!("检查MongoDB集合是否存在失败: {}", e),
                })?;
            
            Ok(collection_names.contains(&table.to_string()))
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望MongoDB连接".to_string(),
            })
        }
    }

    async fn drop_table(
        &self,
        connection: &DatabaseConnection,
        table: &str,
    ) -> QuickDbResult<()> {
        if let DatabaseConnection::MongoDB(db) = connection {
            debug!("执行MongoDB删除集合: {}", table);

            let collection = db.collection::<mongodb::bson::Document>(table);
            collection.drop(None).await
                .map_err(|e| QuickDbError::QueryError {
                    message: format!("删除MongoDB集合失败: {}", e),
                })?;

            debug!("成功删除MongoDB集合: {}", table);
            Ok(())
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望MongoDB连接".to_string(),
            })
        }
    }

    async fn get_server_version(
        &self,
        connection: &DatabaseConnection,
    ) -> QuickDbResult<String> {
        if let DatabaseConnection::MongoDB(db) = connection {
            debug!("执行MongoDB版本查询");

            // 使用MongoDB的buildInfo命令获取版本信息
            let command = mongodb::bson::doc! {
                "buildInfo": 1
            };

            let result = db.run_command(command, None).await
                .map_err(|e| QuickDbError::QueryError {
                    message: format!("查询MongoDB版本失败: {}", e),
                })?;

            // 从结果中提取版本信息
            if let Some(version) = result.get("version") {
                let version_str = match version {
                    mongodb::bson::Bson::String(v) => v.clone(),
                    _ => return Err(QuickDbError::QueryError {
                        message: "MongoDB版本信息格式错误".to_string(),
                    }),
                };

                debug!("成功获取MongoDB版本: {}", version_str);
                Ok(version_str)
            } else {
                Err(QuickDbError::QueryError {
                    message: "MongoDB版本查询结果中没有版本信息".to_string(),
                })
            }
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望MongoDB连接".to_string(),
            })
        }
    }
}