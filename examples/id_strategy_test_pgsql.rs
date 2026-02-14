//! ID策略测试示例
//!
//! 本示例测试不同的ID生成策略是否能正常工作：
//! - AutoIncrement (自增数字)
//! - UUID (字符串)
//! - Snowflake (雪花算法)
//! - ObjectId (MongoDB风格)

use chrono::{DateTime, Utc};
use rat_logger::{LoggerBuilder, debug, handler::term::TermConfig};
use rat_quickdb::types::{ConnectionConfig, DatabaseType, IdStrategy, PoolConfig};
use rat_quickdb::*;
use rat_quickdb::{ModelManager, ModelOperations, datetime_field, integer_field, string_field};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// 定义测试模型
define_model! {
    /// 测试用户模型
    struct TestUser {
        id: String,
        username: String,
        email: String,
        created_at: DateTime<Utc>,
    }
    collection = "test_users",
    fields = {
        id: string_field(None, None, None).required().unique(),
        username: string_field(None, None, None).required(),
        email: string_field(None, None, None).required(),
        created_at: datetime_field().required(),
    }
    indexes = [
        { fields: ["username"], unique: true, name: "idx_username" },
    ],
}

impl TestUser {
    /// 创建新用户（ID由框架自动生成）
    fn new(username: &str, email: &str) -> Self {
        Self {
            id: String::new(), // 框架会自动替换为正确的ID
            username: username.to_string(),
            email: email.to_string(),
            created_at: Utc::now(),
        }
    }
}

/// 清理测试文件
async fn cleanup_test_files() {
    let test_files = vec![
        "./id_strategy_test.db",
        "./id_strategy_test.db-wal",
        "./id_strategy_test.db-shm",
    ];

    for file in test_files {
        if let Err(e) = tokio::fs::remove_file(file).await {
            if !e.to_string().contains("No such file or directory") {
                eprintln!("警告：无法删除测试文件 {}: {}", file, e);
            }
        }
    }
}

/// 测试自增ID策略
async fn test_auto_increment() -> QuickDbResult<()> {
    println!("🔢 测试 AutoIncrement ID 策略");
    println!("===============================");

    // 配置数据库，使用自增ID - 从MySQL版本复制配置
    let db_config = DatabaseConfig {
        db_type: DatabaseType::PostgreSQL,
        connection: ConnectionConfig::PostgreSQL {
            host: "172.16.0.96".to_string(),
            port: 5432,
            database: "testdb".to_string(),
            username: "testdb".to_string(),
            password: "testdb123456".to_string(),
            ssl_mode: Some("prefer".to_string()),
            tls_config: None,
        },
        pool: PoolConfig::builder()
            .max_connections(5)
            .min_connections(1)
            .connection_timeout(30)
            .idle_timeout(300)
            .max_lifetime(1800)
            .max_retries(3)
            .retry_interval_ms(1000)
            .keepalive_interval_sec(60)
            .health_check_timeout_sec(10)
            .build()
            .unwrap(),
        alias: "auto_increment_db".to_string(),
        cache: None,
        id_strategy: IdStrategy::AutoIncrement,
    };

    add_database(db_config).await?;

    // 设置默认数据库别名
    rat_quickdb::set_default_alias("auto_increment_db").await?;

    // 清理之前的表，确保使用正确的ID策略创建新表
    let _ = rat_quickdb::drop_table("auto_increment_db", "test_users").await;

    // 创建测试用户
    let users = vec![
        TestUser::new("user1", "user1@test.com"),
        TestUser::new("user2", "user2@test.com"),
        TestUser::new("user3", "user3@test.com"),
    ];

    println!("创建3个用户，测试AutoIncrement策略自动生成ID...");
    let mut created_ids = Vec::new();

    for (i, user) in users.iter().enumerate() {
        match user.save().await {
            Ok(id) => {
                println!("✅ 用户 {} 创建成功，生成的ID: {}", i + 1, id);
                created_ids.push(id);
            }
            Err(e) => {
                println!("❌ 用户 {} 创建失败: {}", i + 1, e);
                return Err(e);
            }
        }
    }

    // 验证ID是否是数字且递增
    println!("\n验证ID是否正确生成:");
    for (i, id) in created_ids.iter().enumerate() {
        println!("用户 {} ID: {} (应该是数字且递增)", i + 1, id);
        if let Ok(num_id) = id.parse::<i64>() {
            println!("  ✅ ID是数字: {}", num_id);
        } else {
            println!("  ❌ ID不是数字: {}", id);
        }
    }

    // 清理数据 - 暂时注释掉以便检查数据库表结构
    // let _ = rat_quickdb::delete("test_users", vec![], Some("auto_increment_db")).await;
    println!("✅ AutoIncrement ID 测试完成（数据保留以便检查表结构）\n");

    Ok(())
}

