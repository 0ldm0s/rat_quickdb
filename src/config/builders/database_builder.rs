//! # 数据库配置构建器模块
//!
//! 提供数据库配置的构建器实现，支持链式调用和严格验证

use crate::error::QuickDbError;
use crate::types::*;
use rat_logger::info;
use std::path::PathBuf;

/// 数据库配置构建器
///
/// 严格要求所有配置项必须显式设置，严禁使用默认值
#[derive(Debug)]
pub struct DatabaseConfigBuilder {
    db_type: Option<DatabaseType>,
    connection: Option<ConnectionConfig>,
    pool: Option<PoolConfig>,
    alias: Option<String>,
    /// 缓存配置（可选）
    cache: Option<CacheConfig>,
    /// ID 生成策略
    id_strategy: Option<IdStrategy>,
}
impl DatabaseConfig {
    /// 创建数据库配置构建器
    pub fn builder() -> DatabaseConfigBuilder {
        DatabaseConfigBuilder::new()
    }
}

impl DatabaseConfigBuilder {
    /// 创建新的构建器
    pub fn new() -> Self {
        Self {
            db_type: None,
            connection: None,
            pool: None,
            alias: None,
            cache: None,
            id_strategy: None,
        }
    }

    /// 设置数据库类型
    ///
    /// # 参数
    ///
    /// * `db_type` - 数据库类型
    pub fn db_type(mut self, db_type: DatabaseType) -> Self {
        self.db_type = Some(db_type);
        self
    }

    /// 设置连接配置
    ///
    /// # 参数
    ///
    /// * `connection` - 连接配置
    pub fn connection(mut self, connection: ConnectionConfig) -> Self {
        self.connection = Some(connection);
        self
    }

    /// 设置连接池配置
    ///
    /// # 参数
    ///
    /// * `pool` - 连接池配置
    pub fn pool(mut self, pool: PoolConfig) -> Self {
        self.pool = Some(pool);
        self
    }

    /// 设置数据库别名
    ///
    /// # 参数
    ///
    /// * `alias` - 数据库别名
    pub fn alias<S: Into<String>>(mut self, alias: S) -> Self {
        self.alias = Some(alias.into());
        self
    }

    /// 设置ID生成策略
    ///
    /// # 参数
    ///
    /// * `id_strategy` - ID生成策略
    pub fn id_strategy(mut self, id_strategy: IdStrategy) -> Self {
        self.id_strategy = Some(id_strategy);
        self
    }

    /// 设置缓存配置
    ///
    /// # 参数
    ///
    /// * `cache` - 缓存配置
    pub fn cache(mut self, cache: CacheConfig) -> Self {
        self.cache = Some(cache);
        self
    }

    /// 禁用缓存
    pub fn disable_cache(mut self) -> Self {
        let cache_config = CacheConfig {
            enabled: false, // 禁用缓存
            strategy: CacheStrategy::Lru,
            ttl_config: TtlConfig {
                default_ttl_secs: 300,
                max_ttl_secs: 3600,
                check_interval_secs: 60,
            },
            l1_config: L1CacheConfig {
                max_capacity: 100,
                max_memory_mb: 16,
                enable_stats: false,
            },
            l2_config: None,
            compression_config: CompressionConfig {
                enabled: false,
                algorithm: CompressionAlgorithm::Lz4,
                threshold_bytes: 1024,
            },
            version: "v1".to_string(),
        };
        self.cache = Some(cache_config);
        self
    }

    /// 构建数据库配置
    ///
    /// # 错误
    ///
    /// 如果任何必需的配置项未设置，将返回错误
    pub fn build(self) -> Result<DatabaseConfig, QuickDbError> {
        let db_type = self
            .db_type
            .ok_or_else(|| crate::quick_error!(config, "数据库类型必须设置"))?;

        let connection = self
            .connection
            .ok_or_else(|| crate::quick_error!(config, "连接配置必须设置"))?;

        let pool = self
            .pool
            .ok_or_else(|| crate::quick_error!(config, "连接池配置必须设置"))?;

        let alias = self
            .alias
            .ok_or_else(|| crate::quick_error!(config, "数据库别名必须设置"))?;

        let id_strategy = self
            .id_strategy
            .ok_or_else(|| crate::quick_error!(config, "ID生成策略必须设置"))?;

        // 验证配置的一致性
        Self::validate_config(&db_type, &connection)?;

        info!("创建数据库配置: 别名={}, 类型={:?}", alias, db_type);

        Ok(DatabaseConfig {
            db_type,
            connection,
            pool,
            alias,
            cache: self.cache,
            id_strategy,
        })
    }

    /// 验证配置的一致性
    fn validate_config(
        db_type: &DatabaseType,
        connection: &ConnectionConfig,
    ) -> Result<(), QuickDbError> {
        match (db_type, connection) {
            (DatabaseType::SQLite, ConnectionConfig::SQLite { .. }) => Ok(()),
            (DatabaseType::PostgreSQL, ConnectionConfig::PostgreSQL { .. }) => Ok(()),
            (DatabaseType::MySQL, ConnectionConfig::MySQL { .. }) => Ok(()),
            (DatabaseType::MongoDB, ConnectionConfig::MongoDB { .. }) => Ok(()),
            _ => Err(crate::quick_error!(
                config,
                format!("数据库类型 {:?} 与连接配置不匹配", db_type)
            )),
        }
    }
}
impl Default for DatabaseConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}
