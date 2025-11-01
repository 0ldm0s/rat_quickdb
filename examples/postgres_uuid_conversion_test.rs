//! PostgreSQL UUID自动转换功能测试

use rat_quickdb::*;
use rat_quickdb::types::{DatabaseType, ConnectionConfig, PoolConfig, DataValue, QueryCondition, QueryOperator};
use rat_quickdb::manager::add_database;
use rat_quickdb::{ModelManager, ModelOperations};
use rat_logger::{LoggerBuilder, LevelFilter, handler::term::TermConfig, debug};
use std::collections::HashMap;

// 定义测试模型
define_model! {
    /// 用户模型
    struct User {
        id: String,
        username: String,
        email: String,
        created_at: chrono::DateTime<chrono::Utc>,
    }
    collection = "users",
    fields = {
        id: uuid_field().required().unique(),
        username: string_field(None, None, None).required(),
        email: string_field(None, None, None).required(),
        created_at: datetime_field().required(),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    LoggerBuilder::new()
        .with_level(LevelFilter::Debug)
        .add_terminal_with_config(TermConfig::default())
        .init()
        .expect("Failed to initialize logger");

    println!("=== PostgreSQL UUID自动转换功能测试 ===");

    // 创建数据库配置
    let db_config = DatabaseConfig {
        db_type: DatabaseType::PostgreSQL,
        connection: ConnectionConfig::PostgreSQL {
            host: "172.16.0.23".to_string(),
            port: 5432,
            database: "testdb".to_string(),
            username: "testdb".to_string(),
            password: "testdb123456".to_string(),
            ssl_mode: Some("prefer".to_string()),
            tls_config: None,
        },
        pool: PoolConfig::builder()
            .min_connections(2)
            .max_connections(10)
            .connection_timeout(30)
            .idle_timeout(300)
            .max_lifetime(1800)
            .max_retries(3)
            .retry_interval_ms(1000)
            .keepalive_interval_sec(60)
            .health_check_timeout_sec(10)
            .build()?,
        alias: "test".to_string(),
        id_strategy: IdStrategy::Uuid,
        cache: None,
    };

    // 添加数据库连接
    add_database(db_config).await?;

    // 清理测试表
    let _ = rat_quickdb::drop_table("test", "users").await;

    println!("\n=== 测试1: 使用字符串UUID创建用户 ===");
    let user_id_string = "550e8400-e29b-41d4-a716-446655440000";

    let user = User {
        id: user_id_string.to_string(),
        username: "test_user".to_string(),
        email: "test@example.com".to_string(),
        created_at: chrono::Utc::now(),
    };

    match user.save().await {
        Ok(created_id) => {
            println!("✅ 用户创建成功，ID: {}", created_id);
        }
        Err(e) => {
            println!("❌ 用户创建失败: {}", e);
            return Err(e.into());
        }
    }

    println!("\n=== 测试2: 使用字符串UUID查询用户 ===");
    let conditions = vec![
        QueryCondition {
            field: "id".to_string(),
            operator: QueryOperator::Eq,
            value: DataValue::String(user_id_string.to_string()),
        }
    ];

    match ModelManager::<User>::find(conditions, None).await {
        Ok(users) => {
            if !users.is_empty() {
                println!("✅ 字符串UUID查询成功，找到 {} 个用户", users.len());
                for user in users {
                    println!("   用户: {} - {}", user.id, user.username);
                }
            } else {
                println!("❌ 字符串UUID查询失败：未找到用户");
            }
        }
        Err(e) => {
            println!("❌ 字符串UUID查询失败: {}", e);
        }
    }

    println!("\n=== 测试3: 使用无效UUID格式（应该失败） ===");
    let invalid_uuid = "invalid-uuid-format";
    let invalid_conditions = vec![
        QueryCondition {
            field: "id".to_string(),
            operator: QueryOperator::Eq,
            value: DataValue::String(invalid_uuid.to_string()),
        }
    ];

    match ModelManager::<User>::find(invalid_conditions, None).await {
        Ok(_) => {
            println!("❌ 无效UUID查询应该失败但却成功了");
        }
        Err(e) => {
            println!("✅ 无效UUID查询正确失败: {}", e);
        }
    }

    println!("\n=== 测试完成 ===");
    Ok(())
}