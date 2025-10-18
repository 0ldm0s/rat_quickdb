//! 连接池管理器
//!
//! 管理多个数据库别名的连接池，提供统一的连接获取和释放接口

use crate::error::{QuickDbError, QuickDbResult};
use crate::pool::{ConnectionPool, PooledConnection, ExtendedPoolConfig};
use crate::types::{DatabaseConfig, DatabaseType, IdType};
use crate::id_generator::{IdGenerator, MongoAutoIncrementGenerator};
use crate::cache::{CacheManager, CacheStats};
use crate::model::ModelMeta;
use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use rat_logger::{info, warn, error, debug};

/// 连接池管理器 - 管理多个数据库连接池
#[derive(Debug)]
pub struct PoolManager {
    /// 数据库连接池映射 (别名 -> 连接池)
    pools: Arc<DashMap<String, Arc<ConnectionPool>>>,
    /// 默认数据库别名
    default_alias: Arc<RwLock<Option<String>>>,
    /// 清理任务句柄
    cleanup_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    /// ID生成器映射 (别名 -> ID生成器)
    id_generators: Arc<DashMap<String, Arc<IdGenerator>>>,
    /// MongoDB自增ID生成器映射 (别名 -> 自增生成器)
    mongo_auto_increment_generators: Arc<DashMap<String, Arc<MongoAutoIncrementGenerator>>>,
    /// 缓存管理器映射 (别名 -> 缓存管理器)
    cache_managers: Arc<DashMap<String, Arc<CacheManager>>>,
    /// 模型元数据注册表 (集合名 -> 模型元数据)
    model_registry: Arc<DashMap<String, ModelMeta>>,
    /// 索引创建锁，防止并发创建同一个索引 (表名 -> 索引名 -> ())
    index_creation_locks: Arc<tokio::sync::Mutex<HashMap<String, HashMap<String, ()>>>>,
}

impl PoolManager {
    /// 创建新的连接池管理器
    pub fn new() -> Self {
        info!("创建连接池管理器");

        Self {
            pools: Arc::new(DashMap::new()),
            default_alias: Arc::new(RwLock::new(None)),
            cleanup_handle: Arc::new(RwLock::new(None)),
            id_generators: Arc::new(DashMap::new()),
            mongo_auto_increment_generators: Arc::new(DashMap::new()),
            cache_managers: Arc::new(DashMap::new()),
            model_registry: Arc::new(DashMap::new()),
            index_creation_locks: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        }
    }

    /// 添加数据库配置并创建连接池
    pub async fn add_database(&self, config: DatabaseConfig) -> QuickDbResult<()> {
        let alias = config.alias.clone();
        
        info!("添加数据库配置: 别名={}, 类型={:?}", alias, config.db_type);
        
        // 检查别名是否已存在
        if self.pools.contains_key(&alias) {
            warn!("数据库别名已存在，将替换现有配置: {}", alias);
            self.remove_database(&alias).await?;
        }
        
        // 初始化缓存管理器（如果配置了缓存）
        let cache_manager_arc = if let Some(cache_config) = &config.cache {
            let cache_manager = CacheManager::new(cache_config.clone()).await.map_err(|e| {
                error!("为数据库 {} 创建缓存管理器失败: {}", alias, e);
                e
            })?;
            let cache_manager_arc = Arc::new(cache_manager);
            // 保存到管理器中
            self.cache_managers.insert(alias.clone(), cache_manager_arc.clone());
            info!("为数据库 {} 创建缓存管理器", alias);
            Some(cache_manager_arc)
        } else {
            None
        };
        
        // 创建连接池（传入缓存管理器）
        let pool_config = ExtendedPoolConfig::default();
        let pool = ConnectionPool::with_config_and_cache(config.clone(), pool_config, cache_manager_arc).await.map_err(|e| {
            error!("连接池创建失败: 别名={}, 错误={}", alias, e);
            e
        })?;
        
        // 添加到管理器
        self.pools.insert(alias.clone(), Arc::new(pool));
        
        // 初始化ID生成器
        match IdGenerator::new(config.id_strategy.clone()) {
            Ok(generator) => {
                self.id_generators.insert(alias.clone(), Arc::new(generator));
                info!("为数据库 {} 创建ID生成器: {:?}", alias, config.id_strategy);
            }
            Err(e) => {
                warn!("为数据库 {} 创建ID生成器失败: {}", alias, e);
            }
        }
        
        // 为MongoDB创建自增ID生成器
        if matches!(config.db_type, DatabaseType::MongoDB) {
            let mongo_generator = MongoAutoIncrementGenerator::new(alias.clone());
            self.mongo_auto_increment_generators.insert(alias.clone(), Arc::new(mongo_generator));
            info!("为MongoDB数据库 {} 创建自增ID生成器", alias);
        }
        
        // 如果这是第一个数据库，设置为默认
        {
            let mut default_alias = self.default_alias.write().await;
            if default_alias.is_none() {
                *default_alias = Some(alias.clone());
                info!("设置默认数据库别名: {}", alias);
            }
        }
        
        // 启动清理任务（如果还没有启动）
        self.start_cleanup_task().await;
        
        info!("数据库添加成功: 别名={}", alias);
        Ok(())
    }

