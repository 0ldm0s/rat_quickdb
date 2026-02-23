//! ModelManager 实现模块
//!
//! 提供模型的通用操作实现

use crate::error::{QuickDbError, QuickDbResult};
use crate::model::traits::{Model, ModelOperations};
use crate::odm::{self, OdmOperations};
use crate::types::*;
use async_trait::async_trait;
use rat_logger::debug;
use std::collections::HashMap;
use std::marker::PhantomData;

/// 模型管理器
///
/// 提供模型的通用操作实现
pub struct ModelManager<T: Model> {
    _phantom: PhantomData<T>,
}

impl<T: Model> ModelManager<T> {
    /// 创建新的模型管理器
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }

    // ========== 简化方法：接受 QueryCondition（自动转换） ==========

    /// 查找模型（简化方法）
    ///
    /// 接受 `Vec<QueryCondition>` 并自动转换为 `Vec<QueryConditionWithConfig>`
    /// 适用于不需要额外配置的常见查询场景
    pub async fn find(
        conditions: Vec<QueryCondition>,
        options: Option<QueryOptions>,
    ) -> QuickDbResult<Vec<T>> {
        let conditions_with_config: Vec<QueryConditionWithConfig> = conditions
            .into_iter()
            .map(|c| c.into())
            .collect();
        Self::find_with_config(conditions_with_config, options).await
    }

    /// 统计模型数量（简化方法）
    ///
    /// 接受 `Vec<QueryCondition>` 并自动转换为 `Vec<QueryConditionWithConfig>`
    pub async fn count(conditions: Vec<QueryCondition>) -> QuickDbResult<u64> {
        let conditions_with_config: Vec<QueryConditionWithConfig> = conditions
            .into_iter()
            .map(|c| c.into())
            .collect();
        Self::count_with_config(conditions_with_config).await
    }

    /// 批量删除模型（简化方法）
    ///
    /// 接受 `Vec<QueryCondition>` 并自动转换为 `Vec<QueryConditionWithConfig>`
    pub async fn delete_many(conditions: Vec<QueryCondition>) -> QuickDbResult<u64> {
        let conditions_with_config: Vec<QueryConditionWithConfig> = conditions
            .into_iter()
            .map(|c| c.into())
            .collect();
        Self::delete_many_with_config(conditions_with_config).await
    }

    /// 查找模型（简化方法，支持缓存控制）
    ///
    /// 接受 `Vec<QueryCondition>` 并自动转换为 `Vec<QueryConditionWithConfig>`
    pub async fn find_with_cache_control(
        conditions: Vec<QueryCondition>,
        options: Option<QueryOptions>,
        bypass_cache: bool,
    ) -> QuickDbResult<Vec<T>> {
        let conditions_with_config: Vec<QueryConditionWithConfig> = conditions
            .into_iter()
            .map(|c| c.into())
            .collect();
        <Self as ModelOperations<T>>::find_with_cache_control(conditions_with_config, options, bypass_cache).await
    }

    // ========== 完整方法：接受 QueryConditionWithConfig ==========

    /// 查找模型（带配置）
    ///
    /// 接受 `Vec<QueryConditionWithConfig>`，支持大小写不敏感等高级配置
    pub async fn find_with_config(
        conditions: Vec<QueryConditionWithConfig>,
        options: Option<QueryOptions>,
    ) -> QuickDbResult<Vec<T>> {
        <Self as ModelOperations<T>>::find_with_cache_control(conditions, options, false).await
    }

    /// 统计模型数量（带配置）
    ///
    /// 接受 `Vec<QueryConditionWithConfig>`，支持大小写不敏感等高级配置
    pub async fn count_with_config(conditions: Vec<QueryConditionWithConfig>) -> QuickDbResult<u64> {
        <Self as ModelOperations<T>>::count_with_config(conditions).await
    }

    /// 使用条件组统计模型数量（简化版）
    ///
    /// 接受 `Vec<QueryConditionGroup>`，支持复杂的AND/OR逻辑组合
    pub async fn count_with_groups(
        condition_groups: Vec<QueryConditionGroup>,
    ) -> QuickDbResult<u64> {
        <Self as ModelOperations<T>>::count_with_groups(condition_groups).await
    }

    /// 使用条件组统计模型数量（完整版）
    ///
    /// 接受 `Vec<QueryConditionGroupWithConfig>`，支持大小写不敏感等高级配置
    pub async fn count_with_groups_with_config(
        condition_groups: Vec<QueryConditionGroupWithConfig>,
    ) -> QuickDbResult<u64> {
        <Self as ModelOperations<T>>::count_with_groups_with_config(condition_groups).await
    }

    /// 批量删除模型（带配置）
    ///
    /// 接受 `Vec<QueryConditionWithConfig>`，支持大小写不敏感等高级配置
    pub async fn delete_many_with_config(conditions: Vec<QueryConditionWithConfig>) -> QuickDbResult<u64> {
        <Self as ModelOperations<T>>::delete_many(conditions).await
    }

    /// 创建表（静态便利方法）
    ///
    /// 使用模型的元数据直接创建表，无需插入数据
    /// 这是 ModelOperations::create_table 的静态便利包装器
    ///
    /// # 不推荐使用
    ///
    /// 这个方法通常不需要手动调用，因为在执行其他数据库操作时，
    /// 表会根据需要自动创建。仅在确实需要预先创建表结构时使用。
    pub async fn create_table() -> QuickDbResult<()> {
        <Self as ModelOperations<T>>::create_table().await
    }
}

