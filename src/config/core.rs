//! # 配置管理模块 - 核心配置类型
//!
//! 提供统一的配置管理系统，支持构建器模式和链式配置
//! 严格遵循项目规范：所有配置项必须显式设置，严禁使用默认值

use crate::error::QuickDbError;
use crate::types::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use rat_logger::{info};

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

impl GlobalConfig {
    /// 创建全局配置构建器
    pub fn builder() -> super::builders::GlobalConfigBuilder {
        super::builders::GlobalConfigBuilder::new()
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

impl AppConfig {
    /// 创建应用配置构建器
    pub fn builder() -> super::builders::AppConfigBuilder {
        super::builders::AppConfigBuilder::new()
    }
}

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