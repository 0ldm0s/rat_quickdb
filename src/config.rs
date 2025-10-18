//! # 配置管理模块
//!
//! 提供统一的配置管理系统，支持构建器模式和链式配置
//! 严格遵循项目规范：所有配置项必须显式设置，严禁使用默认值

use crate::error::QuickDbError;
use crate::types::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use rat_logger::{info, warn, error};

/// 全局配置管理器
/// 
/// 负责管理整个应用的配置，包括数据库配置、日志配置等
#[derive(Debug, Clone)]
pub struct GlobalConfig {
    /// 数据库配置映射 (别名 -> 配置)
    pub databases: HashMap<String, DatabaseConfig>,
    /// 默认数据库别名
    pub default_database: Option<String>,
    /// 应用配置
    pub app: AppConfig,
    /// 日志配置
    pub logging: LoggingConfig,
}

/// 应用配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// 应用名称
    pub name: String,
    /// 应用版本
    pub version: String,
    /// 环境类型
    pub environment: Environment,
    /// 是否启用调试模式
    pub debug: bool,
    /// 工作目录
    pub work_dir: PathBuf,
}

/// 环境类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Environment {
    /// 开发环境
    Development,
    /// 测试环境
    Testing,
    /// 预发布环境
    Staging,
    /// 生产环境
    Production,
}

/// 日志配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// 日志级别
    pub level: LogLevel,
    /// 是否输出到控制台
    pub console: bool,
    /// 日志文件路径
    pub file_path: Option<PathBuf>,
    /// 日志文件最大大小（字节）
    pub max_file_size: u64,
    /// 保留的日志文件数量
    pub max_files: u32,
    /// 是否启用结构化日志
    pub structured: bool,
}

/// 日志级别
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogLevel {
    /// 错误级别
    Error,
    /// 警告级别
    Warn,
    /// 信息级别
    Info,
    /// 调试级别
    Debug,
    /// 跟踪级别
    Trace,
}

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

/// 连接池配置构建器
/// 
/// 严格要求所有配置项必须显式设置，严禁使用默认值
#[derive(Debug)]
pub struct PoolConfigBuilder {
    min_connections: Option<u32>,
    max_connections: Option<u32>,
    connection_timeout: Option<u64>,
    idle_timeout: Option<u64>,
    max_lifetime: Option<u64>,
}

/// 全局配置构建器
/// 
/// 提供链式配置接口，支持流畅的API调用
#[derive(Debug)]
pub struct GlobalConfigBuilder {
    databases: HashMap<String, DatabaseConfig>,
    default_database: Option<String>,
    app: Option<AppConfig>,
    logging: Option<LoggingConfig>,
}

/// 应用配置构建器
#[derive(Debug)]
pub struct AppConfigBuilder {
    name: Option<String>,
    version: Option<String>,
    environment: Option<Environment>,
    debug: Option<bool>,
    work_dir: Option<PathBuf>,
}

/// 日志配置构建器
#[derive(Debug)]
pub struct LoggingConfigBuilder {
    level: Option<LogLevel>,
    console: Option<bool>,
    file_path: Option<PathBuf>,
    max_file_size: Option<u64>,
    max_files: Option<u32>,
    structured: Option<bool>,
}

