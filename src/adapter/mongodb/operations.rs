//! MongoDB适配器trait实现

use crate::adapter::DatabaseAdapter;
use crate::adapter::MongoAdapter;
use crate::adapter::mongodb::query_builder::build_query_document;
use crate::adapter::mongodb::utils::build_update_document;
use crate::error::{QuickDbError, QuickDbResult};
use crate::manager;
use crate::model::FieldDefinition;
use crate::pool::DatabaseConnection;
use crate::types::*;
use async_trait::async_trait;
use mongodb::bson::{Bson, Document, doc};
use rat_logger::debug;
use serde_json::json;
use std::collections::HashMap;

use super::query as mongodb_query;
use super::schema as mongodb_schema;
use super::utils as mongodb_utils;

/// 检查MongoDB错误是否为集合不存在错误
fn check_collection_not_exist_error(error: &mongodb::error::Error, collection: &str) -> bool {
    let error_string = error.to_string().to_lowercase();
    error_string.contains("namespace not found") ||
    error_string.contains(&format!("ns not found: {}", collection.to_lowercase())) ||
    error_string.contains("collection") && error_string.contains("does not exist") ||
    error_string.contains("invalid namespace") ||
    error_string.contains("command failed") && error_string.contains("find")
}

#[async_trait]
impl DatabaseAdapter for MongoAdapter {
    async fn create(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        data: &HashMap<String, DataValue>,
        id_strategy: &IdStrategy,
        alias: &str,
    ) -> QuickDbResult<DataValue> {
        if let DatabaseConnection::MongoDB(db) = connection {
            // 调试：打印原始接收到的数据
            // 自动建表逻辑：检查集合是否存在，如果不存在则创建
            if !mongodb_schema::table_exists(self, connection, table).await? {
                // 获取表创建锁，防止并发创建
                let _lock = self.acquire_table_lock(table).await;

                // 双重检查：再次确认集合不存在
                if !mongodb_schema::table_exists(self, connection, table).await? {
                    // 尝试从模型管理器获取预定义的元数据
                    if let Some(model_meta) = crate::manager::get_model_with_alias(table, alias) {
                        debug!("集合 {} 不存在，使用预定义模型元数据创建", table);

                        // MongoDB不需要预创建表结构，集合是无模式的
                    } else {
                        return Err(QuickDbError::ValidationError {
                            field: "collection_creation".to_string(),
                            message: format!(
                                "集合 '{}' 不存在，且没有预定义的模型元数据。MongoDB使用无模式设计，但建议先定义模型。",
                                table
                            ),
                        });
                    }

                    // 等待一小段时间确保数据库事务完成
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
            }

            let collection = mongodb_utils::get_collection(self, db, table);

            // 映射字段名（id -> _id）并处理ID策略
            let mut mapped_data = mongodb_utils::map_data_fields(self, data);

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
                    }
                    IdStrategy::Snowflake { .. } | IdStrategy::Uuid => {
                        // 对于雪花和UUID策略，移除空的ID字段，让ODM层生成的ID生效
                        if let Some(DataValue::String(s)) = mapped_data.get("_id") {
                            if s.is_empty() {
                                mapped_data.remove("_id");
                            }
                        }
                    }
                    IdStrategy::Custom(_) => {
                        // 自定义策略保留ID字段
                    }
                }
            } else {
                // 没有ID字段，检查策略是否需要ID
                match id_strategy {
                    IdStrategy::Snowflake { .. } => {
                        // 雪花策略需要ID字段
                        return Err(QuickDbError::ValidationError {
                            field: "_id".to_string(),
                            message: format!("使用{:?}策略时必须提供ID字段", id_strategy),
                        });
                    }
                    IdStrategy::Uuid => {
                        // MongoDB的UUID策略不要求提供ID字段，可以自动生成字符串UUID
                        // 符合我们的设计：MongoDB将UUID作为字符串处理
                    }
                    _ => {} // 其他策略不需要ID字段
                }
            }

