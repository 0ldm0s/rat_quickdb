//! 异步ODM层 - 使用无锁队列解决生命周期问题
//! 
//! 通过消息传递和无锁队列机制避免直接持有连接引用，解决生命周期问题

use crate::error::{QuickDbError, QuickDbResult};
use crate::types::*;
use crate::manager::get_global_pool_manager;
use crate::adapter::{DatabaseAdapter, create_adapter};
use crate::pool::{DatabaseOperation, DatabaseConnection};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use tokio::sync::{mpsc, oneshot};
use crossbeam_queue::SegQueue;
use std::sync::Arc;
use rat_logger::{debug, error, info, warn};

/// ODM操作接口
#[async_trait]
pub trait OdmOperations {
    /// 创建记录
    async fn create(
        &self,
        collection: &str,
        data: HashMap<String, DataValue>,
        alias: Option<&str>,
    ) -> QuickDbResult<DataValue>;

    /// 根据ID查找记录
    async fn find_by_id(
        &self,
        collection: &str,
        id: &str,
        alias: Option<&str>,
    ) -> QuickDbResult<Option<DataValue>>;

    /// 查找记录
    async fn find(
        &self,
        collection: &str,
        conditions: Vec<QueryCondition>,
        options: Option<QueryOptions>,
        alias: Option<&str>,
    ) -> QuickDbResult<Vec<DataValue>>;
    
    /// 使用条件组合查找记录（支持复杂OR/AND逻辑）
    async fn find_with_groups(
        &self,
        collection: &str,
        condition_groups: Vec<QueryConditionGroup>,
        options: Option<QueryOptions>,
        alias: Option<&str>,
    ) -> QuickDbResult<Vec<DataValue>>;
    
    /// 更新记录
    async fn update(
        &self,
        collection: &str,
        conditions: Vec<QueryCondition>,
        updates: HashMap<String, DataValue>,
        alias: Option<&str>,
    ) -> QuickDbResult<u64>;

    /// 使用操作数组更新记录
    async fn update_with_operations(
        &self,
        collection: &str,
        conditions: Vec<QueryCondition>,
        operations: Vec<crate::types::UpdateOperation>,
        alias: Option<&str>,
    ) -> QuickDbResult<u64>;

    /// 根据ID更新记录
    async fn update_by_id(
        &self,
        collection: &str,
        id: &str,
        updates: HashMap<String, DataValue>,
        alias: Option<&str>,
    ) -> QuickDbResult<bool>;
    
    /// 删除记录
    async fn delete(
        &self,
        collection: &str,
        conditions: Vec<QueryCondition>,
        alias: Option<&str>,
    ) -> QuickDbResult<u64>;
    
    /// 根据ID删除记录
    async fn delete_by_id(
        &self,
        collection: &str,
        id: &str,
        alias: Option<&str>,
    ) -> QuickDbResult<bool>;
    
    /// 统计记录数量
    async fn count(
        &self,
        collection: &str,
        conditions: Vec<QueryCondition>,
        alias: Option<&str>,
    ) -> QuickDbResult<u64>;
    
    /// 检查记录是否存在
    async fn exists(
        &self,
        collection: &str,
        conditions: Vec<QueryCondition>,
        alias: Option<&str>,
    ) -> QuickDbResult<bool>;

    /// 获取数据库服务器版本信息
    async fn get_server_version(
        &self,
        alias: Option<&str>,
    ) -> QuickDbResult<String>;
}

/// ODM操作请求类型
#[derive(Debug)]
pub enum OdmRequest {
    Create {
        collection: String,
        data: HashMap<String, DataValue>,
        alias: Option<String>,
        response: oneshot::Sender<QuickDbResult<DataValue>>,
    },
    FindById {
        collection: String,
        id: String,
        alias: Option<String>,
        response: oneshot::Sender<QuickDbResult<Option<DataValue>>>,
    },
    Find {
        collection: String,
        conditions: Vec<QueryCondition>,
        options: Option<QueryOptions>,
        alias: Option<String>,
        response: oneshot::Sender<QuickDbResult<Vec<DataValue>>>,
    },
    FindWithGroups {
        collection: String,
        condition_groups: Vec<QueryConditionGroup>,
        options: Option<QueryOptions>,
        alias: Option<String>,
        response: oneshot::Sender<QuickDbResult<Vec<DataValue>>>,
    },
    Update {
        collection: String,
        conditions: Vec<QueryCondition>,
        updates: HashMap<String, DataValue>,
        alias: Option<String>,
        response: oneshot::Sender<QuickDbResult<u64>>,
    },
    UpdateWithOperations {
        collection: String,
        conditions: Vec<QueryCondition>,
        operations: Vec<crate::types::UpdateOperation>,
        alias: Option<String>,
        response: oneshot::Sender<QuickDbResult<u64>>,
    },
    UpdateById {
        collection: String,
        id: String,
        updates: HashMap<String, DataValue>,
        alias: Option<String>,
        response: oneshot::Sender<QuickDbResult<bool>>,
    },
    Delete {
        collection: String,
        conditions: Vec<QueryCondition>,
        alias: Option<String>,
        response: oneshot::Sender<QuickDbResult<u64>>,
    },
    DeleteById {
        collection: String,
        id: String,
        alias: Option<String>,
        response: oneshot::Sender<QuickDbResult<bool>>,
    },
    Count {
        collection: String,
        conditions: Vec<QueryCondition>,
        alias: Option<String>,
        response: oneshot::Sender<QuickDbResult<u64>>,
    },
    Exists {
        collection: String,
        conditions: Vec<QueryCondition>,
        alias: Option<String>,
        response: oneshot::Sender<QuickDbResult<bool>>,
    },
    GetServerVersion {
        alias: Option<String>,
        response: oneshot::Sender<QuickDbResult<String>>,
    },
}

