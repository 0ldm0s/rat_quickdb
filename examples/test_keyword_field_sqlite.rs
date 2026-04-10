//! SQL 关键词作为字段名/表名的兼容性验证（SQLite 版）
//!
//! 验证 order、group、user 等 SQL 关键词可以安全地用作字段名和表名。

use chrono::Utc;
use rat_logger::{LevelFilter, LoggerBuilder, handler::term::TermConfig};
use rat_quickdb::types::*;
use rat_quickdb::*;
use rat_quickdb::{ModelManager, ModelOperations, datetime_field, float_field, string_field, uuid_field};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

define_model! {
    struct KeywordOrder {
        id: String,
        order: String,       // ORDER
        group: String,       // GROUP
        user: String,        // USER
        select: String,      // SELECT
        amount: f64,
        status: String,
        created_at: chrono::DateTime<chrono::Utc>,
    }
    collection = "order",
    database = "main",
    fields = {
        id: uuid_field().required().unique(),
        order: string_field(None, None, None).required(),
        group: string_field(None, None, None).required(),
        user: string_field(None, None, None).required(),
        select: string_field(None, None, None).required(),
        amount: float_field(None, None).required(),
        status: string_field(None, None, None).required(),
        created_at: datetime_field().required(),
    }
    indexes = [
        { fields: ["order"], unique: false, name: "idx_order" },
        { fields: ["status"], unique: false, name: "idx_status" },
    ],
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== SQL 关键词字段名兼容性验证（SQLite）===\n");

    LoggerBuilder::new()
        .with_level(LevelFilter::Warn)
        .add_terminal_with_config(TermConfig::default())
        .init()?;

    let db_config = DatabaseConfig::builder()
        .db_type(DatabaseType::SQLite)
        .connection(ConnectionConfig::SQLite {
            path: "./test_keyword_field_sqlite.db".to_string(),
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
    println!("[PASS] 数据库连接成功");
    println!("[PASS] 模型: 表名=order, 关键词字段=order/group/user/select");

    // 清理旧数据
    let _ = ModelManager::<KeywordOrder>::delete_many(vec![]).await;

    // 1. INSERT
    println!("\n--- 测试 INSERT ---");
    let o1 = KeywordOrder {
        id: String::new(),
        order: "ORD-001".to_string(),
        group: "A组".to_string(),
        user: "张三".to_string(),
        select: "标准配送".to_string(),
        amount: 99.9,
        status: "pending".to_string(),
        created_at: Utc::now(),
    };
    let id1 = o1.save().await?;
    println!("[PASS] INSERT: order={}, group={}, user={}, select={}", "ORD-001", "A组", "张三", "标准配送");

    for (ord, grp, usr, amt, st) in [
        ("ORD-002", "B组", "李四", 199.0, "completed"),
        ("ORD-003", "A组", "王五", 49.5, "pending"),
        ("ORD-004", "B组", "赵六", 299.0, "completed"),
    ] {
        let o = KeywordOrder {
            id: String::new(),
            order: ord.to_string(),
            group: grp.to_string(),
            user: usr.to_string(),
            select: "标准配送".to_string(),
            amount: amt,
            status: st.to_string(),
            created_at: Utc::now(),
        };
        o.save().await?;
    }
    println!("[PASS] 批量 INSERT 成功，共 4 条记录");

    // 2. WHERE
    println!("\n--- 测试 WHERE ---");
    let results = ModelManager::<KeywordOrder>::find(
        vec![QueryCondition {
            field: "status".to_string(),
            operator: QueryOperator::Eq,
            value: DataValue::String("pending".to_string()),
        }],
        None,
    ).await?;
    println!("[PASS] WHERE status='pending': {} 条", results.len());

    let results = ModelManager::<KeywordOrder>::find(
        vec![QueryCondition {
            field: "group".to_string(),
            operator: QueryOperator::Eq,
            value: DataValue::String("A组".to_string()),
        }],
        None,
    ).await?;
    println!("[PASS] WHERE \"group\"='A组': {} 条", results.len());

    let results = ModelManager::<KeywordOrder>::find(
        vec![QueryCondition {
            field: "user".to_string(),
            operator: QueryOperator::Eq,
            value: DataValue::String("张三".to_string()),
        }],
        None,
    ).await?;
    println!("[PASS] WHERE \"user\"='张三': {} 条", results.len());

    // 3. ORDER BY
    println!("\n--- 测试 ORDER BY ---");
    let results = ModelManager::<KeywordOrder>::find(
        vec![],
        Some(QueryOptions {
            sort: vec![SortConfig {
                field: "order".to_string(),
                direction: SortDirection::Desc,
            }],
            ..Default::default()
        }),
    ).await?;
    println!("[PASS] ORDER BY \"order\" DESC: {:?}",
        results.iter().map(|r| r.order.as_str()).collect::<Vec<_>>());

    let results = ModelManager::<KeywordOrder>::find(
        vec![],
        Some(QueryOptions {
            sort: vec![SortConfig {
                field: "amount".to_string(),
                direction: SortDirection::Asc,
            }],
            ..Default::default()
        }),
    ).await?;
    println!("[PASS] ORDER BY amount ASC: {:?}",
        results.iter().map(|r| format!("{}({})", r.order, r.amount)).collect::<Vec<_>>());

    // 4. UPDATE
    println!("\n--- 测试 UPDATE ---");
    let updated = ModelManager::<KeywordOrder>::update_many(
        vec![QueryCondition {
            field: "id".to_string(),
            operator: QueryOperator::Eq,
            value: DataValue::String(id1.clone()),
        }.into()],
        HashMap::from([
            ("status".to_string(), DataValue::String("shipped".to_string())),
            ("select".to_string(), DataValue::String("加急配送".to_string())),
        ]),
    ).await?;
    println!("[PASS] UPDATE status+select: 影响 {} 条", updated);

    // 5. FIND BY ID
    println!("\n--- 测试 FIND BY ID ---");
    let found = ModelManager::<KeywordOrder>::find_by_id(&id1).await?;
    if let Some(o) = found {
        println!("[PASS] FIND BY ID: order={}, status={}, select={}", o.order, o.status, o.select);
    }

    // 6. DELETE
    println!("\n--- 测试 DELETE ---");
    let deleted = ModelManager::<KeywordOrder>::delete_many(
        vec![QueryCondition {
            field: "status".to_string(),
            operator: QueryOperator::Eq,
            value: DataValue::String("completed".to_string()),
        }.into()],
    ).await?;
    println!("[PASS] DELETE status='completed': 删除 {} 条", deleted);

    // 最终统计
    let remaining = ModelManager::<KeywordOrder>::find(vec![], None).await?;
    println!("\n=== 全部验证通过 ===");
    println!("剩余记录: {} 条", remaining.len());
    for r in &remaining {
        println!("  {} | {} | {} | {} | {}", r.order, r.group, r.user, r.amount, r.status);
    }

    // 清理
    let _ = ModelManager::<KeywordOrder>::delete_many(vec![]).await;
    std::fs::remove_file("./test_keyword_field_sqlite.db").ok();

    Ok(())
}
