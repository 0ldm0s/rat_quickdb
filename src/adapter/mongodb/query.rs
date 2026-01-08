
//! MongoDB查询操作模块

use crate::adapter::DatabaseConnection;
use crate::adapter::mongodb::MongoAdapter;
use crate::error::{QuickDbError, QuickDbResult};
use crate::types::*;
use mongodb::bson::{Document, doc};
use rat_logger::debug;

/// 检查MongoDB错误是否为集合不存在错误
fn check_collection_not_exist_error(error: &mongodb::error::Error, collection: &str) -> bool {
    let error_string = error.to_string().to_lowercase();
    error_string.contains("namespace not found") ||
    error_string.contains(&format!("ns not found: {}", collection.to_lowercase())) ||
    error_string.contains("collection") && error_string.contains("does not exist") ||
    error_string.contains("invalid namespace") ||
    error_string.contains("command failed") && error_string.contains("find")
}

pub(crate) async fn find_by_id(
    adapter: &MongoAdapter,
    connection: &DatabaseConnection,
    table: &str,
    id: &DataValue,
    alias: &str,
) -> QuickDbResult<Option<DataValue>> {
    if let DatabaseConnection::MongoDB(db) = connection {
        let collection = crate::adapter::mongodb::utils::get_collection(adapter, db, table);

        let query = match id {
            DataValue::String(id_str) => {
                // 处理ObjectId格式：ObjectId("xxx") 或直接是ObjectId字符串
                let actual_id = if id_str.starts_with("ObjectId(\"") && id_str.ends_with("\")") {
                    // 提取ObjectId字符串部分
                    &id_str[10..id_str.len() - 2]
                } else {
                    id_str
                };

                // 尝试解析为ObjectId，如果失败则作为字符串查询
                if let Ok(object_id) = mongodb::bson::oid::ObjectId::parse_str(actual_id) {
                    doc! { "_id": object_id }
                } else {
                    doc! { "_id": actual_id }
                }
            }
            _ => {
                match crate::adapter::mongodb::utils::data_value_to_bson(adapter, id) {
                    Ok(bson_id) => doc! { "_id": bson_id },
                    Err(e) => {
                        return Err(QuickDbError::QueryError {
                            message: format!("转换ID为BSON失败: {}", e),
                        });
                    }
                }
            }
        };

        debug!("执行MongoDB根据ID查询: {:?}", query);

        let result =
            collection
                .find_one(query, None)
                .await
                .map_err(|e| {
                    if check_collection_not_exist_error(&e, table) {
                        QuickDbError::TableNotExistError {
                            table: table.to_string(),
                            message: format!("MongoDB集合 '{}' 不存在", table),
                        }
                    } else {
                        QuickDbError::QueryError {
                            message: format!("MongoDB查询失败: {}", e),
                        }
                    }
                })?;

        if let Some(doc) = result {
            let data_map = crate::adapter::mongodb::utils::document_to_data_map(adapter, &doc)?;
            // 直接返回Object，避免双重包装
            Ok(Some(DataValue::Object(data_map)))
        } else {
            // MongoDB查询不存在的集合或空集合都返回None
            // 为了提供统一的错误处理接口，将其视为TableNotExistError
            // 这样调用者可以得到明确的预期，并在需要时通过插入操作自动创建集合
            Err(QuickDbError::TableNotExistError {
                table: table.to_string(),
                message: format!("MongoDB集合 '{}' 不存在或为空", table),
            })
        }
    } else {
        Err(QuickDbError::ConnectionError {
            message: "连接类型不匹配，期望MongoDB连接".to_string(),
        })
    }
}

pub(crate) async fn find(
    adapter: &MongoAdapter,
    connection: &DatabaseConnection,
    table: &str,
    conditions: &[QueryConditionWithConfig],
    options: &QueryOptions,
    alias: &str,
) -> QuickDbResult<Vec<DataValue>> {
    // 将简单条件转换为条件组合（AND逻辑）
    let condition_groups = if conditions.is_empty() {
        vec![]
    } else {
        let group_conditions = conditions
            .iter()
            .map(|c| QueryConditionGroupWithConfig::Single(c.clone()))
            .collect();
        vec![QueryConditionGroupWithConfig::GroupWithConfig {
            operator: LogicalOperator::And,
            conditions: group_conditions,
        }]
    };

    // 统一使用 find_with_groups_with_config 实现
    find_with_groups_with_config(
        adapter,
        connection,
        table,
        &condition_groups,
        options,
        alias,
    )
    .await
}

