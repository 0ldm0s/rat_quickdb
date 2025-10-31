//! 连接池配置模块

use crate::types::*;

/// 连接池配置扩展
#[derive(Debug, Clone)]
pub struct ExtendedPoolConfig {
    /// 基础连接池配置
    pub base: PoolConfig,
    /// 最大重试次数
    pub max_retries: u32,
    /// 重试间隔（毫秒）
    pub retry_interval_ms: u64,
    /// 保活检测间隔（秒）
    pub keepalive_interval_sec: u64,
    /// 连接健康检查超时（秒）
    pub health_check_timeout_sec: u64,
}

impl Default for ExtendedPoolConfig {
    fn default() -> Self {
        Self {
            base: PoolConfig::default(),
            max_retries: 3,
            retry_interval_ms: 1000,
            keepalive_interval_sec: 30,
            health_check_timeout_sec: 5,
        }
    }
}

impl ExtendedPoolConfig {
    /// 从用户PoolConfig创建ExtendedPoolConfig，保留用户配置
    pub fn from_pool_config(pool_config: PoolConfig) -> Self {
        Self {
            base: PoolConfig {
                min_connections: pool_config.min_connections,
                max_connections: pool_config.max_connections,
                connection_timeout: pool_config.connection_timeout,
                idle_timeout: pool_config.idle_timeout,
                max_lifetime: pool_config.max_lifetime,
                max_retries: pool_config.max_retries,
                retry_interval_ms: pool_config.retry_interval_ms,
                keepalive_interval_sec: pool_config.keepalive_interval_sec,
                health_check_timeout_sec: pool_config.health_check_timeout_sec,
            },
            max_retries: pool_config.max_retries,
            retry_interval_ms: pool_config.retry_interval_ms,
            keepalive_interval_sec: pool_config.keepalive_interval_sec,
            health_check_timeout_sec: pool_config.health_check_timeout_sec,
        }
    }
}