    /// 移除数据库配置
    pub async fn remove_database(&self, alias: &str) -> QuickDbResult<()> {
        info!("移除数据库配置: 别名={}", alias);
        
        if let Some((_, _pool)) = self.pools.remove(alias) {
            // 清理ID生成器
            self.id_generators.remove(alias);
            self.mongo_auto_increment_generators.remove(alias);
            
            // 清理缓存管理器
            if let Some((_, cache_manager)) = self.cache_managers.remove(alias) {
                // 这里可以添加缓存清理逻辑
                info!("清理数据库 {} 的缓存管理器", alias);
            }
            
            info!("数据库配置已移除: 别名={}", alias);
            
            // 如果移除的是默认数据库，重新设置默认
            {
                let mut default_alias = self.default_alias.write().await;
                if default_alias.as_ref() == Some(&alias.to_string()) {
                    *default_alias = self.pools.iter().next().map(|entry| entry.key().clone());
                    if let Some(new_default) = default_alias.as_ref() {
                        info!("重新设置默认数据库别名: {}", new_default);
                    } else {
                        info!("没有可用的数据库，清空默认别名");
                    }
                }
            }
            
            Ok(())
        } else {
            Err(crate::quick_error!(alias_not_found, alias))
        }
    }

    /// 获取连接（使用指定别名）
    pub async fn get_connection(&self, alias: Option<&str>) -> QuickDbResult<PooledConnection> {
        let target_alias = match alias {
            Some(a) => a.to_string(),
            None => {
                // 使用默认别名
                let default_alias = self.default_alias.read().await;
                match default_alias.as_ref() {
                    Some(a) => a.clone(),
                    None => {
                        return Err(crate::quick_error!(config, "没有配置默认数据库别名"));
                    }
                }
            }
        };
        
        debug!("获取数据库连接: 别名={}", target_alias);
        
        if let Some(pool) = self.pools.get(&target_alias) {
            pool.get_connection().await
        } else {
            Err(crate::quick_error!(alias_not_found, target_alias))
        }
    }

    /// 释放连接
    pub async fn release_connection(&self, connection: &PooledConnection) -> QuickDbResult<()> {
        debug!("释放数据库连接: ID={}, 别名={}", connection.id, connection.alias);
        
        if let Some(pool) = self.pools.get(&connection.alias) {
            pool.release_connection(&connection.id).await
        } else {
            warn!("尝试释放连接到不存在的数据库别名: {}", connection.alias);
            Err(crate::quick_error!(alias_not_found, &connection.alias))
        }
    }