pub(crate) async fn find_with_groups(
    adapter: &MongoAdapter,
    connection: &DatabaseConnection,
    table: &str,
    condition_groups: &[QueryConditionGroup],
    options: &QueryOptions,
    alias: &str,
) -> QuickDbResult<Vec<DataValue>> {
    if let DatabaseConnection::MongoDB(db) = connection {
        let collection = crate::adapter::mongodb::utils::get_collection(adapter, db, table);

        let query = crate::adapter::mongodb::query_builder::MongoQueryBuilder::new()
            .where_condition_groups(condition_groups)
            .build(table, alias)?;

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

        let mut cursor =
            collection
                .find(query, find_options)
                .await
                .map_err(|e| QuickDbError::QueryError {
                    message: format!("MongoDB条件组合查询失败: {}", e),
                })?;

        let mut results = Vec::new();
        while cursor
            .advance()
            .await
            .map_err(|e| QuickDbError::QueryError {
                message: format!("MongoDB游标遍历失败: {}", e),
            })?
        {
            let doc = cursor
                .deserialize_current()
                .map_err(|e| QuickDbError::QueryError {
                    message: format!("MongoDB文档反序列化失败: {}", e),
                })?;
            let data_map = crate::adapter::mongodb::utils::document_to_data_map(adapter, &doc)?;
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

pub(crate) async fn find_with_groups_with_config(
    adapter: &MongoAdapter,
    connection: &DatabaseConnection,
    table: &str,
    condition_groups: &[QueryConditionGroupWithConfig],
    options: &QueryOptions,
    alias: &str,
) -> QuickDbResult<Vec<DataValue>> {
    if let DatabaseConnection::MongoDB(db) = connection {
        let collection = crate::adapter::mongodb::utils::get_collection(adapter, db, table);

        let query = crate::adapter::mongodb::query_builder::MongoQueryBuilder::new()
            .where_condition_groups_with_config(condition_groups)
            .build(table, alias)?;

        debug!("执行MongoDB条件组合查询（完整版）: {:?}", query);

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

        let mut cursor =
            collection
                .find(query, find_options)
                .await
                .map_err(|e| QuickDbError::QueryError {
                    message: format!("MongoDB条件组合查询失败: {}", e),
                })?;

        let mut results = Vec::new();
        while cursor
            .advance()
            .await
            .map_err(|e| QuickDbError::QueryError {
                message: format!("MongoDB游标遍历失败: {}", e),
            })?
        {
            let doc = cursor
                .deserialize_current()
                .map_err(|e| QuickDbError::QueryError {
                    message: format!("MongoDB文档反序列化失败: {}", e),
                })?;
            let data_map = crate::adapter::mongodb::utils::document_to_data_map(adapter, &doc)?;
            results.push(DataValue::Object(data_map));
        }

        Ok(results)
    } else {
        Err(QuickDbError::ConnectionError {
            message: "连接类型不匹配，期望MongoDB连接".to_string(),
        })
    }
}

pub(crate) async fn count(
    adapter: &MongoAdapter,
    connection: &DatabaseConnection,
    table: &str,
    conditions: &[QueryConditionWithConfig],
    alias: &str,
) -> QuickDbResult<u64> {
    if let DatabaseConnection::MongoDB(db) = connection {
        let collection = crate::adapter::mongodb::utils::get_collection(adapter, db, table);

        let query =
            crate::adapter::mongodb::query_builder::build_query_document(table, alias, conditions)?;

        debug!("执行MongoDB计数: {:?}", query);

        let count = collection.count_documents(query, None).await.map_err(|e| {
            if check_collection_not_exist_error(&e, table) {
                QuickDbError::TableNotExistError {
                    table: table.to_string(),
                    message: format!("MongoDB集合 '{}' 不存在", table),
                }
            } else {
                QuickDbError::QueryError {
                    message: format!("MongoDB计数失败: {}", e),
                }
            }
        })?;

        Ok(count)
    } else {
        Err(QuickDbError::ConnectionError {
            message: "连接类型不匹配，期望MongoDB连接".to_string(),
        })
    }
}