#[async_trait]
impl<T: Model> ModelOperations<T> for ModelManager<T> {
    async fn save(&self) -> QuickDbResult<String> {
        // 这个方法需要模型实例，应该在具体的模型实现中调用
        Err(QuickDbError::ValidationError {
            field: "save".to_string(),
            message: "save方法需要在模型实例上调用".to_string(),
        })
    }

    async fn find_by_id(id: &str) -> QuickDbResult<Option<T>> {
        let collection_name = T::collection_name();
        let database_alias = T::database_alias();

        debug!("根据ID查找模型: collection={}, id={}", collection_name, id);

        let result = odm::find_by_id(&collection_name, id, database_alias.as_deref()).await?;

        if let Some(data_value) = result {
            // 处理 DataValue::Object 格式的数据
            match data_value {
                DataValue::Object(data_map) => {
                    debug!("从数据库收到的数据: {:?}", data_map);
                    let model: T = match T::from_data_map(data_map.clone()) {
                        Ok(model) => model,
                        Err(e) => {
                            debug!("❌ from_data_map失败: {}, 数据: {:?}", e, data_map);
                            return Err(e);
                        }
                    };
                    Ok(Some(model))
                }
                _ => {
                    // 兼容其他格式，使用直接反序列化
                    debug!("收到非Object格式数据: {:?}", data_value);
                    let model: T = data_value.deserialize_to()?;
                    Ok(Some(model))
                }
            }
        } else {
            Ok(None)
        }
    }

    async fn find_with_cache_control(
        conditions: Vec<QueryConditionWithConfig>,
        options: Option<QueryOptions>,
        bypass_cache: bool,
    ) -> QuickDbResult<Vec<T>> {
        let collection_name = T::collection_name();
        let database_alias = T::database_alias();

        debug!("查找模型（bypass_cache={}）: collection={}", bypass_cache, collection_name);

        let result = odm::find_with_cache_control(
            &collection_name,
            conditions,
            options,
            database_alias.as_deref(),
            bypass_cache,
        )
        .await?;

        // result 已经是 Vec<DataValue>，直接处理
        let mut models = Vec::new();
        for data_value in result {
            // 处理 DataValue::Object 格式的数据
            match data_value {
                DataValue::Object(data_map) => {
                    debug!("查询收到的数据: {:?}", data_map);
                    let model: T = match T::from_data_map(data_map.clone()) {
                        Ok(model) => model,
                        Err(e) => {
                            debug!("❌ 查询from_data_map失败: {}, 数据: {:?}", e, data_map);
                            continue;
                        }
                    };
                    models.push(model);
                }
                _ => {
                    // 兼容其他格式，使用直接反序列化
                    debug!("查询收到非Object格式数据: {:?}", data_value);
                    let model: T = data_value.deserialize_to()?;
                    models.push(model);
                }
            }
        }
        Ok(models)
    }

