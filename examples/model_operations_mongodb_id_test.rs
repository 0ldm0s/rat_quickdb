//! RatQuickDB MongoDB 模型操作演示 - ID字段查询测试
//!
//! 专门测试ID字段查询功能：
//! - 验证id字段是否正确映射为_id
//! - 通过id字段进行条件查询
//! - 对比其他查询方式

use chrono::Utc;
use rat_logger::{LevelFilter, LoggerBuilder, handler::term::TermConfig};
use rat_quickdb::types::*;
use rat_quickdb::*;
use rat_quickdb::{
    ModelManager, ModelOperations, array_field, boolean_field, datetime_field, float_field,
    integer_field, json_field, string_field, uuid_field,
};

// 数据库别名常量
const DATABASE_ALIAS: &str = "main";
use std::collections::HashMap;
use std::time::Instant;
use tokio::join;

// 用户模型
define_model! {
    struct User {
        id: String,
        username: String,
        email: String,
        password_hash: String,
        full_name: String,
        age: Option<i32>,
        is_active: bool,
        created_at: chrono::DateTime<chrono::Utc>,
        tags: Option<Vec<String>>,
    }
    collection = "users",
    database = DATABASE_ALIAS,
    fields = {
        id: uuid_field().required().unique(),
        username: string_field(None, None, None).required().unique(),
        email: string_field(None, None, None).required().unique(),
        password_hash: string_field(None, None, None).required(),
        full_name: string_field(None, None, None).required(),
        age: integer_field(None, None),
        is_active: boolean_field().required(),
        created_at: datetime_field().required(),
        tags: array_field(field_types!(string), None, None),
    }
    indexes = [
        { fields: ["username"], unique: true, name: "idx_username" },
        { fields: ["email"], unique: true, name: "idx_email" },
        { fields: ["is_active", "created_at"], unique: false, name: "idx_active_created" },
    ],
}

// 清理测试数据
async fn cleanup_test_data() {
    println!("清理测试数据...");

    // 清理用户表
    println!("正在清理用户表...");
    match rat_quickdb::drop_table(DATABASE_ALIAS, "users").await {
        Ok(_) => println!("✅ 用户表清理成功"),
        Err(e) => println!("⚠️  清理用户表失败: {}", e),
    }
}

