//! # 便利配置函数模块
//!
//! 提供常用数据库配置的便利函数，简化配置过程

use crate::config::builders::*;
use crate::error::QuickDbError;
use crate::types::*;

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
#[deprecated(
    since = "0.3.2",
    note = "将在v0.4.0版本中移除，请使用DatabaseConfig::builder()模式"
)]
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
#[deprecated(
    since = "0.3.2",
    note = "将在v0.4.0版本中移除，请使用DatabaseConfig::builder()模式"
)]
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
#[deprecated(
    since = "0.3.2",
    note = "将在v0.4.0版本中移除，请使用DatabaseConfig::builder()模式"
)]
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
#[deprecated(
    since = "0.3.2",
    note = "将在v0.4.0版本中移除，请使用DatabaseConfig::builder()模式"
)]
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
            password.map(|p| p.into()).unwrap_or_default(),
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