    /// 获取所有数据库别名
    pub fn get_aliases(&self) -> Vec<String> {
        self.pools.iter().map(|entry| entry.key().clone()).collect()
    }

    /// 获取默认数据库别名
    pub async fn get_default_alias(&self) -> Option<String> {
        self.default_alias.read().await.clone()
    }

    /// 设置默认数据库别名
    pub async fn set_default_alias(&self, alias: &str) -> QuickDbResult<()> {
        if self.pools.contains_key(alias) {
            let mut default_alias = self.default_alias.write().await;
            *default_alias = Some(alias.to_string());
            info!("设置默认数据库别名: {}", alias);
            Ok(())
        } else {
            Err(crate::quick_error!(alias_not_found, alias))
        }
    }



    /// 获取数据库类型
    pub fn get_database_type(&self, alias: &str) -> QuickDbResult<DatabaseType> {
        if let Some(pool) = self.pools.get(alias) {
            Ok(pool.get_database_type().clone())
        } else {
            Err(crate::quick_error!(alias_not_found, alias))
        }
    }

    /// 获取连接池映射的引用
    pub fn get_connection_pools(&self) -> Arc<DashMap<String, Arc<ConnectionPool>>> {
        self.pools.clone()
    }

    /// 获取ID生成器
    pub fn get_id_generator(&self, alias: &str) -> QuickDbResult<Arc<IdGenerator>> {
        if let Some(generator) = self.id_generators.get(alias) {
            Ok(generator.clone())
        } else {
            Err(crate::quick_error!(config, format!("数据库 {} 没有配置ID生成器", alias)))
        }
    }

    /// 获取MongoDB自增ID生成器
    pub fn get_mongo_auto_increment_generator(&self, alias: &str) -> QuickDbResult<Arc<MongoAutoIncrementGenerator>> {
        if let Some(generator) = self.mongo_auto_increment_generators.get(alias) {
            Ok(generator.clone())
        } else {
            Err(crate::quick_error!(config, format!("数据库 {} 没有MongoDB自增ID生成器", alias)))
        }
    }

    /// 获取缓存管理器
    pub fn get_cache_manager(&self, alias: &str) -> QuickDbResult<Arc<CacheManager>> {
        if let Some(cache_manager) = self.cache_managers.get(alias) {
            Ok(cache_manager.clone())
        } else {
            Err(crate::quick_error!(config, format!("数据库 {} 没有配置缓存管理器", alias)))
        }
    }

    /// 获取缓存统计信息
    pub async fn get_cache_stats(&self, alias: &str) -> QuickDbResult<CacheStats> {
        let cache_manager = self.get_cache_manager(alias)?;
        Ok(cache_manager.get_stats().await?)
    }

    /// 清理指定数据库的缓存
    pub async fn clear_cache(&self, alias: &str) -> QuickDbResult<()> {
        let cache_manager = self.get_cache_manager(alias)?;
        cache_manager.clear_all().await;
        info!("已清理数据库 {} 的缓存", alias);
        Ok(())
    }

    /// 清理所有数据库的缓存
    pub async fn clear_all_caches(&self) -> QuickDbResult<()> {
        for entry in self.cache_managers.iter() {
            let alias = entry.key();
            let cache_manager = entry.value();
            cache_manager.clear_all().await;
            info!("已清理数据库 {} 的缓存", alias);
        }
        Ok(())
    }



    /// 启动清理任务
    async fn start_cleanup_task(&self) {
        let mut cleanup_handle = self.cleanup_handle.write().await;
        
        // 如果清理任务已经在运行，不需要重复启动
        if cleanup_handle.is_some() {
            return;
        }
        
        let pools = self.pools.clone();
        
        let handle = tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(300)); // 每5分钟清理一次
            
            info!("启动连接池清理任务");
            
