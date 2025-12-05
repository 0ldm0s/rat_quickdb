//! 模型定义系统模块
//!
//! 参考mongoengine的设计，支持通过结构体定义数据表结构
//! 提供字段类型、验证、索引等功能

pub mod convenience;
pub mod conversion;
pub mod data_conversion;
pub mod field_types;
pub mod macros;
pub mod manager;
pub mod traits;

// 重新导出核心类型（保持向后兼容）
pub use convenience::*;
pub use conversion::ToDataValue;
pub use data_conversion::{create_model_from_data_map, create_model_from_data_map_with_debug};
pub use field_types::{FieldDefinition, FieldType, IndexDefinition, ModelMeta};
pub use macros::*;
pub use manager::ModelManager;
pub use traits::{Model, ModelOperations};