// ============================================================================
// DatabaseConfig 实现
// ============================================================================

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
        let db_type = self.db_type.ok_or_else(|| {
            crate::quick_error!(config, "数据库类型必须设置")
        })?;
        
        let connection = self.connection.ok_or_else(|| {
            crate::quick_error!(config, "连接配置必须设置")
        })?;
        
        let pool = self.pool.ok_or_else(|| {
            crate::quick_error!(config, "连接池配置必须设置")
        })?;
        
        let alias = self.alias.ok_or_else(|| {
            crate::quick_error!(config, "数据库别名必须设置")
        })?;

        let id_strategy = self.id_strategy.ok_or_else(|| {
            crate::quick_error!(config, "ID生成策略必须设置")
        })?;

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
    fn validate_config(db_type: &DatabaseType, connection: &ConnectionConfig) -> Result<(), QuickDbError> {
        match (db_type, connection) {
            (DatabaseType::SQLite, ConnectionConfig::SQLite { .. }) => Ok(()),
            (DatabaseType::PostgreSQL, ConnectionConfig::PostgreSQL { .. }) => Ok(()),
            (DatabaseType::MySQL, ConnectionConfig::MySQL { .. }) => Ok(()),
            (DatabaseType::MongoDB, ConnectionConfig::MongoDB { .. }) => Ok(()),
            _ => Err(crate::quick_error!(config, 
                format!("数据库类型 {:?} 与连接配置不匹配", db_type)
            )),
        }
    }
}

// ============================================================================
// PoolConfig 实现
// ============================================================================

impl PoolConfig {
    /// 创建连接池配置构建器
    pub fn builder() -> PoolConfigBuilder {
        PoolConfigBuilder::new()
    }
}

impl PoolConfigBuilder {
    /// 创建新的构建器
    pub fn new() -> Self {
        Self {
            min_connections: None,
            max_connections: None,
            connection_timeout: None,
            idle_timeout: None,
            max_lifetime: None,
        }
    }

    /// 设置最小连接数
    /// 
    /// # 参数
    /// 
    /// * `min_connections` - 最小连接数
    pub fn min_connections(mut self, min_connections: u32) -> Self {
        self.min_connections = Some(min_connections);
        self
    }

    /// 设置最大连接数
    /// 
    /// # 参数
    /// 
    /// * `max_connections` - 最大连接数
    pub fn max_connections(mut self, max_connections: u32) -> Self {
        self.max_connections = Some(max_connections);
        self
    }

    /// 设置连接超时时间（秒）
    /// 
    /// # 参数
    /// 
    /// * `timeout` - 连接超时时间（秒）
    pub fn connection_timeout(mut self, timeout: u64) -> Self {
        self.connection_timeout = Some(timeout);
        self
    }

    /// 设置空闲连接超时时间（秒）
    /// 
    /// # 参数
    /// 
    /// * `timeout` - 空闲连接超时时间（秒）
    pub fn idle_timeout(mut self, timeout: u64) -> Self {
        self.idle_timeout = Some(timeout);
        self
    }

    /// 设置连接最大生存时间（秒）
    /// 
    /// # 参数
    /// 
    /// * `lifetime` - 连接最大生存时间（秒）
    pub fn max_lifetime(mut self, lifetime: u64) -> Self {
        self.max_lifetime = Some(lifetime);
        self
    }

    /// 构建连接池配置
    /// 
    /// # 错误
    /// 
    /// 如果任何必需的配置项未设置，将返回错误
    pub fn build(self) -> Result<PoolConfig, QuickDbError> {
        let min_connections = self.min_connections.ok_or_else(|| {
            crate::quick_error!(config, "最小连接数必须设置")
        })?;
        
        let max_connections = self.max_connections.ok_or_else(|| {
            crate::quick_error!(config, "最大连接数必须设置")
        })?;
        
        let connection_timeout = self.connection_timeout.ok_or_else(|| {
            crate::quick_error!(config, "连接超时时间必须设置")
        })?;
        
        let idle_timeout = self.idle_timeout.ok_or_else(|| {
            crate::quick_error!(config, "空闲连接超时时间必须设置")
        })?;
        
        let max_lifetime = self.max_lifetime.ok_or_else(|| {
            crate::quick_error!(config, "连接最大生存时间必须设置")
        })?;

        // 验证配置的合理性
        if min_connections > max_connections {
            return Err(crate::quick_error!(config, "最小连接数不能大于最大连接数"));
        }

        if connection_timeout == 0 {
            return Err(crate::quick_error!(config, "连接超时时间不能为零"));
        }

        if idle_timeout == 0 {
            return Err(crate::quick_error!(config, "空闲连接超时时间不能为零"));
        }

        if max_lifetime == 0 {
            return Err(crate::quick_error!(config, "连接最大生存时间不能为零"));
        }

        info!("创建连接池配置: 最小连接数={}, 最大连接数={}, 连接超时={}s", 
              min_connections, max_connections, connection_timeout);

        Ok(PoolConfig {
            min_connections,
            max_connections,
            connection_timeout,
            idle_timeout,
            max_lifetime,
        })
    }
}

