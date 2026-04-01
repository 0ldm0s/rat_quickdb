//! SQLite 字符串模糊搜索功能演示
//!
//! 本示例专门演示 SQLite 中的字符串模糊搜索功能：
//! - Contains - 包含匹配（LIKE '%value%'）
//! - StartsWith - 前缀匹配（LIKE 'value%'）
//! - EndsWith - 后缀匹配（LIKE '%value'）
//! - Regex - 正则表达式匹配（暂不支持，SQLite需要扩展)
//!
//! 运行方式：
//! ```bash
//! cargo run --example string_fuzzy_search_sqlite --features sqlite-support
//! ```

use chrono::Utc;
use rat_logger::{LevelFilter, LoggerBuilder, handler::term::TermConfig};
use rat_quickdb::types::*;
use rat_quickdb::*;
use rat_quickdb::{ModelManager, ModelOperations, string_field, uuid_field};
use serde::{Deserialize, Serialize};

/// 用户模型 - 用于字符串模糊搜索演示
define_model! {
    struct StringSearchUser {
        id: String,
        username: String,
        email: String,
        full_name: String,
        bio: String,
    }
    collection = "string_search_users",
    database = "main",
    fields = {
        id: uuid_field().required().unique(),
        username: string_field(None, None, None).required().unique(),
        email: string_field(None, None, None).required(),
        full_name: string_field(None, None, None),
        bio: string_field(None, None, None),
    }
}