/// 异步ODM管理器 - 使用消息传递避免生命周期问题
pub struct AsyncOdmManager {
    /// 请求发送器
    request_sender: mpsc::UnboundedSender<OdmRequest>,
    /// 默认别名
    default_alias: String,
    /// 后台任务句柄（用于优雅关闭）
    _task_handle: Option<tokio::task::JoinHandle<()>>,
}

impl AsyncOdmManager {
    /// 创建新的异步ODM管理器
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        
        // 启动后台处理任务
        let task_handle = tokio::spawn(Self::process_requests(receiver));
        
        info!("创建异步ODM管理器");
        
        Self {
            request_sender: sender,
            default_alias: "default".to_string(),
            _task_handle: Some(task_handle),
        }
    }
    
    /// 设置默认别名
    pub fn set_default_alias(&mut self, alias: &str) {
        info!("设置默认别名: {}", alias);
        self.default_alias = alias.to_string();
    }
    
    /// 获取实际使用的别名
    fn get_actual_alias(&self, alias: Option<&str>) -> String {
        alias.unwrap_or(&self.default_alias).to_string()
    }
    
    /// 后台请求处理任务
    async fn process_requests(mut receiver: mpsc::UnboundedReceiver<OdmRequest>) {
        info!("启动ODM后台处理任务");
        
        while let Some(request) = receiver.recv().await {
            match request {
                OdmRequest::Create { collection, data, alias, response } => {
                    let result = Self::handle_create(&collection, data, alias).await;
                    let _ = response.send(result);
                },
                OdmRequest::FindById { collection, id, alias, response } => {
                    let result = Self::handle_find_by_id(&collection, &id, alias).await;
                    let _ = response.send(result);
                },
                OdmRequest::Find { collection, conditions, options, alias, response } => {
                    let result = Self::handle_find(&collection, conditions, options, alias).await;
                    let _ = response.send(result);
                },
                OdmRequest::FindWithGroups { collection, condition_groups, options, alias, response } => {
                    let result = Self::handle_find_with_groups(&collection, condition_groups, options, alias).await;
                    let _ = response.send(result);
                },
                OdmRequest::Update { collection, conditions, updates, alias, response } => {
                    let result = Self::handle_update(&collection, conditions, updates, alias).await;
                    let _ = response.send(result);
                },
                OdmRequest::UpdateWithOperations { collection, conditions, operations, alias, response } => {
                    let result = Self::handle_update_with_operations(&collection, conditions, operations, alias).await;
                    let _ = response.send(result);
                },
                OdmRequest::UpdateById { collection, id, updates, alias, response } => {
                    let result = Self::handle_update_by_id(&collection, &id, updates, alias).await;
                    let _ = response.send(result);
                },
                OdmRequest::Delete { collection, conditions, alias, response } => {
                    let result = Self::handle_delete(&collection, conditions, alias).await;
                    let _ = response.send(result);
                },
                OdmRequest::DeleteById { collection, id, alias, response } => {
                    let result = Self::handle_delete_by_id(&collection, &id, alias).await;
                    let _ = response.send(result);
                },
                OdmRequest::Count { collection, conditions, alias, response } => {
                    let result = Self::handle_count(&collection, conditions, alias).await;
                    let _ = response.send(result);
                },
                OdmRequest::Exists { collection, conditions, alias, response } => {
                    let result = Self::handle_exists(&collection, conditions, alias).await;
                    let _ = response.send(result);
                },
                OdmRequest::GetServerVersion { alias, response } => {
                    let result = Self::handle_get_server_version(alias).await;
                    let _ = response.send(result);
                },
            }
        }
        
        warn!("ODM后台处理任务结束");
    }
    
    /// 处理创建请求
    async fn handle_create(
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
    
    /// 处理根据ID查询请求
    async fn handle_find_by_id(
        collection: &str,
        id: &str,
        alias: Option<String>,
    ) -> QuickDbResult<Option<DataValue>> {
        let manager = get_global_pool_manager();
        let actual_alias = match alias {
            Some(a) => a,
            None => {
                manager.get_default_alias().await
                    .unwrap_or_else(|| "default".to_string())
            }
        };
        debug!("处理根据ID查询请求: collection={}, id={}, alias={}", collection, id, actual_alias);
        
        let manager = get_global_pool_manager();
        let connection_pools = manager.get_connection_pools();
        let connection_pool = connection_pools.get(&actual_alias)
            .ok_or_else(|| QuickDbError::AliasNotFound {
                alias: actual_alias.clone(),
            })?;
        
        // 创建oneshot通道用于接收响应
        let (response_tx, response_rx) = oneshot::channel();
        
        // 发送DatabaseOperation::FindById请求到连接池
        let operation = DatabaseOperation::FindById {
            table: collection.to_string(),
            id: DataValue::String(id.to_string()),
            response: response_tx,
        };
        
        connection_pool.operation_sender.send(operation)
            .map_err(|_| QuickDbError::ConnectionError {
                message: "连接池操作通道已关闭".to_string(),
            })?;
        
        // 等待响应
        response_rx.await
            .map_err(|_| QuickDbError::ConnectionError {
                message: "等待连接池响应超时".to_string(),
            })?
    }
    
    /// 处理查询请求
    async fn handle_find(
        collection: &str,
        conditions: Vec<QueryCondition>,
        options: Option<QueryOptions>,
        alias: Option<String>,
    ) -> QuickDbResult<Vec<DataValue>> {
        let manager = get_global_pool_manager();
        let actual_alias = match alias {
            Some(a) => a,
            None => {
                manager.get_default_alias().await
                    .unwrap_or_else(|| "default".to_string())
            }
        };
        debug!("处理查询请求: collection={}, alias={}", collection, actual_alias);
        
        let manager = get_global_pool_manager();
        let connection_pools = manager.get_connection_pools();
        let connection_pool = connection_pools.get(&actual_alias)
            .ok_or_else(|| QuickDbError::AliasNotFound {
                alias: actual_alias.clone(),
            })?;
        
        // 创建oneshot通道用于接收响应
        let (response_tx, response_rx) = oneshot::channel();
        
        // 发送DatabaseOperation::Find请求到连接池
        let operation = DatabaseOperation::Find {
            table: collection.to_string(),
            conditions,
            options: options.unwrap_or_default(),
            response: response_tx,
        };
        
        connection_pool.operation_sender.send(operation)
            .map_err(|_| QuickDbError::ConnectionError {
                message: "连接池操作通道已关闭".to_string(),
            })?;
        
        // 等待响应
        response_rx.await
            .map_err(|_| QuickDbError::ConnectionError {
                message: "等待连接池响应超时".to_string(),
            })?
    }
    
    /// 处理分组查询请求
    async fn handle_find_with_groups(
        collection: &str,
        condition_groups: Vec<QueryConditionGroup>,
        options: Option<QueryOptions>,
        alias: Option<String>,
    ) -> QuickDbResult<Vec<DataValue>> {
        let manager = get_global_pool_manager();
        let actual_alias = match alias {
            Some(a) => a,
            None => {
                manager.get_default_alias().await
                    .unwrap_or_else(|| "default".to_string())
            }
        };
        debug!("处理分组查询请求: collection={}, alias={}", collection, actual_alias);
        
        let manager = get_global_pool_manager();
        let connection_pools = manager.get_connection_pools();
        let connection_pool = connection_pools.get(&actual_alias)
            .ok_or_else(|| QuickDbError::AliasNotFound {
                alias: actual_alias.clone(),
            })?;
        
        // 创建oneshot通道用于接收响应
        let (response_tx, response_rx) = oneshot::channel();
        
        // 发送DatabaseOperation::FindWithGroups请求到连接池
        let operation = DatabaseOperation::FindWithGroups {
            table: collection.to_string(),
            condition_groups,
            options: options.unwrap_or_default(),
            response: response_tx,
        };
        
        connection_pool.operation_sender.send(operation)
            .map_err(|_| QuickDbError::ConnectionError {
                message: "连接池操作通道已关闭".to_string(),
            })?;
        
        // 等待响应
        response_rx.await
            .map_err(|_| QuickDbError::ConnectionError {
                message: "等待连接池响应超时".to_string(),
            })?
    }
    
    /// 处理更新请求
    async fn handle_update(
        collection: &str,
        conditions: Vec<QueryCondition>,
        updates: HashMap<String, DataValue>,
        alias: Option<String>,
    ) -> QuickDbResult<u64> {
        let manager = get_global_pool_manager();
        let actual_alias = match alias {
            Some(a) => a,
            None => {
                manager.get_default_alias().await
                    .unwrap_or_else(|| "default".to_string())
            }
        };
        debug!("处理更新请求: collection={}, alias={}", collection, actual_alias);
        
        let manager = get_global_pool_manager();
        let connection_pools = manager.get_connection_pools();
        let connection_pool = connection_pools.get(&actual_alias)
            .ok_or_else(|| QuickDbError::AliasNotFound {
                alias: actual_alias.clone(),
            })?;
        
        // 创建oneshot通道用于接收响应
        let (response_tx, response_rx) = oneshot::channel();
        
        // 发送DatabaseOperation::Update请求到连接池
        let operation = DatabaseOperation::Update {
            table: collection.to_string(),
            conditions,
            data: updates,
            response: response_tx,
        };
        
        connection_pool.operation_sender.send(operation)
            .map_err(|_| QuickDbError::ConnectionError {
                message: "连接池操作通道已关闭".to_string(),
            })?;
        
        // 等待响应
        let affected_rows = response_rx.await
            .map_err(|_| QuickDbError::ConnectionError {
                message: "等待连接池响应超时".to_string(),
            })??;
        
        Ok(affected_rows)
    }

    /// 处理使用操作数组更新请求
    async fn handle_update_with_operations(
        collection: &str,
        conditions: Vec<QueryCondition>,
        operations: Vec<crate::types::UpdateOperation>,
        alias: Option<String>,
    ) -> QuickDbResult<u64> {
        let manager = get_global_pool_manager();
        let actual_alias = match alias {
            Some(a) => a,
            None => {
                manager.get_default_alias().await
                    .unwrap_or_else(|| "default".to_string())
            }
        };
        debug!("处理操作更新请求: collection={}, alias={}", collection, actual_alias);

        let manager = get_global_pool_manager();
        let connection_pools = manager.get_connection_pools();
        let connection_pool = connection_pools.get(&actual_alias)
            .ok_or_else(|| QuickDbError::AliasNotFound {
                alias: actual_alias.clone(),
            })?;

        // 创建oneshot通道用于接收响应
        let (response_tx, response_rx) = oneshot::channel();

        // 发送DatabaseOperation::UpdateWithOperations请求到连接池
        let operation = DatabaseOperation::UpdateWithOperations {
            table: collection.to_string(),
            conditions,
            operations,
            response: response_tx,
        };

        connection_pool.operation_sender.send(operation)
            .map_err(|_| QuickDbError::ConnectionError {
                message: "连接池操作通道已关闭".to_string(),
            })?;

        // 等待响应
        let affected_rows = response_rx.await
            .map_err(|_| QuickDbError::ConnectionError {
                message: "等待连接池响应超时".to_string(),
            })??;

        Ok(affected_rows)
    }

    /// 处理根据ID更新请求
    async fn handle_update_by_id(
        collection: &str,
        id: &str,
        updates: HashMap<String, DataValue>,
        alias: Option<String>,
    ) -> QuickDbResult<bool> {
        let manager = get_global_pool_manager();
        let actual_alias = match alias {
            Some(a) => a,
            None => {
                manager.get_default_alias().await
                    .unwrap_or_else(|| "default".to_string())
            }
        };
        debug!("处理根据ID更新请求: collection={}, id={}, alias={}", collection, id, actual_alias);
        
        let manager = get_global_pool_manager();
        let connection_pools = manager.get_connection_pools();
        let connection_pool = connection_pools.get(&actual_alias)
            .ok_or_else(|| QuickDbError::AliasNotFound {
                alias: actual_alias.clone(),
            })?;
        
        // 创建oneshot通道用于接收响应
        let (response_tx, response_rx) = oneshot::channel();
        
        // 发送DatabaseOperation::UpdateById请求到连接池
        let operation = DatabaseOperation::UpdateById {
            table: collection.to_string(),
            id: DataValue::String(id.to_string()),
            data: updates,
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
        
        Ok(result)
    }
    
    /// 处理删除请求
    async fn handle_delete(
        collection: &str,
        conditions: Vec<QueryCondition>,
        alias: Option<String>,
    ) -> QuickDbResult<u64> {
        let manager = get_global_pool_manager();
        let actual_alias = match alias {
            Some(a) => a,
            None => {
                manager.get_default_alias().await
                    .unwrap_or_else(|| "default".to_string())
            }
        };
        debug!("处理删除请求: collection={}, alias={}", collection, actual_alias);
        
        let manager = get_global_pool_manager();
        let connection_pools = manager.get_connection_pools();
        let connection_pool = connection_pools.get(&actual_alias)
            .ok_or_else(|| QuickDbError::AliasNotFound {
                alias: actual_alias.clone(),
            })?;
        
        // 创建oneshot通道用于接收响应
        let (response_tx, response_rx) = oneshot::channel();
        
        // 发送DatabaseOperation::Delete请求到连接池
        let operation = DatabaseOperation::Delete {
            table: collection.to_string(),
            conditions,
            response: response_tx,
        };
        
        connection_pool.operation_sender.send(operation)
            .map_err(|_| QuickDbError::ConnectionError {
                message: "连接池操作通道已关闭".to_string(),
            })?;
        
        // 等待响应
        let affected_rows = response_rx.await
            .map_err(|_| QuickDbError::ConnectionError {
                message: "等待连接池响应超时".to_string(),
            })??;
        
        Ok(affected_rows)
    }
    
    /// 处理根据ID删除请求
    async fn handle_delete_by_id(
        collection: &str,
        id: &str,
        alias: Option<String>,
    ) -> QuickDbResult<bool> {
        let manager = get_global_pool_manager();
        let actual_alias = match alias {
            Some(a) => a,
            None => {
                manager.get_default_alias().await
                    .unwrap_or_else(|| "default".to_string())
            }
        };
        debug!("处理根据ID删除请求: collection={}, id={}, alias={}", collection, id, actual_alias);
        
        let manager = get_global_pool_manager();
        let connection_pools = manager.get_connection_pools();
        let connection_pool = connection_pools.get(&actual_alias)
            .ok_or_else(|| QuickDbError::AliasNotFound {
                alias: actual_alias.clone(),
            })?;
        
        // 创建oneshot通道用于接收响应
        let (response_tx, response_rx) = oneshot::channel();
        
        // 发送DatabaseOperation::DeleteById请求到连接池
        let operation = DatabaseOperation::DeleteById {
            table: collection.to_string(),
            id: DataValue::String(id.to_string()),
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
        
        Ok(result)
    }
    
    /// 处理计数请求
    async fn handle_count(
        collection: &str,
        conditions: Vec<QueryCondition>,
        alias: Option<String>,
    ) -> QuickDbResult<u64> {
        let manager = get_global_pool_manager();
        let actual_alias = match alias {
            Some(a) => a,
            None => {
                manager.get_default_alias().await
                    .unwrap_or_else(|| "default".to_string())
            }
        };
        debug!("处理计数请求: collection={}, alias={}", collection, actual_alias);
        
        let manager = get_global_pool_manager();
        let connection_pools = manager.get_connection_pools();
        let connection_pool = connection_pools.get(&actual_alias)
            .ok_or_else(|| QuickDbError::AliasNotFound {
                alias: actual_alias.clone(),
            })?;
        
        // 创建oneshot通道用于接收响应
        let (response_tx, response_rx) = oneshot::channel();
        
        // 发送DatabaseOperation::Count请求到连接池
        let operation = DatabaseOperation::Count {
            table: collection.to_string(),
            conditions,
            response: response_tx,
        };
        
        connection_pool.operation_sender.send(operation)
            .map_err(|_| QuickDbError::ConnectionError {
                message: "连接池操作通道已关闭".to_string(),
            })?;
        
        // 等待响应
        let count = response_rx.await
            .map_err(|_| QuickDbError::ConnectionError {
                message: "等待连接池响应超时".to_string(),
            })??;
        
        Ok(count)
    }
    
    /// 处理存在性检查请求
    async fn handle_exists(
        collection: &str,
        conditions: Vec<QueryCondition>,
        alias: Option<String>,
    ) -> QuickDbResult<bool> {
        let manager = get_global_pool_manager();
        let actual_alias = match alias {
            Some(a) => a,
            None => {
                manager.get_default_alias().await
                    .unwrap_or_else(|| "default".to_string())
            }
        };
        debug!("处理存在性检查请求: collection={}, alias={}", collection, actual_alias);
        
        let manager = get_global_pool_manager();
        let connection_pools = manager.get_connection_pools();
        let connection_pool = connection_pools.get(&actual_alias)
            .ok_or_else(|| QuickDbError::AliasNotFound {
                alias: actual_alias.clone(),
            })?;
        
        // 使用生产者/消费者模式发送操作到连接池
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();
        let operation = DatabaseOperation::Exists {
            table: collection.to_string(),
            conditions,
            response: response_tx,
        };

        connection_pool.operation_sender.send(operation)
            .map_err(|_| QuickDbError::ConnectionError {
                message: "连接池操作通道已关闭".to_string(),
            })?;

        let result = response_rx.await
            .map_err(|_| QuickDbError::ConnectionError {
                message: "等待数据库操作结果超时".to_string(),
            })??;

        Ok(result)
    }

    async fn handle_get_server_version(alias: Option<String>) -> QuickDbResult<String> {
        let manager = get_global_pool_manager();
        let actual_alias = match alias {
            Some(a) => a,
            None => {
                manager.get_default_alias().await
                    .unwrap_or_else(|| "default".to_string())
            }
        };
        debug!("处理版本查询请求: alias={}", actual_alias);

        let manager = get_global_pool_manager();
        let connection_pools = manager.get_connection_pools();
        let connection_pool = connection_pools.get(&actual_alias)
            .ok_or_else(|| QuickDbError::AliasNotFound {
                alias: actual_alias.clone(),
            })?;

        // 使用生产者/消费者模式发送操作到连接池
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();
        let operation = DatabaseOperation::GetServerVersion {
            response: response_tx,
        };

        connection_pool.operation_sender.send(operation)
            .map_err(|_| QuickDbError::ConnectionError {
                message: "连接池操作通道已关闭".to_string(),
            })?;

        let result = response_rx.await
            .map_err(|_| QuickDbError::ConnectionError {
                message: "等待数据库操作结果超时".to_string(),
            })??;

        Ok(result)
    }
}

// 实现Drop trait以确保资源正确清理
impl Drop for AsyncOdmManager {
    fn drop(&mut self) {
        info!("开始清理AsyncOdmManager资源");
        
        // 关闭请求发送器，这会导致后台任务自然退出
        // 注意：这里不需要显式关闭sender，因为当所有sender被drop时，
        // receiver会自动关闭，导致process_requests循环退出
        
        // 如果有任务句柄，尝试取消任务
        if let Some(handle) = self._task_handle.take() {
            if !handle.is_finished() {
                warn!("ODM后台任务仍在运行，将被取消");
                handle.abort();
            } else {
                info!("ODM后台任务已正常结束");
            }
        }
        
        info!("AsyncOdmManager资源清理完成");
    }
}

/// 异步ODM操作接口实现
#[async_trait]
impl OdmOperations for AsyncOdmManager {
    async fn create(
        &self,
        collection: &str,
        data: HashMap<String, DataValue>,
        alias: Option<&str>,
    ) -> QuickDbResult<DataValue> {
        let (sender, receiver) = oneshot::channel();
        
        let request = OdmRequest::Create {
            collection: collection.to_string(),
            data,
            alias: alias.map(|s| s.to_string()),
            response: sender,
        };
        
        self.request_sender.send(request)
            .map_err(|_| QuickDbError::ConnectionError {
                message: "ODM后台任务已停止".to_string(),
            })?;
        
        receiver.await
            .map_err(|_| QuickDbError::ConnectionError {
                message: "ODM请求处理失败".to_string(),
            })?
    }
    
    async fn find_by_id(
        &self,
        collection: &str,
        id: &str,
        alias: Option<&str>,
    ) -> QuickDbResult<Option<DataValue>> {
        let (sender, receiver) = oneshot::channel();
        
        let request = OdmRequest::FindById {
            collection: collection.to_string(),
            id: id.to_string(),
            alias: alias.map(|s| s.to_string()),
            response: sender,
        };
        
        self.request_sender.send(request)
            .map_err(|_| QuickDbError::ConnectionError {
                message: "ODM后台任务已停止".to_string(),
            })?;
        
        receiver.await
            .map_err(|_| QuickDbError::ConnectionError {
                message: "ODM请求处理失败".to_string(),
            })?
    }
    
    async fn find(
        &self,
        collection: &str,
        conditions: Vec<QueryCondition>,
        options: Option<QueryOptions>,
        alias: Option<&str>,
    ) -> QuickDbResult<Vec<DataValue>> {
        let (sender, receiver) = oneshot::channel();
        
        let request = OdmRequest::Find {
            collection: collection.to_string(),
            conditions,
            options,
            alias: alias.map(|s| s.to_string()),
            response: sender,
        };
        
        self.request_sender.send(request)
            .map_err(|_| QuickDbError::ConnectionError {
                message: "ODM后台任务已停止".to_string(),
            })?;
        
        receiver.await
            .map_err(|_| QuickDbError::ConnectionError {
                message: "ODM请求处理失败".to_string(),
            })?
    }
    
    async fn find_with_groups(
        &self,
        collection: &str,
        condition_groups: Vec<QueryConditionGroup>,
        options: Option<QueryOptions>,
        alias: Option<&str>,
    ) -> QuickDbResult<Vec<DataValue>> {
        let (sender, receiver) = oneshot::channel();
        
        let request = OdmRequest::FindWithGroups {
            collection: collection.to_string(),
            condition_groups,
            options,
            alias: alias.map(|s| s.to_string()),
            response: sender,
        };
        
        self.request_sender.send(request)
            .map_err(|_| QuickDbError::ConnectionError {
                message: "ODM后台任务已停止".to_string(),
            })?;
        
        receiver.await
            .map_err(|_| QuickDbError::ConnectionError {
                message: "ODM请求处理失败".to_string(),
            })?
    }
    
    async fn update(
        &self,
        collection: &str,
        conditions: Vec<QueryCondition>,
        updates: HashMap<String, DataValue>,
        alias: Option<&str>,
    ) -> QuickDbResult<u64> {
        let (sender, receiver) = oneshot::channel();
        
        let request = OdmRequest::Update {
            collection: collection.to_string(),
            conditions,
            updates,
            alias: alias.map(|s| s.to_string()),
            response: sender,
        };
        
        self.request_sender.send(request)
            .map_err(|_| QuickDbError::ConnectionError {
                message: "ODM后台任务已停止".to_string(),
            })?;
        
        receiver.await
            .map_err(|_| QuickDbError::ConnectionError {
                message: "ODM请求处理失败".to_string(),
            })?
    }

    async fn update_with_operations(
        &self,
        collection: &str,
        conditions: Vec<QueryCondition>,
        operations: Vec<crate::types::UpdateOperation>,
        alias: Option<&str>,
    ) -> QuickDbResult<u64> {
        let (sender, receiver) = oneshot::channel();

        let request = OdmRequest::UpdateWithOperations {
            collection: collection.to_string(),
            conditions,
            operations,
            alias: alias.map(|s| s.to_string()),
            response: sender,
        };

        self.request_sender.send(request)
            .map_err(|_| QuickDbError::ConnectionError {
                message: "ODM后台任务已停止".to_string(),
            })?;

        receiver.await
            .map_err(|_| QuickDbError::ConnectionError {
                message: "ODM请求处理失败".to_string(),
            })?
    }

    async fn update_by_id(
        &self,
        collection: &str,
        id: &str,
        updates: HashMap<String, DataValue>,
        alias: Option<&str>,
    ) -> QuickDbResult<bool> {
        let (sender, receiver) = oneshot::channel();
        
        let request = OdmRequest::UpdateById {
            collection: collection.to_string(),
            id: id.to_string(),
            updates,
            alias: alias.map(|s| s.to_string()),
            response: sender,
        };
        
        self.request_sender.send(request)
            .map_err(|_| QuickDbError::ConnectionError {
                message: "ODM后台任务已停止".to_string(),
            })?;
        
        receiver.await
            .map_err(|_| QuickDbError::ConnectionError {
                message: "ODM请求处理失败".to_string(),
            })?
    }
    
    async fn delete(
        &self,
        collection: &str,
        conditions: Vec<QueryCondition>,
        alias: Option<&str>,
    ) -> QuickDbResult<u64> {
        let (sender, receiver) = oneshot::channel();
        
        let request = OdmRequest::Delete {
            collection: collection.to_string(),
            conditions,
            alias: alias.map(|s| s.to_string()),
            response: sender,
        };
        
        self.request_sender.send(request)
            .map_err(|_| QuickDbError::ConnectionError {
                message: "ODM后台任务已停止".to_string(),
            })?;
        
        receiver.await
            .map_err(|_| QuickDbError::ConnectionError {
                message: "ODM请求处理失败".to_string(),
            })?
    }
    
    async fn delete_by_id(
        &self,
        collection: &str,
        id: &str,
        alias: Option<&str>,
    ) -> QuickDbResult<bool> {
        let (sender, receiver) = oneshot::channel();
        
        let request = OdmRequest::DeleteById {
            collection: collection.to_string(),
            id: id.to_string(),
            alias: alias.map(|s| s.to_string()),
            response: sender,
        };
        
        self.request_sender.send(request)
            .map_err(|_| QuickDbError::ConnectionError {
                message: "ODM后台任务已停止".to_string(),
            })?;
        
        receiver.await
            .map_err(|_| QuickDbError::ConnectionError {
                message: "ODM请求处理失败".to_string(),
            })?
    }
    
    async fn count(
        &self,
        collection: &str,
        conditions: Vec<QueryCondition>,
        alias: Option<&str>,
    ) -> QuickDbResult<u64> {
        let (sender, receiver) = oneshot::channel();
        
        let request = OdmRequest::Count {
            collection: collection.to_string(),
            conditions,
            alias: alias.map(|s| s.to_string()),
            response: sender,
        };
        
        self.request_sender.send(request)
            .map_err(|_| QuickDbError::ConnectionError {
                message: "ODM后台任务已停止".to_string(),
            })?;
        
        receiver.await
            .map_err(|_| QuickDbError::ConnectionError {
                message: "ODM请求处理失败".to_string(),
            })?
    }
    
    async fn exists(
        &self,
        collection: &str,
        conditions: Vec<QueryCondition>,
        alias: Option<&str>,
    ) -> QuickDbResult<bool> {
        let (sender, receiver) = oneshot::channel();
        
        let request = OdmRequest::Exists {
            collection: collection.to_string(),
            conditions,
            alias: alias.map(|s| s.to_string()),
            response: sender,
        };
        
        self.request_sender.send(request)
            .map_err(|_| QuickDbError::ConnectionError {
                message: "ODM后台任务已停止".to_string(),
            })?;
        
        receiver.await
            .map_err(|_| QuickDbError::ConnectionError {
                message: "ODM请求处理失败".to_string(),
            })?
    }

    async fn get_server_version(
        &self,
        alias: Option<&str>,
    ) -> QuickDbResult<String> {
        let (sender, receiver) = oneshot::channel();

        let request = OdmRequest::GetServerVersion {
            alias: alias.map(|s| s.to_string()),
            response: sender,
        };

        self.request_sender.send(request)
            .map_err(|_| QuickDbError::ConnectionError {
                message: "ODM后台任务已停止".to_string(),
            })?;

        receiver.await
            .map_err(|_| QuickDbError::ConnectionError {
                message: "ODM请求处理失败".to_string(),
            })?
    }
}

/// 全局异步ODM管理器实例
static ASYNC_ODM_MANAGER: once_cell::sync::Lazy<tokio::sync::RwLock<AsyncOdmManager>> = 
    once_cell::sync::Lazy::new(|| {
        tokio::sync::RwLock::new(AsyncOdmManager::new())
    });

/// 获取全局ODM管理器
pub async fn get_odm_manager() -> tokio::sync::RwLockReadGuard<'static, AsyncOdmManager> {
    ASYNC_ODM_MANAGER.read().await
}

/// 获取可变的全局ODM管理器
pub async fn get_odm_manager_mut() -> tokio::sync::RwLockWriteGuard<'static, AsyncOdmManager> {
    ASYNC_ODM_MANAGER.write().await
}