// ============================================================================
// GlobalConfig 实现
// ============================================================================

impl GlobalConfig {
    /// 创建全局配置构建器
    pub fn builder() -> GlobalConfigBuilder {
        GlobalConfigBuilder::new()
    }

    /// 从配置文件加载配置
    /// 
    /// # 参数
    /// 
    /// * `config_path` - 配置文件路径
    pub fn from_file<P: AsRef<std::path::Path>>(config_path: P) -> Result<Self, QuickDbError> {
        let content = std::fs::read_to_string(config_path.as_ref())
            .map_err(|e| QuickDbError::IoError(e))?;

        let config: GlobalConfig = if config_path.as_ref().extension().and_then(|s| s.to_str()) == Some("toml") {
            toml::from_str(&content)
                .map_err(|e| crate::quick_error!(config, format!("解析TOML配置文件失败: {}", e)))?
        } else {
            serde_json::from_str(&content)
                .map_err(|e| crate::quick_error!(config, format!("解析JSON配置文件失败: {}", e)))?
        };

        info!("从文件加载配置: {:?}", config_path.as_ref());
        Ok(config)
    }

    /// 保存配置到文件
    /// 
    /// # 参数
    /// 
    /// * `config_path` - 配置文件路径
    pub fn save_to_file<P: AsRef<std::path::Path>>(&self, config_path: P) -> Result<(), QuickDbError> {
        let content = if config_path.as_ref().extension().and_then(|s| s.to_str()) == Some("toml") {
            toml::to_string_pretty(self)
                .map_err(|e| crate::quick_error!(config, format!("序列化TOML配置失败: {}", e)))?
        } else {
            serde_json::to_string_pretty(self)
                .map_err(|e| crate::quick_error!(config, format!("序列化JSON配置失败: {}", e)))?
        };

        std::fs::write(config_path.as_ref(), content)
            .map_err(|e| QuickDbError::IoError(e))?;

        info!("保存配置到文件: {:?}", config_path.as_ref());
        Ok(())
    }

    /// 获取默认数据库配置
    pub fn get_default_database(&self) -> Result<&DatabaseConfig, QuickDbError> {
        let alias = self.default_database.as_ref()
            .ok_or_else(|| crate::quick_error!(config, "未设置默认数据库"))?;
        
        self.databases.get(alias)
            .ok_or_else(|| crate::quick_error!(config, format!("找不到默认数据库配置: {}", alias)))
    }

    /// 获取指定别名的数据库配置
    /// 
    /// # 参数
    /// 
    /// * `alias` - 数据库别名
    pub fn get_database(&self, alias: &str) -> Result<&DatabaseConfig, QuickDbError> {
        self.databases.get(alias)
            .ok_or_else(|| crate::quick_error!(config, format!("找不到数据库配置: {}", alias)))
    }
}

impl GlobalConfigBuilder {
    /// 创建新的构建器
    pub fn new() -> Self {
        Self {
            databases: HashMap::new(),
            default_database: None,
            app: None,
            logging: None,
        }
    }

    /// 添加数据库配置
    /// 
    /// # 参数
    /// 
    /// * `config` - 数据库配置
    pub fn add_database(mut self, config: DatabaseConfig) -> Self {
        let alias = config.alias.clone();
        self.databases.insert(alias, config);
        self
    }

    /// 设置默认数据库
    /// 
    /// # 参数
    /// 
    /// * `alias` - 数据库别名
    pub fn default_database<S: Into<String>>(mut self, alias: S) -> Self {
        self.default_database = Some(alias.into());
        self
    }

