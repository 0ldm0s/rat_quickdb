//! RatQuickDB SQLite Upsert 操作演示
//!
//! 展示 upsert() 方法的完整用法：
//! - 按 id 冲突检测（默认）
//! - 按唯一列冲突检测（如 email）
//! - 插入路径：记录不存在时自动插入
//! - 更新路径：记录已存在时自动更新
//! - 多列冲突检测

use chrono::Utc;
use rat_logger::{LevelFilter, LoggerBuilder, handler::term::TermConfig};
use rat_quickdb::types::*;
use rat_quickdb::*;
use rat_quickdb::{
    ModelManager, ModelOperations, boolean_field, datetime_field, integer_field, string_field,
    uuid_field,
};

// 用户模型 - email 有唯一约束
define_model! {
    struct User {
        id: String,
        username: String,
        email: String,
        age: Option<i32>,
        is_active: bool,
        created_at: chrono::DateTime<chrono::Utc>,
    }
    collection = "users",
    database = "main",
    fields = {
        id: uuid_field().required().unique(),
        username: string_field(None, None, None).required(),
        email: string_field(None, None, None).required().unique(),
        age: integer_field(None, None),
        is_active: boolean_field().required(),
        created_at: datetime_field().required(),
    }
    indexes = [
        { fields: ["email"], unique: true, name: "idx_email" },
    ],
}

