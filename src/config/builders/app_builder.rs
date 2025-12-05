//! # 应用配置构建器模块
//!
//! 提供应用配置的构建器实现，支持链式调用和严格验证

use crate::config::core::{AppConfig, Environment};
use crate::error::QuickDbError;
use rat_logger::info;
use std::path::PathBuf;

/// 应用配置构建器
#[derive(Debug)]
pub struct AppConfigBuilder {
    name: Option<String>,
    version: Option<String>,
    environment: Option<Environment>,
    debug: Option<bool>,
    work_dir: Option<PathBuf>,
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
        let name = self
            .name
            .ok_or_else(|| crate::quick_error!(config, "应用名称必须设置"))?;

        let version = self
            .version
            .ok_or_else(|| crate::quick_error!(config, "应用版本必须设置"))?;

        let environment = self
            .environment
            .ok_or_else(|| crate::quick_error!(config, "环境类型必须设置"))?;

        let debug = self
            .debug
            .ok_or_else(|| crate::quick_error!(config, "调试模式必须设置"))?;

        let work_dir = self
            .work_dir
            .ok_or_else(|| crate::quick_error!(config, "工作目录必须设置"))?;

        info!(
            "创建应用配置: 名称={}, 版本={}, 环境={:?}",
            name, version, environment
        );

        Ok(AppConfig {
            name,
            version,
            environment,
            debug,
            work_dir,
        })
    }
}
impl Default for AppConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}
