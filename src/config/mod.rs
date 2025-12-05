//! # 配置管理模块
//!
//! 提供统一的配置管理系统，支持构建器模式和链式配置
//! 严格遵循项目规范：所有配置项必须显式设置，严禁使用默认值

pub mod builders;
pub mod convenience;
pub mod core;

// 重新导出所有公共类型以保持API兼容性
pub use builders::{
    AppConfigBuilder, DatabaseConfigBuilder, GlobalConfigBuilder, LoggingConfigBuilder,
    PoolConfigBuilder,
};
pub use convenience::{mongodb_config, mysql_config, postgres_config, sqlite_config};
pub use core::{AppConfig, Environment, GlobalConfig, LogLevel, LoggingConfig};
