//! JSON字段数组类型处理验证测试 (PostgreSQL版本)
//!
//! 验证JSON字段在处理数组类型数据时是否会正常转换为object而不是string

use chrono::Utc;
use rat_logger::{LevelFilter, LoggerBuilder, handler::term::TermConfig};
use rat_quickdb::types::*;
use rat_quickdb::*;
use rat_quickdb::{
    ModelManager, ModelOperations,
    json_field, array_field, uuid_field,
};

// 简化的测试模型 - 专注于JSON和数组字段
define_model! {
    struct TestModel {
        id: String,
        // JSON字段 - 用于测试数组类型处理
        json_field: Option<serde_json::Value>,
        // 数组字段 - 用于对比
        array_field: Option<Vec<String>>,
        // 创建时间
        created_at: chrono::DateTime<chrono::Utc>,
    }
    collection = "json_array_test",
    database = "main",
    fields = {
        id: uuid_field().required().unique(),
        json_field: json_field(),
        array_field: array_field(field_types!(string), None, None),
        created_at: datetime_field().required(),
    }
    indexes = [],
}

// 清理测试数据
async fn cleanup_test_data() {
    println!("清理测试数据...");
    if let Err(e) = rat_quickdb::drop_table("main", "json_array_test").await {
        println!("清理测试表失败: {}", e);
    }
}

// 测试1: 同时给两个字段写入数组类型数据
async fn test_array_type_assignment() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== 测试1: 数组类型数据写入 ===");

    // 创建测试数据 - 使用数组类型数据
    let test_data = TestModel {
        id: String::new(),
        // 给json_field写入数组类型数据
        json_field: Some(serde_json::Value::Array(vec![
            serde_json::Value::String("元素1".to_string()),
            serde_json::Value::String("元素2".to_string()),
            serde_json::Value::String("元素3".to_string()),
        ])),
        // 给array_field写入数组类型数据
        array_field: Some(vec![
            "数组元素1".to_string(),
            "数组元素2".to_string(),
            "数组元素3".to_string(),
        ]),
        created_at: Utc::now(),
    };

    // 插入数据
    let record_id = match test_data.save().await {
        Ok(id) => {
            println!("✅ 测试数据创建成功: {}", id);
            id
        }
        Err(e) => {
            println!("❌ 测试数据创建失败: {}", e);
            return Err(e.into());
        }
    };

    // 查询并验证数据类型
    match ModelManager::<TestModel>::find_by_id(&record_id).await {
        Ok(Some(record)) => {
            println!("✅ 数据查询成功");

            // 检查json_field的数据类型
            match &record.json_field {
                Some(serde_json::Value::Array(arr)) => {
                    println!("✅ json_field 是Array类型，包含 {} 个元素", arr.len());
                    for (i, item) in arr.iter().enumerate() {
                        println!("  元素{}: {:?}", i + 1, item);
                    }
                }
                Some(serde_json::Value::Object(obj)) => {
                    println!("✅ json_field 是Object类型: {:?}", obj);
                }
                Some(other) => {
                    println!("⚠️  json_field 是其他类型: {:?}", other);
                }
                None => {
                    println!("❌ json_field 为None");
                }
            }

            // 检查array_field的数据类型
            match &record.array_field {
                Some(arr) => {
                    println!("✅ array_field 是Array类型，包含 {} 个元素", arr.len());
                    for (i, item) in arr.iter().enumerate() {
                        println!("  元素{}: {:?}", i + 1, item);
                    }
                }
                None => {
                    println!("❌ array_field 为None");
                }
            }

            // 保留测试数据以便后续验证
            println!("✅ 测试数据已保留，可用于数据库验证");
        }
        Ok(None) => {
            println!("❌ 测试数据未找到");
        }
        Err(e) => {
            println!("❌ 查询失败: {}", e);
        }
    }

    Ok(())
}