    async fn update(&self, _updates: HashMap<String, DataValue>) -> QuickDbResult<bool> {
        // 这个方法需要模型实例，应该在具体的模型实现中调用
        Err(QuickDbError::ValidationError {
            field: "update".to_string(),
            message: "update方法需要在模型实例上调用".to_string(),
        })
    }

    async fn delete(&self) -> QuickDbResult<bool> {
        // 这个方法需要模型实例，应该在具体的模型实现中调用
        Err(QuickDbError::ValidationError {
            field: "delete".to_string(),
            message: "delete方法需要在模型实例上调用".to_string(),
        })
    }

    async fn count(conditions: Vec<QueryCondition>) -> QuickDbResult<u64> {
        let conditions_with_config: Vec<QueryConditionWithConfig> = conditions
            .into_iter()
            .map(|c| c.into())
            .collect();
        Self::count_with_config(conditions_with_config).await
    }

    async fn count_with_config(conditions: Vec<QueryConditionWithConfig>) -> QuickDbResult<u64> {
        let collection_name = T::collection_name();
        let database_alias = T::database_alias();

        debug!("统计模型数量: collection={}", collection_name);

        odm::count(&collection_name, conditions, database_alias.as_deref()).await
    }

    async fn count_with_groups_with_config(
        condition_groups: Vec<QueryConditionGroupWithConfig>,
    ) -> QuickDbResult<u64> {
        let collection_name = T::collection_name();
        let database_alias = T::database_alias();

        debug!("使用条件组统计模型数量: collection={}", collection_name);

        odm::count_with_groups(&collection_name, condition_groups, database_alias.as_deref()).await
    }

    async fn find_with_groups_with_cache_control(
        condition_groups: Vec<QueryConditionGroup>,
        options: Option<QueryOptions>,
        bypass_cache: bool,
    ) -> QuickDbResult<Vec<T>> {
        let collection_name = T::collection_name();
        let database_alias = T::database_alias();

        debug!("使用条件组查找模型（bypass_cache={}）: collection={}", bypass_cache, collection_name);

        // 转换为完整版
        let condition_groups_with_config: Vec<QueryConditionGroupWithConfig> = condition_groups
            .into_iter()
            .map(|g| g.into())
            .collect();

        let result = odm::find_with_groups_with_cache_control(
            &collection_name,
            condition_groups_with_config,
            options,
            database_alias.as_deref(),
            bypass_cache,
        )
        .await?;

        // 处理返回的 DataValue 数据
        let mut models = Vec::new();
        for data_value in result {
            let model: T = T::from_data_map(data_value.expect_object()?)?;
            models.push(model);
        }
        Ok(models)
    }

    async fn find_with_groups(
        condition_groups: Vec<QueryConditionGroup>,
        options: Option<QueryOptions>,
    ) -> QuickDbResult<Vec<T>> {
        Self::find_with_groups_with_cache_control(condition_groups, options, false).await
    }

    async fn find_with_groups_with_config(
        condition_groups: Vec<QueryConditionGroupWithConfig>,
        options: Option<QueryOptions>,
    ) -> QuickDbResult<Vec<T>> {
        Self::find_with_groups_with_cache_control_and_config(condition_groups, options, false).await
    }

    async fn find_with_groups_with_cache_control_and_config(
        condition_groups: Vec<QueryConditionGroupWithConfig>,
        options: Option<QueryOptions>,
        bypass_cache: bool,
    ) -> QuickDbResult<Vec<T>> {
        let collection_name = T::collection_name();
        let database_alias = T::database_alias();

        debug!("使用条件组查找模型（bypass_cache={}）: collection={}", bypass_cache, collection_name);

        let result = odm::find_with_groups_with_cache_control(
            &collection_name,
            condition_groups,
            options,
            database_alias.as_deref(),
            bypass_cache,
        )
        .await?;

        // 处理返回的 DataValue 数据
        let mut models = Vec::new();
        for data_value in result {
            let model: T = T::from_data_map(data_value.expect_object()?)?;
            models.push(model);
        }
        Ok(models)
    }