/// 便捷函数：创建记录
///
/// 【注意】这是一个内部函数，建议通过ModelManager或模型的save方法进行操作
/// 除非您明确知道自己在做什么，否则不要直接调用此函数
#[doc(hidden)]
pub async fn create(
    collection: &str,
    data: HashMap<String, DataValue>,
    alias: Option<&str>,
) -> QuickDbResult<DataValue> {
    let manager = get_odm_manager().await;
    manager.create(collection, data, alias).await
}


/// 便捷函数：根据ID查询记录
///
/// 【注意】这是一个内部函数，建议通过ModelManager或模型的find_by_id方法进行操作
/// 除非您明确知道自己在做什么，否则不要直接调用此函数
#[doc(hidden)]
pub async fn find_by_id(
    collection: &str,
    id: &str,
    alias: Option<&str>,
) -> QuickDbResult<Option<DataValue>> {
    let manager = get_odm_manager().await;
    manager.find_by_id(collection, id, alias).await
}

/// 便捷函数：查询记录
///
/// 【注意】这是一个内部函数，建议通过ModelManager或模型的find方法进行操作
/// 除非您明确知道自己在做什么，否则不要直接调用此函数
#[doc(hidden)]
pub async fn find(
    collection: &str,
    conditions: Vec<QueryCondition>,
    options: Option<QueryOptions>,
    alias: Option<&str>,
) -> QuickDbResult<Vec<DataValue>> {
    let manager = get_odm_manager().await;
    manager.find(collection, conditions, options, alias).await
}

