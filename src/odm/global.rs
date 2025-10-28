
//! # 全局ODM管理器和便捷函数

use crate::error::QuickDbResult;
use crate::types::*;
use crate::odm::manager_core::AsyncOdmManager;
use crate::odm::traits::OdmOperations;
use std::collections::HashMap;

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
    // 锁定全局操作
    crate::lock_global_operations();

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
    // 锁定全局操作
    crate::lock_global_operations();

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
    // 锁定全局操作
    crate::lock_global_operations();

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
    // 锁定全局操作
    crate::lock_global_operations();

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
    // 锁定全局操作
    crate::lock_global_operations();

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
    // 锁定全局操作
    crate::lock_global_operations();

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
    // 锁定全局操作
    crate::lock_global_operations();

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
    // 锁定全局操作
    crate::lock_global_operations();

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
    // 锁定全局操作
    crate::lock_global_operations();

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
    // 锁定全局操作
    crate::lock_global_operations();

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
    // 锁定全局操作
    crate::lock_global_operations();

    let manager = get_odm_manager().await;
    manager.exists(collection, conditions, alias).await
}

/// 获取数据库服务器版本信息
pub async fn get_server_version(alias: Option<&str>) -> QuickDbResult<String> {
    // 锁定全局操作
    crate::lock_global_operations();

    let manager = get_odm_manager().await;
    manager.get_server_version(alias).await
}