// 测试ID字段查询功能
async fn test_id_field_queries() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== ID字段查询功能测试 ===");

    // 创建测试用户
    println!("\n1. 创建测试用户...");
    let mut test_users = Vec::new();
    for i in 0..5 {
        let user = User {
            id: String::new(),
            username: format!("test_user_{}", i),
            email: format!("test_{}@example.com", i),
            password_hash: "hashed_password".to_string(),
            full_name: format!("测试用户 {}", i),
            age: Some(20 + i),
            is_active: true,
            created_at: Utc::now(),
            tags: Some(vec!["测试".to_string(), "用户".to_string()]),
        };

        match user.save().await {
            Ok(id) => {
                println!("✅ 创建用户成功: {} (ID: {})", user.username, id);
                test_users.push((id, user.username));
            }
            Err(e) => {
                println!("❌ 创建用户失败: {}", e);
                return Err(e.into());
            }
        }
    }

    // 测试1: 使用find_by_id查询（应该正常工作）
    println!("\n2. 测试find_by_id查询...");
    let (first_id, first_username) = &test_users[0];
    match ModelManager::<User>::find_by_id(first_id).await {
        Ok(Some(user)) => {
            println!("✅ find_by_id查询成功: {}", user.username);
            assert_eq!(user.username, *first_username);
        }
        Ok(None) => {
            println!("❌ find_by_id查询未找到用户");
            return Err("find_by_id查询失败".into());
        }
        Err(e) => {
            println!("❌ find_by_id查询错误: {}", e);
            return Err(e.into());
        }
    }

    // 测试2: 使用id字段进行条件查询（这是我们修复的bug）
    println!("\n3. 测试通过id字段条件查询（修复的bug）...");
    let (test_id, _) = &test_users[1];
    let conditions = vec![QueryCondition {
        field: "id".to_string(),  // 使用"id"字段
        operator: QueryOperator::Eq,
        value: DataValue::String(test_id.clone()),
    }];

    let start = Instant::now();
    match ModelManager::<User>::find(conditions, None).await {
        Ok(users) => {
            let duration = start.elapsed().as_millis();
            if users.is_empty() {
                println!("❌ id字段条件查询未找到用户 (耗时: {}ms)", duration);
                println!("   这表明可能仍存在id-> _id字段映射问题");
                return Err("id字段条件查询失败".into());
            } else {
                println!("✅ id字段条件查询成功: 找到 {} 条记录 (耗时: {}ms)", users.len(), duration);
                assert_eq!(users.len(), 1);
                assert_eq!(users[0].username, test_users[1].1);
                println!("   验证: 查询到的用户是 {}", users[0].username);
            }
        }
        Err(e) => {
            println!("❌ id字段条件查询错误: {}", e);
            return Err(e.into());
        }
    }

    // 测试3: 直接使用_id字段进行条件查询（对比测试）
    println!("\n4. 测试通过_id字段条件查询（对比）...");
    let conditions = vec![QueryCondition {
        field: "_id".to_string(),  // 使用"_id"字段
        operator: QueryOperator::Eq,
        value: DataValue::String(test_id.clone()),
    }];

    let start = Instant::now();
    match ModelManager::<User>::find(conditions, None).await {
        Ok(users) => {
            let duration = start.elapsed().as_millis();
            if users.is_empty() {
                println!("❌ _id字段条件查询未找到用户 (耗时: {}ms)", duration);
                return Err("_id字段条件查询失败".into());
            } else {
                println!("✅ _id字段条件查询成功: 找到 {} 条记录 (耗时: {}ms)", users.len(), duration);
                assert_eq!(users.len(), 1);
                assert_eq!(users[0].username, test_users[1].1);
            }
        }
        Err(e) => {
            println!("❌ _id字段条件查询错误: {}", e);
            return Err(e.into());
        }
    }

    // 测试4: 多ID查询（IN操作）
    println!("\n5. 测试多ID查询（IN操作）...");
    let ids: Vec<String> = test_users.iter().take(3).map(|(id, _)| id.clone()).collect();
    let id_values: Vec<DataValue> = ids.iter().map(|id| DataValue::String(id.clone())).collect();

    let conditions = vec![QueryCondition {
        field: "id".to_string(),  // 使用"id"字段
        operator: QueryOperator::In,
        value: DataValue::Array(id_values),
    }];

    let start = Instant::now();
    match ModelManager::<User>::find(conditions, None).await {
        Ok(users) => {
            let duration = start.elapsed().as_millis();
            if users.is_empty() {
                println!("❌ 多ID查询未找到用户 (耗时: {}ms)", duration);
                return Err("多ID查询失败".into());
            } else {
                println!("✅ 多ID查询成功: 找到 {} 条记录 (耗时: {}ms)", users.len(), duration);
                assert_eq!(users.len(), 3);
                println!("   查询到的用户: {:?}", users.iter().map(|u| &u.username).collect::<Vec<_>>());
            }
        }
        Err(e) => {
            println!("❌ 多ID查询错误: {}", e);
            return Err(e.into());
        }
    }

    // 测试5: 验证username字段查询仍然正常工作
    println!("\n6. 测试其他字段查询（确保修复没有破坏其他功能）...");
    let conditions = vec![QueryCondition {
        field: "username".to_string(),
        operator: QueryOperator::Eq,
        value: DataValue::String("test_user_2".to_string()),
    }];

    let start = Instant::now();
    match ModelManager::<User>::find(conditions, None).await {
        Ok(users) => {
            let duration = start.elapsed().as_millis();
            if users.is_empty() {
                println!("❌ username字段查询未找到用户 (耗时: {}ms)", duration);
                return Err("username字段查询失败".into());
            } else {
                println!("✅ username字段查询成功: 找到 {} 条记录 (耗时: {}ms)", users.len(), duration);
                assert_eq!(users.len(), 1);
                assert_eq!(users[0].username, "test_user_2");
            }
        }
        Err(e) => {
            println!("❌ username字段查询错误: {}", e);
            return Err(e.into());
        }
    }

    // 清理测试数据
    println!("\n7. 清理测试数据...");
    for (id, _) in &test_users {
        if let Ok(Some(user)) = ModelManager::<User>::find_by_id(id).await {
            let _ = user.delete().await;
        }
    }
    println!("✅ 测试数据清理完成");

    println!("\n=== ID字段查询功能测试完成 ===");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== RatQuickDB MongoDB ID字段查询测试 ===");

    // 初始化日志
    LoggerBuilder::new()
        .with_level(LevelFilter::Info)
        .add_terminal_with_config(TermConfig::default())
        .init()?;

    // 初始化数据库
    // TLS配置
    let tls_config = rat_quickdb::types::TlsConfig {
        enabled: true,
        ca_cert_path: None,
        client_cert_path: None,
        client_key_path: None,
        verify_server_cert: false,
        verify_hostname: false,
        min_tls_version: Some("1.2".to_string()),
        cipher_suites: None,
    };

    // ZSTD压缩配置
    let zstd_config = rat_quickdb::types::ZstdConfig {
        enabled: true,
        compression_level: Some(3),
        compression_threshold: Some(1024),
    };

    let db_config = DatabaseConfig::builder()
        .db_type(DatabaseType::MongoDB)
        .connection(ConnectionConfig::MongoDB {
            host: "db0.0ldm0s.net".to_string(),
            port: 27017,
            database: "testdb".to_string(),
            username: Some("testdb".to_string()),
            password: Some("testdb123456".to_string()),
            auth_source: Some("testdb".to_string()),
            direct_connection: true,
            options: None,
            tls_config: Some(tls_config),
            zstd_config: Some(zstd_config),
        })
        .pool(
            PoolConfig::builder()
                .max_connections(25)
                .min_connections(5)
                .connection_timeout(10)
                .idle_timeout(30)
                .max_lifetime(1200)
                .max_retries(6)
                .retry_interval_ms(250)
                .keepalive_interval_sec(20)
                .health_check_timeout_sec(3)
                .build()?,
        )
        .alias(DATABASE_ALIAS)
        .id_strategy(IdStrategy::Uuid)
        .build()?;

    add_database(db_config).await?;
    println!("数据库连接成功");

    // 清理测试数据
    cleanup_test_data().await;
    println!("清理完成");

    // 执行ID字段查询测试
    test_id_field_queries().await?;

    // 健康检查
    println!("\n=== 健康检查 ===");
    let health = health_check().await;
    for (alias, is_healthy) in health {
        let status = if is_healthy { "✅" } else { "❌" };
        println!("{}: {}", alias, status);
    }

    // 清理
    cleanup_test_data().await;
    println!("\n测试完成");

    Ok(())
}
