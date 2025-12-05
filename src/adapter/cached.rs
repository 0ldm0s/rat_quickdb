//! 缓存数据库适配器
//!
//! 提供带缓存功能的数据库适配器包装器，在适配器层实现缓存逻辑

use super::DatabaseAdapter;
use crate::cache::CacheManager;
use crate::error::{QuickDbError, QuickDbResult};
use crate::model::{FieldDefinition, FieldType};
use crate::pool::DatabaseConnection;
use crate::types::*;
use async_trait::async_trait;
use rat_logger::{debug, warn};
use serde_json::Value;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;

/// 带缓存功能的数据库适配器包装器
pub struct CachedDatabaseAdapter {
    /// 内部真实的数据库适配器
    inner: Box<dyn DatabaseAdapter>,
    /// 缓存管理器
    cache_manager: Arc<CacheManager>,
}

impl CachedDatabaseAdapter {
    /// 创建新的缓存适配器
    pub fn new(inner: Box<dyn DatabaseAdapter>, cache_manager: Arc<CacheManager>) -> Self {
        Self {
            inner,
            cache_manager,
        }
    }
}

#[async_trait]
impl DatabaseAdapter for CachedDatabaseAdapter {
    /// 创建记录 - 创建成功后智能清理相关缓存
    async fn create(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        data: &HashMap<String, DataValue>,
        id_strategy: &IdStrategy,
        alias: &str,
    ) -> QuickDbResult<DataValue> {
        let result = self
            .inner
            .create(connection, table, data, id_strategy, alias)
            .await;

        // 创建成功后只清理查询缓存，保留记录缓存
        if result.is_ok() {
            if let Err(e) = self.cache_manager.clear_table_query_cache(table).await {
                warn!("清理表查询缓存失败: {}", e);
            }
            debug!("已清理表查询缓存: table={}", table);
        }

        result
    }

    /// 根据ID查找记录 - 先检查缓存，缓存未命中时查询数据库并缓存结果
    async fn find_by_id(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        id: &DataValue,
        alias: &str,
    ) -> QuickDbResult<Option<DataValue>> {
        // 将DataValue转换为IdType
        let id_type = match id {
            DataValue::Int(n) => IdType::Number(*n),
            DataValue::String(s) => IdType::String(s.clone()),
            _ => {
                warn!("无法将DataValue转换为IdType: {:?}", id);
                return self.inner.find_by_id(connection, table, id, alias).await;
            }
        };

        // 先检查缓存
        match self.cache_manager.get_cached_record(table, &id_type).await {
            Ok(Some(cached_result)) => {
                debug!("缓存命中: 表={}, ID={:?}", table, id);
                return Ok(Some(cached_result));
            }
            Ok(None) => {
                debug!("缓存未命中: 表={}, ID={:?}", table, id);
            }
            Err(e) => {
                warn!("缓存查询失败: {}, 继续查询数据库", e);
            }
        }

        // 缓存未命中或查询失败，查询数据库
        let result = self.inner.find_by_id(connection, table, id, alias).await;

        // 查询成功时缓存结果
        if let Ok(Some(ref record)) = result {
            if let Err(e) = self
                .cache_manager
                .cache_record(table, &id_type, record)
                .await
            {
                warn!("缓存记录失败: {}", e);
            }
        }

        result
    }

    /// 查找记录 - 内部统一使用 find_with_groups 实现
    async fn find(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
        options: &QueryOptions,
        alias: &str,
    ) -> QuickDbResult<Vec<DataValue>> {
        // 将简单条件转换为条件组合（AND逻辑）
        let condition_groups = if conditions.is_empty() {
            vec![]
        } else {
            let group_conditions = conditions
                .iter()
                .map(|c| QueryConditionGroup::Single(c.clone()))
                .collect();
            vec![QueryConditionGroup::Group {
                operator: LogicalOperator::And,
                conditions: group_conditions,
            }]
        };

        // 统一使用 find_with_groups 实现
        self.find_with_groups(connection, table, &condition_groups, options, alias)
            .await
    }

