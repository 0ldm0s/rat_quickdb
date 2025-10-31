    //! 模型操作相关方法

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
                    debug!("表 {} 不存在，正在创建", collection_name);
                    pool.create_table(&collection_name, &fields, &pool.db_config.id_strategy).await?;
                    debug!("✅ 创建表成功: {}", collection_name);
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
