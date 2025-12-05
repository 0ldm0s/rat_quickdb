//! ToDataValue trait 定义
//!
//! 定义了将各种类型转换为 DataValue 的统一接口

use crate::types::DataValue;

/// 支持直接转换为 DataValue 的 trait
/// 避免 JSON 序列化的性能开销
pub trait ToDataValue {
    fn to_data_value(&self) -> DataValue;
}