            loop {
                interval.tick().await;
                
                debug!("执行连接池清理任务");
                
                // 清理所有连接池的过期连接
                for entry in pools.iter() {
                    let alias = entry.key();
                    let pool = entry.value();
                    
                    debug!("清理连接池过期连接: 别名={}", alias);
                    pool.cleanup_expired_connections().await;
                }
                
                debug!("连接池清理任务完成");
            }
        });
        
        *cleanup_handle = Some(handle);
    }

    /// 停止清理任务
    pub async fn stop_cleanup_task(&self) {
        let mut cleanup_handle = self.cleanup_handle.write().await;
        
        if let Some(handle) = cleanup_handle.take() {
            handle.abort();
            info!("连接池清理任务已停止");
        }
    }

    /// 关闭所有连接池
    pub async fn shutdown(&self) -> QuickDbResult<()> {
        info!("开始关闭连接池管理器");

        // 停止清理任务
        self.stop_cleanup_task().await;

        // 清空所有连接池
        self.pools.clear();

        // 清空ID生成器
        self.id_generators.clear();
        self.mongo_auto_increment_generators.clear();

        // 清空缓存管理器
        self.cache_managers.clear();

        // 清空模型注册表
        self.model_registry.clear();

        // 清空索引创建锁
        {
            let mut locks = self.index_creation_locks.lock().await;
            locks.clear();
        }

        // 清空默认别名
        {
            let mut default_alias = self.default_alias.write().await;
            *default_alias = None;
        }

        info!("连接池管理器已关闭");
        Ok(())
    }

    /// 检查连接池健康状态
    pub async fn health_check(&self) -> std::collections::HashMap<String, bool> {
        let mut health_status = std::collections::HashMap::new();
        
        for entry in self.pools.iter() {
            let alias = entry.key().clone();
            let pool = entry.value();
            
            // 尝试获取连接来检查健康状态
            let is_healthy = match pool.get_connection().await {
                Ok(conn) => {
                    // 立即释放连接
                    let _ = pool.release_connection(&conn.id).await;
                    true
                }
                Err(_) => false,
            };
            
            health_status.insert(alias, is_healthy);
        }
        
        health_status
    }

    /// 获取所有活跃连接池的详细状态信息
    /// 
    /// 返回包含每个连接池状态的详细信息，包括：
    /// - 数据库别名
    /// - 数据库类型
    /// - 连接池配置信息
    /// - 健康状态
    /// - 缓存状态（如果启用）
    pub async fn get_active_pools_status(&self) -> std::collections::HashMap<String, serde_json::Value> {
        use serde_json::json;
        let mut pools_status = std::collections::HashMap::new();
        
        info!("获取所有活跃连接池状态，当前池数量: {}", self.pools.len());
        
        for entry in self.pools.iter() {
            let alias = entry.key().clone();
            let pool = entry.value();
            
            // 获取数据库类型
            let db_type = pool.get_database_type();
            
            // 检查健康状态
            let is_healthy = match pool.get_connection().await {
                Ok(conn) => {
                    let _ = pool.release_connection(&conn.id).await;
                    true
                }
                Err(e) => {
                    warn!("连接池 {} 健康检查失败: {}", alias, e);
                    false
                }
            };
            
            // 获取缓存状态（如果存在）
            let cache_info = if let Some(cache_manager) = self.cache_managers.get(&alias) {
                match cache_manager.get_stats().await {
                    Ok(stats) => json!({
                        "enabled": true,
                        "entries": stats.entries,
                        "memory_usage_bytes": stats.memory_usage_bytes,
                        "disk_usage_bytes": stats.disk_usage_bytes,
                        "hit_rate": stats.hit_rate,
                        "hits": stats.hits,
                        "misses": stats.misses
                    }),
                    Err(_) => json!({
                        "enabled": true,
                        "error": "无法获取缓存统计信息"
                    })
                }
            } else {
                json!({
                    "enabled": false
                })
            };
            
            // 构建连接池状态信息
            let pool_status = json!({
                "alias": alias,
                "database_type": format!("{:?}", db_type),
                "is_healthy": is_healthy,
                "pool_config": {
                    "min_connections": pool.config.base.min_connections,
                    "max_connections": pool.config.base.max_connections,
                    "connection_timeout": pool.config.base.connection_timeout,
                    "idle_timeout": pool.config.base.idle_timeout,
                    "max_lifetime": pool.config.base.max_lifetime,
                    "max_retries": pool.config.max_retries,
                    "retry_interval_ms": pool.config.retry_interval_ms,
                    "keepalive_interval_sec": pool.config.keepalive_interval_sec,
                    "health_check_timeout_sec": pool.config.health_check_timeout_sec
                },
                "cache": cache_info,
                "has_id_generator": self.id_generators.contains_key(&alias),
                "has_mongo_auto_increment": self.mongo_auto_increment_generators.contains_key(&alias)
            });
            
            pools_status.insert(alias.clone(), pool_status);
            debug!("连接池 {} 状态已收集", alias);
        }
        
        // 获取默认别名信息
        let default_alias = self.default_alias.read().await.clone();
        if let Some(default) = &default_alias {
            info!("当前默认数据库别名: {}", default);
        } else {
            info!("未设置默认数据库别名");
        }
        
        info!("活跃连接池状态收集完成，共 {} 个连接池", pools_status.len());
        pools_status
    }

    /// 注册模型元数据
    pub fn register_model(&self, model_meta: ModelMeta) -> QuickDbResult<()> {
        let collection_name = model_meta.collection_name.clone();

        // 检查是否已注册
        if self.model_registry.contains_key(&collection_name) {
            debug!("模型已存在，将更新元数据: {}", collection_name);
        }

        self.model_registry.insert(collection_name.clone(), model_meta.clone());
        debug!("注册模型元数据: 集合={}, 索引数量={}", collection_name, model_meta.indexes.len());

        Ok(())
    }

    /// 获取模型元数据
    pub fn get_model(&self, collection_name: &str) -> Option<ModelMeta> {
        self.model_registry.get(collection_name).map(|meta| meta.clone())
    }

    /// 检查模型是否已注册
    pub fn has_model(&self, collection_name: &str) -> bool {
        self.model_registry.contains_key(collection_name)
    }

    /// 获取所有已注册的模型
    pub fn get_registered_models(&self) -> Vec<(String, ModelMeta)> {
        self.model_registry
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect()
    }

    /// 获取索引创建锁
    async fn acquire_index_lock(&self, table: &str, index: &str) -> tokio::sync::MutexGuard<'_, HashMap<String, HashMap<String, ()>>> {
        let mut locks = self.index_creation_locks.lock().await;
        if !locks.contains_key(table) {
            locks.insert(table.to_string(), HashMap::new());
        }
        let table_locks = locks.get_mut(table).unwrap();
        if !table_locks.contains_key(index) {
            table_locks.insert(index.to_string(), ());
            debug!("🔒 获取表 {} 索引 {} 的创建锁", table, index);
        }
        locks
    }

    /// 释放索引创建锁
    fn release_index_lock(&self, table: &str, index: &str, mut locks: tokio::sync::MutexGuard<'_, HashMap<String, HashMap<String, ()>>>) {
        if let Some(table_locks) = locks.get_mut(table) {
            table_locks.remove(index);
            if table_locks.is_empty() {
                locks.remove(table);
            }
        }
        debug!("🔓 释放表 {} 索引 {} 的创建锁", table, index);
    }

    /// 创建表和索引（基于注册的模型元数据）
    pub async fn ensure_table_and_indexes(&self, collection_name: &str, alias: &str) -> QuickDbResult<()> {
        if let Some(model_meta) = self.get_model(collection_name) {
            debug!("为集合 {} 创建表和索引", collection_name);

            // 获取连接池
            if let Some(pool) = self.pools.get(alias) {
                // 创建表（如果不存在）
                let fields: HashMap<String, crate::model::FieldDefinition> = model_meta.fields.iter()
                    .map(|(name, field_def)| (name.clone(), field_def.clone()))
                    .collect();

                // 检查表是否存在
                let table_exists = pool.table_exists(&collection_name).await?;
                if !table_exists {
                    info!("表 {} 不存在，正在创建", collection_name);
                    pool.create_table(&collection_name, &fields, &pool.db_config.id_strategy).await?;
                    info!("✅ 创建表成功: {}", collection_name);
                }

                // 创建索引
                for index in &model_meta.indexes {
                    let default_name = format!("idx_{}", index.fields.join("_"));
                    let index_name = index.name.as_deref().unwrap_or(&default_name);
                    debug!("创建索引: {} (字段: {:?}, 唯一: {})", index_name, index.fields, index.unique);

                    // 获取索引创建锁，防止并发创建同一个索引
                    let _lock = self.acquire_index_lock(&collection_name, index_name).await;

                    // 双重检查：再次检查索引是否可能已存在
                    // 这里我们直接尝试创建，因为数据库层面会报错，我们捕获错误即可
                    if let Err(e) = pool.create_index(
                        &collection_name,
                        index_name,
                        &index.fields,
                        index.unique
                    ).await {
                        // 如果是索引已存在的错误，忽略它
                        let error_msg = e.to_string().to_lowercase();
                        if error_msg.contains("duplicate") ||
                           error_msg.contains("already exists") ||
                           error_msg.contains("already exist") ||
                           error_msg.contains("already") ||
                           error_msg.contains("exists") {
                            debug!("索引 {} 已存在，跳过创建", index_name);
                        } else {
                            warn!("创建索引失败: {} (错误: {})", index_name, e);
                        }
                    } else {
                        debug!("✅ 创建索引成功: {}", index_name);
                    }
                }
            } else {
                return Err(QuickDbError::AliasNotFound {
                    alias: alias.to_string(),
                });
            }
        } else {
            debug!("集合 {} 没有注册的模型元数据，跳过表和索引创建", collection_name);
        }

        Ok(())
    }
}

