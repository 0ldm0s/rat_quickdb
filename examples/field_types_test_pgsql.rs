//! 字段类型验证测试 - PostgreSQL
//!
//! 验证 array_field、dict_field 和 json_field 在 PostgreSQL 中的使用

use rat_logger::{LevelFilter, LoggerBuilder, handler::term::TermConfig};
use rat_quickdb::types::*;
use rat_quickdb::*;
use rat_quickdb::{
    ModelManager, ModelOperations,
    array_field, dict_field, json_field, string_field, integer_field,
};
use std::collections::HashMap;

// 测试模型1 - 只包含 array_field 和 json_field（用于验证这两个是否正常）
define_model! {
    struct ArrayJsonTest {
        id: String,
        name: String,
        // 数组字段
        tags: Vec<String>,
        // JSON字段
        metadata: serde_json::Value,
    }
    collection = "array_json_test",
    database = "main",
    fields = {
        id: string_field(None, None, None).required().unique(),
        name: string_field(Some(100), None, None).required(),
        tags: array_field(field_types!(string), None, None),
        metadata: json_field(),
    }
    indexes = [],
}

// 测试模型2 - 包含 dict_field（用于验证 dict_field 的问题）
define_model! {
    struct DictFieldTest {
        id: String,
        name: String,
        // 字典字段 - 使用 HashMap<String, DataValue>
        config: HashMap<String, DataValue>,
    }
    collection = "dict_field_test",
    database = "main",
    fields = {
        id: string_field(None, None, None).required().unique(),
        name: string_field(Some(100), None, None).required(),
        config: dict_field({
            let mut fields = HashMap::new();
            fields.insert("theme".to_string(), string_field(None, None, None));
            fields.insert("language".to_string(), string_field(None, None, None));
            fields.insert("count".to_string(), integer_field(None, None));
            fields
        }),
    }
    indexes = [],
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== 字段类型验证测试 (PostgreSQL) ===\n");

    // 初始化日志
    LoggerBuilder::new()
        .with_level(LevelFilter::Info)
        .add_terminal_with_config(TermConfig::default())
        .init()?;

    // 初始化数据库 - PostgreSQL
    let db_config = DatabaseConfig {
        db_type: DatabaseType::PostgreSQL,
        connection: ConnectionConfig::PostgreSQL {
            host: "172.16.0.96".to_string(),
            port: 5432,
            database: "testdb".to_string(),
            username: "testdb".to_string(),
            password: "testdb".to_string(),
            ssl_mode: Some("prefer".to_string()),
            tls_config: None,
        },
        pool: PoolConfig::builder()
            .min_connections(1)
            .max_connections(5)
            .connection_timeout(30)
            .idle_timeout(300)
            .max_lifetime(1800)
            .max_retries(3)
            .retry_interval_ms(1000)
            .keepalive_interval_sec(60)
            .health_check_timeout_sec(10)
            .build()?,
        alias: "main".to_string(),
        cache: None,
        id_strategy: IdStrategy::Uuid,
    };

    add_database(db_config).await?;
    println!("✅ 数据库连接成功");

    // 清理旧表
    println!("\n清理旧测试表...");
    let _ = drop_table("main", "array_json_test").await;
    let _ = drop_table("main", "dict_field_test").await;

    // ============================================================
    // 测试1: array_field 和 json_field（这两个应该正常工作）
    // ============================================================
    println!("\n========================================");
    println!("测试1: array_field 和 json_field");
    println!("========================================\n");

    let test_record = ArrayJsonTest {
        id: String::new(),
        name: "测试记录1".to_string(),
        tags: vec!["tag1".to_string(), "tag2".to_string(), "tag3".to_string()],
        metadata: serde_json::json!({
            "version": "1.0",
            "author": "测试者",
            "count": 42,
            "nested": {
                "key": "value",
                "array": [1, 2, 3]
            }
        }),
    };

    let record_id = match test_record.save().await {
        Ok(id) => {
            println!("✅ 数据插入成功, ID: {}", id);
            id
        }
        Err(e) => {
            println!("❌ 数据插入失败: {}", e);
            return Err(e.into());
        }
    };

    // 查询并验证
    match ModelManager::<ArrayJsonTest>::find_by_id(&record_id).await {
        Ok(Some(record)) => {
            println!("✅ 数据查询成功");
            println!("   ID: {}", record.id);
            println!("   Name: {}", record.name);

            // 验证 array_field
            println!("\n   [array_field 验证]");
            println!("   tags: {:?}", record.tags);
            if record.tags.len() == 3 && record.tags[0] == "tag1" {
                println!("   ✅ array_field 正常工作");
            } else {
                println!("   ❌ array_field 数据不正确");
            }

            // 验证 json_field
            println!("\n   [json_field 验证]");
            println!("   metadata: {}", serde_json::to_string_pretty(&record.metadata).unwrap());
            if record.metadata["version"] == "1.0" && record.metadata["count"] == 42 {
                println!("   ✅ json_field 正常工作");
            } else {
                println!("   ❌ json_field 数据有问题");
            }
        }
        Ok(None) => {
            println!("❌ 数据未找到");
        }
        Err(e) => {
            println!("❌ 查询失败: {}", e);
            println!("\n⚠️  这就是 json_field 在 PostgreSQL 中的问题!");
        }
    }

    // ============================================================
    // 测试2: dict_field（预期会失败）
    // ============================================================
    println!("\n========================================");
    println!("测试2: dict_field（预期会有问题）");
    println!("========================================\n");

    let mut config_map = HashMap::new();
    config_map.insert("theme".to_string(), DataValue::String("dark".to_string()));
    config_map.insert("language".to_string(), DataValue::String("zh-CN".to_string()));
    config_map.insert("count".to_string(), DataValue::Int(42));

    let dict_record = DictFieldTest {
        id: String::new(),
        name: "dict测试记录".to_string(),
        config: config_map,
    };

    let dict_record_id = match dict_record.save().await {
        Ok(id) => {
            println!("✅ dict_field 数据插入成功, ID: {}", id);
            id
        }
        Err(e) => {
            println!("❌ dict_field 数据插入失败: {}", e);
            "N/A".to_string()
        }
    };

    if dict_record_id != "N/A" {
        match ModelManager::<DictFieldTest>::find_by_id(&dict_record_id).await {
            Ok(Some(record)) => {
                println!("✅ dict_field 数据查询成功");
                println!("   config: {:?}", record.config);

                if record.config.contains_key("theme") && record.config.contains_key("count") {
                    println!("   ✅ dict_field 正常工作");
                    if let Some(DataValue::Int(count)) = record.config.get("count") {
                        println!("      count = {}", count);
                    }
                } else {
                    println!("   ❌ dict_field 数据丢失");
                }
            }
            Ok(None) => {
                println!("❌ dict_field 数据未找到");
            }
            Err(e) => {
                println!("❌ dict_field 查询失败: {}", e);
                println!("\n⚠️  dict_field 在 PostgreSQL 中存在反序列化问题!");
                println!("   问题原因: DataValue 反序列化时，serde 无法将原始值(如42)转换为 DataValue 枚举");
            }
        }
    }

    println!("\n=== 测试完成 ===");
    rat_quickdb::manager::shutdown().await?;

    Ok(())
}
