//! PyO3 兼容序列化测试
//!
//! 测试 to_data_map_with_types() 方法的正确性，
//! 确保所有字段都使用 {类型名: 值} 的格式

use rat_quickdb::*;
use rat_quickdb::types::{DatabaseType, ConnectionConfig, PoolConfig, DataValue};
use std::collections::HashMap;
use chrono::{Utc, DateTime};
use serde::{Serialize, Deserialize};
use serde_json::Value as JsonValue;

// 定义简单的测试用户模型
define_model! {
    struct TestUser {
        id: String,
        username: String,
        age: Option<i32>,
        is_active: bool,
        last_login: Option<chrono::DateTime<chrono::Utc>>,
        score: f64,
        tags: Option<Vec<String>>,
    }
    collection = "test_users",
    fields = {
        id: string_field(None, None, None).required().unique(),
        username: string_field(None, None, None).required().unique(),
        age: integer_field(None, None),
        is_active: boolean_field().required(),
        last_login: datetime_field(),
        score: float_field(None, None).required(),
        tags: array_field(field_types!(string), None, None),
    }
}

#[tokio::test]
async fn test_pyo3_compatible_serialization() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志系统
    let _ = rat_logger::LoggerBuilder::new()
        .with_level(rat_logger::LevelFilter::Debug)
        .add_terminal_with_config(rat_logger::handler::term::TermConfig::default())
        .init();

    // 清理旧数据库文件
    let _ = std::fs::remove_file("./test_pyo3.db");

    // 初始化数据库
    let pool_config = PoolConfig::builder()
        .max_connections(10)
        .min_connections(2)
        .connection_timeout(5000)
        .idle_timeout(300000)
        .max_lifetime(1800000)
        .build()?;

    let db_config = DatabaseConfig::builder()
        .db_type(DatabaseType::SQLite)
        .connection(ConnectionConfig::SQLite {
            path: "./test_pyo3.db".to_string(),
            create_if_missing: true,
        })
        .pool(pool_config)
        .alias("default")
        .id_strategy(IdStrategy::Uuid)
        .build()?;

    add_database(db_config).await?;

    // === 测试1：包含各种数据类型的用户 ===
    println!("\n=== 测试1：包含各种数据类型的用户 ===");

    let user = TestUser {
        id: String::new(),
        username: "test_user".to_string(),
        age: Some(25),
        is_active: true,
        last_login: None,  // 这是我们要测试的 null 值
        score: 95.5,
        tags: Some(vec!["developer".to_string(), "rust".to_string()]),
    };

    // 保存用户
    let created_id = user.save().await?;
    println!("✅ 用户创建成功，ID: {}", created_id);

    // 查询用户
    let found_user = ModelManager::<TestUser>::find_by_id(&created_id).await?
        .ok_or("用户未找到")?;

    println!("✅ 找到用户: {} - {}", found_user.id, found_user.username);

    // 测试 PyO3 兼容序列化
    println!("\n--- PyO3 兼容数据映射测试 ---");
    let data_map = found_user.to_data_map_with_types_json()?;

    // 验证每个字段的格式
    println!("\n--- 字段验证 ---");

    // 验证 id 字段 (String)
    if let Some(id_value) = data_map.get("id") {
        println!("id: {}", id_value);
        match id_value {
            JsonValue::Object(obj) => {
                if let ((type_name, JsonValue::String(_))) = obj.iter().next().unwrap() {
                    assert_eq!(type_name, "String", "id 字段应该是 String 类型");
                    println!("✅ id 字段格式正确: {{{}: \"...\"}}", type_name);
                } else {
                    panic!("id 字段应该是包含 String 的对象");
                }
            },
            _ => panic!("id 字段应该是 JSON 对象"),
        }
    }

    // 验证 username 字段 (String)
    if let Some(username_value) = data_map.get("username") {
        println!("username: {}", username_value);
        match username_value {
            JsonValue::Object(obj) => {
                if let ((type_name, JsonValue::String(s))) = obj.iter().next().unwrap() {
                    assert_eq!(type_name, "String", "username 字段应该是 String 类型");
                    assert_eq!(s, "test_user", "username 值应该正确");
                    println!("✅ username 字段格式正确: {{{}: \"{}\"}}", type_name, s);
                } else {
                    panic!("username 字段应该是包含 String 的对象");
                }
            },
            _ => panic!("username 字段应该是 JSON 对象"),
        }
    }

    // 验证 age 字段 (Int)
    if let Some(age_value) = data_map.get("age") {
        println!("age: {}", age_value);
        match age_value {
            JsonValue::Object(obj) => {
                if let ((type_name, JsonValue::Number(n))) = obj.iter().next().unwrap() {
                    assert_eq!(type_name, "Int", "age 字段应该是 Int 类型");
                    assert_eq!(n.as_i64().unwrap(), 25, "age 值应该正确");
                    println!("✅ age 字段格式正确: {{{}: {}}}", type_name, n);
                } else {
                    panic!("age 字段应该是包含 Int 的对象");
                }
            },
            _ => panic!("age 字段应该是 JSON 对象"),
        }
    }

    // 验证 is_active 字段 (Bool)
    if let Some(is_active_value) = data_map.get("is_active") {
        println!("is_active: {}", is_active_value);
        match is_active_value {
            JsonValue::Object(obj) => {
                if let ((type_name, JsonValue::Bool(b))) = obj.iter().next().unwrap() {
                    assert_eq!(type_name, "Bool", "is_active 字段应该是 Bool 类型");
                    assert_eq!(*b, true, "is_active 值应该正确");
                    println!("✅ is_active 字段格式正确: {{{}: {}}}", type_name, b);
                } else {
                    panic!("is_active 字段应该是包含 Bool 的对象");
                }
            },
            _ => panic!("is_active 字段应该是 JSON 对象"),
        }
    }

    // 验证 last_login 字段 (DateTime with null)
    if let Some(last_login_value) = data_map.get("last_login") {
        println!("last_login: {}", last_login_value);
        match last_login_value {
            JsonValue::Object(obj) => {
                if let ((type_name, JsonValue::Null)) = obj.iter().next().unwrap() {
                    assert_eq!(type_name, "DateTime", "last_login 字段应该是 DateTime 类型");
                    println!("✅ last_login 字段格式正确: {{{}: null}}", type_name);
                } else {
                    panic!("last_login 字段应该是包含 Null 的对象");
                }
            },
            _ => panic!("last_login 字段应该是 JSON 对象"),
        }
    }

    // 验证 score 字段 (Float)
    if let Some(score_value) = data_map.get("score") {
        println!("score: {}", score_value);
        match score_value {
            JsonValue::Object(obj) => {
                if let ((type_name, JsonValue::Number(n))) = obj.iter().next().unwrap() {
                    assert_eq!(type_name, "Float", "score 字段应该是 Float 类型");
                    assert!((n.as_f64().unwrap() - 95.5).abs() < 0.001, "score 值应该正确");
                    println!("✅ score 字段格式正确: {{{}: {}}}", type_name, n);
                } else {
                    panic!("score 字段应该是包含 Float 的对象");
                }
            },
            _ => panic!("score 字段应该是 JSON 对象"),
        }
    }

    // 验证 tags 字段 (Array) - 修复后的数组元素类型标记
    if let Some(tags_value) = data_map.get("tags") {
        println!("tags: {}", tags_value);
        match tags_value {
            JsonValue::Object(obj) => {
                if let ((type_name, JsonValue::Array(arr))) = obj.iter().next().unwrap() {
                    assert_eq!(type_name, "Array", "tags 字段应该是 Array 类型");
                    assert_eq!(arr.len(), 2, "tags 数组长度应该正确");

                    // 验证数组中每个元素都有类型标记
                    for (i, element) in arr.iter().enumerate() {
                        match element {
                            JsonValue::Object(elem_obj) => {
                                if let ((elem_type_name, JsonValue::String(s))) = elem_obj.iter().next().unwrap() {
                                    assert_eq!(elem_type_name, "String", "数组元素应该是 String 类型");
                                    if i == 0 {
                                        assert_eq!(s, "developer", "第一个元素应该正确");
                                    } else {
                                        assert_eq!(s, "rust", "第二个元素应该正确");
                                    }
                                } else {
                                    panic!("数组元素应该是包含 String 的对象");
                                }
                            },
                            _ => panic!("数组元素应该是 JSON 对象"),
                        }
                    }

                    println!("✅ tags 字段格式正确: {{{}: [带类型标记的数组元素]}}", type_name);
                } else {
                    panic!("tags 字段应该是包含 Array 的对象");
                }
            },
            _ => panic!("tags 字段应该是 JSON 对象"),
        }
    }

    // 输出测试1的最终 JSON 字符串格式
    println!("\n--- 测试1：最终 JSON 输出 ---");
    let json_string = serde_json::to_string_pretty(&data_map)?;
    println!("{}", json_string);

    // === 测试2：所有可选字段都是 null 的用户 ===
    println!("\n=== 测试2：所有可选字段都是 null 的用户 ===");

    let null_user = TestUser {
        id: String::new(),
        username: "null_test_user".to_string(),
        age: None,
        is_active: false,
        last_login: None,
        score: 0.0,
        tags: None,
    };

    let null_created_id = null_user.save().await?;
    let found_null_user = ModelManager::<TestUser>::find_by_id(&null_created_id).await?
        .ok_or("null 用户未找到")?;

    println!("✅ null 用户创建成功，ID: {}", null_created_id);

    println!("\n--- Null 值处理测试 ---");
    let null_data_map = found_null_user.to_data_map_with_types_json()?;

    // 输出完整的 JSON 格式以便验证
    println!("\n--- 测试2：完整 JSON 输出 ---");
    let null_json_string = serde_json::to_string_pretty(&null_data_map)?;
    println!("{}", null_json_string);

    // 验证 null 字段的格式
    if let Some(age_value) = null_data_map.get("age") {
        println!("age (null): {}", age_value);
        match age_value {
            JsonValue::Object(obj) if obj.len() == 1 => {
                if let ((type_name, JsonValue::Null)) = obj.iter().next().unwrap() {
                    assert_eq!(type_name, "Int", "age 字段应该是 Int 类型，即使为 null");
                    println!("✅ age null 值格式正确: {{{}: null}}", type_name);
                } else {
                    panic!("age null 字段格式错误");
                }
            },
            _ => panic!("age null 字段应该是 Object 格式，实际: {}", age_value),
        }
    }

    if let Some(last_login_value) = null_data_map.get("last_login") {
        println!("last_login (null): {}", last_login_value);
        match last_login_value {
            JsonValue::Object(obj) if obj.len() == 1 => {
                if let ((type_name, JsonValue::Null)) = obj.iter().next().unwrap() {
                    assert_eq!(type_name, "DateTime", "last_login 字段应该是 DateTime 类型");
                    println!("✅ last_login null 值格式正确: {{{}: null}}", type_name);
                } else {
                    panic!("last_login null 字段格式错误");
                }
            },
            _ => panic!("last_login null 字段应该是 Object 格式，实际: {}", last_login_value),
        }
    }

    if let Some(tags_value) = null_data_map.get("tags") {
        println!("tags (null): {}", tags_value);
        match tags_value {
            JsonValue::Object(obj) if obj.len() == 1 => {
                if let ((type_name, JsonValue::Null)) = obj.iter().next().unwrap() {
                    assert_eq!(type_name, "Array", "tags 字段应该是 Array 类型，即使为 null");
                    println!("✅ tags null 值格式正确: {{{}: null}}", type_name);
                } else {
                    panic!("tags null 字段格式错误");
                }
            },
            _ => panic!("tags null 字段应该是 Object 格式，实际: {}", tags_value),
        }
    }

    println!("\n🎉 所有 PyO3 兼容序列化测试通过！包括数组元素类型标记和 null 值处理！");

    // 清理测试文件
    let _ = std::fs::remove_file("./test_pyo3.db");

    Ok(())
}