    /// 使用条件组合查找记录 - 先检查缓存，缓存未命中时查询数据库并缓存结果
    async fn find_with_groups(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        condition_groups: &[QueryConditionGroup],
        options: &QueryOptions,
        alias: &str,
    ) -> QuickDbResult<Vec<DataValue>> {
        // 生成条件组合查询缓存键
        let cache_key = self.cache_manager.generate_condition_groups_cache_key(
            table,
            condition_groups,
            options,
        );

        // 先检查缓存
        match self
            .cache_manager
            .get_cached_condition_groups_result(table, condition_groups, options)
            .await
        {
            Ok(Some(cached_result)) => {
                debug!("条件组合查询缓存命中: 表={}, 键={}", table, cache_key);
                return Ok(cached_result);
            }
            Ok(None) => {
                debug!("条件组合查询缓存未命中: 表={}, 键={}", table, cache_key);
            }
            Err(e) => {
                warn!("获取条件组合查询缓存失败: {}", e);
            }
        }

        // 缓存未命中，查询数据库
        let result = self
            .inner
            .find_with_groups(connection, table, condition_groups, options, alias)
            .await?;

        // 缓存查询结果
        if let Err(e) = self
            .cache_manager
            .cache_condition_groups_result(table, condition_groups, options, &result)
            .await
        {
            warn!("缓存条件组合查询结果失败: {}", e);
        } else {
            debug!(
                "已缓存条件组合查询结果: 表={}, 键={}, 结果数量={}",
                table,
                cache_key,
                result.len()
            );
        }

        Ok(result)
    }

    /// 更新记录 - 更新成功后智能清理相关缓存
    async fn update(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
        data: &HashMap<String, DataValue>,
        alias: &str,
    ) -> QuickDbResult<u64> {
        // 直接调用内部适配器更新记录
        let result = self
            .inner
            .update(connection, table, conditions, data, alias)
            .await;

        // 更新成功后只清理查询缓存，避免过度清理
        if let Ok(updated_count) = result {
            if updated_count > 0 {
                if let Err(e) = self.cache_manager.clear_table_query_cache(table).await {
                    warn!("清理表查询缓存失败: {}", e);
                }
                debug!(
                    "已清理表查询缓存: table={}, updated_count={}",
                    table, updated_count
                );
            }
        }

        result
    }

    /// 使用操作数组更新记录 - 更新成功后智能清理相关缓存
    async fn update_with_operations(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
        operations: &[crate::types::UpdateOperation],
        alias: &str,
    ) -> QuickDbResult<u64> {
        // 直接调用内部适配器更新记录
        let result = self
            .inner
            .update_with_operations(connection, table, conditions, operations, alias)
            .await;

        // 更新成功后只清理查询缓存，避免过度清理
        if let Ok(updated_count) = result {
            if updated_count > 0 {
                if let Err(e) = self.cache_manager.clear_table_query_cache(table).await {
                    warn!("清理表查询缓存失败: {}", e);
                }
                debug!(
                    "已清理表查询缓存: table={}, updated_count={}",
                    table, updated_count
                );
            }
        }

        result
    }

    /// 根据ID更新记录 - 更新成功后精确清理相关缓存
    async fn update_by_id(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        id: &DataValue,
        data: &HashMap<String, DataValue>,
        alias: &str,
    ) -> QuickDbResult<bool> {
        // 直接调用内部适配器更新记录
        let result = self
            .inner
            .update_by_id(connection, table, id, data, alias)
            .await;

        // 更新成功后精确清理相关缓存
        if let Ok(true) = result {
            // 清理特定记录的缓存
            let id_value = match id {
                DataValue::Int(n) => IdType::Number(*n),
                DataValue::String(s) => IdType::String(s.clone()),
                _ => {
                    warn!("无法将DataValue转换为IdType: {:?}", id);
                    return result;
                }
            };

            // 清理记录缓存
            if let Err(e) = self.cache_manager.invalidate_record(table, &id_value).await {
                warn!("清理记录缓存失败: {}", e);
            }

            // 只清理查询缓存，不清理其他记录缓存
            if let Err(e) = self.cache_manager.clear_table_query_cache(table).await {
                warn!("清理表查询缓存失败: {}", e);
            }

            debug!("已清理记录和查询缓存: table={}, id={:?}", table, id);
        }

        result
    }

    /// 删除记录 - 删除成功后智能清理相关缓存
    async fn delete(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
        alias: &str,
    ) -> QuickDbResult<u64> {
        // 直接调用内部适配器删除记录
        let result = self
            .inner
            .delete(connection, table, conditions, alias)
            .await;

        // 删除成功后智能清理相关缓存
        if let Ok(deleted_count) = result {
            if deleted_count > 0 {
                // 对于批量删除，清理整个表的缓存是合理的
                if let Err(e) = self.cache_manager.clear_table_query_cache(table).await {
                    warn!("清理表查询缓存失败: {}", e);
                }
                if let Err(e) = self.cache_manager.clear_table_record_cache(table).await {
                    warn!("清理表记录缓存失败: {}", e);
                }
                debug!(
                    "已清理表缓存: table={}, deleted_count={}",
                    table, deleted_count
                );
            }
        }

        result
    }

