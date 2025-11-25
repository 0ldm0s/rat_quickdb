    //! MongoDB集合和索引管理模块

use crate::adapter::mongodb::MongoAdapter;
use crate::adapter::DatabaseConnection;
use crate::error::{QuickDbError, QuickDbResult};
use crate::types::*;
use crate::model::{FieldType, FieldDefinition};
use rat_logger::debug;
use std::collections::HashMap;
use mongodb::bson::{doc, Document};

pub(crate) async fn create_table(
    adapter: &MongoAdapter,
    connection: &DatabaseConnection,
    table: &str,
    _fields: &HashMap<String, FieldDefinition>,
    _id_strategy: &IdStrategy,
    alias: &str,
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

    pub(crate) async fn create_index(
    adapter: &MongoAdapter,
    connection: &DatabaseConnection,
    table: &str,
    index_name: &str,
    fields: &[String],
    unique: bool,
) -> QuickDbResult<()> {
        if let DatabaseConnection::MongoDB(db) = connection {
            let collection = crate::adapter::mongodb::utils::get_collection(adapter, db, table);
            
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

    pub(crate) async fn table_exists(
    adapter: &MongoAdapter,
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

    pub(crate) async fn drop_table(
    adapter: &MongoAdapter,
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

    pub(crate) async fn get_server_version(
    adapter: &MongoAdapter,
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
