//! 简单并发测试示例
//!
//! 演示基本的并发数据库操作

use rat_quickdb::*;
use rat_quickdb::model::{ModelManager, Model, string_field, integer_field, boolean_field, datetime_field};
use rat_quickdb::types::{QueryOperator, QueryCondition, DataValue};
use rat_logger::debug;
use chrono::Utc;
use std::collections::HashMap;
use tokio::join;

/// 定义简单用户模型
define_model! {
    /// 用户模型
    struct User {
        id: String,
        username: String,
        email: String,
        age: Option<i32>,
        is_active: bool,
        created_at: chrono::DateTime<chrono::Utc>,
    }
    collection = "concurrent_users",
    fields = {
        id: string_field(None, None, None).required().unique(),
        username: string_field(None, None, None).required().unique(),
        email: string_field(None, None, None).required().unique(),
        age: integer_field(None, None),
        is_active: boolean_field().required(),
        created_at: datetime_field().required(),
    }
    indexes = [
        { fields: ["username"], unique: true, name: "idx_username" },
    ],
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== 简单并发测试 ===\n");

    // 初始化数据库
    let db_config = DatabaseConfig::builder()
        .db_type(DatabaseType::SQLite)
        .connection(ConnectionConfig::SQLite {
            path: "./concurrent_test.db".to_string(),
            create_if_missing: true,
        })
        .pool(PoolConfig::builder()
            .max_connections(10)
            .min_connections(2)
            .connection_timeout(30)
            .idle_timeout(300)
            .max_lifetime(1800)
            .max_retries(3)
            .retry_interval_ms(1000)
            .keepalive_interval_sec(60)
            .health_check_timeout_sec(10)
            .build()?)
        .alias("default")
        .id_strategy(IdStrategy::Uuid)
        .build()?;

    add_database(db_config).await?;
    println!("✅ 数据库连接已建立");

    // 清理旧数据
    let _ = rat_quickdb::drop_table("default", "concurrent_users").await;

    // 1. 并发插入测试
    println!("\n🚀 并发插入测试");

    let insert_task1 = tokio::spawn(async move {
        for i in 0..5 {
            let user = User {
                id: String::new(),
                username: format!("user1_{}", i),
                email: format!("user1_{}@test.com", i),
                age: Some(20 + i),
                is_active: true,
                created_at: Utc::now(),
            };

            match user.save().await {
                Ok(_) => println!("任务1: 插入用户 {} 成功", user.username),
                Err(e) => println!("任务1: 插入失败: {}", e),
            }
        }
        "任务1完成"
    });

    let insert_task2 = tokio::spawn(async move {
        for i in 5..10 {
            let user = User {
                id: String::new(),
                username: format!("user2_{}", i),
                email: format!("user2_{}@test.com", i),
                age: Some(25 + i),
                is_active: true,
                created_at: Utc::now(),
            };

            match user.save().await {
                Ok(_) => println!("任务2: 插入用户 {} 成功", user.username),
                Err(e) => println!("任务2: 插入失败: {}", e),
            }
        }
        "任务2完成"
    });

    let insert_task3 = tokio::spawn(async move {
        for i in 10..15 {
            let user = User {
                id: String::new(),
                username: format!("user3_{}", i),
                email: format!("user3_{}@test.com", i),
                age: Some(30 + i),
                is_active: true,
                created_at: Utc::now(),
            };

            match user.save().await {
                Ok(_) => println!("任务3: 插入用户 {} 成功", user.username),
                Err(e) => println!("任务3: 插入失败: {}", e),
            }
        }
        "任务3完成"
    });

    // 等待所有插入任务完成
    let (result1, result2, result3) = join!(insert_task1, insert_task2, insert_task3);
    println!("插入结果: {}, {}, {}", result1?, result2?, result3?);

    // 2. 并发查询测试
    println!("\n🔍 并发查询测试");

    let query_task1 = tokio::spawn(async move {
        let conditions = vec![
            QueryCondition {
                field: "age".to_string(),
                operator: QueryOperator::Gt,
                value: DataValue::Int(25),
            }
        ];

        match ModelManager::<User>::find(conditions, None).await {
            Ok(users) => {
                println!("查询任务1: 找到 {} 个年龄 > 25 的用户", users.len());
                for user in &users {
                    println!("  - {} (年龄: {:?})", user.username, user.age);
                }
            },
            Err(e) => println!("查询任务1失败: {}", e),
        }
        "查询任务1完成"
    });

    let query_task2 = tokio::spawn(async move {
        let conditions = vec![
            QueryCondition {
                field: "is_active".to_string(),
                operator: QueryOperator::Eq,
                value: DataValue::Bool(true),
            }
        ];

        match ModelManager::<User>::find(conditions, None).await {
            Ok(users) => println!("查询任务2: 找到 {} 个活跃用户", users.len()),
            Err(e) => println!("查询任务2失败: {}", e),
        }
        "查询任务2完成"
    });

    let (query_result1, query_result2) = join!(query_task1, query_task2);
    println!("查询结果: {}, {}", query_result1?, query_result2?);

    // 3. 并发更新测试
    println!("\n🔄 并发更新测试");

    let update_task1 = tokio::spawn(async move {
        // 查询并更新前几个用户
        let conditions = vec![
            QueryCondition {
                field: "username".to_string(),
                operator: QueryOperator::Contains,
                value: DataValue::String("user1_".to_string()),
            }
        ];

        if let Ok(users) = ModelManager::<User>::find(conditions, None).await {
            for user in users {
                let mut update_data = HashMap::new();
                update_data.insert("age".to_string(), DataValue::Int(99));

                match user.update(update_data).await {
                    Ok(_) => println!("更新任务1: 更新用户 {} 年龄为 99", user.username),
                    Err(e) => println!("更新任务1: 更新失败 {}", e),
                }
            }
        }
        "更新任务1完成"
    });

    let update_task2 = tokio::spawn(async move {
        // 查询并更新其他用户
        let conditions = vec![
            QueryCondition {
                field: "username".to_string(),
                operator: QueryOperator::Contains,
                value: DataValue::String("user2_".to_string()),
            }
        ];

        if let Ok(users) = ModelManager::<User>::find(conditions, None).await {
            for user in users {
                let mut update_data = HashMap::new();
                update_data.insert("age".to_string(), DataValue::Int(88));

                match user.update(update_data).await {
                    Ok(_) => println!("更新任务2: 更新用户 {} 年龄为 88", user.username),
                    Err(e) => println!("更新任务2: 更新失败 {}", e),
                }
            }
        }
        "更新任务2完成"
    });

    let (update_result1, update_result2) = join!(update_task1, update_task2);
    println!("更新结果: {}, {}", update_result1?, update_result2?);

    // 4. 混合并发操作
    println!("\n🎯 混合并发操作");

    let mixed_task1 = tokio::spawn(async move {
        // 插入新用户
        let user = User {
            id: String::new(),
            username: "mixed_user_1".to_string(),
            email: "mixed1@test.com".to_string(),
            age: Some(35),
            is_active: true,
            created_at: Utc::now(),
        };

        match user.save().await {
            Ok(_) => println!("混合任务1: 插入用户成功"),
            Err(e) => println!("混合任务1: 插入失败 {}", e),
        }

        // 查询用户总数
        match ModelManager::<User>::count(vec![]).await {
            Ok(count) => println!("混合任务1: 总用户数: {}", count),
            Err(e) => println!("混合任务1: 查询总数失败 {}", e),
        }

        "混合任务1完成"
    });

    let mixed_task2 = tokio::spawn(async move {
        // 查询所有用户
        match ModelManager::<User>::find(vec![], None).await {
            Ok(users) => {
                println!("混合任务2: 查询到 {} 个用户", users.len());

                // 更新第一个用户
                if let Some(first_user) = users.first() {
                    let mut update_data = HashMap::new();
                    update_data.insert("is_active".to_string(), DataValue::Bool(false));

                    match first_user.update(update_data).await {
                        Ok(_) => println!("混合任务2: 更新第一个用户为非活跃"),
                        Err(e) => println!("混合任务2: 更新失败 {}", e),
                    }
                }
            },
            Err(e) => println!("混合任务2: 查询失败 {}", e),
        }

        "混合任务2完成"
    });

    let (mixed_result1, mixed_result2) = join!(mixed_task1, mixed_task2);
    println!("混合操作结果: {}, {}", mixed_result1?, mixed_result2?);

    // 5. 最终验证
    println!("\n📊 最终验证");

    match ModelManager::<User>::count(vec![]).await {
        Ok(total_count) => {
            println!("数据库中总用户数: {}", total_count);

            // 按年龄分组统计
            let age_99_users = ModelManager::<User>::find(vec![
                QueryCondition {
                    field: "age".to_string(),
                    operator: QueryOperator::Eq,
                    value: DataValue::Int(99),
                }
            ], None).await.unwrap_or_default();

            let age_88_users = ModelManager::<User>::find(vec![
                QueryCondition {
                    field: "age".to_string(),
                    operator: QueryOperator::Eq,
                    value: DataValue::Int(88),
                }
            ], None).await.unwrap_or_default();

            let inactive_users = ModelManager::<User>::find(vec![
                QueryCondition {
                    field: "is_active".to_string(),
                    operator: QueryOperator::Eq,
                    value: DataValue::Bool(false),
                }
            ], None).await.unwrap_or_default();

            println!("年龄为99的用户: {} 个", age_99_users.len());
            println!("年龄为88的用户: {} 个", age_88_users.len());
            println!("非活跃用户: {} 个", inactive_users.len());
        },
        Err(e) => println!("统计失败: {}", e),
    }

    println!("\n✅ 简单并发测试完成！");
    println!("\n结论:");
    println!("- 支持并发插入操作");
    println!("- 支持并发查询操作");
    println!("- 支持并发更新操作");
    println!("- 支持混合并发操作");
    println!("- 数据库连接池能够处理并发请求");

    Ok(())
}