    /// 设置应用配置
    /// 
    /// # 参数
    /// 
    /// * `app` - 应用配置
    pub fn app(mut self, app: AppConfig) -> Self {
        self.app = Some(app);
        self
    }

    /// 设置日志配置
    /// 
    /// # 参数
    /// 
    /// * `logging` - 日志配置
    pub fn logging(mut self, logging: LoggingConfig) -> Self {
        self.logging = Some(logging);
        self
    }

    /// 构建全局配置
    /// 
    /// # 错误
    /// 
    /// 如果任何必需的配置项未设置，将返回错误
    pub fn build(self) -> Result<GlobalConfig, QuickDbError> {
        if self.databases.is_empty() {
            return Err(crate::quick_error!(config, "至少需要配置一个数据库"));
        }

        let app = self.app.ok_or_else(|| {
            crate::quick_error!(config, "应用配置必须设置")
        })?;
        
        let logging = self.logging.ok_or_else(|| {
            crate::quick_error!(config, "日志配置必须设置")
        })?;

        // 验证默认数据库是否存在
        if let Some(ref default_alias) = self.default_database {
            if !self.databases.contains_key(default_alias) {
                return Err(crate::quick_error!(config, 
                    format!("默认数据库 '{}' 不存在于数据库配置中", default_alias)
                ));
            }
        }

        info!("创建全局配置: 数据库数量={}, 默认数据库={:?}", 
              self.databases.len(), self.default_database);

        Ok(GlobalConfig {
            databases: self.databases,
            default_database: self.default_database,
            app,
            logging,
        })
    }
}

// ============================================================================
// AppConfig 实现
// ============================================================================

impl AppConfig {
    /// 创建应用配置构建器
    pub fn builder() -> AppConfigBuilder {
        AppConfigBuilder::new()
    }
}

impl AppConfigBuilder {
    /// 创建新的构建器
    pub fn new() -> Self {
        Self {
            name: None,
            version: None,
            environment: None,
            debug: None,
            work_dir: None,
        }
    }

    /// 设置应用名称
    /// 
    /// # 参数
    /// 
    /// * `name` - 应用名称
    pub fn name<S: Into<String>>(mut self, name: S) -> Self {
        self.name = Some(name.into());
        self
    }

    /// 设置应用版本
    /// 
    /// # 参数
    /// 
    /// * `version` - 应用版本
    pub fn version<S: Into<String>>(mut self, version: S) -> Self {
        self.version = Some(version.into());
        self
    }

    /// 设置环境类型
    /// 
    /// # 参数
    /// 
    /// * `environment` - 环境类型
    pub fn environment(mut self, environment: Environment) -> Self {
        self.environment = Some(environment);
        self
    }

    /// 设置调试模式
    /// 
    /// # 参数
    /// 
    /// * `debug` - 是否启用调试模式
    pub fn debug(mut self, debug: bool) -> Self {
        self.debug = Some(debug);
        self
    }

    /// 设置工作目录
    /// 
    /// # 参数
    /// 
    /// * `work_dir` - 工作目录路径
    pub fn work_dir<P: Into<PathBuf>>(mut self, work_dir: P) -> Self {
        self.work_dir = Some(work_dir.into());
        self
    }

    /// 构建应用配置
    /// 
    /// # 错误
    /// 
    /// 如果任何必需的配置项未设置，将返回错误
    pub fn build(self) -> Result<AppConfig, QuickDbError> {
        let name = self.name.ok_or_else(|| {
            crate::quick_error!(config, "应用名称必须设置")
        })?;
        
        let version = self.version.ok_or_else(|| {
            crate::quick_error!(config, "应用版本必须设置")
        })?;
        
        let environment = self.environment.ok_or_else(|| {
            crate::quick_error!(config, "环境类型必须设置")
        })?;
        
        let debug = self.debug.ok_or_else(|| {
            crate::quick_error!(config, "调试模式必须设置")
        })?;
        
        let work_dir = self.work_dir.ok_or_else(|| {
            crate::quick_error!(config, "工作目录必须设置")
        })?;

        info!("创建应用配置: 名称={}, 版本={}, 环境={:?}", name, version, environment);

        Ok(AppConfig {
            name,
            version,
            environment,
            debug,
            work_dir,
        })
    }
}