impl Default for PoolManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 全局连接池管理器实例
pub static GLOBAL_POOL_MANAGER: once_cell::sync::Lazy<PoolManager> = 
    once_cell::sync::Lazy::new(|| PoolManager::new());

/// 获取全局连接池管理器
pub(crate) fn get_global_pool_manager() -> &'static PoolManager {
    &GLOBAL_POOL_MANAGER
}

/// 便捷函数 - 添加数据库配置
pub async fn add_database(config: DatabaseConfig) -> QuickDbResult<()> {
    get_global_pool_manager().add_database(config).await
}

/// 便捷函数 - 移除数据库配置
pub async fn remove_database(alias: &str) -> QuickDbResult<()> {
    get_global_pool_manager().remove_database(alias).await
}

/// 便捷函数 - 获取连接
pub async fn get_connection(alias: Option<&str>) -> QuickDbResult<PooledConnection> {
    get_global_pool_manager().get_connection(alias).await
}

/// 便捷函数 - 释放连接
pub async fn release_connection(connection: &PooledConnection) -> QuickDbResult<()> {
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
    get_global_pool_manager().health_check().await
}

/// 便捷函数 - 获取所有活跃连接池的详细状态信息
pub async fn get_active_pools_status() -> std::collections::HashMap<String, serde_json::Value> {
    get_global_pool_manager().get_active_pools_status().await
}

