//! 连接池管理器模块
//!
//! 提供多数据库连接池的管理功能，包括连接池创建、维护、缓存管理等

mod manager;
mod database_ops;
mod cache_ops;
mod model_ops;
mod maintenance;

// 重新导出主要类型
pub use manager::PoolManager;

// 全局便捷函数（从原manager.rs的第631行开始）
use std::sync::Arc;
use tokio::sync::RwLock;
use dashmap::DashMap;

use crate::error::{QuickDbError, QuickDbResult};
use crate::pool::{ConnectionPool, PooledConnection};
use crate::types::{DatabaseConfig, IdType};
use crate::id_generator::{IdGenerator, MongoAutoIncrementGenerator};
use crate::cache::{CacheManager, CacheStats};
use crate::model::ModelMeta;
use crate::types::id_types::IdStrategy;
use once_cell::sync::Lazy;

/// 全局连接池管理器实例
pub static GLOBAL_POOL_MANAGER: Lazy<PoolManager> =
    Lazy::new(|| PoolManager::new());

/// 获取全局连接池管理器
pub(crate) fn get_global_pool_manager() -> &'static PoolManager {
    &GLOBAL_POOL_MANAGER
}

/// 便捷函数 - 添加数据库配置
pub async fn add_database(config: DatabaseConfig) -> QuickDbResult<()> {
    // 检查全局操作锁状态，如果已锁定则惊恐退出
    if crate::is_global_operations_locked() {
        panic!("全局操作已锁定，禁止添加数据库！系统已开始执行查询操作，不允许再添加数据库配置");
    }

    get_global_pool_manager().add_database(config).await
}


/// 便捷函数 - 获取连接
pub async fn get_connection(alias: Option<&str>) -> QuickDbResult<PooledConnection> {
    // 锁定全局操作
    crate::lock_global_operations();

    get_global_pool_manager().get_connection(alias).await
}

/// 便捷函数 - 释放连接
pub async fn release_connection(connection: &PooledConnection) -> QuickDbResult<()> {
    // 锁定全局操作
    crate::lock_global_operations();

    get_global_pool_manager().release_connection(connection).await
}

/// 便捷函数 - 获取所有别名
pub fn get_aliases() -> Vec<String> {
    get_global_pool_manager().get_aliases()
}

/// 便捷函数 - 设置默认别名
pub async fn set_default_alias(alias: &str) -> QuickDbResult<()> {
    get_global_pool_manager().set_default_alias(alias).await
}



/// 便捷函数 - 健康检查
pub async fn health_check() -> std::collections::HashMap<String, bool> {
    // 锁定全局操作
    crate::lock_global_operations();

    get_global_pool_manager().health_check().await
}

/// 便捷函数 - 获取所有活跃连接池的详细状态信息
pub async fn get_active_pools_status() -> std::collections::HashMap<String, serde_json::Value> {
    // 锁定全局操作
    crate::lock_global_operations();

    get_global_pool_manager().get_active_pools_status().await
}

#[cfg(feature = "python-bindings")]
#[doc(hidden)]
/// 便捷函数 - 获取连接池映射（仅用于Python绑定，不推荐直接使用）
pub fn get_connection_pools() -> Arc<DashMap<String, Arc<ConnectionPool>>> {
    get_global_pool_manager().get_connection_pools()
}


/// 便捷函数 - 注册模型元数据
///
/// # Python API专用
///
/// 此函数主要用于Python绑定，用于在ODM层注册模型元数据
/// Rust代码内部通常不需要直接调用此函数
pub fn register_model(model_meta: ModelMeta) -> QuickDbResult<()> {
    get_global_pool_manager().register_model(model_meta)
}

/// 便捷函数 - 获取模型元数据
#[deprecated(note = "使用 get_model(collection_name, alias) 替代")]
pub fn get_model(collection_name: &str) -> Option<ModelMeta> {
    get_global_pool_manager().get_model(collection_name)
}

