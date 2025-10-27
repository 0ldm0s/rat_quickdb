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
