//! 特殊字符密码连接测试
//!
//! 验证 PostgreSQL 连接密码中包含特殊字符（@、#、!、$、%、: 等）时
//! urlencoding 编码是否正确处理。

use chrono::Utc;
use rat_logger::{LevelFilter, LoggerBuilder, handler::term::TermConfig};
use rat_quickdb::types::*;
use rat_quickdb::*;
use rat_quickdb::{ModelManager, ModelOperations, datetime_field, float_field, string_field, uuid_field};
use serde::{Deserialize, Serialize};

define_model! {
    struct TestRecord {
        id: String,
        name: String,
        value: f64,
        created_at: chrono::DateTime<chrono::Utc>,
    }
    collection = "test_special_pwd",
    database = "main",
    fields = {
        id: uuid_field().required().unique(),
        name: string_field(None, None, None).required(),
        value: float_field(None, None).required(),
        created_at: datetime_field().required(),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== 特殊字符密码连接测试 ===\n");

    LoggerBuilder::new()
        .with_level(LevelFilter::Debug)
        .add_terminal_with_config(TermConfig::default())
        .init()?;

    // 密码包含特殊字符: p@ss#w0rd!2024$%test
    let special_password = "p@ss#w0rd!2024$%test";
    println!("测试密码: {}", special_password);
    println!("密码中的特殊字符: @ # ! $ %");

    let db_config = DatabaseConfig::builder()
        .db_type(DatabaseType::PostgreSQL)
        .connection(ConnectionConfig::PostgreSQL {
            host: "172.16.0.96".to_string(),
            port: 5432,
            database: "testdb".to_string(),
            username: "testdb".to_string(),
            password: special_password.to_string(),
            ssl_mode: Some("prefer".to_string()),
            tls_config: None,
        })
        .pool(
            PoolConfig::builder()
                .max_connections(1)
                .min_connections(1)
                .connection_timeout(30)
                .idle_timeout(300)
                .max_lifetime(3600)
                .max_retries(3)
                .retry_interval_ms(1000)
                .keepalive_interval_sec(60)
                .health_check_timeout_sec(10)
                .build()?,
        )
        .alias("main")
        .id_strategy(IdStrategy::Uuid)
        .build()?;

    // 1. 测试连接
    println!("\n--- 测试连接 ---");
    match add_database(db_config).await {
        Ok(()) => println!("[PASS] 特殊字符密码连接成功！"),
        Err(e) => {
            eprintln!("[FAIL] 连接失败: {}", e);
            return Err(e.into());
        }
    }

    // 2. 基本 CRUD 操作验证连接确实可用
    println!("\n--- 测试基本操作 ---");

    let record = TestRecord {
        id: String::new(),
        name: "特殊密码测试".to_string(),
        value: 42.0,
        created_at: Utc::now(),
    };
    let id = record.save().await?;
    println!("[PASS] INSERT 成功, id={}", &id[..8]);

    let found = ModelManager::<TestRecord>::find_by_id(&id).await?;
    if let Some(r) = found {
        println!("[PASS] FIND BY ID: name={}, value={}", r.name, r.value);
    }

    // 清理
    let _ = ModelManager::<TestRecord>::delete_many(vec![]).await;

    println!("\n=== 测试通过：特殊字符密码连接正常工作 ===");
    Ok(())
}