// ============================================================================
// LoggingConfig 实现
// ============================================================================

impl LoggingConfig {
    /// 创建日志配置构建器
    pub fn builder() -> LoggingConfigBuilder {
        LoggingConfigBuilder::new()
    }
}

impl LoggingConfigBuilder {
    /// 创建新的构建器
    pub fn new() -> Self {
        Self {
            level: None,
            console: None,
            file_path: None,
            max_file_size: None,
            max_files: None,
            structured: None,
        }
    }

    /// 设置日志级别
    /// 
    /// # 参数
    /// 
    /// * `level` - 日志级别
    pub fn level(mut self, level: LogLevel) -> Self {
        self.level = Some(level);
        self
    }

    /// 设置是否输出到控制台
    /// 
    /// # 参数
    /// 
    /// * `console` - 是否输出到控制台
    pub fn console(mut self, console: bool) -> Self {
        self.console = Some(console);
        self
    }

    /// 设置日志文件路径
    /// 
    /// # 参数
    /// 
    /// * `file_path` - 日志文件路径
    pub fn file_path<P: Into<PathBuf>>(mut self, file_path: Option<P>) -> Self {
        self.file_path = file_path.map(|p| p.into());
        self
    }

    /// 设置日志文件最大大小
    /// 
    /// # 参数
    /// 
    /// * `max_file_size` - 日志文件最大大小（字节）
    pub fn max_file_size(mut self, max_file_size: u64) -> Self {
        self.max_file_size = Some(max_file_size);
        self
    }

    /// 设置保留的日志文件数量
    /// 
    /// # 参数
    /// 
    /// * `max_files` - 保留的日志文件数量
    pub fn max_files(mut self, max_files: u32) -> Self {
        self.max_files = Some(max_files);
        self
    }

    /// 设置是否启用结构化日志
    /// 
    /// # 参数
    /// 
    /// * `structured` - 是否启用结构化日志
    pub fn structured(mut self, structured: bool) -> Self {
        self.structured = Some(structured);
        self
    }

    /// 构建日志配置
    /// 
    /// # 错误
    /// 
    /// 如果任何必需的配置项未设置，将返回错误
    pub fn build(self) -> Result<LoggingConfig, QuickDbError> {
        let level = self.level.ok_or_else(|| {
            crate::quick_error!(config, "日志级别必须设置")
        })?;
        
        let console = self.console.ok_or_else(|| {
            crate::quick_error!(config, "控制台输出选项必须设置")
        })?;
        
        let max_file_size = self.max_file_size.ok_or_else(|| {
            crate::quick_error!(config, "日志文件最大大小必须设置")
        })?;
        
        let max_files = self.max_files.ok_or_else(|| {
            crate::quick_error!(config, "保留日志文件数量必须设置")
        })?;
        
        let structured = self.structured.ok_or_else(|| {
            crate::quick_error!(config, "结构化日志选项必须设置")
        })?;

        if max_file_size == 0 {
            return Err(crate::quick_error!(config, "日志文件最大大小不能为零"));
        }

        if max_files == 0 {
            return Err(crate::quick_error!(config, "保留日志文件数量不能为零"));
        }

        info!("创建日志配置: 级别={:?}, 控制台={}, 结构化={}", level, console, structured);

        Ok(LoggingConfig {
            level,
            console,
            file_path: self.file_path,
            max_file_size,
            max_files,
            structured,
        })
    }
}

// ============================================================================
// 便捷构造函数
// ============================================================================

