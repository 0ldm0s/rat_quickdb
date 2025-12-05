//! # 配置构建器模块
//!
//! 提供所有配置类型的构建器实现，支持链式调用和严格验证

pub mod app_builder;
pub mod database_builder;
pub mod global_builder;
pub mod logging_builder;
pub mod pool_builder;

// 重新导出所有Builder类型以保持API兼容性
pub use app_builder::AppConfigBuilder;
pub use database_builder::DatabaseConfigBuilder;
pub use global_builder::GlobalConfigBuilder;
pub use logging_builder::LoggingConfigBuilder;
pub use pool_builder::PoolConfigBuilder;