// 测试2: 给json_field写入正常JSON，给array_field写入空数组
async fn test_json_and_empty_array() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== 测试2: 正常JSON和空数组处理 ===");

    // 创建测试数据 - json_field用正常JSON，array_field用空数组
    let test_data = TestModel {
        id: String::new(),
        // 给json_field写入正常JSON对象
        json_field: Some(serde_json::json!({
            "name": "测试对象",
            "count": 42,
            "active": true
        })),
        // 给array_field写入空数组
        array_field: Some(vec![]),
        created_at: Utc::now(),
    };

    // 插入数据
    let record_id = match test_data.save().await {
        Ok(id) => {
            println!("✅ 测试数据创建成功: {}", id);
            id
        }
        Err(e) => {
            println!("❌ 测试数据创建失败: {}", e);
            return Err(e.into());
        }
    };

    // 查询并验证数据类型
    match ModelManager::<TestModel>::find_by_id(&record_id).await {
        Ok(Some(record)) => {
            println!("✅ 数据查询成功");

            // 检查json_field的数据类型
            match &record.json_field {
                Some(serde_json::Value::Object(obj)) => {
                    println!("✅ json_field 是Object类型，包含 {} 个字段", obj.len());
                    for (key, value) in obj.iter() {
                        println!("  字段 '{}': {:?}", key, value);
                    }
                }
                Some(serde_json::Value::Array(arr)) => {
                    println!("⚠️  json_field 是Array类型，包含 {} 个元素", arr.len());
                    for (i, item) in arr.iter().enumerate() {
                        println!("  元素{}: {:?}", i + 1, item);
                    }
                }
                Some(other) => {
                    println!("⚠️  json_field 是其他类型: {:?}", other);
                }
                None => {
                    println!("❌ json_field 为None");
                }
            }

            // 检查array_field的数据类型
            match &record.array_field {
                Some(arr) => {
                    if arr.is_empty() {
                        println!("✅ array_field 是空数组");
                    } else {
                        println!("✅ array_field 是Array类型，包含 {} 个元素", arr.len());
                        for (i, item) in arr.iter().enumerate() {
                            println!("  元素{}: {:?}", i + 1, item);
                        }
                    }
                }
                None => {
                    println!("❌ array_field 为None");
                }
            }

            // 保留测试数据以便后续验证
            println!("✅ 测试数据已保留，可用于数据库验证");
        }
        Ok(None) => {
            println!("❌ 测试数据未找到");
        }
        Err(e) => {
            println!("❌ 查询失败: {}", e);
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== JSON字段数组类型处理验证测试 (PostgreSQL) ===");

    // 初始化日志
    LoggerBuilder::new()
        .with_level(LevelFilter::Info)
        .add_terminal_with_config(TermConfig::default())
        .init()?;

    // 初始化数据库 - 使用PostgreSQL连接信息
    let db_config = DatabaseConfig::builder()
        .db_type(DatabaseType::PostgreSQL)
        .connection(ConnectionConfig::PostgreSQL {
            host: "172.16.0.23".to_string(),
            port: 5432,
            database: "testdb".to_string(),
            username: "testdb".to_string(),
            password: "testdb".to_string(),
            ssl_mode: Some("prefer".to_string()),
            tls_config: Some(rat_quickdb::types::TlsConfig {
                enabled: true,
                ca_cert_path: None,
                client_cert_path: None,
                client_key_path: None,
                verify_server_cert: false,
                verify_hostname: false,
                min_tls_version: None,
                cipher_suites: None,
            }),
        })
        .pool(
            PoolConfig::builder()
                .max_connections(20)
                .min_connections(5)
                .connection_timeout(10)
                .idle_timeout(60)
                .max_lifetime(1800)
                .max_retries(5)
                .retry_interval_ms(500)
                .keepalive_interval_sec(30)
                .health_check_timeout_sec(5)
                .build()?,
        )
        .alias("main")
        .id_strategy(IdStrategy::Uuid) // 使用UUID策略
        .build()?;

    add_database(db_config).await?;
    println!("✅ 数据库连接成功");

    // 清理之前的测试数据
    cleanup_test_data().await;
    println!("✅ 清理完成");

    // 执行测试
    println!("\n开始执行测试...");

    // 测试1: 数组类型数据写入
    if let Err(e) = test_array_type_assignment().await {
        println!("❌ 测试1失败: {}", e);
    }

    // 测试2: 正常JSON和空数组处理
    if let Err(e) = test_json_and_empty_array().await {
        println!("❌ 测试2失败: {}", e);
    }

    println!("\n✅ 测试完成，数据已保留在数据库中供验证");
    println!("可用以下命令查看测试结果:");
    println!("psql -h 172.16.0.23 -U testdb -d testdb -c \"SELECT * FROM json_array_test;\"");

    Ok(())
}