/// 🚨 废弃：创建SQLite数据库配置
///
/// **重要警告**：此函数已标记为废弃，将在v0.4.0版本中移除。
/// 请使用 `DatabaseConfig::builder()` 模式替代。
///
/// # 参数
///
/// * `alias` - 数据库别名
/// * `path` - 数据库文件路径
/// * `pool_config` - 连接池配置
/// * `id_strategy` - ID生成策略（可选，默认为AutoIncrement）
#[deprecated(since = "0.3.2", note = "将在v0.4.0版本中移除，请使用DatabaseConfig::builder()模式")]
pub fn sqlite_config<S: Into<String>, P: Into<String>>(
    alias: S,
    path: P,
    pool_config: PoolConfig,
    id_strategy: Option<IdStrategy>,
) -> Result<DatabaseConfig, QuickDbError> {
    DatabaseConfig::builder()
        .db_type(DatabaseType::SQLite)
        .connection(ConnectionConfig::SQLite {
            path: path.into(),
            create_if_missing: true,
        })
        .pool(pool_config)
        .alias(alias)
        .id_strategy(id_strategy.unwrap_or(IdStrategy::AutoIncrement))
        .build()
}

/// 🚨 废弃：创建PostgreSQL数据库配置
///
/// **重要警告**：此函数已标记为废弃，将在v0.4.0版本中移除。
/// 请使用 `DatabaseConfig::builder()` 模式替代。
///
/// # 参数
///
/// * `alias` - 数据库别名
/// * `host` - 主机地址
/// * `port` - 端口号
/// * `database` - 数据库名
/// * `username` - 用户名
/// * `password` - 密码
/// * `pool_config` - 连接池配置
/// * `id_strategy` - ID生成策略（可选，默认为AutoIncrement）
#[deprecated(since = "0.3.2", note = "将在v0.4.0版本中移除，请使用DatabaseConfig::builder()模式")]
pub fn postgres_config<S: Into<String>>(
    alias: S,
    host: S,
    port: u16,
    database: S,
    username: S,
    password: S,
    pool_config: PoolConfig,
    id_strategy: Option<IdStrategy>,
) -> Result<DatabaseConfig, QuickDbError> {
    DatabaseConfig::builder()
        .db_type(DatabaseType::PostgreSQL)
        .connection(ConnectionConfig::PostgreSQL {
            host: host.into(),
            port,
            database: database.into(),
            username: username.into(),
            password: password.into(),
            ssl_mode: Some("prefer".to_string()),
            tls_config: None,
        })
        .pool(pool_config)
        .alias(alias)
        .id_strategy(id_strategy.unwrap_or(IdStrategy::AutoIncrement))
        .build()
}

/// 🚨 废弃：创建MySQL数据库配置
///
/// **重要警告**：此函数已标记为废弃，将在v0.4.0版本中移除。
/// 请使用 `DatabaseConfig::builder()` 模式替代。
///
/// # 参数
///
/// * `alias` - 数据库别名
/// * `host` - 主机地址
/// * `port` - 端口号
/// * `database` - 数据库名
/// * `username` - 用户名
/// * `password` - 密码
/// * `pool_config` - 连接池配置
/// * `id_strategy` - ID生成策略（可选，默认为AutoIncrement）
#[deprecated(since = "0.3.2", note = "将在v0.4.0版本中移除，请使用DatabaseConfig::builder()模式")]
pub fn mysql_config<S: Into<String>>(
    alias: S,
    host: S,
    port: u16,
    database: S,
    username: S,
    password: S,
    pool_config: PoolConfig,
    id_strategy: Option<IdStrategy>,
) -> Result<DatabaseConfig, QuickDbError> {
    DatabaseConfig::builder()
        .db_type(DatabaseType::MySQL)
        .connection(ConnectionConfig::MySQL {
            host: host.into(),
            port,
            database: database.into(),
            username: username.into(),
            password: password.into(),
            ssl_opts: None,
            tls_config: None,
        })
        .pool(pool_config)
        .alias(alias)
        .id_strategy(id_strategy.unwrap_or(IdStrategy::AutoIncrement))
        .build()
}

