
//! 维护操作相关方法

use crate::cache::{CacheManager, CacheStats};
use crate::error::{QuickDbError, QuickDbResult};
use crate::id_generator::{IdGenerator, MongoAutoIncrementGenerator};
use crate::model::ModelMeta;
use crate::pool::{ConnectionPool, ExtendedPoolConfig, PooledConnection};
use crate::types::{DatabaseConfig, DatabaseType, IdType};
use dashmap::DashMap;
use rat_logger::{debug, error, info, warn};
use serde_json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{Duration, interval};

use super::PoolManager;

impl PoolManager {
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
    pub async fn get_active_pools_status(
        &self,
    ) -> std::collections::HashMap<String, serde_json::Value> {
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
                    }),
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
}