#[cfg(feature = "python-bindings")]
#[doc(hidden)]
/// 便捷函数 - 获取连接池映射（仅用于Python绑定，不推荐直接使用）
pub fn get_connection_pools() -> Arc<DashMap<String, Arc<ConnectionPool>>> {
    get_global_pool_manager().get_connection_pools()
}

/// 便捷函数 - 获取ID生成器
pub fn get_id_generator(alias: &str) -> QuickDbResult<Arc<IdGenerator>> {
    get_global_pool_manager().get_id_generator(alias)
}

/// 便捷函数 - 获取MongoDB自增ID生成器
pub fn get_mongo_auto_increment_generator(alias: &str) -> QuickDbResult<Arc<MongoAutoIncrementGenerator>> {
    get_global_pool_manager().get_mongo_auto_increment_generator(alias)
}

/// 便捷函数 - 注册模型元数据
pub fn register_model(model_meta: ModelMeta) -> QuickDbResult<()> {
    get_global_pool_manager().register_model(model_meta)
}

/// 便捷函数 - 获取模型元数据
pub fn get_model(collection_name: &str) -> Option<ModelMeta> {
    get_global_pool_manager().get_model(collection_name)
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
    get_global_pool_manager().get_cache_stats(alias).await
}

/// 便捷函数 - 清理指定数据库的缓存
pub async fn clear_cache(alias: &str) -> QuickDbResult<()> {
    get_global_pool_manager().clear_cache(alias).await
}

