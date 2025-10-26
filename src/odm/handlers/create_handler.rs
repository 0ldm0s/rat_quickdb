  //! # 创建操作处理器

use crate::error::{QuickDbError, QuickDbResult};
use crate::types::*;
use crate::manager::get_global_pool_manager;
use crate::odm::manager_core::AsyncOdmManager;
use rat_logger::{debug, info, warn, error};
use tokio::sync::oneshot;
use std::collections::HashMap;

impl AsyncOdmManager {
    /// 处理创建请求
    #[doc(hidden)]
    pub async fn handle_create(
        collection: &str,
        data: HashMap<String, DataValue>,
        alias: Option<String>,
    ) -> QuickDbResult<DataValue> {
        let manager = get_global_pool_manager();
        let actual_alias = match alias {
            Some(a) => a,
            None => {
                // 使用连接池管理器的默认别名
                manager.get_default_alias().await
                    .unwrap_or_else(|| "default".to_string())
            }
        };
        debug!("处理创建请求: collection={}, alias={}", collection, actual_alias);

        // 调试打印：主库ODM层接收到的数据
        println!("🔍 主库ODM层 - 接收到的数据 collection: {}", collection);
        println!("🔍 主库ODM层 - 接收到的data_map:");
        for (key, data_value) in &data {
            println!("  {}: {:?}", key, data_value);
        }

        // 确保表和索引存在（基于注册的模型元数据）
        if let Err(e) = manager.ensure_table_and_indexes(collection, &actual_alias).await {
            debug!("自动创建表和索引失败: {}", e);
            // 不返回错误，让适配器处理自动创建逻辑
        }

        let manager = get_global_pool_manager();
        let connection_pools = manager.get_connection_pools();
        let connection_pool = connection_pools.get(&actual_alias)
            .ok_or_else(|| QuickDbError::AliasNotFound {
                alias: actual_alias.clone(),
            })?;

        // 获取ID策略用于传递给适配器，必须提供有效策略
        let id_strategy = connection_pool.db_config.id_strategy.clone();

        // 根据ID策略处理ID字段
        let mut processed_data = data.clone();
        if let Ok(id_generator) = manager.get_id_generator(&actual_alias) {
            match id_generator.strategy() {
                crate::types::IdStrategy::AutoIncrement => {
                    // AutoIncrement策略：移除用户传入的id字段，让数据库自动生成
                    debug!("AutoIncrement策略，移除id字段让数据库自动生成");
                    processed_data.remove("id");
                    processed_data.remove("_id");
                },
                _ => {
                    // 检查是否有有效的ID字段（非空、非零）
                    println!("🔍 ODM ID检查 - 检查id字段有效性");
                    let id_is_valid = match processed_data.get("id") {
                        Some(crate::types::DataValue::String(s)) => {
                            println!("🔍 ODM ID检查 - 找到String类型ID: '{}', 长度: {}, is_empty: {}", s, s.len(), s.is_empty());
                            !s.is_empty()
                        },
                        Some(crate::types::DataValue::Int(i)) => {
                            println!("🔍 ODM ID检查 - 找到Int类型ID: {}, >0: {}", i, *i > 0);
                            *i > 0
                        },
                        Some(crate::types::DataValue::Null) => {
                            println!("🔍 ODM ID检查 - 找到Null类型ID");
                            false
                        },
                        Some(other) => {
                            println!("🔍 ODM ID检查 - 找到其他类型ID: {:?}", other);
                            true // 其他非空类型认为是有效ID
                        },
                        None => {
                            println!("🔍 ODM ID检查 - 没有找到id字段");
                            false
                        },
                    };
                    println!("🔍 ODM ID检查 - id_is_valid: {}", id_is_valid);
                    let _id_is_valid = match processed_data.get("_id") {
                        Some(crate::types::DataValue::String(s)) => !s.is_empty(),
                        Some(crate::types::DataValue::Int(i)) => *i > 0,
                        Some(crate::types::DataValue::Null) => false,
                        Some(_) => true, // 其他非空类型认为是有效ID
                        None => false,
                    };
                    let has_valid_id = id_is_valid || _id_is_valid;

                    if !has_valid_id {
                        println!("🔍 ODM ID生成 - 没有有效ID，开始生成ID");
                        debug!("数据中没有有效ID字段，使用IdGenerator生成ID");
                        match id_generator.generate().await {
                            Ok(id_type) => {
                                let id_value = match &id_type {
                                    crate::types::IdType::Number(n) => DataValue::Int(*n),
                                    crate::types::IdType::String(s) => DataValue::String(s.clone()),
                                };
                                println!("🔍 ODM ID生成 - ✅ 成功生成ID: {:?}, 转换后: {:?}", id_type, id_value);
                                debug!("✅ 成功生成ID: {:?}, 转换后: {:?}", id_type, id_value);
                                // 根据数据库类型决定使用"id"还是"_id"字段
                                match connection_pool.db_config.db_type {
                                    crate::types::DatabaseType::MongoDB => {
                                        debug!("为MongoDB生成_id字段");
                                        processed_data.insert("_id".to_string(), id_value);
                                    },
                                    _ => {
                                        debug!("为SQL数据库生成id字段");
                                        processed_data.insert("id".to_string(), id_value);
                                    }
                                }
                            },
                            Err(e) => {
                                error!("❌❌❌ ID生成失败: {} - 立即停止执行！❌❌❌", e);
                                return Err(QuickDbError::Other(e));
                            }
                        }
                    }
                }
            }
        } else {
            warn!("获取IdGenerator失败，使用原始数据");
        }

        // 创建oneshot通道用于接收响应
        let (response_tx, response_rx) = oneshot::channel();

        // 发送DatabaseOperation::Create请求到连接池
        println!("🔍 ODM最终数据 - 发送给适配器的processed_data:");
        for (key, data_value) in &processed_data {
            println!("  {}: {:?}", key, data_value);
        }

        let operation = crate::pool::DatabaseOperation::Create {
            table: collection.to_string(),
            data: processed_data,
            id_strategy,
            response: response_tx,
        };
        
        connection_pool.operation_sender.send(operation)
            .map_err(|_| QuickDbError::ConnectionError {
                message: "连接池操作通道已关闭".to_string(),
            })?;
        
        // 等待响应
        let result = response_rx.await
            .map_err(|_| QuickDbError::ConnectionError {
                message: "等待连接池响应超时".to_string(),
            })??;
        
        // 从返回的Object中提取id字段
        match result {
            DataValue::Object(map) => {
                // 优先查找"id"字段（SQL数据库），如果没有则查找"_id"字段（MongoDB）
                if let Some(id_value) = map.get("id") {
                    Ok(id_value.clone())
                } else if let Some(id_value) = map.get("_id") {
                    Ok(id_value.clone())
                } else {
                    Err(QuickDbError::QueryError {
                        message: "创建操作返回的数据中缺少id字段".to_string(),
                    })
                }
            },
            // 如果返回的不是Object，可能是其他数据库的直接ID值，直接返回
            other => Ok(other),
        }
    }
}
