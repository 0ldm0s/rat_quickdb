    //! # 删除操作处理器

use crate::error::{QuickDbError, QuickDbResult};
use crate::types::*;
use crate::manager::get_global_pool_manager;
use crate::odm::manager_core::AsyncOdmManager;
use crate::pool::DatabaseOperation;
use rat_logger::{debug, info, warn};
use tokio::sync::oneshot;

impl AsyncOdmManager {
    /// 处理删除请求
    #[doc(hidden)]
    pub async fn handle_delete(
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
            alias: actual_alias.clone(),
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
    #[doc(hidden)]
    pub async fn handle_delete_by_id(
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
            alias: actual_alias.clone(),
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
    #[doc(hidden)]
    pub async fn handle_count(
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
            alias: actual_alias.clone(),
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
    
    
    /// 处理获取服务器版本请求
    #[doc(hidden)]
    pub async fn handle_get_server_version(alias: Option<String>) -> QuickDbResult<String> {
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
