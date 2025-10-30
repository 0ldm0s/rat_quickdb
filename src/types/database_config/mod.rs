use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::types::cache_config::CacheConfig;
use crate::types::id_types::IdStrategy;

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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub fn with_client_cert<P1: Into<String>, P2: Into<String>>(mut self, cert_path: P1, key_path: P2) -> Self {
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConfig {
    /// 最小连接数
    pub min_connections: u32,
    /// 最大连接数
    pub max_connections: u32,
    /// 连接超时时间（秒）
    pub connection_timeout: u64,
    /// 空闲连接超时时间（秒）
    pub idle_timeout: u64,
    /// 连接最大生存时间（秒）
    pub max_lifetime: u64,
    /// 最大重试次数
    pub max_retries: u32,
    /// 重试间隔（毫秒）
    pub retry_interval_ms: u64,
    /// 保活检测间隔（秒）
    pub keepalive_interval_sec: u64,
    /// 连接健康检查超时（秒）
    pub health_check_timeout_sec: u64,
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