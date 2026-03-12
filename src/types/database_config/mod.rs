use crate::types::cache_config::CacheConfig;
use crate::types::id_types::IdStrategy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 支持的数据库类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DatabaseType {
    /// SQLite 数据库
    SQLite,
    /// PostgreSQL 数据库
    PostgreSQL,
    /// MySQL 数据库
    MySQL,
    /// MongoDB 数据库
    MongoDB,
}

impl DatabaseType {
    /// 获取数据库类型的字符串表示
    pub fn as_str(&self) -> &'static str {
        match self {
            DatabaseType::SQLite => "sqlite",
            DatabaseType::PostgreSQL => "postgresql",
            DatabaseType::MySQL => "mysql",
            DatabaseType::MongoDB => "mongodb",
        }
    }

    /// 从字符串解析数据库类型
    pub fn from_str(s: &str) -> Result<Self, crate::error::QuickDbError> {
        match s.to_lowercase().as_str() {
            "sqlite" => Ok(DatabaseType::SQLite),
            "postgresql" | "postgres" | "pg" => Ok(DatabaseType::PostgreSQL),
            "mysql" => Ok(DatabaseType::MySQL),
            "mongodb" | "mongo" => Ok(DatabaseType::MongoDB),
            _ => Err(crate::quick_error!(unsupported_db, s)),
        }
    }
}

/// 数据库连接配置
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    /// 数据库类型
    pub db_type: DatabaseType,
    /// 连接字符串或配置
    pub connection: ConnectionConfig,
    /// 连接池配置
    pub pool: PoolConfig,
    /// 数据库别名（默认为 "default"）
    pub alias: String,
    /// 缓存配置（可选）
    pub cache: Option<CacheConfig>,
    /// ID 生成策略
    pub id_strategy: IdStrategy,
    /// 版本管理存储路径（可选，默认为 ~/.rat_quickdb/{alias}/）
    pub version_storage_path: Option<String>,
    /// 是否启用版本管理（默认 false）
    pub enable_versioning: Option<bool>,
}

// 手动实现序列化，以支持 PoolConfig 字段私有化
impl Serialize for DatabaseConfig {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("DatabaseConfig", 8)?;
        state.serialize_field("db_type", &self.db_type)?;
        state.serialize_field("connection", &self.connection)?;
        state.serialize_field("pool", &self.pool)?;
        state.serialize_field("alias", &self.alias)?;
        state.serialize_field("cache", &self.cache)?;
        state.serialize_field("id_strategy", &self.id_strategy)?;
        state.serialize_field("version_storage_path", &self.version_storage_path)?;
        state.serialize_field("enable_versioning", &self.enable_versioning)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for DatabaseConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, Visitor};
        use std::fmt;

        struct DatabaseConfigVisitor;

        impl<'de> Visitor<'de> for DatabaseConfigVisitor {
            type Value = DatabaseConfig;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct DatabaseConfig")
            }

            fn visit_map<V>(self, mut map: V) -> Result<DatabaseConfig, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut db_type = None;
                let mut connection = None;
                let mut pool = None;
                let mut alias = None;
                let mut cache = None;
                let mut id_strategy = None;
                let mut version_storage_path = None;
                let mut enable_versioning = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        "db_type" => {
                            if db_type.is_some() {
                                return Err(de::Error::duplicate_field("db_type"));
                            }
                            db_type = Some(map.next_value()?);
                        }
                        "connection" => {
                            if connection.is_some() {
                                return Err(de::Error::duplicate_field("connection"));
                            }
                            connection = Some(map.next_value()?);
                        }
                        "pool" => {
                            if pool.is_some() {
                                return Err(de::Error::duplicate_field("pool"));
                            }
                            pool = Some(map.next_value()?);
                        }
                        "alias" => {
                            if alias.is_some() {
                                return Err(de::Error::duplicate_field("alias"));
                            }
                            alias = Some(map.next_value()?);
                        }
                        "cache" => {
                            if cache.is_some() {
                                return Err(de::Error::duplicate_field("cache"));
                            }
                            cache = Some(map.next_value()?);
                        }
                        "id_strategy" => {
                            if id_strategy.is_some() {
                                return Err(de::Error::duplicate_field("id_strategy"));
                            }
                            id_strategy = Some(map.next_value()?);
                        }
                        "version_storage_path" => {
                            if version_storage_path.is_some() {
                                return Err(de::Error::duplicate_field("version_storage_path"));
                            }
                            version_storage_path = Some(map.next_value()?);
                        }
                        "enable_versioning" => {
                            if enable_versioning.is_some() {
                                return Err(de::Error::duplicate_field("enable_versioning"));
                            }
                            enable_versioning = Some(map.next_value()?);
                        }
                        _ => {
                            let _ = map.next_value::<serde_json::Value>()?;
                        }
                    }
                }

                let db_type = db_type.ok_or_else(|| de::Error::missing_field("db_type"))?;
                let connection = connection.ok_or_else(|| de::Error::missing_field("connection"))?;
                let pool = pool.ok_or_else(|| de::Error::missing_field("pool"))?;
                let alias = alias.ok_or_else(|| de::Error::missing_field("alias"))?;
                let id_strategy = id_strategy.ok_or_else(|| de::Error::missing_field("id_strategy"))?;

                Ok(DatabaseConfig {
                    db_type,
                    connection,
                    pool,
                    alias,
                    cache,
                    id_strategy,
                    version_storage_path,
                    enable_versioning,
                })
            }
        }

        const FIELDS: &'static [&'static str] = &[
            "db_type",
            "connection",
            "pool",
            "alias",
            "cache",
            "id_strategy",
            "version_storage_path",
            "enable_versioning",
        ];
        deserializer.deserialize_struct("DatabaseConfig", FIELDS, DatabaseConfigVisitor)
    }
}

