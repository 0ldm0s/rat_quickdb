//! 基础类型的 ToDataValue 实现
//!
//! 为 String、数值类型、布尔类型等基础类型实现 ToDataValue

use crate::types::DataValue;
use crate::model::conversion::ToDataValue;

// 字符串类型实现
impl ToDataValue for String {
    fn to_data_value(&self) -> DataValue {
        DataValue::String(self.clone())
    }
}

impl ToDataValue for &str {
    fn to_data_value(&self) -> DataValue {
        DataValue::String(self.to_string())
    }
}

// 整数类型实现
impl ToDataValue for i32 {
    fn to_data_value(&self) -> DataValue {
        DataValue::Int(*self as i64)
    }
}

impl ToDataValue for i64 {
    fn to_data_value(&self) -> DataValue {
        DataValue::Int(*self)
    }
}

// 浮点类型实现
impl ToDataValue for f32 {
    fn to_data_value(&self) -> DataValue {
        DataValue::Float(*self as f64)
    }
}

impl ToDataValue for f64 {
    fn to_data_value(&self) -> DataValue {
        DataValue::Float(*self)
    }
}

// 布尔类型实现
impl ToDataValue for bool {
    fn to_data_value(&self) -> DataValue {
        DataValue::Bool(*self)
    }
}

// DateTime类型实现
impl ToDataValue for chrono::DateTime<chrono::Utc> {
    fn to_data_value(&self) -> DataValue {
        let fixed_dt = self.with_timezone(&chrono::FixedOffset::east(0));
               DataValue::DateTime(fixed_dt)
    }
}

impl ToDataValue for chrono::DateTime<chrono::FixedOffset> {
    fn to_data_value(&self) -> DataValue {
             DataValue::DateTime(*self)
    }
}

// UUID类型实现
impl ToDataValue for uuid::Uuid {
    fn to_data_value(&self) -> DataValue {
        DataValue::Uuid(*self)
    }
}

// JsonValue类型实现
impl ToDataValue for serde_json::Value {
    fn to_data_value(&self) -> DataValue {
        DataValue::Json(self.clone())
    }
}

// Option类型实现
impl<T> ToDataValue for Option<T>
where
    T: ToDataValue,
{
    fn to_data_value(&self) -> DataValue {
        match self {
            Some(v) => v.to_data_value(),
            None => DataValue::Null,
        }
    }
}