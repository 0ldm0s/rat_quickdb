//! 连接池管理器核心定义

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
    pub(crate) pools: Arc<DashMap<String, Arc<ConnectionPool>>>,
    /// 默认数据库别名
    pub(crate) default_alias: Arc<RwLock<Option<String>>>,
    /// 清理任务句柄
    pub(crate) cleanup_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    /// ID生成器映射 (别名 -> ID生成器)
    pub(crate) id_generators: Arc<DashMap<String, Arc<IdGenerator>>>,
    /// MongoDB自增ID生成器映射 (别名 -> 自增生成器)
    pub(crate) mongo_auto_increment_generators: Arc<DashMap<String, Arc<MongoAutoIncrementGenerator>>>,
    /// 缓存管理器映射 (别名 -> 缓存管理器)
    pub(crate) cache_managers: Arc<DashMap<String, Arc<CacheManager>>>,
    /// 模型元数据注册表 (集合名 -> 模型元数据)
    pub(crate) model_registry: Arc<DashMap<String, ModelMeta>>,
    /// 索引创建锁，防止并发创建同一个索引 (表名 -> 索引名 -> ())
    pub(crate) index_creation_locks: Arc<tokio::sync::Mutex<HashMap<String, HashMap<String, ()>>>>,
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
}

