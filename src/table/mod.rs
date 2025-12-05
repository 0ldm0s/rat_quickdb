//! 表管理模块
//!
//! 提供表的自动创建、版本管理和模式定义功能

pub mod manager;
pub mod schema;
pub mod version;

pub use manager::TableManager;
pub use schema::{
    ColumnDefinition, ColumnType, ConstraintDefinition, ConstraintType, IndexDefinition, IndexType,
    TableSchema,
};
pub use version::{MigrationScript, SchemaVersion, VersionManager};
