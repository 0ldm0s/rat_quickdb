//! 集合类型的 ToDataValue 实现
//!
//! 为 Vec、HashMap 等集合类型实现 ToDataValue

use crate::model::conversion::ToDataValue;
use crate::types::DataValue;
use std::collections::HashMap;

// Vec<String> 实现
impl ToDataValue for Vec<String> {
    fn to_data_value(&self) -> DataValue {
        // 将字符串数组转换为DataValue::Array
        let data_values: Vec<DataValue> =
            self.iter().map(|s| DataValue::String(s.clone())).collect();
        DataValue::Array(data_values)
    }
}

// Vec<i32> 实现
impl ToDataValue for Vec<i32> {
    fn to_data_value(&self) -> DataValue {
        // 将整数数组转换为DataValue::Array
        let data_values: Vec<DataValue> = self.iter().map(|&i| DataValue::Int(i as i64)).collect();
        DataValue::Array(data_values)
    }
}

// Vec<i64> 实现
impl ToDataValue for Vec<i64> {
    fn to_data_value(&self) -> DataValue {
        // 将整数数组转换为DataValue::Array
        let data_values: Vec<DataValue> = self.iter().map(|&i| DataValue::Int(i)).collect();
        DataValue::Array(data_values)
    }
}

// Vec<f64> 实现
impl ToDataValue for Vec<f64> {
    fn to_data_value(&self) -> DataValue {
        // 将浮点数组转换为DataValue::Array
        let data_values: Vec<DataValue> = self.iter().map(|&f| DataValue::Float(f)).collect();
        DataValue::Array(data_values)
    }
}

// Vec<bool> 实现
impl ToDataValue for Vec<bool> {
    fn to_data_value(&self) -> DataValue {
        // 将布尔数组转换为DataValue::Array
        let data_values: Vec<DataValue> = self.iter().map(|&b| DataValue::Bool(b)).collect();
        DataValue::Array(data_values)
    }
}

// HashMap<String, DataValue> 实现
impl ToDataValue for HashMap<String, DataValue> {
    fn to_data_value(&self) -> DataValue {
        // 将字典转换为DataValue::Object
        DataValue::Object(self.clone())
    }
}