/// 连接配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectionConfig {
    /// SQLite 文件路径
    SQLite {
        /// 数据库文件路径
        path: String,
        /// 是否创建数据库文件（如果不存在）
        create_if_missing: bool,
    },
    /// PostgreSQL 连接配置
    PostgreSQL {
        /// 主机地址
        host: String,
        /// 端口号
        port: u16,
        /// 数据库名
        database: String,
        /// 用户名
        username: String,
        /// 密码
        password: String,
        /// SSL 模式 (disable, allow, prefer, require, verify-ca, verify-full)
        ssl_mode: Option<String>,
        /// TLS 配置选项
        tls_config: Option<TlsConfig>,
    },
    /// MySQL 连接配置
    MySQL {
        /// 主机地址
        host: String,
        /// 端口号
        port: u16,
        /// 数据库名
        database: String,
        /// 用户名
        username: String,
        /// 密码
        password: String,
        /// SSL 配置
        ssl_opts: Option<HashMap<String, String>>,
        /// TLS 配置选项
        tls_config: Option<TlsConfig>,
    },
    /// MongoDB 连接配置
    MongoDB {
        /// 主机地址（支持IP或域名）
        host: String,
        /// 端口号（默认27017）
        port: u16,
        /// 数据库名
        database: String,
        /// 用户名（可选）
        username: Option<String>,
        /// 密码（可选）
        password: Option<String>,
        /// 认证源数据库（可选，默认为admin）
        auth_source: Option<String>,
        /// 是否启用直连模式
        direct_connection: bool,
        /// TLS 配置选项
        tls_config: Option<TlsConfig>,
        /// ZSTD 压缩配置
        zstd_config: Option<ZstdConfig>,
        /// 其他连接选项
        options: Option<HashMap<String, String>>,
    },
}

/// TLS 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    /// 是否启用 TLS
    pub enabled: bool,
    /// CA 证书文件路径
    pub ca_cert_path: Option<String>,
    /// 客户端证书文件路径
    pub client_cert_path: Option<String>,
    /// 客户端私钥文件路径
    pub client_key_path: Option<String>,
    /// 是否验证服务器证书
    pub verify_server_cert: bool,
    /// 是否验证主机名
    pub verify_hostname: bool,
    /// 允许的 TLS 版本（如 "1.2", "1.3"）
    pub min_tls_version: Option<String>,
    /// 允许的密码套件
    pub cipher_suites: Option<Vec<String>>,
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            ca_cert_path: None,
            client_cert_path: None,
            client_key_path: None,
            verify_server_cert: true,
            verify_hostname: true,
            min_tls_version: Some("1.2".to_string()),
            cipher_suites: None,
        }
    }
}

impl TlsConfig {
    /// 创建启用 TLS 的配置
    pub fn enabled() -> Self {
        Self {
            enabled: true,
            ..Default::default()
        }
    }

