//! # 全局配置构建器模块
//!
//! 提供全局配置的构建器实现，支持链式调用和严格验证

use crate::config::core::{AppConfig, Environment, GlobalConfig, LogLevel, LoggingConfig};
use crate::error::QuickDbError;
use crate::types::*;
use rat_logger::info;
use std::collections::HashMap;
use std::path::PathBuf;

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

        let app = self
            .app
            .ok_or_else(|| crate::quick_error!(config, "应用配置必须设置"))?;

        let logging = self
            .logging
            .ok_or_else(|| crate::quick_error!(config, "日志配置必须设置"))?;

        // 验证默认数据库是否存在
        if let Some(ref default_alias) = self.default_database {
            if !self.databases.contains_key(default_alias) {
                return Err(crate::quick_error!(
                    config,
                    format!("默认数据库 '{}' 不存在于数据库配置中", default_alias)
                ));
            }
        }

        info!(
            "创建全局配置: 数据库数量={}, 默认数据库={:?}",
            self.databases.len(),
            self.default_database
        );

        Ok(GlobalConfig {
            databases: self.databases,
            default_database: self.default_database,
            app,
            logging,
        })
    }
}
impl Default for GlobalConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}
