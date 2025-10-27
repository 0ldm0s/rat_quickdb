    //! 缓存操作相关方法

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
    pub(crate) async fn start_cleanup_task(&self) {
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
}