/// 便捷函数 - 清理所有数据库的缓存
pub async fn clear_all_caches() -> QuickDbResult<()> {
    get_global_pool_manager().clear_all_caches().await
}

/// 便捷函数 - 按模式清理缓存
/// 
/// # 参数
/// * `alias` - 数据库别名
/// * `pattern` - 缓存键模式，支持通配符 * 和 ?
/// 
/// # 示例
/// ```no_run
/// # use rat_quickdb::manager::clear_cache_by_pattern;
/// # async fn example() -> rat_quickdb::QuickDbResult<()> {
/// // 清理所有用户表相关的缓存
/// clear_cache_by_pattern("main", "rat_quickdb:users:*").await?;
/// // 清理所有查询缓存
/// clear_cache_by_pattern("main", "*:query:*").await?;
/// # Ok(())
/// # }
/// ```
pub async fn clear_cache_by_pattern(alias: &str, pattern: &str) -> QuickDbResult<usize> {
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
    let cache_manager = get_global_pool_manager().get_cache_manager(alias)?;
    cache_manager.clear_records_batch(table, ids).await
        .map_err(|e| QuickDbError::CacheError { message: e.to_string() })
}

/// 便捷函数 - 强制清理过期缓存
/// 
/// 手动触发过期缓存的清理，通常用于内存紧张或需要立即释放空间的场景
pub async fn force_cleanup_expired_cache(alias: &str) -> QuickDbResult<usize> {
    let cache_manager = get_global_pool_manager().get_cache_manager(alias)?;
    cache_manager.force_cleanup_expired().await
        .map_err(|e| QuickDbError::CacheError { message: e.to_string() })
}

/// 便捷函数 - 获取所有缓存键列表（按表分组）
/// 
/// 用于调试和监控，可以查看当前缓存中有哪些键
pub async fn list_cache_keys(alias: &str) -> QuickDbResult<std::collections::HashMap<String, Vec<String>>> {
    let cache_manager = get_global_pool_manager().get_cache_manager(alias)?;
    cache_manager.list_cache_keys().await
        .map_err(|e| QuickDbError::CacheError { message: e.to_string() })
}

/// 便捷函数 - 获取指定表的缓存键列表
pub async fn list_table_cache_keys(alias: &str, table: &str) -> QuickDbResult<Vec<String>> {
    let cache_manager = get_global_pool_manager().get_cache_manager(alias)?;
    cache_manager.list_table_cache_keys(table).await
        .map_err(|e| QuickDbError::CacheError { message: e.to_string() })
}

/// 便捷函数 - 清理指定表的查询缓存
/// 
/// 只清理查询缓存，保留记录缓存
pub async fn clear_table_query_cache(alias: &str, table: &str) -> QuickDbResult<usize> {
    let cache_manager = get_global_pool_manager().get_cache_manager(alias)?;
    cache_manager.clear_table_query_cache(table).await
        .map_err(|e| QuickDbError::CacheError { message: e.to_string() })
}