            let mut doc = Document::new();
            for (key, value) in &mapped_data {
                // 特殊处理_id字段，根据ID策略决定BSON类型
                if key == "_id" {
                    let bson_value = match (value, id_strategy) {
                        (crate::types::DataValue::String(s), crate::types::IdStrategy::Uuid) => {
                            // UUID策略：保持字符串格式，防止被MongoDB转换为ObjectId
                            // 使用Bson::String包装，MongoDB应该保持字符串格式
                            Bson::String(s.clone())
                        }
                        (
                            crate::types::DataValue::String(s),
                            crate::types::IdStrategy::ObjectId,
                        ) => {
                            // ObjectId策略：尝试转换为ObjectId
                            if let Ok(object_id) = mongodb::bson::oid::ObjectId::parse_str(s) {
                                Bson::ObjectId(object_id)
                            } else {
                                Bson::String(s.clone()) // 如果解析失败，保持字符串
                            }
                        }
                        _ => {
                            // 其他情况，使用默认转换
                            match mongodb_utils::data_value_to_bson(self, value) {
                                Ok(bson_val) => bson_val,
                                Err(e) => {
                                    return Err(QuickDbError::QueryError {
                                        message: format!("转换DataValue为BSON失败: {}", e),
                                    });
                                }
                            }
                        }
                    };
                    doc.insert(key, bson_value);
                } else {
                    match mongodb_utils::data_value_to_bson(self, value) {
                        Ok(bson_val) => doc.insert(key, bson_val),
                        Err(e) => {
                            return Err(QuickDbError::QueryError {
                                message: format!("转换DataValue为BSON失败: {}", e),
                            });
                        }
                    };
                }
            }

            debug!("执行MongoDB插入到集合 {}: {:?}", table, doc);

