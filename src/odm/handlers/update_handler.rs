
//! # 更新操作处理器

use crate::error::{QuickDbError, QuickDbResult};
use crate::manager::get_global_pool_manager;
use crate::odm::manager_core::AsyncOdmManager;
use crate::pool::DatabaseOperation;
use crate::types::*;
use rat_logger::{debug, info, warn};
use std::collections::HashMap;
use tokio::sync::oneshot;

impl AsyncOdmManager {
    /// 处理更新请求
    #[doc(hidden)]
    pub async fn handle_update(
        collection: &str,
        conditions: Vec<QueryCondition>,
        updates: HashMap<String, DataValue>,
        alias: Option<String>,
    ) -> QuickDbResult<u64> {
        let manager = get_global_pool_manager();
        let actual_alias = match alias {
            Some(a) => a,
            None => manager
                .get_default_alias()
                .await
                .unwrap_or_else(|| "default".to_string()),
        };
        debug!(
            "处理更新请求: collection={}, alias={}",
            collection, actual_alias
        );

        let manager = get_global_pool_manager();
        let connection_pools = manager.get_connection_pools();
        let connection_pool =
            connection_pools
                .get(&actual_alias)
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
            alias: actual_alias.clone(),
            response: response_tx,
        };

        connection_pool
            .operation_sender
            .send(operation)
            .map_err(|_| QuickDbError::ConnectionError {
                message: "连接池操作通道已关闭".to_string(),
            })?;

        // 等待响应
        let affected_rows = response_rx
            .await
            .map_err(|_| QuickDbError::ConnectionError {
                message: "等待连接池响应超时".to_string(),
            })??;

        Ok(affected_rows)
    }

    /// 处理使用操作数组更新请求
    #[doc(hidden)]
    pub async fn handle_update_with_operations(
        collection: &str,
        conditions: Vec<QueryCondition>,
        operations: Vec<crate::types::UpdateOperation>,
        alias: Option<String>,
    ) -> QuickDbResult<u64> {
        let manager = get_global_pool_manager();
        let actual_alias = match alias {
            Some(a) => a,
            None => manager
                .get_default_alias()
                .await
                .unwrap_or_else(|| "default".to_string()),
        };
        debug!(
            "处理操作更新请求: collection={}, alias={}",
            collection, actual_alias
        );

        let manager = get_global_pool_manager();
        let connection_pools = manager.get_connection_pools();
        let connection_pool =
            connection_pools
                .get(&actual_alias)
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
            alias: actual_alias.clone(),
            response: response_tx,
        };

        connection_pool
            .operation_sender
            .send(operation)
            .map_err(|_| QuickDbError::ConnectionError {
                message: "连接池操作通道已关闭".to_string(),
            })?;

        // 等待响应
        let affected_rows = response_rx
            .await
            .map_err(|_| QuickDbError::ConnectionError {
                message: "等待连接池响应超时".to_string(),
            })??;

        Ok(affected_rows)
    }

    /// 处理根据ID更新请求
    #[doc(hidden)]
    pub async fn handle_update_by_id(
        collection: &str,
        id: &str,
        updates: HashMap<String, DataValue>,
        alias: Option<String>,
    ) -> QuickDbResult<bool> {
        let manager = get_global_pool_manager();
        let actual_alias = match alias {
            Some(a) => a,
            None => manager
                .get_default_alias()
                .await
                .unwrap_or_else(|| "default".to_string()),
        };
        debug!(
            "处理根据ID更新请求: collection={}, id={}, alias={}",
            collection, id, actual_alias
        );

        let manager = get_global_pool_manager();
        let connection_pools = manager.get_connection_pools();
        let connection_pool =
            connection_pools
                .get(&actual_alias)
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
            alias: actual_alias.clone(),
            response: response_tx,
        };

        connection_pool
            .operation_sender
            .send(operation)
            .map_err(|_| QuickDbError::ConnectionError {
                message: "连接池操作通道已关闭".to_string(),
            })?;

        // 等待响应
        let result = response_rx
            .await
            .map_err(|_| QuickDbError::ConnectionError {
                message: "等待连接池响应超时".to_string(),
            })??;

        Ok(result)
    }
}
