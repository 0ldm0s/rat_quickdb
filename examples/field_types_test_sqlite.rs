//! 字段类型验证测试 - SQLite
//!
//! 验证 array_field 和 json_field 在 SQLite 中的使用
//! 注意：dict_field 已废弃，请使用 json_field 替代

use rat_logger::{LevelFilter, LoggerBuilder, handler::term::TermConfig};
use rat_quickdb::types::*;
use rat_quickdb::*;
use rat_quickdb::{
    ModelManager, ModelOperations,
    array_field, json_field, string_field,
};

// 测试模型 - 包含 array_field 和 json_field
define_model! {
    struct FieldTypesTest {
        id: String,
        name: String,
        // 数组字段
        tags: Vec<String>,
        // JSON字段 - 可存储任意JSON数据
        config: serde_json::Value,
    }
    collection = "field_types_test",
    database = "main",
    fields = {
        id: string_field(None, None, None).required().unique(),
        name: string_field(Some(100), None, None).required(),
        tags: array_field(field_types!(string), None, None),
        config: json_field(),
    }
    indexes = [],
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== 字段类型验证测试 (SQLite) ===\n");

    // 初始化日志
    LoggerBuilder::new()
        .with_level(LevelFilter::Info)
        .add_terminal_with_config(TermConfig::default())
        .init()?;

    // 初始化数据库 - SQLite
    let db_config = DatabaseConfig {
        db_type: DatabaseType::SQLite,
        connection: ConnectionConfig::SQLite {
            path: "field_types_test.db".to_string(),
            create_if_missing: true,
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
    let _ = drop_table("main", "field_types_test").await;

    // ============================================================
    // 测试: array_field 和 json_field
    // ============================================================
    println!("\n========================================");
    println!("测试: array_field 和 json_field");
    println!("========================================\n");

    let test_record = FieldTypesTest {
        id: String::new(),
        name: "测试记录1".to_string(),
        tags: vec!["tag1".to_string(), "tag2".to_string(), "tag3".to_string()],
        config: serde_json::json!({
            "theme": "dark",
            "language": "zh-CN",
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
    match ModelManager::<FieldTypesTest>::find_by_id(&record_id).await {
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
            println!("   config: {}", serde_json::to_string_pretty(&record.config).unwrap());
            if record.config["theme"] == "dark" && record.config["count"] == 42 {
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
        }
    }

    println!("\n=== 测试完成 ===");
    println!("\n说明：dict_field 已废弃，请使用 json_field 替代。");
    println!("json_field 可以存储任意 JSON 数据，包括对象和数组。");

    rat_quickdb::manager::shutdown().await?;

    Ok(())
}