            let result =
                collection
                    .insert_one(doc, None)
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
        alias: &str,
    ) -> QuickDbResult<Option<DataValue>> {
        mongodb_query::find_by_id(self, connection, table, id, alias).await
    }

    async fn find_with_cache_control(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryConditionWithConfig],
        options: &QueryOptions,
        alias: &str,
        bypass_cache: bool,
    ) -> QuickDbResult<Vec<DataValue>> {
        mongodb_query::find(self, connection, table, conditions, options, alias).await
    }

    async fn find_with_groups_with_cache_control(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        condition_groups: &[QueryConditionGroup],
        options: &QueryOptions,
        alias: &str,
        bypass_cache: bool,
    ) -> QuickDbResult<Vec<DataValue>> {
        mongodb_query::find_with_groups(self, connection, table, condition_groups, options, alias)
            .await
    }

    async fn find_with_groups_with_cache_control_and_config(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        condition_groups: &[QueryConditionGroupWithConfig],
        options: &QueryOptions,
        alias: &str,
        bypass_cache: bool,
    ) -> QuickDbResult<Vec<DataValue>> {
        mongodb_query::find_with_groups_with_config(self, connection, table, condition_groups, options, alias)
            .await
    }

    async fn find(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryConditionWithConfig],
        options: &QueryOptions,
        alias: &str,
    ) -> QuickDbResult<Vec<DataValue>> {
        self.find_with_cache_control(connection, table, conditions, options, alias, false).await
    }

    async fn find_with_groups(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        condition_groups: &[QueryConditionGroup],
        options: &QueryOptions,
        alias: &str,
    ) -> QuickDbResult<Vec<DataValue>> {
        self.find_with_groups_with_cache_control(connection, table, condition_groups, options, alias, false).await
    }

    async fn update(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryConditionWithConfig],
        data: &HashMap<String, DataValue>,
        alias: &str,
    ) -> QuickDbResult<u64> {
        if let DatabaseConnection::MongoDB(db) = connection {
            let collection = mongodb_utils::get_collection(self, db, table);

            let query = build_query_document(table, alias, conditions)?;
            let update = mongodb_utils::build_update_document(self, data)?;

            debug!("执行MongoDB更新: 查询={:?}, 更新={:?}", query, update);

            let result = collection
                .update_many(query, update, None)
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
        alias: &str,
    ) -> QuickDbResult<bool> {
        let conditions = vec![QueryConditionWithConfig {
            field: "_id".to_string(), // MongoDB使用_id作为主键
            operator: QueryOperator::Eq,
            value: id.clone(),
            case_insensitive: false,
        }];

        let affected = self
            .update(connection, table, &conditions, data, alias)
            .await?;
        Ok(affected > 0)
    }

    async fn update_with_operations(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryConditionWithConfig],
        operations: &[crate::types::UpdateOperation],
        alias: &str,
    ) -> QuickDbResult<u64> {
        if let DatabaseConnection::MongoDB(db) = connection {
            let collection = mongodb_utils::get_collection(self, db, table);

            let query = build_query_document(table, alias, conditions)?;
            let mut update_doc = Document::new();

            let mut set_doc = Document::new();
            let mut inc_doc = Document::new();

            for operation in operations {
                match &operation.operation {
                    crate::types::UpdateOperator::Set => {
                        match mongodb_utils::data_value_to_bson(self, &operation.value) {
                            Ok(bson_value) => set_doc.insert(&operation.field, bson_value),
                            Err(e) => {
                                return Err(QuickDbError::QueryError {
                                    message: format!("转换更新值为BSON失败: {}", e),
                                });
                            }
                        };
                    }
                    crate::types::UpdateOperator::Increment => {
                        match mongodb_utils::data_value_to_bson(self, &operation.value) {
                            Ok(bson_value) => inc_doc.insert(&operation.field, bson_value),
                            Err(e) => {
                                return Err(QuickDbError::QueryError {
                                    message: format!("转换递增值为BSON失败: {}", e),
                                });
                            }
                        };
                    }
                    crate::types::UpdateOperator::Decrement => {
                        // 对于减少操作，使用负数的inc操作
                        let neg_value = match &operation.value {
                            crate::types::DataValue::Int(i) => crate::types::DataValue::Int(-i),
                            crate::types::DataValue::Float(f) => crate::types::DataValue::Float(-f),
                            _ => {
                                return Err(QuickDbError::ValidationError {
                                    field: operation.field.clone(),
                                    message: "Decrement操作只支持数值类型".to_string(),
                                });
                            }
                        };
                        match mongodb_utils::data_value_to_bson(self, &neg_value) {
                            Ok(bson_value) => inc_doc.insert(&operation.field, bson_value),
                            Err(e) => {
                                return Err(QuickDbError::QueryError {
                                    message: format!("转换递减值为BSON失败: {}", e),
                                });
                            }
                        };
                    }
                    crate::types::UpdateOperator::Multiply => {
                        // MongoDB使用$multiply操作符
                        match mongodb_utils::data_value_to_bson(self, &operation.value) {
                            Ok(bson_value) => {
                                if !set_doc.contains_key("$mul") {
                                    set_doc.insert("$mul", Document::new());
                                }
                                let mul_doc = set_doc.get_mut("$mul").unwrap().as_document_mut().unwrap();
                                mul_doc.insert(&operation.field, bson_value);
                            }
                            Err(e) => {
                                return Err(QuickDbError::QueryError {
                                    message: format!("转换乘数值为BSON失败: {}", e),
                                });
                            }
                        };
                    }
                    crate::types::UpdateOperator::Divide => {
                        // MongoDB不支持直接除法，但可以使用乘法配合小数
                        let divisor = match &operation.value {
                            crate::types::DataValue::Int(i) => 1.0 / *i as f64,
                            crate::types::DataValue::Float(f) => 1.0 / f,
                            _ => {
                                return Err(QuickDbError::ValidationError {
                                    field: operation.field.clone(),
                                    message: "Divide操作只支持数值类型".to_string(),
                                });
                            }
                        };
                        match mongodb_utils::data_value_to_bson(
                            self,
                            &crate::types::DataValue::Float(divisor),
                        ) {
                            Ok(bson_value) => {
                                if !set_doc.contains_key("$mul") {
                                    set_doc.insert("$mul", Document::new());
                                }
                                let mul_doc = set_doc.get_mut("$mul").unwrap().as_document_mut().unwrap();
                                mul_doc.insert(&operation.field, bson_value);
                            }
                            Err(e) => {
                                return Err(QuickDbError::QueryError {
                                    message: format!("转换除数值为BSON失败: {}", e),
                                });
                            }
                        };
                    }
                    crate::types::UpdateOperator::PercentIncrease => {
                        // 百分比增加：转换为乘法 (1 + percentage/100)
                        let percentage = match &operation.value {
                            crate::types::DataValue::Float(f) => *f,
                            crate::types::DataValue::Int(i) => *i as f64,
                            _ => {
                                return Err(QuickDbError::ValidationError {
                                    field: operation.field.clone(),
                                    message: "PercentIncrease操作只支持数值类型".to_string(),
                                });
                            }
                        };
                        let multiplier = 1.0 + percentage / 100.0;
                        match mongodb_utils::data_value_to_bson(
                            self,
                            &crate::types::DataValue::Float(multiplier),
                        ) {
                            Ok(bson_value) => {
                                if !set_doc.contains_key("$mul") {
                                    set_doc.insert("$mul", Document::new());
                                }
                                let mul_doc = set_doc.get_mut("$mul").unwrap().as_document_mut().unwrap();
                                mul_doc.insert(&operation.field, bson_value);
                            }
                            Err(e) => {
                                return Err(QuickDbError::QueryError {
                                    message: format!("转换百分比增加值为BSON失败: {}", e),
                                });
                            }
                        };
                    }
                    crate::types::UpdateOperator::PercentDecrease => {
                        // 百分比减少：转换为乘法 (1 - percentage/100)
                        let percentage = match &operation.value {
                            crate::types::DataValue::Float(f) => *f,
                            crate::types::DataValue::Int(i) => *i as f64,
                            _ => {
                                return Err(QuickDbError::ValidationError {
                                    field: operation.field.clone(),
                                    message: "PercentDecrease操作只支持数值类型".to_string(),
                                });
                            }
                        };
                        let multiplier = 1.0 - percentage / 100.0;
                        match mongodb_utils::data_value_to_bson(
                            self,
                            &crate::types::DataValue::Float(multiplier),
                        ) {
                            Ok(bson_value) => {
                                if !set_doc.contains_key("$mul") {
                                    set_doc.insert("$mul", Document::new());
                                }
                                let mul_doc = set_doc.get_mut("$mul").unwrap().as_document_mut().unwrap();
                                mul_doc.insert(&operation.field, bson_value);
                            }
                            Err(e) => {
                                return Err(QuickDbError::QueryError {
                                    message: format!("转换百分比减少值为BSON失败: {}", e),
                                });
                            }
                        };
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

            debug!(
                "执行MongoDB操作更新: query={:?}, update={:?}",
                query, update_doc
            );

            let result = collection
                .update_many(query, update_doc, None)
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
        conditions: &[QueryConditionWithConfig],
        alias: &str,
    ) -> QuickDbResult<u64> {
        if let DatabaseConnection::MongoDB(db) = connection {
            let collection = mongodb_utils::get_collection(self, db, table);

            let query = build_query_document(table, alias, conditions)?;

            debug!("执行MongoDB删除: {:?}", query);

            let result = collection.delete_many(query, None).await.map_err(|e| {
                if check_collection_not_exist_error(&e, table) {
                    QuickDbError::TableNotExistError {
                        table: table.to_string(),
                        message: format!("MongoDB集合 '{}' 不存在", table),
                    }
                } else {
                    QuickDbError::QueryError {
                        message: format!("MongoDB删除失败: {}", e),
                    }
                }
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
        alias: &str,
    ) -> QuickDbResult<bool> {
        let conditions = vec![QueryConditionWithConfig {
            field: "_id".to_string(), // MongoDB使用_id作为主键
            operator: QueryOperator::Eq,
            value: id.clone(),
            case_insensitive: false,
        }];

        let affected = self.delete(connection, table, &conditions, alias).await?;
        Ok(affected > 0)
    }

    async fn count(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryConditionWithConfig],
        alias: &str,
    ) -> QuickDbResult<u64> {
        mongodb_query::count(self, connection, table, conditions, alias).await
    }

    async fn count_with_groups(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        condition_groups: &[QueryConditionGroupWithConfig],
        alias: &str,
    ) -> QuickDbResult<u64> {
        mongodb_query::count_with_groups(self, connection, table, condition_groups, alias).await
    }

    async fn create_table(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        _fields: &HashMap<String, FieldDefinition>,
        _id_strategy: &IdStrategy,
        alias: &str,
    ) -> QuickDbResult<()> {
        mongodb_schema::create_table(self, connection, table, _fields, _id_strategy, alias).await
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

    async fn drop_table(&self, connection: &DatabaseConnection, table: &str) -> QuickDbResult<()> {
        mongodb_schema::drop_table(self, connection, table).await
    }

    async fn get_server_version(&self, connection: &DatabaseConnection) -> QuickDbResult<String> {
        mongodb_schema::get_server_version(self, connection).await
    }

    async fn create_stored_procedure(
        &self,
        connection: &DatabaseConnection,
        config: &crate::stored_procedure::StoredProcedureConfig,
    ) -> QuickDbResult<crate::stored_procedure::StoredProcedureCreateResult> {
        use crate::stored_procedure::StoredProcedureCreateResult;
        use crate::types::id_types::IdStrategy;

        debug!("开始创建MongoDB存储过程: {}", config.procedure_name);

        // 验证配置
        config
            .validate()
            .map_err(|e| crate::error::QuickDbError::ValidationError {
                field: "config".to_string(),
                message: format!("存储过程配置验证失败: {}", e),
            })?;

        // 1. 确保依赖集合存在
        for model_meta in &config.dependencies {
            let collection_name = &model_meta.collection_name;
            if !self.table_exists(connection, collection_name).await? {
                debug!("依赖集合 {} 不存在，尝试创建", collection_name);
                // 使用存储的模型元数据和数据库的ID策略创建集合
                let id_strategy = crate::manager::get_id_strategy(&config.database)
                    .unwrap_or(IdStrategy::AutoIncrement);

                self.create_table(
                    connection,
                    collection_name,
                    &model_meta.fields,
                    &id_strategy,
                    &config.database,
                )
                .await?;
            }
        }

        // 2. 生成MongoDB聚合管道（带占位符）
        let pipeline_json = self.generate_stored_procedure_pipeline(&config).await?;
        debug!("生成MongoDB存储过程聚合管道: {}", pipeline_json);

        // 3. 将存储过程信息存储到适配器映射表中（MongoDB不需要执行创建聚合管道）
        let procedure_info = crate::stored_procedure::StoredProcedureInfo {
            config: config.clone(),
            template: pipeline_json.clone(),
            db_type: "MongoDB".to_string(),
            created_at: chrono::Utc::now(),
        };

        let mut procedures = self.stored_procedures.lock().await;
        procedures.insert(config.procedure_name.clone(), procedure_info);

        Ok(StoredProcedureCreateResult {
            success: true,
            procedure_name: config.procedure_name.clone(),
            error: None,
        })
    }

    /// 执行存储过程查询（MongoDB使用聚合管道实现）
    async fn execute_stored_procedure(
        &self,
        connection: &DatabaseConnection,
        procedure_name: &str,
        database: &str,
        params: Option<std::collections::HashMap<String, crate::types::DataValue>>,
    ) -> QuickDbResult<crate::stored_procedure::StoredProcedureQueryResult> {
        use crate::adapter::mongodb::adapter::MongoAdapter;
        use serde_json::json;

        // 获取存储过程信息
        let procedures = self.stored_procedures.lock().await;
        let procedure_info = procedures.get(procedure_name).ok_or_else(|| {
            crate::error::QuickDbError::ValidationError {
                field: "procedure_name".to_string(),
                message: format!("存储过程 '{}' 不存在", procedure_name),
            }
        })?;
        let pipeline_template = procedure_info.template.clone();
        drop(procedures);

        debug!(
            "执行MongoDB存储过程查询: {}, 模板: {}",
            procedure_name, pipeline_template
        );

        // 解析聚合管道模板
        let pipeline_value: serde_json::Value =
            serde_json::from_str(&pipeline_template).map_err(|e| {
                crate::error::QuickDbError::SerializationError {
                    message: format!("解析聚合管道模板失败: {}", e),
                }
            })?;

        // 根据参数动态构建最终的聚合管道
        let final_pipeline = self
            .build_final_pipeline_from_template(&pipeline_value, params)
            .await?;

        // 提取集合名
        let collection_name = final_pipeline
            .get("collection")
            .and_then(|v| v.as_str())
            .ok_or_else(|| crate::error::QuickDbError::ValidationError {
                field: "pipeline".to_string(),
                message: "聚合管道模板缺少collection字段".to_string(),
            })?;

        let pipeline_stages = final_pipeline
            .get("pipeline")
            .and_then(|v| v.as_array())
            .ok_or_else(|| crate::error::QuickDbError::ValidationError {
                field: "pipeline".to_string(),
                message: "聚合管道模板缺少pipeline字段".to_string(),
            })?;

        debug!(
            "执行MongoDB聚合管道: 集合={}, 阶段数={}",
            collection_name,
            pipeline_stages.len()
        );

        // 执行聚合管道查询
        let query_result = self
            .aggregate_query(connection, collection_name, pipeline_stages.to_vec())
            .await?;

        // 转换结果格式
        let mut result = Vec::new();
        for row_data in query_result {
            let mut row_map = std::collections::HashMap::new();
            for (key, value) in row_data {
                row_map.insert(key, value);
            }
            result.push(row_map);
        }

        debug!(
            "MongoDB存储过程 {} 执行完成，返回 {} 条记录",
            procedure_name,
            result.len()
        );
        Ok(result)
    }
}

impl MongoAdapter {
    /// 根据模板和参数构建最终聚合管道
    async fn build_final_pipeline_from_template(
        &self,
        pipeline_template: &serde_json::Value,
        params: Option<std::collections::HashMap<String, crate::types::DataValue>>,
    ) -> QuickDbResult<serde_json::Value> {
        let mut final_pipeline = pipeline_template.clone();

        // 简单过滤占位符阶段
        if let Some(pipeline_array) = final_pipeline
            .get_mut("pipeline")
            .and_then(|v| v.as_array_mut())
        {
            let filtered_stages: Vec<serde_json::Value> = pipeline_array
                .iter()
                .filter(|stage| {
                    // 过滤掉纯占位符的$addFields阶段
                    if let Some(add_fields) = stage.get("$addFields") {
                        if let Some(obj) = add_fields.as_object() {
                            // 检查是否所有字段都是占位符
                            !obj.keys()
                                .all(|key| key.starts_with("_") && key.ends_with("_PLACEHOLDER"))
                        } else {
                            true
                        }
                    } else {
                        true
                    }
                })
                .cloned()
                .collect();

            final_pipeline["pipeline"] = serde_json::Value::Array(filtered_stages);
        }

        debug!(
            "构建的最终聚合管道: {}",
            serde_json::to_string_pretty(&final_pipeline).unwrap_or_default()
        );
        Ok(final_pipeline)
    }
}