/// 演示字符串模糊搜索功能
async fn demonstrate_string_fuzzy_search() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== 字符串模糊搜索演示 ===");
    println!("演示 SQLite 中的字符串模糊搜索功能：");
    println!("• Contains - 包含匹配");
    println!("• StartsWith - 前缀匹配");
    println!("• EndsWith - 后缀匹配");
    println!("• Regex - 正则表达式匹配（需要扩展）\n");

    // 创建测试数据
    println!("1. 创建测试数据...");
    let test_users = vec![
        StringSearchUser {
            id: String::new(),
            username: "alice_wang".to_string(),
            email: "alice.wang@example.com".to_string(),
            full_name: "王爱丽丝".to_string(),
            bio: "热爱编程和开源项目的软件工程师".to_string(),
        },
        StringSearchUser {
            id: String::new(),
            username: "bob_chen".to_string(),
            email: "bob.chen@company.com".to_string(),
            full_name: "陈博博".to_string(),
            bio: "全栈开发者，擅长前后端开发".to_string(),
        },
        StringSearchUser {
            id: String::new(),
            username: "charlie_zhang".to_string(),
            email: "charlie.zhang@tech.io".to_string(),
            full_name: "张查理".to_string(),
            bio: "数据科学家，专注于机器学习和AI".to_string(),
        },
        StringSearchUser {
            id: String::new(),
            username: "david_li".to_string(),
            email: "david.li@startup.co".to_string(),
            full_name: "李大卫".to_string(),
            bio: "DevOps 工程师，自动化爱好者".to_string(),
        },
        StringSearchUser {
            id: String::new(),
            username: "emma_wu".to_string(),
            email: "emma.wu@example.com".to_string(),
            full_name: "吴艾玛".to_string(),
            bio: "UI/UX 设计师，追求极致用户体验".to_string(),
        },
    ];

    let mut created_users = Vec::new();
    for user in test_users {
        match user.save().await {
            Ok(user_id) => {
                println!("✓ 创建用户: {} (ID: {})", user.username, user_id);
                created_users.push(user);
            }
            Err(e) => {
                eprintln!("❌ 创建用户失败: {}", e);
            }
        }
    }

    if created_users.is_empty() {
        eprintln!("❌ 未能创建任何用户，跳过字符串模糊搜索演示");
        return Ok(());
    }

    println!("\n✓ 成功创建 {} 个用户", created_users.len());

    // 1. Contains - 包含匹配
    println!("\n\n2. Contains - 包含匹配");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("查找条件: username 包含 'wang'");
    println!("SQLite 原生查询: SELECT * FROM string_search_users WHERE username LIKE '%wang%'");

    let conditions = vec![QueryCondition {
        field: "username".to_string(),
        operator: QueryOperator::Contains,
        value: DataValue::String("wang".to_string()),
    }];
    match ModelManager::<StringSearchUser>::find(conditions, None).await {
        Ok(users) => {
            println!("✓ 查询结果: {} 个用户", users.len());
            for (i, user) in users.iter().enumerate() {
                println!("  {}. {} | {} | {}",
                    i + 1,
                    user.username,
                    user.email,
                    user.full_name
                );
            }
        }
        Err(e) => {
            eprintln!("❌ 查询失败: {}", e);
        }
    }

    // 2. StartsWith - 前缀匹配
    println!("\n\n3. StartsWith - 前缀匹配");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("查找条件: username 以 'alice' 开头");
    println!("SQLite 原生查询: SELECT * FROM string_search_users WHERE username LIKE 'alice%'");

    let conditions = vec![QueryCondition {
        field: "username".to_string(),
        operator: QueryOperator::StartsWith,
        value: DataValue::String("alice".to_string()),
    }];
    match ModelManager::<StringSearchUser>::find(conditions, None).await {
        Ok(users) => {
            println!("✓ 查询结果: {} 个用户", users.len());
            for (i, user) in users.iter().enumerate() {
                println!("  {}. {} | {} | {}",
                    i + 1,
                    user.username,
                    user.email,
                    user.full_name
                );
            }
        }
        Err(e) => {
            eprintln!("❌ 查询失败: {}", e);
        }
    }

    // 3. EndsWith - 后缀匹配
    println!("\n\n4. EndsWith - 后缀匹配");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("查找条件: email 以 '.com' 结尾");
    println!("SQLite 原生查询: SELECT * FROM string_search_users WHERE email LIKE '%.com'");

    let conditions = vec![QueryCondition {
        field: "email".to_string(),
        operator: QueryOperator::EndsWith,
        value: DataValue::String(".com".to_string()),
    }];
    match ModelManager::<StringSearchUser>::find(conditions, None).await {
        Ok(users) => {
            println!("✓ 查询结果: {} 个用户", users.len());
            for (i, user) in users.iter().enumerate() {
                println!("  {}. {} | {} | {}",
                    i + 1,
                    user.username,
                    user.email,
                    user.full_name
                );
            }
        }
        Err(e) => {
            eprintln!("❌ 查询失败: {}", e);
        }
    }

    // 4. Regex - 正则表达式匹配
    println!("\n\n5. Regex - 正则表达式匹配");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("查找条件: username 匹配正则表达式 '.*_wang|_chen.*'");
    println!("SQLite 原生查询: SELECT * FROM string_search_users WHERE username REGEXP '.*_wang|_chen.*' (需要扩展)");

    let conditions = vec![QueryCondition {
        field: "username".to_string(),
        operator: QueryOperator::Regex,
        value: DataValue::String(".*_wang|_chen.*".to_string()),
    }];
    match ModelManager::<StringSearchUser>::find(conditions, None).await {
        Ok(users) => {
            println!("✓ 查询结果: {} 个用户", users.len());
            for (i, user) in users.iter().enumerate() {
                println!("  {}. {} | {} | {}",
                    i + 1,
                    user.username,
                    user.email,
                    user.full_name
                );
            }
        }
        Err(e) => {
            eprintln!("❌ 查询失败: {}", e);
        }
    }

    // 5. 组合查询示例
    println!("\n\n6. 组合查询（Contains AND StartsWith）");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("查找条件: (username 包含 'wang') AND (email 以 'example' 开头)");
    println!("SQLite 原生查询:");
    println!("  SELECT * FROM string_search_users");
    println!("  WHERE username LIKE '%wang%' AND email LIKE 'example%'");

    let conditions = vec![
        QueryCondition {
            field: "username".to_string(),
            operator: QueryOperator::Contains,
            value: DataValue::String("wang".to_string()),
        },
        QueryCondition {
            field: "email".to_string(),
            operator: QueryOperator::StartsWith,
            value: DataValue::String("example".to_string()),
        },
    ];
    match ModelManager::<StringSearchUser>::find(conditions, None).await {
        Ok(users) => {
            println!("✓ 查询结果: {} 个用户", users.len());
            for (i, user) in users.iter().enumerate() {
                println!("  {}. {} | {} | {}",
                    i + 1,
                    user.username,
                    user.email,
                    user.full_name
                );
            }
        }
        Err(e) => {
            eprintln!("❌ 查询失败: {}", e);
        }
    }

    // 6. Ne（不等于）示例
    println!("\n\n7. Ne（不等于）示例");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("查找条件: username 不等于 'david'");
    println!("SQLite 原生查询: SELECT * FROM string_search_users WHERE username != 'david'");

    let conditions = vec![QueryCondition {
        field: "username".to_string(),
        operator: QueryOperator::Ne,
        value: DataValue::String("david".to_string()),
    }];
    match ModelManager::<StringSearchUser>::find(conditions, None).await {
        Ok(users) => {
            println!("✓ 查询结果: {} 个用户（username ≠ 'david'）", users.len());
            for (i, user) in users.iter().take(5).enumerate() {
                println!("  {}. {} | {} | {}",
                    i + 1,
                    user.username,
                    user.email,
                    user.full_name
                );
            }
        }
        Err(e) => {
            eprintln!("❌ 查询失败: {}", e);
        }
    }

    // 7. 总结
    println!("\n\n=== 字符串模糊搜索总结 ===");
    println!("📚 支持的查询操作符：");
    println!("  • Contains    - 包含匹配（包含子字符串）");
    println!("  • StartsWith  - 前缀匹配（以特定字符串开头）");
    println!("  • EndsWith    - 后缀匹配（以特定字符串结尾）");
    println!("  • Regex       - 正则表达式匹配（最灵活的模式匹配）");
    println!("  • Ne          - 不等于（精确排除）");
    println!("\n💡 提示：SQLite 使用 LIKE 实现字符串模糊搜索，REGEXP需要扩展支持");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== SQLite 字符串模糊搜索功能演示 ===");

    // 初始化日志
    LoggerBuilder::new()
        .with_level(LevelFilter::Info)
        .add_terminal_with_config(TermConfig::default())
        .init()?;

    // 先删除旧的数据库文件
    if std::path::Path::new("./string_fuzzy_search_sqlite.db").exists() {
        let _ = tokio::fs::remove_file("./string_fuzzy_search_sqlite.db").await;
    }

    // 初始化数据库（使用与 query_operations_sqlite.rs 相同的连接配置）
    let db_config = DatabaseConfig::builder()
        .db_type(DatabaseType::SQLite)
        .connection(ConnectionConfig::SQLite {
            path: "./string_fuzzy_search_sqlite.db".to_string(),
            create_if_missing: true,
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

    add_database(db_config).await?;
    println!("✓ 数据库连接成功");

    // 清理测试数据
    println!("\n清理旧的测试数据...");
    match drop_table("main", "string_search_users").await {
        Ok(_) => println!("✓ 清理完成"),
        Err(e) => println!("注意: 清理失败或表不存在: {}", e),
    }

    // 执行演示
    demonstrate_string_fuzzy_search().await?;

    println!("\n=== 演示完成 ===");
    println!("数据保留在数据库中以便检查");

    Ok(())
}