/// 测试UUID ID策略
async fn test_uuid() -> QuickDbResult<()> {
    println!("🆔 测试 UUID ID 策略");
    println!("========================");

    // 配置数据库，使用UUID ID - 从MySQL版本复制配置
    let db_config = DatabaseConfig {
        alias: "uuid_db".to_string(),
        db_type: DatabaseType::PostgreSQL,
        connection: ConnectionConfig::PostgreSQL {
            host: "172.16.0.96".to_string(),
            port: 5432,
            database: "testdb".to_string(),
            username: "testdb".to_string(),
            password: "testdb123456".to_string(),
            ssl_mode: Some("prefer".to_string()),
            tls_config: None,
        },
        pool: PoolConfig::builder()
            .max_connections(5)
            .min_connections(1)
            .connection_timeout(30)
            .idle_timeout(300)
            .max_lifetime(1800)
            .max_retries(3)
            .retry_interval_ms(1000)
            .keepalive_interval_sec(60)
            .health_check_timeout_sec(10)
            .build()
            .unwrap(),
        id_strategy: IdStrategy::Uuid,
        cache: None,
    };

    add_database(db_config).await?;

    // 设置默认数据库别名
    rat_quickdb::set_default_alias("uuid_db").await?;

    // 清理之前的表，确保使用正确的ID策略创建新表
    let _ = rat_quickdb::drop_table("uuid_db", "test_users").await;

    // 创建测试用户
    let users = vec![
        TestUser::new("uuid_user1", "uuid1@test.com"),
        TestUser::new("uuid_user2", "uuid2@test.com"),
        TestUser::new("uuid_user3", "uuid3@test.com"),
    ];

    println!("创建3个用户，测试UUID自动生成...");
    let mut created_ids = Vec::new();

    for (i, user) in users.iter().enumerate() {
        match user.save().await {
            Ok(id) => {
                println!("✅ 用户 {} 创建成功，生成的ID: {}", i + 1, id);
                created_ids.push(id);
            }
            Err(e) => {
                println!("❌ 用户 {} 创建失败: {}", i + 1, e);
                return Err(e);
            }
        }
    }

    // 验证ID是否是有效的UUID
    println!("\n验证ID是否为有效UUID:");
    for (i, id) in created_ids.iter().enumerate() {
        println!("用户 {} ID: {}", i + 1, id);
        if id.len() == 36 {
            println!("  ✅ ID长度正确 (36字符)");
            if id.contains('-') && id.split('-').count() == 5 {
                println!("  ✅ UUID格式正确");
            } else {
                println!("  ❌ UUID格式错误");
            }
        } else {
            println!("  ❌ ID长度错误: {}", id.len());
        }
    }

    // 清理数据 - 暂时注释掉以便检查数据库表结构
    // let _ = rat_quickdb::delete("test_users", vec![], Some("uuid_db")).await;
    println!("✅ UUID ID 测试完成（数据保留以便检查表结构）\n");

    Ok(())
}