    /// 设置 CA 证书路径
    pub fn with_ca_cert<P: Into<String>>(mut self, path: P) -> Self {
        self.ca_cert_path = Some(path.into());
        self
    }

    /// 设置客户端证书和私钥路径
    pub fn with_client_cert<P1: Into<String>, P2: Into<String>>(
        mut self,
        cert_path: P1,
        key_path: P2,
    ) -> Self {
        self.client_cert_path = Some(cert_path.into());
        self.client_key_path = Some(key_path.into());
        self
    }

    /// 设置是否验证服务器证书
    pub fn verify_server_cert(mut self, verify: bool) -> Self {
        self.verify_server_cert = verify;
        self
    }

    /// 设置是否验证主机名
    pub fn verify_hostname(mut self, verify: bool) -> Self {
        self.verify_hostname = verify;
        self
    }

    /// 设置最小 TLS 版本
    pub fn with_min_tls_version<V: Into<String>>(mut self, version: V) -> Self {
        self.min_tls_version = Some(version.into());
        self
    }

    /// 设置允许的密码套件
    pub fn with_cipher_suites(mut self, suites: Vec<String>) -> Self {
        self.cipher_suites = Some(suites);
        self
    }
}

/// ZSTD 压缩配置（主要用于 MongoDB）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZstdConfig {
    /// 是否启用 ZSTD 压缩
    pub enabled: bool,
    /// 压缩级别（1-22，默认为 3）
    pub compression_level: Option<i32>,
    /// 压缩阈值（字节数，小于此值不压缩）
    pub compression_threshold: Option<usize>,
}

impl Default for ZstdConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            compression_level: Some(3),
            compression_threshold: Some(1024), // 1KB
        }
    }
}

impl ZstdConfig {
    /// 创建启用 ZSTD 压缩的配置
    pub fn enabled() -> Self {
        Self {
            enabled: true,
            ..Default::default()
        }
    }

    /// 设置压缩级别（1-22）
    pub fn with_compression_level(mut self, level: i32) -> Self {
        self.compression_level = Some(level.clamp(1, 22));
        self
    }

    /// 设置压缩阈值（字节数）
    pub fn with_compression_threshold(mut self, threshold: usize) -> Self {
        self.compression_threshold = Some(threshold);
        self
    }
}

/// 连接池配置
///
/// ⚠️ **重要**：所有字段仅在 crate 内可见，外部代码**必须通过 `PoolConfig::builder()` 创建**
/// 直接构造结构体会导致编译错误
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// 最小连接数
    pub(crate) min_connections: u32,
    /// 最大连接数
    pub(crate) max_connections: u32,
    /// 连接超时时间（秒）
    pub(crate) connection_timeout: u64,
    /// 空闲连接超时时间（秒）
    pub(crate) idle_timeout: u64,
    /// 连接最大生存时间（秒）
    pub(crate) max_lifetime: u64,
    /// 最大重试次数
    pub(crate) max_retries: u32,
    /// 重试间隔（毫秒）
    pub(crate) retry_interval_ms: u64,
    /// 保活检测间隔（秒）
    pub(crate) keepalive_interval_sec: u64,
    /// 连接健康检查超时（秒）
    pub(crate) health_check_timeout_sec: u64,
}