    /// 批量更新模型
    ///
    /// 根据条件批量更新多条记录，返回受影响的行数
    async fn update_many(
        conditions: Vec<QueryConditionWithConfig>,
        updates: HashMap<String, DataValue>,
    ) -> QuickDbResult<u64> {
        let collection_name = T::collection_name();
        let database_alias = T::database_alias();

        debug!(
            "批量更新模型: collection={}, 条件数量={}",
            collection_name,
            conditions.len()
        );

        odm::update(
            &collection_name,
            conditions,
            updates,
            database_alias.as_deref(),
        )
        .await
    }

    /// 使用操作数组批量更新模型
    ///
    /// 根据条件使用操作数组批量更新多条记录，支持原子性增减操作，返回受影响的行数
    async fn update_many_with_operations(
        conditions: Vec<QueryConditionWithConfig>,
        operations: Vec<crate::types::UpdateOperation>,
    ) -> QuickDbResult<u64> {
        let collection_name = T::collection_name();
        let database_alias = T::database_alias();

        debug!(
            "使用操作数组批量更新模型: collection={}, 条件数量={}, 操作数量={}",
            collection_name,
            conditions.len(),
            operations.len()
        );

        odm::update_with_operations(
            &collection_name,
            conditions,
            operations,
            database_alias.as_deref(),
        )
        .await
    }

    /// 批量删除模型
    ///
    /// 根据条件批量删除多条记录，返回受影响的行数
    async fn delete_many(conditions: Vec<QueryConditionWithConfig>) -> QuickDbResult<u64> {
        let collection_name = T::collection_name();
        let database_alias = T::database_alias();

        debug!(
            "批量删除模型: collection={}, 条件数量={}",
            collection_name,
            conditions.len()
        );

        odm::delete(&collection_name, conditions, database_alias.as_deref()).await
    }

    /// 创建表
    ///
    /// 使用模型的元数据直接创建表，无需插入数据
    /// 复用现有的ensure_table_and_indexes功能
    async fn create_table() -> QuickDbResult<()> {
        let collection_name = T::collection_name();
        let database_alias = T::database_alias();

        debug!("直接创建表: collection={}", collection_name);

        // 获取默认数据库别名
        let alias = database_alias.as_deref().unwrap_or("default");

        // 使用现有的ensure_table_and_indexes功能
        crate::manager::ensure_table_and_indexes(&collection_name, alias).await
    }

    /// 创建存储过程
    ///
    /// 通过模型管理器创建跨模型的存储过程，以当前模型作为基表
    async fn create_stored_procedure(
        config: crate::stored_procedure::StoredProcedureConfig,
    ) -> QuickDbResult<crate::stored_procedure::StoredProcedureCreateResult> {
        debug!("通过模型管理器创建存储过程: {}", config.procedure_name);
        let odm_manager = odm::get_odm_manager().await;
        // 使用config中包含的数据库别名，不传递额外的alias参数
        odm_manager.create_stored_procedure(config).await
    }

    /// 执行存储过程查询
    ///
    /// 通过模型管理器执行存储过程查询，使用当前模型的数据库别名
    async fn execute_stored_procedure(
        procedure_name: &str,
        params: Option<std::collections::HashMap<String, crate::types::DataValue>>,
    ) -> QuickDbResult<crate::stored_procedure::StoredProcedureQueryResult> {
        debug!("通过模型管理器执行存储过程: {}", procedure_name);
        let odm_manager = odm::get_odm_manager().await;
        // 使用模型的数据库别名，如果没有则使用默认
        let database_alias = T::database_alias().or_else(|| Some("default".to_string()));
        odm_manager
            .execute_stored_procedure(procedure_name, database_alias.as_deref(), params)
            .await
    }
}
