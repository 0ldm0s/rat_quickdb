//! 字段版本管理模块
//!
//! 提供模型字段版本控制功能，支持升级/回滚并生成 DDL

pub mod ddl;
pub mod manager;
pub mod types;

pub use manager::FieldVersionManager;
pub use types::{ModelVersionMeta, VersionChange, VersionUpgradeResult};