/// 便捷函数 - 获取带别名的模型元数据
pub fn get_model_with_alias(collection_name: &str, alias: &str) -> Option<ModelMeta> {
    get_global_pool_manager().get_model_with_alias(collection_name, alias)
}

/// 便捷函数 - 检查模型是否已注册
pub fn has_model(collection_name: &str) -> bool {
    get_global_pool_manager().has_model(collection_name)
}

/// 便捷函数 - 创建表和索引（基于注册的模型元数据）
pub async fn ensure_table_and_indexes(collection_name: &str, alias: &str) -> QuickDbResult<()> {
    get_global_pool_manager().ensure_table_and_indexes(collection_name, alias).await
}

/// 便捷函数 - 获取缓存管理器
pub fn get_cache_manager(alias: &str) -> QuickDbResult<Arc<CacheManager>> {
    get_global_pool_manager().get_cache_manager(alias)
}

/// 便捷函数 - 获取缓存统计信息
pub async fn get_cache_stats(alias: &str) -> QuickDbResult<CacheStats> {
    // 锁定全局操作
    crate::lock_global_operations();

    get_global_pool_manager().get_cache_stats(alias).await
}

/// 便捷函数 - 清理指定数据库的缓存
pub async fn clear_cache(alias: &str) -> QuickDbResult<()> {
    // 锁定全局操作
    crate::lock_global_operations();

    get_global_pool_manager().clear_cache(alias).await
}

/// 便捷函数 - 清理所有数据库的缓存
pub async fn clear_all_caches() -> QuickDbResult<()> {
    // 锁定全局操作
    crate::lock_global_operations();

    get_global_pool_manager().clear_all_caches().await
}

/// 便捷函数 - 按模式清理缓存
///
/// # 参数
/// * `alias` - 数据库别名
/// * `pattern` - 缓存键模式，支持通配符 * 和 ?
pub async fn clear_cache_by_pattern(alias: &str, pattern: &str) -> QuickDbResult<usize> {
    // 锁定全局操作
    crate::lock_global_operations();

    let cache_manager = get_global_pool_manager().get_cache_manager(alias)?;
    cache_manager.clear_by_pattern(pattern).await
        .map_err(|e| QuickDbError::CacheError { message: e.to_string() })
}

/// 便捷函数 - 批量清理记录缓存
///
/// # 参数
/// * `alias` - 数据库别名
/// * `table` - 表名
/// * `ids` - 要清理的记录ID列表
pub async fn clear_records_cache_batch(alias: &str, table: &str, ids: &[IdType]) -> QuickDbResult<usize> {
    // 锁定全局操作
    crate::lock_global_operations();

    let cache_manager = get_global_pool_manager().get_cache_manager(alias)?;
    cache_manager.clear_records_batch(table, ids).await
        .map_err(|e| QuickDbError::CacheError { message: e.to_string() })
}

/// 便捷函数 - 强制清理过期缓存
///
/// 手动触发过期缓存的清理，通常用于内存紧张或需要立即释放空间的场景
pub async fn force_cleanup_expired_cache(alias: &str) -> QuickDbResult<usize> {
    // 锁定全局操作
    crate::lock_global_operations();

    let cache_manager = get_global_pool_manager().get_cache_manager(alias)?;
    cache_manager.force_cleanup_expired().await
        .map_err(|e| QuickDbError::CacheError { message: e.to_string() })
}

/// 便捷函数 - 获取所有缓存键列表（按表分组）
///
/// 用于调试和监控，可以查看当前缓存中有哪些键
pub async fn list_cache_keys(alias: &str) -> QuickDbResult<std::collections::HashMap<String, Vec<String>>> {
    // 锁定全局操作
    crate::lock_global_operations();

    let cache_manager = get_global_pool_manager().get_cache_manager(alias)?;
    cache_manager.list_cache_keys().await
        .map_err(|e| QuickDbError::CacheError { message: e.to_string() })
}

/// 便捷函数 - 获取指定表的缓存键列表
pub async fn list_table_cache_keys(alias: &str, table: &str) -> QuickDbResult<Vec<String>> {
    // 锁定全局操作
    crate::lock_global_operations();

    let cache_manager = get_global_pool_manager().get_cache_manager(alias)?;
    cache_manager.list_table_cache_keys(table).await
        .map_err(|e| QuickDbError::CacheError { message: e.to_string() })
}

