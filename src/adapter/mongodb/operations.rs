//! MongoDB适配器trait实现

use crate::adapter::MongoAdapter;
use crate::adapter::DatabaseAdapter;
use crate::pool::DatabaseConnection;
use crate::error::{QuickDbError, QuickDbResult};
use crate::types::*;
use crate::model::{FieldType, FieldDefinition};
use crate::manager;
use async_trait::async_trait;
use rat_logger::debug;
use std::collections::HashMap;
use mongodb::bson::{doc, Document};

use super::query as mongodb_query;
use super::schema as mongodb_schema;
use super::utils as mongodb_utils;

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
            if !mongodb_schema::table_exists(self, connection, table).await? {
                // 获取表创建锁，防止并发创建
                let _lock = self.acquire_table_lock(table).await;

                // 双重检查：再次确认集合不存在
                if !mongodb_schema::table_exists(self, connection, table).await? {
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

            let collection = mongodb_utils::get_collection(self, db, table);

            // 映射字段名（id -> _id）并处理ID策略
            let mut mapped_data = mongodb_utils::map_data_fields(self, data);

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
                    IdStrategy::Custom(_) => {
                        // 自定义策略保留ID字段
                    }
                }
            } else {
                // 没有ID字段，检查策略是否需要ID
                match id_strategy {
                    IdStrategy::Snowflake { .. } | IdStrategy::Uuid => {
                        return Err(QuickDbError::ValidationError {
                            field: "_id".to_string(),
                            message: format!("使用{:?}策略时必须提供ID字段", id_strategy),
                        });
                    },
                    _ => {} // 其他策略不需要ID字段
                }
            }

            let mut doc = Document::new();
            for (key, value) in &mapped_data {
                doc.insert(key, mongodb_utils::data_value_to_bson(self, value));
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
        mongodb_query::find_by_id(self, connection, table, id).await
    }

    async fn find(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
        options: &QueryOptions,
    ) -> QuickDbResult<Vec<DataValue>> {
        mongodb_query::find(self, connection, table, conditions, options).await
    }

    async fn find_with_groups(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        condition_groups: &[QueryConditionGroup],
        options: &QueryOptions,
    ) -> QuickDbResult<Vec<DataValue>> {
        mongodb_query::find_with_groups(self, connection, table, condition_groups, options).await
    }

    async fn update(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
        data: &HashMap<String, DataValue>,
    ) -> QuickDbResult<u64> {
        if let DatabaseConnection::MongoDB(db) = connection {
            let collection = mongodb_utils::get_collection(self, db, table);

            let query = mongodb_utils::build_query_document(self, conditions)?;
            let update = mongodb_utils::build_update_document(self, data);

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
            let collection = mongodb_utils::get_collection(self, db, table);

            let query = mongodb_utils::build_query_document(self, conditions)?;
            let mut update_doc = Document::new();

            let mut set_doc = Document::new();
            let mut inc_doc = Document::new();

            for operation in operations {
                match &operation.operation {
                    crate::types::UpdateOperator::Set => {
                        let bson_value = mongodb_utils::data_value_to_bson(self, &operation.value);
                        set_doc.insert(&operation.field, bson_value);
                    }
                    crate::types::UpdateOperator::Increment => {
                        let bson_value = mongodb_utils::data_value_to_bson(self, &operation.value);
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
                        let bson_value = mongodb_utils::data_value_to_bson(self, &neg_value);
                        inc_doc.insert(&operation.field, bson_value);
                    }
                    crate::types::UpdateOperator::Multiply => {
                        // MongoDB使用$multiply操作符
                        let bson_value = mongodb_utils::data_value_to_bson(self, &operation.value);
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
                        let bson_value = mongodb_utils::data_value_to_bson(self, &crate::types::DataValue::Float(divisor));
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
                        let bson_value = mongodb_utils::data_value_to_bson(self, &crate::types::DataValue::Float(multiplier));
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
                        let bson_value = mongodb_utils::data_value_to_bson(self, &crate::types::DataValue::Float(multiplier));
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
            let collection = mongodb_utils::get_collection(self, db, table);

            let query = mongodb_utils::build_query_document(self, conditions)?;

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
        mongodb_query::count(self, connection, table, conditions).await
    }

    async fn exists(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
    ) -> QuickDbResult<bool> {
        mongodb_query::exists(self, connection, table, conditions).await
    }

    async fn create_table(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        _fields: &HashMap<String, FieldDefinition>,
        _id_strategy: &IdStrategy,
    ) -> QuickDbResult<()> {
        mongodb_schema::create_table(self, connection, table, _fields, _id_strategy).await
    }

    async fn create_index(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        index_name: &str,
        fields: &[String],
        unique: bool,
    ) -> QuickDbResult<()> {
        mongodb_schema::create_index(self, connection, table, index_name, fields, unique).await
    }

    async fn table_exists(
        &self,
        connection: &DatabaseConnection,
        table: &str,
    ) -> QuickDbResult<bool> {
        mongodb_schema::table_exists(self, connection, table).await
    }

    async fn drop_table(
        &self,
        connection: &DatabaseConnection,
        table: &str,
    ) -> QuickDbResult<()> {
        mongodb_schema::drop_table(self, connection, table).await
    }

    async fn get_server_version(
        &self,
        connection: &DatabaseConnection,
    ) -> QuickDbResult<String> {
        mongodb_schema::get_server_version(self, connection).await
    }
}