// 手动实现序列化，以支持字段私有化
impl Serialize for PoolConfig {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("PoolConfig", 9)?;
        state.serialize_field("min_connections", &self.min_connections)?;
        state.serialize_field("max_connections", &self.max_connections)?;
        state.serialize_field("connection_timeout", &self.connection_timeout)?;
        state.serialize_field("idle_timeout", &self.idle_timeout)?;
        state.serialize_field("max_lifetime", &self.max_lifetime)?;
        state.serialize_field("max_retries", &self.max_retries)?;
        state.serialize_field("retry_interval_ms", &self.retry_interval_ms)?;
        state.serialize_field("keepalive_interval_sec", &self.keepalive_interval_sec)?;
        state.serialize_field("health_check_timeout_sec", &self.health_check_timeout_sec)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for PoolConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, Visitor};
        use std::fmt;

        struct PoolConfigVisitor;

        impl<'de> Visitor<'de> for PoolConfigVisitor {
            type Value = PoolConfig;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct PoolConfig")
            }

            fn visit_map<V>(self, mut map: V) -> Result<PoolConfig, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut min_connections = None;
                let mut max_connections = None;
                let mut connection_timeout = None;
                let mut idle_timeout = None;
                let mut max_lifetime = None;
                let mut max_retries = None;
                let mut retry_interval_ms = None;
                let mut keepalive_interval_sec = None;
                let mut health_check_timeout_sec = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        "min_connections" => {
                            if min_connections.is_some() {
                                return Err(de::Error::duplicate_field("min_connections"));
                            }
                            min_connections = Some(map.next_value()?);
                        }
                        "max_connections" => {
                            if max_connections.is_some() {
                                return Err(de::Error::duplicate_field("max_connections"));
                            }
                            max_connections = Some(map.next_value()?);
                        }
                        "connection_timeout" => {
                            if connection_timeout.is_some() {
                                return Err(de::Error::duplicate_field("connection_timeout"));
                            }
                            connection_timeout = Some(map.next_value()?);
                        }
                        "idle_timeout" => {
                            if idle_timeout.is_some() {
                                return Err(de::Error::duplicate_field("idle_timeout"));
                            }
                            idle_timeout = Some(map.next_value()?);
                        }
                        "max_lifetime" => {
                            if max_lifetime.is_some() {
                                return Err(de::Error::duplicate_field("max_lifetime"));
                            }
                            max_lifetime = Some(map.next_value()?);
                        }
                        "max_retries" => {
                            if max_retries.is_some() {
                                return Err(de::Error::duplicate_field("max_retries"));
                            }
                            max_retries = Some(map.next_value()?);
                        }
                        "retry_interval_ms" => {
                            if retry_interval_ms.is_some() {
                                return Err(de::Error::duplicate_field("retry_interval_ms"));
                            }
                            retry_interval_ms = Some(map.next_value()?);
                        }
                        "keepalive_interval_sec" => {
                            if keepalive_interval_sec.is_some() {
                                return Err(de::Error::duplicate_field("keepalive_interval_sec"));
                            }
                            keepalive_interval_sec = Some(map.next_value()?);
                        }
                        "health_check_timeout_sec" => {
                            if health_check_timeout_sec.is_some() {
                                return Err(de::Error::duplicate_field("health_check_timeout_sec"));
                            }
                            health_check_timeout_sec = Some(map.next_value()?);
                        }
                        _ => {
                            let _ = map.next_value::<serde_json::Value>()?;
                        }
                    }
                }

                let min_connections = min_connections.ok_or_else(|| de::Error::missing_field("min_connections"))?;
                let max_connections = max_connections.ok_or_else(|| de::Error::missing_field("max_connections"))?;
                let connection_timeout = connection_timeout.ok_or_else(|| de::Error::missing_field("connection_timeout"))?;
                let idle_timeout = idle_timeout.ok_or_else(|| de::Error::missing_field("idle_timeout"))?;
                let max_lifetime = max_lifetime.ok_or_else(|| de::Error::missing_field("max_lifetime"))?;
                let max_retries = max_retries.ok_or_else(|| de::Error::missing_field("max_retries"))?;
                let retry_interval_ms = retry_interval_ms.ok_or_else(|| de::Error::missing_field("retry_interval_ms"))?;
                let keepalive_interval_sec = keepalive_interval_sec.ok_or_else(|| de::Error::missing_field("keepalive_interval_sec"))?;
                let health_check_timeout_sec = health_check_timeout_sec.ok_or_else(|| de::Error::missing_field("health_check_timeout_sec"))?;

                Ok(PoolConfig {
                    min_connections,
                    max_connections,
                    connection_timeout,
                    idle_timeout,
                    max_lifetime,
                    max_retries,
                    retry_interval_ms,
                    keepalive_interval_sec,
                    health_check_timeout_sec,
                })
            }
        }

        const FIELDS: &'static [&'static str] = &[
            "min_connections",
            "max_connections",
            "connection_timeout",
            "idle_timeout",
            "max_lifetime",
            "max_retries",
            "retry_interval_ms",
            "keepalive_interval_sec",
            "health_check_timeout_sec",
        ];
        deserializer.deserialize_struct("PoolConfig", FIELDS, PoolConfigVisitor)
    }
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            min_connections: 1,
            max_connections: 10,
            connection_timeout: 30,
            idle_timeout: 600,
            max_lifetime: 3600,
            max_retries: 3,
            retry_interval_ms: 1000,
            keepalive_interval_sec: 30,
            health_check_timeout_sec: 5,
        }
    }
}
