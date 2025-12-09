
//! # 读取操作处理器

use crate::error::{QuickDbError, QuickDbResult};
use crate::manager::get_global_pool_manager;
use crate::odm::manager_core::AsyncOdmManager;
use crate::pool::DatabaseOperation;
use crate::types::*;
use rat_logger::{debug, info, warn};
use tokio::sync::oneshot;

impl AsyncOdmManager {
    /// 处理根据ID查询请求
    #[doc(hidden)]
    pub async fn handle_find_by_id(
        collection: &str,
        id: &str,
        alias: Option<String>,
    ) -> QuickDbResult<Option<DataValue>> {
        let manager = get_global_pool_manager();
        let actual_alias = match alias {
            Some(a) => a,
            None => manager
                .get_default_alias()
                .await
                .unwrap_or_else(|| "default".to_string()),
        };
        debug!(
            "处理根据ID查询请求: collection={}, id={}, alias={}",
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

        // 发送DatabaseOperation::FindById请求到连接池
        let operation = DatabaseOperation::FindById {
            table: collection.to_string(),
            id: DataValue::String(id.to_string()),
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
        response_rx
            .await
            .map_err(|_| QuickDbError::ConnectionError {
                message: "等待连接池响应超时".to_string(),
            })?
    }

    /// 处理查询请求（支持缓存控制）
    #[doc(hidden)]
    pub async fn handle_find_with_cache_control(
        collection: &str,
        conditions: Vec<QueryCondition>,
        options: Option<QueryOptions>,
        alias: Option<String>,
        bypass_cache: bool,
    ) -> QuickDbResult<Vec<DataValue>> {
        let manager = get_global_pool_manager();
        let actual_alias = match alias {
            Some(a) => a,
            None => manager
                .get_default_alias()
                .await
                .unwrap_or_else(|| "default".to_string()),
        };
        debug!(
            "处理查询请求（bypass_cache={}）: collection={}, alias={}",
            bypass_cache, collection, actual_alias
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

        // 发送DatabaseOperation::FindWithBypassCache请求到连接池
        let operation = DatabaseOperation::FindWithBypassCache {
            table: collection.to_string(),
            conditions,
            options: options.unwrap_or_default(),
            alias: actual_alias.clone(),
            bypass_cache,
            response: response_tx,
        };

        connection_pool
            .operation_sender
            .send(operation)
            .map_err(|_| QuickDbError::ConnectionError {
                message: "连接池操作通道已关闭".to_string(),
            })?;

        // 等待响应
        response_rx
            .await
            .map_err(|_| QuickDbError::ConnectionError {
                message: "等待连接池响应超时".to_string(),
            })?
    }

    /// 处理查询请求
    #[doc(hidden)]
    pub async fn handle_find(
        collection: &str,
        conditions: Vec<QueryCondition>,
        options: Option<QueryOptions>,
        alias: Option<String>,
    ) -> QuickDbResult<Vec<DataValue>> {
        Self::handle_find_with_cache_control(collection, conditions, options, alias, false).await
    }

    /// 处理分组查询请求（支持缓存控制）
    #[doc(hidden)]
    pub async fn handle_find_with_groups_with_cache_control(
        collection: &str,
        condition_groups: Vec<QueryConditionGroup>,
        options: Option<QueryOptions>,
        alias: Option<String>,
        bypass_cache: bool,
    ) -> QuickDbResult<Vec<DataValue>> {
        let manager = get_global_pool_manager();
        let actual_alias = match alias {
            Some(a) => a,
            None => manager
                .get_default_alias()
                .await
                .unwrap_or_else(|| "default".to_string()),
        };
        debug!(
            "处理分组查询请求（bypass_cache={}）: collection={}, alias={}",
            bypass_cache, collection, actual_alias
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

        // 发送DatabaseOperation::FindWithGroupsWithBypassCache请求到连接池
        let operation = DatabaseOperation::FindWithGroupsWithBypassCache {
            table: collection.to_string(),
            condition_groups,
            options: options.unwrap_or_default(),
            alias: actual_alias.clone(),
            bypass_cache,
            response: response_tx,
        };

        connection_pool
            .operation_sender
            .send(operation)
            .map_err(|_| QuickDbError::ConnectionError {
                message: "连接池操作通道已关闭".to_string(),
            })?;

        // 等待响应
        response_rx
            .await
            .map_err(|_| QuickDbError::ConnectionError {
                message: "等待连接池响应超时".to_string(),
            })?
    }

    /// 处理分组查询请求
    #[doc(hidden)]
    pub async fn handle_find_with_groups(
        collection: &str,
        condition_groups: Vec<QueryConditionGroup>,
        options: Option<QueryOptions>,
        alias: Option<String>,
    ) -> QuickDbResult<Vec<DataValue>> {
        Self::handle_find_with_groups_with_cache_control(collection, condition_groups, options, alias, false).await
    }
}