    /// 根据ID删除记录 - 删除成功后精确清理相关缓存
    async fn delete_by_id(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        id: &DataValue,
        alias: &str,
    ) -> QuickDbResult<bool> {
        // 直接调用内部适配器删除记录
        let result = self.inner.delete_by_id(connection, table, id, alias).await;

        // 删除成功后精确清理相关缓存
        if let Ok(true) = result {
            // 清理特定记录的缓存
            let id_value = match id {
                DataValue::Int(n) => IdType::Number(*n),
                DataValue::String(s) => IdType::String(s.clone()),
                _ => {
                    warn!("无法将DataValue转换为IdType: {:?}", id);
                    return result;
                }
            };

            // 清理记录缓存
            if let Err(e) = self.cache_manager.invalidate_record(table, &id_value).await {
                warn!("清理记录缓存失败: {}", e);
            }

            // 只清理查询缓存
            if let Err(e) = self.cache_manager.clear_table_query_cache(table).await {
                warn!("清理表查询缓存失败: {}", e);
            }

            debug!("已清理记录和查询缓存: table={}, id={:?}", table, id);
        }

        result
    }

    /// 统计记录数量 - 直接调用内部适配器，不缓存统计结果
    async fn count(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
        alias: &str,
    ) -> QuickDbResult<u64> {
        // 统计操作不缓存，直接调用内部适配器
        self.inner.count(connection, table, conditions, alias).await
    }

    /// 创建表/集合 - 直接调用内部适配器
    async fn create_table(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        fields: &HashMap<String, FieldDefinition>,
        id_strategy: &IdStrategy,
        alias: &str,
    ) -> QuickDbResult<()> {
        // 表结构操作不缓存，直接调用内部适配器
        self.inner
            .create_table(connection, table, fields, id_strategy, alias)
            .await
    }

    /// 创建索引 - 直接调用内部适配器
    async fn create_index(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        index_name: &str,
        fields: &[String],
        unique: bool,
    ) -> QuickDbResult<()> {
        // 索引操作不缓存，直接调用内部适配器
        self.inner
            .create_index(connection, table, index_name, fields, unique)
            .await
    }

    /// 检查表是否存在 - 直接调用内部适配器
    async fn table_exists(
        &self,
        connection: &DatabaseConnection,
        table: &str,
    ) -> QuickDbResult<bool> {
        self.inner.table_exists(connection, table).await
    }

    /// 删除表 - 删除成功后清理所有相关缓存
    async fn drop_table(&self, connection: &DatabaseConnection, table: &str) -> QuickDbResult<()> {
        let result = self.inner.drop_table(connection, table).await;

        // 删除成功后清理所有相关缓存
        if result.is_ok() {
            if let Err(e) = self.cache_manager.clear_table_query_cache(table).await {
                warn!("清理表缓存失败: {}", e);
            }
            debug!("已清理表缓存: table={}", table);
        }

        result
    }

    async fn get_server_version(&self, connection: &DatabaseConnection) -> QuickDbResult<String> {
        // 版本查询通常不涉及具体数据，直接调用内部适配器
        self.inner.get_server_version(connection).await
    }

    /// 创建存储过程 - 直接调用内部适配器
    async fn create_stored_procedure(
        &self,
        connection: &DatabaseConnection,
        config: &crate::stored_procedure::StoredProcedureConfig,
    ) -> QuickDbResult<crate::stored_procedure::StoredProcedureCreateResult> {
        // 存储过程创建不缓存，直接调用内部适配器
        self.inner.create_stored_procedure(connection, config).await
    }

    /// 执行存储过程 - 直接调用内部适配器
    async fn execute_stored_procedure(
        &self,
        connection: &DatabaseConnection,
        procedure_name: &str,
        database: &str,
        params: Option<std::collections::HashMap<String, crate::types::DataValue>>,
    ) -> QuickDbResult<crate::stored_procedure::StoredProcedureQueryResult> {
        // 存储过程执行不缓存，直接调用内部适配器
        self.inner
            .execute_stored_procedure(connection, procedure_name, database, params)
            .await
    }
}
