//! 模型便捷函数模块
//!
//! 提供创建各种字段类型的便捷函数

use crate::model::field_types::{FieldType, FieldDefinition};
use std::collections::HashMap;

/// 便捷函数：创建数组字段
/// 在 MongoDB 中使用原生数组，在 SQL 数据库中使用 JSON 存储
pub fn array_field(
    item_type: FieldType,
    max_items: Option<usize>,
    min_items: Option<usize>,
) -> FieldDefinition {
    FieldDefinition::new(FieldType::Array {
        item_type: Box::new(item_type),
        max_items,
        min_items,
    })
}

/// 便捷函数：创建列表字段（array_field 的别名）
/// 在 MongoDB 中使用原生数组，在 SQL 数据库中使用 JSON 存储
pub fn list_field(
    item_type: FieldType,
    max_items: Option<usize>,
    min_items: Option<usize>,
) -> FieldDefinition {
    // list_field 是 array_field 的别名，提供更直观的命名
    array_field(item_type, max_items, min_items)
}

/// 便捷函数：创建字符串字段
pub fn string_field(
    max_length: Option<usize>,
    min_length: Option<usize>,
    regex: Option<String>,
) -> FieldDefinition {
    FieldDefinition::new(FieldType::String {
        max_length,
        min_length,
        regex,
    })
}

/// 便捷函数：创建整数字段
pub fn integer_field(
    min_value: Option<i64>,
    max_value: Option<i64>,
) -> FieldDefinition {
    FieldDefinition::new(FieldType::Integer {
        min_value,
        max_value,
    })
}

/// 便捷函数：创建浮点数字段
pub fn float_field(
    min_value: Option<f64>,
    max_value: Option<f64>,
) -> FieldDefinition {
    FieldDefinition::new(FieldType::Float {
        min_value,
        max_value,
    })
}

/// 便捷函数：创建布尔字段
pub fn boolean_field() -> FieldDefinition {
    FieldDefinition::new(FieldType::Boolean)
}

/// 便捷函数：创建日期时间字段
pub fn datetime_field() -> FieldDefinition {
    // 为了向后兼容，内部使用 datetime_with_tz_field，默认UTC时区
    datetime_with_tz_field("+00:00")
}

/// 便捷函数：创建带时区的日期时间字段
///
/// 时区偏移格式："+00:00", "+08:00", "-05:00"
/// 存储为Unix时间戳（INTEGER），提供最佳的性能和范围查询支持
pub fn datetime_with_tz_field(timezone_offset: &str) -> FieldDefinition {
    FieldDefinition::new(FieldType::DateTimeWithTz {
        timezone_offset: timezone_offset.to_string(),
    })
}

/// 便捷函数：创建UUID字段
pub fn uuid_field() -> FieldDefinition {
    FieldDefinition::new(FieldType::Uuid)
}

/// 便捷函数：创建JSON字段
pub fn json_field() -> FieldDefinition {
    FieldDefinition::new(FieldType::Json)
}

/// 便捷函数：创建字典字段（基于Object类型）
pub fn dict_field(fields: HashMap<String, FieldDefinition>) -> FieldDefinition {
    FieldDefinition::new(FieldType::Object { fields })
}

/// 便捷函数：创建引用字段
pub fn reference_field(target_collection: String) -> FieldDefinition {
    FieldDefinition::new(FieldType::Reference {
        target_collection,
    })
}