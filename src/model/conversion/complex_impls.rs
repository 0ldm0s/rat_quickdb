//! 复杂类型的 ToDataValue 实现
//!
//! 为更复杂的类型实现 ToDataValue，包括嵌套结构等

use crate::model::conversion::ToDataValue;
use crate::types::DataValue;
use serde_json::Value as JsonValue;

// 为其他复杂类型提供 ToDataValue 实现的占位符
// 这里可以根据需要添加更多复杂类型的实现

// 示例：为自定义结构体实现 ToDataValue 的宏
// #[macro_export]
// macro_rules! impl_to_data_value_for_struct {
//     ($struct_name:ident, { $($field:ident),* }) => {
//         impl ToDataValue for $struct_name {
//             fn to_data_value(&self) -> DataValue {
//                 let mut obj = std::collections::HashMap::new();
//                 $(
//                     obj.insert(stringify!($field).to_string(), self.$field.to_data_value());
//                 )*
//                 DataValue::Object(obj)
//             }
//         }
//     };
// }
