//! 数据库感知的数据转换模块
//!
//! 根据不同数据库类型实现差异化的字段转换逻辑

use crate::error::{QuickDbError, QuickDbResult};
use crate::types::{DataValue, DatabaseType};
use crate::model::{FieldType, conversion::ToDataValue};

/// 数据库感知的DateTimeWithTz字段转换
///
/// 根据数据库类型和时区偏移进行相应的转换处理
///
/// # 参数
/// * `value` - 待转换的字段值
/// * `timezone_offset` - 时区偏移，如 "+08:00"、"-05:00"
/// * `db_type` - 数据库类型
///
/// # 返回值
/// * `Ok(DataValue)` - 转换后的数据值
/// * `Err(QuickDbError)` - 转换失败时的错误信息
pub fn convert_datetime_with_tz_aware<T: std::fmt::Debug + ToDataValue>(
    value: &T,
    timezone_offset: &str,
    db_type: Option<DatabaseType>,
) -> QuickDbResult<DataValue> {
    println!("🚨 convert_datetime_with_tz_aware被调用！时区偏移: {}, 数据库类型: {:?}, 值: {:?}", timezone_offset, db_type, value);

    let db_type = db_type.expect("严重错误：无法确定数据库类型！这表明框架内部存在严重问题！");

    match db_type {
        DatabaseType::SQLite => {
            // SQLite特殊处理：转换为Unix时间戳
            println!("🔧 SQLite特定处理：时区偏移 {}，值 {:?}", timezone_offset, value);

            // 先调用现有函数获取UTC DateTime
            let utc_result = crate::convert_string_to_datetime_with_tz(value, timezone_offset)?;

            // 然后转换为时间戳
            match utc_result {
                DataValue::DateTime(dt) => {
                    let timestamp = dt.timestamp();
                    println!("🔧 SQLite DateTime转换为时间戳: {} -> {}", dt, timestamp);
                    Ok(DataValue::Int(timestamp))
                },
                other => Ok(other), // 对于非DateTime类型，直接返回原结果
            }
        }
        _ => {
            // MySQL/PostgreSQL/MongoDB：使用通用转换逻辑
            convert_datetime_with_tz_general(value, timezone_offset)
        }
    }
}

/// 通用的DateTimeWithTz字段转换（MySQL/PostgreSQL/MongoDB）
fn convert_datetime_with_tz_general<T: std::fmt::Debug + ToDataValue>(
    value: &T,
    timezone_offset: &str,
) -> QuickDbResult<DataValue> {
    println!("🔧 convert_datetime_with_tz_general被调用！时区偏移: {}, 值: {:?}", timezone_offset, value);

    // 转换为DataValue看看类型
    let data_value = value.to_data_value();

    println!("🔧 to_data_value后的类型: {:?}", data_value);

    match data_value {
        DataValue::DateTime(dt) => {
            // DateTime输入：直接存储为UTC，不做时区转换
            println!("🔧 DateTime输入，直接存储UTC时间: {}", dt);
            Ok(DataValue::DateTime(dt))
        },
        DataValue::String(s) => {
            // String输入：使用时区转换逻辑
            println!("🔧 String输入，使用时区转换: {}", s);
            crate::convert_string_to_datetime_with_tz(value, timezone_offset)
        },
        _ => {
            // 其他类型：直接返回
            Ok(data_value)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_database_aware_conversion() {
        // 测试MySQL类型
        let dt = Utc::now();
        let result = convert_datetime_with_tz_aware(&dt, "+08:00", Some(DatabaseType::MySQL));
        assert!(result.is_ok());

        // 测试SQLite类型
        let result = convert_datetime_with_tz_aware(&dt, "-05:00", Some(DatabaseType::SQLite));
        assert!(result.is_ok());

        // 测试未知类型
        let result = convert_datetime_with_tz_aware(&dt, "+00:00", None);
        assert!(result.is_ok());
    }
}