/// 🚨 废弃：创建MongoDB数据库配置
///
/// **重要警告**：此函数已标记为废弃，将在v0.4.0版本中移除。
/// 请使用 `DatabaseConfig::builder()` 模式替代。
///
/// # 参数
///
/// * `alias` - 数据库别名
/// * `host` - 主机地址
/// * `port` - 端口号
/// * `database` - 数据库名
/// * `username` - 用户名（可选）
/// * `password` - 密码（可选）
/// * `pool_config` - 连接池配置
/// * `id_strategy` - ID生成策略（可选，默认为AutoIncrement）
#[deprecated(since = "0.3.2", note = "将在v0.4.0版本中移除，请使用DatabaseConfig::builder()模式")]
pub fn mongodb_config<S: Into<String>>(
    alias: S,
    host: S,
    port: u16,
    database: S,
    username: Option<S>,
    password: Option<S>,
    pool_config: PoolConfig,
    id_strategy: Option<IdStrategy>,
) -> Result<DatabaseConfig, QuickDbError> {
    let connection_config = MongoDbConnectionBuilder::new(host, port, database)
        .with_auth(
            username.map(|u| u.into()).unwrap_or_default(),
            password.map(|p| p.into()).unwrap_or_default()
        )
        .build();

    DatabaseConfig::builder()
        .db_type(DatabaseType::MongoDB)
        .connection(connection_config)
        .pool(pool_config)
        .alias(alias)
        .id_strategy(id_strategy.unwrap_or(IdStrategy::AutoIncrement))
        .build()
}



// ============================================================================
// Default 实现
// ============================================================================

impl Default for DatabaseConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for PoolConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for GlobalConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for AppConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for LoggingConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Serialize/Deserialize 实现
// ============================================================================

impl Serialize for GlobalConfig {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("GlobalConfig", 4)?;
        state.serialize_field("databases", &self.databases)?;
        state.serialize_field("default_database", &self.default_database)?;
        state.serialize_field("app", &self.app)?;
        state.serialize_field("logging", &self.logging)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for GlobalConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, Visitor};
        use std::fmt;

        struct GlobalConfigVisitor;

        impl<'de> Visitor<'de> for GlobalConfigVisitor {
            type Value = GlobalConfig;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct GlobalConfig")
            }

            fn visit_map<V>(self, mut map: V) -> Result<GlobalConfig, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut databases = None;
                let mut default_database = None;
                let mut app = None;
                let mut logging = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        "databases" => {
                            if databases.is_some() {
                                return Err(de::Error::duplicate_field("databases"));
                            }
                            databases = Some(map.next_value()?);
                        }
                        "default_database" => {
                            if default_database.is_some() {
                                return Err(de::Error::duplicate_field("default_database"));
                            }
                            default_database = Some(map.next_value()?);
                        }
                        "app" => {
                            if app.is_some() {
                                return Err(de::Error::duplicate_field("app"));
                            }
                            app = Some(map.next_value()?);
                        }
                        "logging" => {
                            if logging.is_some() {
                                return Err(de::Error::duplicate_field("logging"));
                            }
                            logging = Some(map.next_value()?);
                        }
                        _ => {
                            let _ = map.next_value::<serde_json::Value>()?;
                        }
                    }
                }

                let databases = databases.ok_or_else(|| de::Error::missing_field("databases"))?;
                let app = app.ok_or_else(|| de::Error::missing_field("app"))?;
                let logging = logging.ok_or_else(|| de::Error::missing_field("logging"))?;

                Ok(GlobalConfig {
                    databases,
                    default_database,
                    app,
                    logging,
                })
            }
        }

        const FIELDS: &'static [&'static str] = &["databases", "default_database", "app", "logging"];
        deserializer.deserialize_struct("GlobalConfig", FIELDS, GlobalConfigVisitor)
    }
}