/// 便捷函数 - 清理指定表的查询缓存
///
/// 只清理查询缓存，保留记录缓存
pub async fn clear_table_query_cache(alias: &str, table: &str) -> QuickDbResult<usize> {
    // 锁定全局操作
    crate::lock_global_operations();

    let cache_manager = get_global_pool_manager().get_cache_manager(alias)?;
    cache_manager.clear_table_query_cache(table).await
        .map_err(|e| QuickDbError::CacheError { message: e.to_string() })
}

/// 便捷函数 - 清理指定表的记录缓存
///
/// 只清理记录缓存，保留查询缓存
pub async fn clear_table_record_cache(alias: &str, table: &str) -> QuickDbResult<usize> {
    // 锁定全局操作
    crate::lock_global_operations();

    let cache_manager = get_global_pool_manager().get_cache_manager(alias)?;
    cache_manager.clear_table_record_cache(table).await
        .map_err(|e| QuickDbError::CacheError { message: e.to_string() })
}

/// 便捷函数 - 清理指定表的所有缓存（记录+查询）
pub async fn clear_table_all_cache(alias: &str, table: &str) -> QuickDbResult<usize> {
    // 锁定全局操作
    crate::lock_global_operations();

    let cache_manager = get_global_pool_manager().get_cache_manager(alias)?;
    let record_count = cache_manager.clear_table_record_cache(table).await
        .map_err(|e| QuickDbError::CacheError { message: e.to_string() })?;
    let query_count = cache_manager.clear_table_query_cache(table).await
        .map_err(|e| QuickDbError::CacheError { message: e.to_string() })?;
    Ok(record_count + query_count)
}

/// 便捷函数 - 检查表是否存在
///
/// # 参数
/// * `alias` - 数据库别名
/// * `table` - 表名或集合名
///
/// # 返回值
/// 返回表是否存在，true表示存在，false表示不存在
///
pub async fn table_exists(alias: &str, table: &str) -> QuickDbResult<bool> {
    // table_exists是只读查询操作，不需要全局锁定

    let pool_manager = get_global_pool_manager();

    // 检查数据库是否存在，不存在则报错
    if !pool_manager.pools.contains_key(alias) {
        return Err(QuickDbError::AliasNotFound {
            alias: alias.to_string(),
        });
    }

    // 获取连接池，检查是否为空
    let pool = pool_manager.pools.get(alias)
        .ok_or_else(|| QuickDbError::AliasNotFound {
            alias: alias.to_string(),
        })?;

    // 执行检查操作
    pool.table_exists(table).await
}

/// 便捷函数 - 删除表/集合
///
/// 如果表不存在则直接返回成功，存在则执行删除操作
///
/// # 参数
/// * `alias` - 数据库别名
/// * `table` - 表名或集合名
///
pub async fn drop_table(alias: &str, table: &str) -> QuickDbResult<()> {
    // 锁定全局操作
    crate::lock_global_operations();

    let pool_manager = get_global_pool_manager();

    // 检查数据库是否存在，不存在则报错
    if !pool_manager.pools.contains_key(alias) {
        return Err(QuickDbError::AliasNotFound {
            alias: alias.to_string(),
        });
    }

    // 获取连接池，检查是否为空
    let pool = pool_manager.pools.get(alias)
        .ok_or_else(|| QuickDbError::AliasNotFound {
            alias: alias.to_string(),
        })?;

    // 执行删除操作
    pool.drop_table(table).await
}
/// 便捷函数 - 获取数据库ID策略
pub fn get_id_strategy(alias: &str) -> QuickDbResult<IdStrategy> {
    get_global_pool_manager().get_id_strategy(alias)
}

/// 便捷函数 - 关闭管理器
pub async fn shutdown() -> QuickDbResult<()> {
    get_global_pool_manager().shutdown().await
}