/// 测试雪花算法ID策略
async fn test_snowflake() -> QuickDbResult<()> {
    println!("❄️ 测试 Snowflake ID 策略");
    println!("=============================");

    // 配置数据库，使用雪花算法ID - PostgreSQL配置
    let db_config = DatabaseConfig {
        alias: "snowflake_db".to_string(),
        db_type: DatabaseType::PostgreSQL,
        connection: ConnectionConfig::PostgreSQL {
            host: "172.16.0.96".to_string(),
            port: 5432,
            database: "testdb".to_string(),
            username: "testdb".to_string(),
            password: "testdb123456".to_string(),
            ssl_mode: Some("prefer".to_string()),
            tls_config: None,
        },
        pool: PoolConfig::builder()
            .max_connections(5)
            .min_connections(1)
            .connection_timeout(30)
            .idle_timeout(300)
            .max_lifetime(1800)
            .max_retries(3)
            .retry_interval_ms(1000)
            .keepalive_interval_sec(60)
            .health_check_timeout_sec(10)
            .build()
            .unwrap(),
        id_strategy: IdStrategy::Snowflake {
            machine_id: 1,
            datacenter_id: 1,
        },
        cache: None,
    };

    add_database(db_config).await?;

    // 设置默认数据库别名
    rat_quickdb::set_default_alias("snowflake_db").await?;

    // 清理之前的表，确保使用正确的ID策略创建新表
    let _ = rat_quickdb::drop_table("snowflake_db", "test_users").await;

    // 创建测试用户
    let users = vec![
        TestUser::new("snowflake_user1", "snowflake1@test.com"),
        TestUser::new("snowflake_user2", "snowflake2@test.com"),
        TestUser::new("snowflake_user3", "snowflake3@test.com"),
    ];

    println!("创建3个用户，测试雪花算法ID生成...");
    let mut created_ids = Vec::new();

    for (i, user) in users.iter().enumerate() {
        match user.save().await {
            Ok(id) => {
                println!("✅ 用户 {} 创建成功，生成的ID: {}", i + 1, id);
                created_ids.push(id);
            }
            Err(e) => {
                println!("❌ 用户 {} 创建失败: {}", i + 1, e);
                return Err(e);
            }
        }
    }

    // 验证雪花算法ID
    println!("\n验证雪花算法ID:");
    for (i, id) in created_ids.iter().enumerate() {
        println!("用户 {} ID: {}", i + 1, id);

        // 验证是否为数字
        match id.parse::<u64>() {
            Ok(num_id) => {
                println!("  ✅ ID是有效的64位数字: {}", num_id);

                // 验证是否在合理范围内（Snowflake ID通常是19位数字）
                if id.len() >= 15 && id.len() <= 20 {
                    println!("  ✅ ID长度符合Snowflake标准: {} 位", id.len());
                } else {
                    println!("  ⚠️  ID长度可能不符合Snowflake标准: {} 位", id.len());
                }

                // 验证时间戳部分（Snowflake ID的前41位是时间戳）
                let timestamp = num_id >> 22; // 右移22位，取出时间戳部分
                let current_time = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64;
                let snowflake_epoch = 1288834974657; // Snowflake算法起始时间
                let id_time = timestamp + snowflake_epoch;

                if id_time <= current_time && (current_time - id_time) < 86400000 {
                    // 不超过一天前
                    println!("  ✅ ID时间戳有效: {}", id_time);
                } else {
                    println!("  ⚠️  ID时间戳可能异常: {}", id_time);
                }
            }
            Err(_) => {
                println!("  ❌ ID不是有效的数字: {}", id);
            }
        }
    }

    // 清理数据 - 暂时注释掉以便检查数据库表结构
    // let _ = rat_quickdb::delete("test_users", vec![], Some("snowflake_db")).await;
    println!("✅ Snowflake ID 测试完成（数据保留以便检查表结构）\n");

    Ok(())
}

#[tokio::main]
async fn main() -> QuickDbResult<()> {
    // 初始化日志
    LoggerBuilder::new()
        .add_terminal_with_config(TermConfig::default())
        .init()
        .expect("日志初始化失败");

    println!("🧪 RatQuickDB ID策略测试");
    println!("========================\n");

    // 解析命令行参数
    let args: Vec<String> = std::env::args().collect();

    let test_choice = if args.len() == 1 {
        // 没有参数，随机选择一个策略避免污染
        use std::collections::HashMap;
        let strategies = vec!["auto-increment", "uuid", "snowflake"];
        let random_index = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            % 3) as usize;
        strategies[random_index]
    } else if args.len() == 2 {
        match args[1].as_str() {
            "--auto-increment" => "auto-increment",
            "--uuid" => "uuid",
            "--snowflake" => "snowflake",
            _ => {
                eprintln!("错误: 未知参数 '{}'", args[1]);
                eprintln!("用法: {} [选项]", args[0]);
                eprintln!("选项:");
                eprintln!("  --auto-increment   运行AutoIncrement策略测试");
                eprintln!("  --uuid             运行UUID策略测试");
                eprintln!("  --snowflake        运行Snowflake策略测试");
                eprintln!("\n  不指定参数时将随机选择一个策略运行");
                return Ok(());
            }
        }
    } else {
        eprintln!("错误: 参数过多");
        eprintln!("用法: {} [选项]", args[0]);
        return Ok(());
    };

    println!("🎯 运行测试策略: {}\n", test_choice);

    // 清理之前的测试文件
    cleanup_test_files().await;

    // 根据选择运行对应的测试
    match test_choice {
        "auto-increment" => {
            test_auto_increment().await?;
        }
        "uuid" => {
            test_uuid().await?;
        }
        "snowflake" => {
            test_snowflake().await?;
        }
        _ => unreachable!(),
    }

    // 清理测试文件
    cleanup_test_files().await;

    println!("🎉 ID策略测试完成！");
    Ok(())
}
