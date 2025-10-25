//! 模型定义系统模块
//!
//! 参考mongoengine的设计，支持通过结构体定义数据表结构
//! 提供字段类型、验证、索引等功能

pub mod conversion;
pub mod field_types;
pub mod traits;
pub mod manager;
pub mod macros;
pub mod convenience;

// 重新导出核心类型（保持向后兼容）
pub use conversion::ToDataValue;
pub use field_types::{FieldType, FieldDefinition, ModelMeta, IndexDefinition};
pub use traits::{Model, ModelOperations};
pub use manager::ModelManager;
pub use macros::*;
pub use convenience::*;