//! # 连接池配置构建器模块
//!
//! 提供连接池配置的构建器实现，支持链式调用和严格验证

use crate::error::QuickDbError;
use crate::types::*;
use rat_logger::info;
use std::path::PathBuf;

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
    max_retries: Option<u32>,
    retry_interval_ms: Option<u64>,
    keepalive_interval_sec: Option<u64>,
    health_check_timeout_sec: Option<u64>,
}
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
            max_retries: None,
            retry_interval_ms: None,
            keepalive_interval_sec: None,
            health_check_timeout_sec: None,
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

    /// 设置最大重试次数
    ///
    /// # 参数
    ///
    /// * `retries` - 最大重试次数
    pub fn max_retries(mut self, retries: u32) -> Self {
        self.max_retries = Some(retries);
        self
    }

    /// 设置重试间隔（毫秒）
    ///
    /// # 参数
    ///
    /// * `interval` - 重试间隔（毫秒）
    pub fn retry_interval_ms(mut self, interval: u64) -> Self {
        self.retry_interval_ms = Some(interval);
        self
    }

    /// 设置保活检测间隔（秒）
    ///
    /// # 参数
    ///
    /// * `interval` - 保活检测间隔（秒）
    pub fn keepalive_interval_sec(mut self, interval: u64) -> Self {
        self.keepalive_interval_sec = Some(interval);
        self
    }

    /// 设置连接健康检查超时（秒）
    ///
    /// # 参数
    ///
    /// * `timeout` - 连接健康检查超时（秒）
    pub fn health_check_timeout_sec(mut self, timeout: u64) -> Self {
        self.health_check_timeout_sec = Some(timeout);
        self
    }

    /// 构建连接池配置
    ///
    /// # 错误
    ///
    /// 如果任何必需的配置项未设置，将返回错误
    pub fn build(self) -> Result<PoolConfig, QuickDbError> {
        let min_connections = self
            .min_connections
            .ok_or_else(|| crate::quick_error!(config, crate::i18n::t("config.min_connections_required")))?;

        let max_connections = self
            .max_connections
            .ok_or_else(|| crate::quick_error!(config, crate::i18n::t("config.max_connections_required")))?;

        let connection_timeout = self
            .connection_timeout
            .ok_or_else(|| crate::quick_error!(config, crate::i18n::t("config.connection_timeout_required")))?;

        let idle_timeout = self
            .idle_timeout
            .ok_or_else(|| crate::quick_error!(config, crate::i18n::t("config.idle_timeout_required")))?;

        let max_lifetime = self
            .max_lifetime
            .ok_or_else(|| crate::quick_error!(config, crate::i18n::t("config.max_lifetime_required")))?;

        let max_retries = self
            .max_retries
            .ok_or_else(|| crate::quick_error!(config, crate::i18n::t("config.max_retries_required")))?;

        let retry_interval_ms = self
            .retry_interval_ms
            .ok_or_else(|| crate::quick_error!(config, crate::i18n::t("config.retry_interval_required")))?;

        let keepalive_interval_sec = self
            .keepalive_interval_sec
            .ok_or_else(|| crate::quick_error!(config, crate::i18n::t("config.keepalive_interval_required")))?;

        let health_check_timeout_sec = self
            .health_check_timeout_sec
            .ok_or_else(|| crate::quick_error!(config, crate::i18n::t("config.health_check_timeout_required")))?;

        // 验证配置的合理性
        if min_connections > max_connections {
            return Err(crate::quick_error!(config, crate::i18n::t("config.min_exceeds_max_connections")));
        }

        if connection_timeout == 0 {
            return Err(crate::quick_error!(config, crate::i18n::t("config.connection_timeout_zero")));
        }

        if idle_timeout == 0 {
            return Err(crate::quick_error!(config, crate::i18n::t("config.idle_timeout_zero")));
        }

        if max_lifetime == 0 {
            return Err(crate::quick_error!(config, crate::i18n::t("config.max_lifetime_zero")));
        }

        info!(
            "创建连接池配置: 最小连接数={}, 最大连接数={}, 连接超时={}s",
            min_connections, max_connections, connection_timeout
        );

        Ok(PoolConfig {
            min_connections,
            max_connections,
            connection_timeout: connection_timeout * 1000, // 转换为毫秒
            idle_timeout,
            max_lifetime,
            max_retries,
            retry_interval_ms,
            keepalive_interval_sec,
            health_check_timeout_sec,
        })
    }
}
impl Default for PoolConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::builders::DatabaseConfigBuilder;

    /// NOTE: These tests require `--test-threads=1` because i18n language state is global.
    ///
    /// ```bash
    /// cargo test --lib config::builders::pool_builder::tests -- --test-threads=1
    /// ```

    fn setup_i18n(lang: &str) {
        crate::i18n::ErrorMessageI18n::init_i18n();
        crate::i18n::set_language(lang);
    }

    /// Extract the `message` string from a `QuickDbError::ConfigError`.
    fn config_message(err: &QuickDbError) -> &str {
        match err {
            QuickDbError::ConfigError { message } => message,
            _ => "",
        }
    }

    // ===== pool_builder i18n tests (zh-CN backward compatibility) =====

    #[test]
    fn test_pool_min_connections_required_zh() {
        setup_i18n("zh-CN");
        let err = PoolConfigBuilder::new()
            .max_connections(10)
            .connection_timeout(30)
            .idle_timeout(600)
            .max_lifetime(1800)
            .max_retries(3)
            .retry_interval_ms(1000)
            .keepalive_interval_sec(60)
            .health_check_timeout_sec(5)
            .build()
            .unwrap_err();
        assert_eq!(config_message(&err), "最小连接数必须设置");
    }

    #[test]
    fn test_pool_max_connections_required_zh() {
        setup_i18n("zh-CN");
        let err = PoolConfigBuilder::new()
            .min_connections(1)
            .connection_timeout(30)
            .idle_timeout(600)
            .max_lifetime(1800)
            .max_retries(3)
            .retry_interval_ms(1000)
            .keepalive_interval_sec(60)
            .health_check_timeout_sec(5)
            .build()
            .unwrap_err();
        assert_eq!(config_message(&err), "最大连接数必须设置");
    }

    #[test]
    fn test_pool_connection_timeout_required_zh() {
        setup_i18n("zh-CN");
        let err = PoolConfigBuilder::new()
            .min_connections(1)
            .max_connections(10)
            .idle_timeout(600)
            .max_lifetime(1800)
            .max_retries(3)
            .retry_interval_ms(1000)
            .keepalive_interval_sec(60)
            .health_check_timeout_sec(5)
            .build()
            .unwrap_err();
        assert_eq!(config_message(&err), "连接超时时间必须设置");
    }

    #[test]
    fn test_pool_idle_timeout_required_zh() {
        setup_i18n("zh-CN");
        let err = PoolConfigBuilder::new()
            .min_connections(1)
            .max_connections(10)
            .connection_timeout(30)
            .max_lifetime(1800)
            .max_retries(3)
            .retry_interval_ms(1000)
            .keepalive_interval_sec(60)
            .health_check_timeout_sec(5)
            .build()
            .unwrap_err();
        assert_eq!(config_message(&err), "空闲连接超时时间必须设置");
    }

    #[test]
    fn test_pool_max_lifetime_required_zh() {
        setup_i18n("zh-CN");
        let err = PoolConfigBuilder::new()
            .min_connections(1)
            .max_connections(10)
            .connection_timeout(30)
            .idle_timeout(600)
            .max_retries(3)
            .retry_interval_ms(1000)
            .keepalive_interval_sec(60)
            .health_check_timeout_sec(5)
            .build()
            .unwrap_err();
        assert_eq!(config_message(&err), "连接最大生存时间必须设置");
    }

    #[test]
    fn test_pool_max_retries_required_zh() {
        setup_i18n("zh-CN");
        let err = PoolConfigBuilder::new()
            .min_connections(1)
            .max_connections(10)
            .connection_timeout(30)
            .idle_timeout(600)
            .max_lifetime(1800)
            .retry_interval_ms(1000)
            .keepalive_interval_sec(60)
            .health_check_timeout_sec(5)
            .build()
            .unwrap_err();
        assert_eq!(config_message(&err), "最大重试次数必须设置");
    }

    #[test]
    fn test_pool_retry_interval_required_zh() {
        setup_i18n("zh-CN");
        let err = PoolConfigBuilder::new()
            .min_connections(1)
            .max_connections(10)
            .connection_timeout(30)
            .idle_timeout(600)
            .max_lifetime(1800)
            .max_retries(3)
            .keepalive_interval_sec(60)
            .health_check_timeout_sec(5)
            .build()
            .unwrap_err();
        assert_eq!(config_message(&err), "重试间隔必须设置");
    }

    #[test]
    fn test_pool_keepalive_interval_required_zh() {
        setup_i18n("zh-CN");
        let err = PoolConfigBuilder::new()
            .min_connections(1)
            .max_connections(10)
            .connection_timeout(30)
            .idle_timeout(600)
            .max_lifetime(1800)
            .max_retries(3)
            .retry_interval_ms(1000)
            .health_check_timeout_sec(5)
            .build()
            .unwrap_err();
        assert_eq!(config_message(&err), "保活检测间隔必须设置");
    }

    #[test]
    fn test_pool_health_check_timeout_required_zh() {
        setup_i18n("zh-CN");
        let err = PoolConfigBuilder::new()
            .min_connections(1)
            .max_connections(10)
            .connection_timeout(30)
            .idle_timeout(600)
            .max_lifetime(1800)
            .max_retries(3)
            .retry_interval_ms(1000)
            .keepalive_interval_sec(60)
            .build()
            .unwrap_err();
        assert_eq!(config_message(&err), "健康检查超时时间必须设置");
    }

    #[test]
    fn test_pool_min_exceeds_max_zh() {
        setup_i18n("zh-CN");
        let err = PoolConfigBuilder::new()
            .min_connections(20)
            .max_connections(10)
            .connection_timeout(30)
            .idle_timeout(600)
            .max_lifetime(1800)
            .max_retries(3)
            .retry_interval_ms(1000)
            .keepalive_interval_sec(60)
            .health_check_timeout_sec(5)
            .build()
            .unwrap_err();
        assert_eq!(config_message(&err), "最小连接数不能大于最大连接数");
    }

    #[test]
    fn test_pool_connection_timeout_zero_zh() {
        setup_i18n("zh-CN");
        let err = PoolConfigBuilder::new()
            .min_connections(1)
            .max_connections(10)
            .connection_timeout(0)
            .idle_timeout(600)
            .max_lifetime(1800)
            .max_retries(3)
            .retry_interval_ms(1000)
            .keepalive_interval_sec(60)
            .health_check_timeout_sec(5)
            .build()
            .unwrap_err();
        assert_eq!(config_message(&err), "连接超时时间不能为零");
    }

    #[test]
    fn test_pool_idle_timeout_zero_zh() {
        setup_i18n("zh-CN");
        let err = PoolConfigBuilder::new()
            .min_connections(1)
            .max_connections(10)
            .connection_timeout(30)
            .idle_timeout(0)
            .max_lifetime(1800)
            .max_retries(3)
            .retry_interval_ms(1000)
            .keepalive_interval_sec(60)
            .health_check_timeout_sec(5)
            .build()
            .unwrap_err();
        assert_eq!(config_message(&err), "空闲连接超时时间不能为零");
    }

    #[test]
    fn test_pool_max_lifetime_zero_zh() {
        setup_i18n("zh-CN");
        let err = PoolConfigBuilder::new()
            .min_connections(1)
            .max_connections(10)
            .connection_timeout(30)
            .idle_timeout(600)
            .max_lifetime(0)
            .max_retries(3)
            .retry_interval_ms(1000)
            .keepalive_interval_sec(60)
            .health_check_timeout_sec(5)
            .build()
            .unwrap_err();
        assert_eq!(config_message(&err), "连接最大生存时间不能为零");
    }

    // ===== pool_builder i18n tests (en-US) =====

    #[test]
    fn test_pool_min_connections_required_en() {
        setup_i18n("en-US");
        let err = PoolConfigBuilder::new()
            .max_connections(10)
            .connection_timeout(30)
            .idle_timeout(600)
            .max_lifetime(1800)
            .max_retries(3)
            .retry_interval_ms(1000)
            .keepalive_interval_sec(60)
            .health_check_timeout_sec(5)
            .build()
            .unwrap_err();
        assert_eq!(config_message(&err), "Min connections is required");
    }

    #[test]
    fn test_pool_max_connections_required_en() {
        setup_i18n("en-US");
        let err = PoolConfigBuilder::new()
            .min_connections(1)
            .connection_timeout(30)
            .idle_timeout(600)
            .max_lifetime(1800)
            .max_retries(3)
            .retry_interval_ms(1000)
            .keepalive_interval_sec(60)
            .health_check_timeout_sec(5)
            .build()
            .unwrap_err();
        assert_eq!(config_message(&err), "Max connections is required");
    }

    #[test]
    fn test_pool_connection_timeout_required_en() {
        setup_i18n("en-US");
        let err = PoolConfigBuilder::new()
            .min_connections(1)
            .max_connections(10)
            .idle_timeout(600)
            .max_lifetime(1800)
            .max_retries(3)
            .retry_interval_ms(1000)
            .keepalive_interval_sec(60)
            .health_check_timeout_sec(5)
            .build()
            .unwrap_err();
        assert_eq!(config_message(&err), "Connection timeout is required");
    }

    #[test]
    fn test_pool_min_exceeds_max_en() {
        setup_i18n("en-US");
        let err = PoolConfigBuilder::new()
            .min_connections(20)
            .max_connections(10)
            .connection_timeout(30)
            .idle_timeout(600)
            .max_lifetime(1800)
            .max_retries(3)
            .retry_interval_ms(1000)
            .keepalive_interval_sec(60)
            .health_check_timeout_sec(5)
            .build()
            .unwrap_err();
        assert_eq!(config_message(&err), "Min connections cannot exceed max connections");
    }

    #[test]
    fn test_pool_connection_timeout_zero_en() {
        setup_i18n("en-US");
        let err = PoolConfigBuilder::new()
            .min_connections(1)
            .max_connections(10)
            .connection_timeout(0)
            .idle_timeout(600)
            .max_lifetime(1800)
            .max_retries(3)
            .retry_interval_ms(1000)
            .keepalive_interval_sec(60)
            .health_check_timeout_sec(5)
            .build()
            .unwrap_err();
        assert_eq!(config_message(&err), "Connection timeout cannot be zero");
    }

    #[test]
    fn test_pool_max_lifetime_zero_en() {
        setup_i18n("en-US");
        let err = PoolConfigBuilder::new()
            .min_connections(1)
            .max_connections(10)
            .connection_timeout(30)
            .idle_timeout(600)
            .max_lifetime(0)
            .max_retries(3)
            .retry_interval_ms(1000)
            .keepalive_interval_sec(60)
            .health_check_timeout_sec(5)
            .build()
            .unwrap_err();
        assert_eq!(config_message(&err), "Max lifetime cannot be zero");
    }

    // ===== pool_builder i18n tests (ja-JP representative) =====

    #[test]
    fn test_pool_min_connections_required_ja() {
        setup_i18n("ja-JP");
        let err = PoolConfigBuilder::new()
            .max_connections(10)
            .connection_timeout(30)
            .idle_timeout(600)
            .max_lifetime(1800)
            .max_retries(3)
            .retry_interval_ms(1000)
            .keepalive_interval_sec(60)
            .health_check_timeout_sec(5)
            .build()
            .unwrap_err();
        assert_eq!(config_message(&err), "最小接続数は必須です");
    }

    #[test]
    fn test_pool_min_exceeds_max_ja() {
        setup_i18n("ja-JP");
        let err = PoolConfigBuilder::new()
            .min_connections(20)
            .max_connections(10)
            .connection_timeout(30)
            .idle_timeout(600)
            .max_lifetime(1800)
            .max_retries(3)
            .retry_interval_ms(1000)
            .keepalive_interval_sec(60)
            .health_check_timeout_sec(5)
            .build()
            .unwrap_err();
        assert_eq!(config_message(&err), "最小接続数は最大接続数を超えることはできません");
    }

    #[test]
    fn test_pool_max_lifetime_zero_ja() {
        setup_i18n("ja-JP");
        let err = PoolConfigBuilder::new()
            .min_connections(1)
            .max_connections(10)
            .connection_timeout(30)
            .idle_timeout(600)
            .max_lifetime(0)
            .max_retries(3)
            .retry_interval_ms(1000)
            .keepalive_interval_sec(60)
            .health_check_timeout_sec(5)
            .build()
            .unwrap_err();
        assert_eq!(config_message(&err), "最大寿命はゼロにできません");
    }

    // ===== database_builder i18n tests =====

    #[test]
    fn test_database_type_required_zh() {
        setup_i18n("zh-CN");
        let pool = valid_pool_config();
        let err = DatabaseConfigBuilder::new()
            .connection(ConnectionConfig::SQLite {
                path: "/tmp/test.db".into(),
                create_if_missing: true,
            })
            .pool(pool)
            .alias("test")
            .id_strategy(IdStrategy::AutoIncrement)
            .build()
            .unwrap_err();
        assert_eq!(config_message(&err), "数据库类型必须设置");
    }

    #[test]
    fn test_database_connection_required_zh() {
        setup_i18n("zh-CN");
        let pool = valid_pool_config();
        let err = DatabaseConfigBuilder::new()
            .db_type(DatabaseType::SQLite)
            .pool(pool)
            .alias("test")
            .id_strategy(IdStrategy::AutoIncrement)
            .build()
            .unwrap_err();
        assert_eq!(config_message(&err), "连接配置必须设置");
    }

    #[test]
    fn test_database_pool_required_zh() {
        setup_i18n("zh-CN");
        let err = DatabaseConfigBuilder::new()
            .db_type(DatabaseType::SQLite)
            .connection(ConnectionConfig::SQLite {
                path: "/tmp/test.db".into(),
                create_if_missing: true,
            })
            .alias("test")
            .id_strategy(IdStrategy::AutoIncrement)
            .build()
            .unwrap_err();
        assert_eq!(config_message(&err), "连接池配置必须设置");
    }

    #[test]
    fn test_database_alias_required_zh() {
        setup_i18n("zh-CN");
        let pool = valid_pool_config();
        let err = DatabaseConfigBuilder::new()
            .db_type(DatabaseType::SQLite)
            .connection(ConnectionConfig::SQLite {
                path: "/tmp/test.db".into(),
                create_if_missing: true,
            })
            .pool(pool)
            .id_strategy(IdStrategy::AutoIncrement)
            .build()
            .unwrap_err();
        assert_eq!(config_message(&err), "数据库别名必须设置");
    }

    #[test]
    fn test_database_id_strategy_required_zh() {
        setup_i18n("zh-CN");
        let pool = valid_pool_config();
        let err = DatabaseConfigBuilder::new()
            .db_type(DatabaseType::SQLite)
            .connection(ConnectionConfig::SQLite {
                path: "/tmp/test.db".into(),
                create_if_missing: true,
            })
            .pool(pool)
            .alias("test")
            .build()
            .unwrap_err();
        assert_eq!(config_message(&err), "ID生成策略必须设置");
    }

    #[test]
    fn test_database_type_mismatch_zh() {
        setup_i18n("zh-CN");
        let pool = valid_pool_config();
        let err = DatabaseConfigBuilder::new()
            .db_type(DatabaseType::SQLite)
            .connection(ConnectionConfig::PostgreSQL {
                host: "localhost".into(),
                port: 5432,
                database: "test".into(),
                username: "user".into(),
                password: "pass".into(),
                ssl_mode: Some("prefer".into()),
                tls_config: None,
            })
            .pool(pool)
            .alias("test")
            .id_strategy(IdStrategy::AutoIncrement)
            .build()
            .unwrap_err();
        assert_eq!(
            config_message(&err),
            "数据库类型 SQLite 与连接配置不匹配"
        );
    }

    #[test]
    fn test_database_type_required_en() {
        setup_i18n("en-US");
        let pool = valid_pool_config();
        let err = DatabaseConfigBuilder::new()
            .connection(ConnectionConfig::SQLite {
                path: "/tmp/test.db".into(),
                create_if_missing: true,
            })
            .pool(pool)
            .alias("test")
            .id_strategy(IdStrategy::AutoIncrement)
            .build()
            .unwrap_err();
        assert_eq!(config_message(&err), "Database type is required");
    }

    #[test]
    fn test_database_type_mismatch_en() {
        setup_i18n("en-US");
        let pool = valid_pool_config();
        let err = DatabaseConfigBuilder::new()
            .db_type(DatabaseType::SQLite)
            .connection(ConnectionConfig::PostgreSQL {
                host: "localhost".into(),
                port: 5432,
                database: "test".into(),
                username: "user".into(),
                password: "pass".into(),
                ssl_mode: Some("prefer".into()),
                tls_config: None,
            })
            .pool(pool)
            .alias("test")
            .id_strategy(IdStrategy::AutoIncrement)
            .build()
            .unwrap_err();
        assert_eq!(
            config_message(&err),
            "Database type SQLite does not match connection config"
        );
    }

    #[test]
    fn test_database_type_required_ja() {
        setup_i18n("ja-JP");
        let pool = valid_pool_config();
        let err = DatabaseConfigBuilder::new()
            .connection(ConnectionConfig::SQLite {
                path: "/tmp/test.db".into(),
                create_if_missing: true,
            })
            .pool(pool)
            .alias("test")
            .id_strategy(IdStrategy::AutoIncrement)
            .build()
            .unwrap_err();
        assert_eq!(config_message(&err), "データベースタイプは必須です");
    }

    // ===== helper =====

    fn valid_pool_config() -> PoolConfig {
        PoolConfigBuilder::new()
            .min_connections(1)
            .max_connections(10)
            .connection_timeout(30)
            .idle_timeout(600)
            .max_lifetime(1800)
            .max_retries(3)
            .retry_interval_ms(1000)
            .keepalive_interval_sec(60)
            .health_check_timeout_sec(5)
            .build()
            .unwrap()
    }
}
