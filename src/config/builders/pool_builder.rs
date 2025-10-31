//! # 连接池配置构建器模块
//!
//! 提供连接池配置的构建器实现，支持链式调用和严格验证

use crate::types::*;
use crate::error::QuickDbError;
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

        let max_retries = self.max_retries.ok_or_else(|| {
            crate::quick_error!(config, "最大重试次数必须设置")
        })?;

        let retry_interval_ms = self.retry_interval_ms.ok_or_else(|| {
            crate::quick_error!(config, "重试间隔必须设置")
        })?;

        let keepalive_interval_sec = self.keepalive_interval_sec.ok_or_else(|| {
            crate::quick_error!(config, "保活检测间隔必须设置")
        })?;

        let health_check_timeout_sec = self.health_check_timeout_sec.ok_or_else(|| {
            crate::quick_error!(config, "健康检查超时时间必须设置")
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
            connection_timeout: connection_timeout * 1000,  // 转换为毫秒
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