/// 分组查询便捷函数
///
/// 【注意】这是一个内部函数，建议通过ModelManager或模型的find_with_groups方法进行操作
/// 除非您明确知道自己在做什么，否则不要直接调用此函数
#[doc(hidden)]
pub async fn find_with_groups(
    collection: &str,
    condition_groups: Vec<QueryConditionGroup>,
    options: Option<QueryOptions>,
    alias: Option<&str>,
) -> QuickDbResult<Vec<DataValue>> {
    let manager = get_odm_manager().await;
    manager.find_with_groups(collection, condition_groups, options, alias).await
}

/// 便捷函数：更新记录
///
/// 【注意】这是一个内部函数，建议通过ModelManager或模型的update方法进行操作
/// 除非您明确知道自己在做什么，否则不要直接调用此函数
#[doc(hidden)]
pub async fn update(
    collection: &str,
    conditions: Vec<QueryCondition>,
    updates: HashMap<String, DataValue>,
    alias: Option<&str>,
) -> QuickDbResult<u64> {
    let manager = get_odm_manager().await;
    manager.update(collection, conditions, updates, alias).await
}

/// 便捷函数：根据ID更新记录
///
/// 【注意】这是一个内部函数，建议通过ModelManager或模型的update方法进行操作
/// 除非您明确知道自己在做什么，否则不要直接调用此函数
#[doc(hidden)]
pub async fn update_by_id(
    collection: &str,
    id: &str,
    updates: HashMap<String, DataValue>,
    alias: Option<&str>,
) -> QuickDbResult<bool> {
    let manager = get_odm_manager().await;
    manager.update_by_id(collection, id, updates, alias).await
}