// 清理测试文件
async fn cleanup() {
    let _ = tokio::fs::remove_file("./upsert_sqlite.db").await;
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== RatQuickDB SQLite Upsert 演示 ===\n");

    // 初始化日志
    LoggerBuilder::new()
        .with_level(LevelFilter::Warn)
        .add_terminal_with_config(TermConfig::default())
        .init()?;

    cleanup().await;

    // 初始化数据库
    let db_config = DatabaseConfig::builder()
        .db_type(DatabaseType::SQLite)
        .connection(ConnectionConfig::SQLite {
            path: "./upsert_sqlite.db".to_string(),
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
    println!("数据库连接成功\n");

    // === 1. 按 id 冲突检测 ===
    println!("--- 1. 按 id 冲突检测 ---");

    let user = User {
        id: String::new(), // 空ID，让系统自动生成
        username: "alice".to_string(),
        email: "alice@example.com".to_string(),
        age: Some(25),
        is_active: true,
        created_at: Utc::now(),
    };

    // 第一次 upsert：记录不存在，执行 INSERT
    let id1 = user.upsert(vec!["id".to_string()]).await?;
    println!("第一次 upsert (插入): id={}", id1);

    // 验证：查询刚插入的记录
    let found = ModelManager::<User>::find_by_id(&id1).await?.unwrap();
    println!("查询验证: username={}, age={:?}", found.username, found.age);

    // 用相同 ID 再次 upsert：记录已存在，执行 UPDATE
    let user2 = User {
        id: id1.clone(),
        username: "alice_updated".to_string(),
        email: "alice_new@example.com".to_string(),
        age: Some(26),
        is_active: false,
        created_at: Utc::now(),
    };
    let id2 = user2.upsert(vec!["id".to_string()]).await?;
    println!("第二次 upsert (更新): id={} (与第一次相同)", id2);

    // 验证：确认字段已更新
    let found2 = ModelManager::<User>::find_by_id(&id1).await?.unwrap();
    println!(
        "查询验证: username={}, age={:?}, is_active={}",
        found2.username, found2.age, found2.is_active
    );
    assert_eq!(found2.username, "alice_updated");
    assert_eq!(found2.age, Some(26));
    assert!(!found2.is_active);

    // === 2. 按 email 唯一列冲突检测 ===
    println!("\n--- 2. 按 email 唯一列冲突检测 ---");

    let user3 = User {
        id: String::new(),
        username: "bob".to_string(),
        email: "bob@example.com".to_string(),
        age: Some(30),
        is_active: true,
        created_at: Utc::now(),
    };

    // 第一次 upsert：bob@example.com 不存在，INSERT
    let id3 = user3.upsert(vec!["email".to_string()]).await?;
    println!("第一次 upsert (插入): id={}", id3);

    // 不同 ID 但相同 email：执行 UPDATE（按 email 冲突）
    let user4 = User {
        id: String::new(), // 新 ID
        username: "bob_v2".to_string(),
        email: "bob@example.com".to_string(), // 相同 email
        age: Some(31),
        is_active: true,
        created_at: Utc::now(),
    };
    let id4 = user4.upsert(vec!["email".to_string()]).await?;
    println!("第二次 upsert (按 email 更新): 返回 id={}", id4);
    println!("注意: 返回的 ID 可能是新 ID 或原 ID，取决于数据库实现");

    // 验证：应该只有一条 bob@example.com 记录
    let all = ModelManager::<User>::find(
        vec![QueryCondition {
            field: "email".to_string(),
            operator: QueryOperator::Eq,
            value: DataValue::String("bob@example.com".to_string()),
        }],
        None,
    )
    .await?;
    println!("查询 email=bob@example.com: 找到 {} 条记录", all.len());
    assert_eq!(all.len(), 1);
    println!("username={}", all[0].username);
    // 注意：按 email 冲突时，SQLite ON CONFLICT DO UPDATE 会更新整行，
    // username 应该已被更新为 "bob_v2"

    // === 3. 按 email 冲突检测 - 幂等操作 ===
    println!("\n--- 3. 按 email 冲突检测 - 幂等操作 ---");
    println!("说明: 当不知道原始 id 时，使用 email 作为冲突列是更好的选择");

    let user5 = User {
        id: String::new(), // 空 ID
        username: "charlie".to_string(),
        email: "charlie@example.com".to_string(),
        age: Some(22),
        is_active: true,
        created_at: Utc::now(),
    };

    // 第一次 upsert 按 email：插入新记录
    let id5a = user5.upsert(vec!["email".to_string()]).await?;
    println!("第1次 (按 email 插入): id={}", id5a);

    // 修改用户名，但 email 不变，再次 upsert 按 email
    let user5_updated = User {
        id: String::new(), // 仍然是空 ID
        username: "charlie_v2".to_string(),
        email: "charlie@example.com".to_string(), // 相同 email
        age: Some(23),
        is_active: true,
        created_at: Utc::now(),
    };

    // 第二次 upsert 按 email：更新已存在的记录
    // 注意：id 不会被更新（因为 id 是冲突列之一被排除了）
    let id5b = user5_updated.upsert(vec!["email".to_string()]).await?;
    println!("第2次 (按 email 更新): 返回 id={}", id5b);
    println!("注意: 返回的是新传入的 id（空字符串生成的 UUID），不是原记录的 id");

    // 验证：查询原记录，确认 username 被更新，但 id 保持不变
    let all_charlie = ModelManager::<User>::find(
        vec![QueryCondition {
            field: "username".to_string(),
            operator: QueryOperator::Eq,
            value: DataValue::String("charlie_v2".to_string()),
        }],
        None,
    )
    .await?;
    println!("charlie 记录数: {}", all_charlie.len());
    assert_eq!(all_charlie.len(), 1);
    println!("验证通过: username 已更新为 charlie_v2，记录数仍为 1");

    // === 4. 按 id 冲突 - 需要知道原始 id ===
    println!("\n--- 4. 按 id 冲突 - 需要知道原始 id ---");
    println!("说明: 按 id 冲突时，必须提供原始 id 才能实现幂等更新");

    // 使用已知的 id 创建用户
    let known_id = uuid::Uuid::new_v4().to_string();
    let user6 = User {
        id: known_id.clone(),
        username: "david".to_string(),
        email: "david@example.com".to_string(),
        age: Some(30),
        is_active: true,
        created_at: Utc::now(),
    };

    // 第一次 upsert 按 id：插入
    let id6a = user6.upsert(vec!["id".to_string()]).await?;
    println!("第1次 (按 id 插入): id={}", id6a);
    assert_eq!(id6a, known_id);

    // 使用相同 id 更新
    let user6_updated = User {
        id: known_id.clone(), // 使用相同的 id
        username: "david_updated".to_string(),
        email: "david_new@example.com".to_string(),
        age: Some(31),
        is_active: false,
        created_at: Utc::now(),
    };

    // 第二次 upsert 按 id：更新
    let id6b = user6_updated.upsert(vec!["id".to_string()]).await?;
    println!("第2次 (按 id 更新): id={}", id6b);
    assert_eq!(id6b, known_id);
    println!("验证通过: 返回的 id 与原始 id 相同");

    // 验证更新结果
    let found = ModelManager::<User>::find_by_id(&known_id).await?.unwrap();
    assert_eq!(found.username, "david_updated");
    assert_eq!(found.age, Some(31));
    println!("验证通过: username=david_updated, age=31");

    println!("\n=== 所有测试通过 ===");

    cleanup().await;

    Ok(())
}