/// 便捷函数 - 清理指定表的记录缓存
/// 
/// 只清理记录缓存，保留查询缓存
pub async fn clear_table_record_cache(alias: &str, table: &str) -> QuickDbResult<usize> {
    let cache_manager = get_global_pool_manager().get_cache_manager(alias)?;
    cache_manager.clear_table_record_cache(table).await
        .map_err(|e| QuickDbError::CacheError { message: e.to_string() })
}

/// 便捷函数 - 清理指定表的所有缓存（记录+查询）
pub async fn clear_table_all_cache(alias: &str, table: &str) -> QuickDbResult<usize> {
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
/// # 示例
/// ```no_run
/// # use rat_quickdb::manager::table_exists;
/// # async fn example() -> rat_quickdb::QuickDbResult<()> {
/// // 检查表是否存在
/// let exists = table_exists("main", "test_users").await?;
/// if exists {
///     println!("表存在");
/// } else {
///     println!("表不存在");
/// }
/// # Ok(())
/// # }
/// ```
pub async fn table_exists(alias: &str, table: &str) -> QuickDbResult<bool> {
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
/// # 示例
/// ```no_run
/// # use rat_quickdb::manager::drop_table;
/// # async fn example() -> rat_quickdb::QuickDbResult<()> {
/// // 删除表（如果存在）
/// drop_table("main", "test_users").await?;
/// # Ok(())
/// # }
/// ```
pub async fn drop_table(alias: &str, table: &str) -> QuickDbResult<()> {
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

/// 便捷函数 - 关闭管理器
pub async fn shutdown() -> QuickDbResult<()> {
    get_global_pool_manager().shutdown().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;

    fn create_test_config(alias: &str) -> DatabaseConfig {
        DatabaseConfig {
            db_type: DatabaseType::SQLite,
            connection: ConnectionConfig::SQLite {
                path: ":memory:".to_string(),
                create_if_missing: true,
            },
            pool: PoolConfig::default(),
            alias: alias.to_string(),
            id_strategy: IdStrategy::AutoIncrement,
            cache: None,
        }
    }

    #[tokio::test]
    async fn test_manager_creation() {
        let manager = PoolManager::new();
        assert!(manager.get_aliases().is_empty());
        assert!(manager.get_default_alias().await.is_none());
    }

    #[tokio::test]
    async fn test_add_database() {
        let manager = PoolManager::new();
        let config = create_test_config("test_db");
        
        let result = manager.add_database(config).await;
        assert!(result.is_ok());
        
        let aliases = manager.get_aliases();
        assert_eq!(aliases.len(), 1);
        assert!(aliases.contains(&"test_db".to_string()));
        
        let default_alias = manager.get_default_alias().await;
        assert_eq!(default_alias, Some("test_db".to_string()));
    }

    #[tokio::test]
    async fn test_get_connection() {
        let manager = PoolManager::new();
        let config = create_test_config("test_db");
        
        manager.add_database(config).await.unwrap();
        
        // 测试使用默认别名获取连接
        let conn = manager.get_connection(None).await;
        assert!(conn.is_ok());
        
        // 测试使用指定别名获取连接
        let conn = manager.get_connection(Some("test_db")).await;
        assert!(conn.is_ok());
        
        // 测试使用不存在的别名
        let conn = manager.get_connection(Some("nonexistent")).await;
        assert!(conn.is_err());
    }

    #[tokio::test]
    async fn test_remove_database() {
        let manager = PoolManager::new();
        let config = create_test_config("test_db");
        
        manager.add_database(config).await.unwrap();
        assert_eq!(manager.get_aliases().len(), 1);
        
        let result = manager.remove_database("test_db").await;
        assert!(result.is_ok());
        assert!(manager.get_aliases().is_empty());
        
        // 测试移除不存在的数据库
        let result = manager.remove_database("nonexistent").await;
        assert!(result.is_err());
    }
}