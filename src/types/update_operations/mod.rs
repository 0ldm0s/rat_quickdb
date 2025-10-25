use serde::{Deserialize, Serialize};
use crate::types::data_value::DataValue;

/// 更新操作符
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UpdateOperator {
    /// 直接设置值
    Set,
    /// 原子性增加
    Increment,
    /// 原子性减少
    Decrement,
    /// 原子性乘法
    Multiply,
    /// 原子性除法
    Divide,
    /// 百分比增加 (值是百分比，如10表示增加10%)
    PercentIncrease,
    /// 百分比减少 (值是百分比，如10表示减少10%)
    PercentDecrease,
}

/// 更新操作定义
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UpdateOperation {
    /// 要更新的字段名
    pub field: String,
    /// 更新操作类型
    pub operation: UpdateOperator,
    /// 更新的值
    pub value: DataValue,
}

impl UpdateOperation {
    /// 创建一个设置操作
    pub fn set(field: impl Into<String>, value: impl Into<DataValue>) -> Self {
        Self {
            field: field.into(),
            operation: UpdateOperator::Set,
            value: value.into(),
        }
    }

    /// 创建一个增加操作
    pub fn increment(field: impl Into<String>, value: impl Into<DataValue>) -> Self {
        Self {
            field: field.into(),
            operation: UpdateOperator::Increment,
            value: value.into(),
        }
    }

    /// 创建一个减少操作
    pub fn decrement(field: impl Into<String>, value: impl Into<DataValue>) -> Self {
        Self {
            field: field.into(),
            operation: UpdateOperator::Decrement,
            value: value.into(),
        }
    }

    /// 创建一个乘法操作
    pub fn multiply(field: impl Into<String>, value: impl Into<DataValue>) -> Self {
        Self {
            field: field.into(),
            operation: UpdateOperator::Multiply,
            value: value.into(),
        }
    }

    /// 创建一个除法操作
    pub fn divide(field: impl Into<String>, value: impl Into<DataValue>) -> Self {
        Self {
            field: field.into(),
            operation: UpdateOperator::Divide,
            value: value.into(),
        }
    }

    /// 创建一个百分比增加操作
    pub fn percent_increase(field: impl Into<String>, percentage: f64) -> Self {
        Self {
            field: field.into(),
            operation: UpdateOperator::PercentIncrease,
            value: DataValue::Float(percentage),
        }
    }

    /// 创建一个百分比减少操作
    pub fn percent_decrease(field: impl Into<String>, percentage: f64) -> Self {
        Self {
            field: field.into(),
            operation: UpdateOperator::PercentDecrease,
            value: DataValue::Float(percentage),
        }
    }
}