/// 便捷函数：使用操作数组更新记录
///
/// 【注意】这是一个内部函数，建议通过ModelManager或模型的update_many_with_operations方法进行操作
/// 除非您明确知道自己在做什么，否则不要直接调用此函数
#[doc(hidden)]
pub async fn update_with_operations(
    collection: &str,
    conditions: Vec<QueryCondition>,
    operations: Vec<crate::types::UpdateOperation>,
    alias: Option<&str>,
) -> QuickDbResult<u64> {
    let manager = get_odm_manager().await;
    manager.update_with_operations(collection, conditions, operations, alias).await
}

/// 便捷函数：删除记录
///
/// 【注意】这是一个内部函数，建议通过ModelManager或模型的delete方法进行操作
/// 除非您明确知道自己在做什么，否则不要直接调用此函数
#[doc(hidden)]
pub async fn delete(
    collection: &str,
    conditions: Vec<QueryCondition>,
    alias: Option<&str>,
) -> QuickDbResult<u64> {
    let manager = get_odm_manager().await;
    manager.delete(collection, conditions, alias).await
}

/// 便捷函数：根据ID删除记录
///
/// 【注意】这是一个内部函数，建议通过ModelManager或模型的delete方法进行操作
/// 除非您明确知道自己在做什么，否则不要直接调用此函数
#[doc(hidden)]
pub async fn delete_by_id(
    collection: &str,
    id: &str,
    alias: Option<&str>,
) -> QuickDbResult<bool> {
    let manager = get_odm_manager().await;
    manager.delete_by_id(collection, id, alias).await
}

/// 便捷函数：统计记录数量
///
/// 【注意】这是一个内部函数，建议通过ModelManager或模型的count方法进行操作
/// 除非您明确知道自己在做什么，否则不要直接调用此函数
#[doc(hidden)]
pub async fn count(
    collection: &str,
    conditions: Vec<QueryCondition>,
    alias: Option<&str>,
) -> QuickDbResult<u64> {
    let manager = get_odm_manager().await;
    manager.count(collection, conditions, alias).await
}

/// 便捷函数：检查记录是否存在
///
/// 【注意】这是一个内部函数，建议通过ModelManager或模型的exists方法进行操作
/// 除非您明确知道自己在做什么，否则不要直接调用此函数
#[doc(hidden)]
pub async fn exists(
    collection: &str,
    conditions: Vec<QueryCondition>,
    alias: Option<&str>,
) -> QuickDbResult<bool> {
    let manager = get_odm_manager().await;
    manager.exists(collection, conditions, alias).await
}

/// 获取数据库服务器版本信息
pub async fn get_server_version(alias: Option<&str>) -> QuickDbResult<String> {
    let manager = get_odm_manager().await;
    manager.get_server_version(alias).await
}