  //! 数据库操作相关方法

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

use super::PoolManager;

impl PoolManager {
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

    /// 获取数据库连接
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

    /// 获取连接池映射的引用
    pub fn get_connection_pools(&self) -> Arc<DashMap<String, Arc<ConnectionPool>>> {
        self.pools.clone()
    }
}
