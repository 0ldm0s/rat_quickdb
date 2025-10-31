//! SQLite 手动表管理示例
//!
//! 展示如何使用框架进行手动表管理操作，包括：
//! - 检查表是否存在
//! - 手动创建表（通过框架API）
//! - 删除表
//! - 遵循框架的模型宏设计原则
//!
//! 注意：这个示例使用模型宏定义表结构，然后演示手动管理操作

use rat_quickdb::*;
use rat_quickdb::types::{QueryCondition, QueryOperator, DataValue};
use rat_quickdb::manager::{shutdown, table_exists, drop_table};
use rat_quickdb::{ModelOperations, string_field, integer_field, boolean_field, datetime_field};
use rat_logger::{LoggerBuilder, handler::term::TermConfig, info, debug, warn};
use std::collections::HashMap;

// 定义测试模型 - 遵循框架的模型宏设计原则
define_model! {
    /// 手动表管理测试模型
    struct ManualTableTest {
        id: String,
        name: String,
        email: Option<String>,
        age: i32,
        is_active: bool,
        created_at: chrono::DateTime<chrono::Utc>,
    }
    collection = "manual_table_test",
    fields = {
        id: string_field(None, None, None).required().unique(),
        name: string_field(Some(100), Some(1), None).required(),
        email: string_field(Some(255), Some(5), None),
        age: integer_field(Some(0), Some(150)),
        is_active: boolean_field(),
        created_at: datetime_field().required(),
    }
    indexes = [
        { fields: ["email"], unique: true, name: "idx_email" },
        { fields: ["name"], unique: false, name: "idx_name" },
        { fields: ["name", "age"], unique: false, name: "idx_name_age" },
    ],
}

#[tokio::main]
async fn main() -> QuickDbResult<()> {
    // 初始化日志系统
    LoggerBuilder::new()
        .add_terminal_with_config(TermConfig::default())
        .init()
        .expect("日志初始化失败");

    rat_quickdb::init();
    println!("=== PostgreSQL 手动表管理示例 ===");

    // 创建数据库配置 - 使用PostgreSQL
    let config = DatabaseConfig {
        db_type: DatabaseType::PostgreSQL,
        connection: ConnectionConfig::PostgreSQL {
            host: "172.16.0.23".to_string(),
            port: 5432,
            database: "testdb".to_string(),
            username: "testdb".to_string(),
            password: "yash2vCiBA&B#h$#i&gb@IGSTh&cP#QC^".to_string(),
            ssl_mode: Some("prefer".to_string()),
            tls_config: None,
        },
        pool: PoolConfig::builder()
                .max_connections(10)
                .min_connections(2)
                .connection_timeout(30)
                .idle_timeout(300)
                .max_lifetime(1800)
                .max_retries(3)
                .retry_interval_ms(1000)
                .keepalive_interval_sec(60)
                .health_check_timeout_sec(10)
                .build()
                .unwrap(),
        alias: "default".to_string(),
        cache: None,
        id_strategy: IdStrategy::Uuid,
    };

    // 初始化数据库
    add_database(config).await?;

    // 设置默认数据库别名
    set_default_alias("default").await?;

    println!("\n=== 开始手动表管理演示 ===\n");

    // 清理操作：首先删除可能存在的旧表
    println!("1. 清理旧的测试表（如果存在）");
    match drop_table("default", "manual_table_test").await {
        Ok(_) => println!("   ✅ 清理完成"),
        Err(e) => {
            println!("   ⚠️  清理失败（可能表不存在）: {}", e);
            // 不返回错误，继续后续操作
        }
    }

    // 演示1：检查表是否存在（预期不存在）
    println!("\n2. 检查测试表是否存在");
    match table_exists("default", "manual_table_test").await {
        Ok(exists) => {
            if !exists {
                println!("   ✅ 测试表不存在，符合预期");
            } else {
                println!("   ⚠️  测试表已存在");
            }
        },
        Err(e) => {
            println!("   ❌ 检查表存在性失败: {}", e);
            return Err(e);
        }
    }

    // 演示2：通过模型管理器直接创建表（使用新添加的功能）
    println!("\n3. 通过模型管理器直接创建表");
    match ModelManager::<ManualTableTest>::create_table().await {
        Ok(_) => {
            println!("   ✅ 通过模型管理器创建表成功");
        },
        Err(e) => {
            println!("   ❌ 通过模型管理器创建表失败: {}", e);
            return Err(e);
        }
    }

    // 演示3：验证表已创建
    println!("\n4. 验证表已成功创建");
    match table_exists("default", "manual_table_test").await {
        Ok(exists) => {
            if exists {
                println!("   ✅ 表创建成功并已验证");
            } else {
                println!("   ❌ 表创建失败");
                return Err(QuickDbError::QueryError {
                    message: "表创建验证失败".to_string(),
                });
            }
        },
        Err(e) => {
            println!("   ❌ 验证表存在性失败: {}", e);
            return Err(e);
        }
    }

    // 演示4：测试基本操作（验证表工作正常）
    println!("\n5. 测试表的基本操作");

    // 插入更多测试数据
    let test_records = vec![
        ManualTableTest {
            id: String::new(),
            name: "另一个用户".to_string(),
            email: Some("another@example.com".to_string()),
            age: 30,
            is_active: false,
            created_at: chrono::Utc::now(),
        },
        ManualTableTest {
            id: String::new(),
            name: "第三个用户".to_string(),
            email: None, // 测试可选字段
            age: 18,
            is_active: true,
            created_at: chrono::Utc::now(),
        },
    ];

    for (i, record) in test_records.into_iter().enumerate() {
        match record.save().await {
            Ok(result) => {
                println!("   ✅ 测试数据 {} 插入成功: {}", i + 1, result);
            },
            Err(e) => {
                println!("   ❌ 测试数据 {} 插入失败: {}", i + 1, e);
            }
        }
    }

    // 查询测试数据
    println!("\n6. 查询测试数据");
    match ModelManager::<ManualTableTest>::find(vec![], None).await {
        Ok(records) => {
            if records.is_empty() {
                println!("   ❌ 查询失败：预期有数据但返回0条记录！");
                return Err(QuickDbError::QueryError {
                    message: "查询失败：预期有数据但返回0条记录".to_string(),
                });
            } else {
                println!("   ✅ 查询成功，共找到 {} 条记录", records.len());
                for record in &records {
                    println!("     - ID: {}, 姓名: {}, 邮箱: {:?}, 年龄: {}, 激活: {}",
                        record.id, record.name, record.email, record.age, record.is_active);
                }
            }
        },
        Err(e) => {
            println!("   ❌ 查询失败: {}", e);
            return Err(e);
        }
    }

    // 演示5：手动删除表（展示手动管理操作）
    println!("\n7. 手动删除表");
    match drop_table("default", "manual_table_test").await {
        Ok(_) => println!("   ✅ 表删除成功"),
        Err(e) => {
            println!("   ❌ 表删除失败: {}", e);
            return Err(e);
        }
    }

    // 演示6：确认表已删除
    println!("\n8. 确认表已删除");
    match table_exists("default", "manual_table_test").await {
        Ok(exists) => {
            if !exists {
                println!("   ✅ 确认表已成功删除");
            } else {
                println!("   ❌ 表仍然存在，删除失败");
                return Err(QuickDbError::QueryError {
                    message: "表删除验证失败".to_string(),
                });
            }
        },
        Err(e) => {
            println!("   ❌ 确认表存在性失败: {}", e);
            return Err(e);
        }
    }

    // 演示7：尝试操作已删除的表（应该失败）
    println!("\n9. 验证表已无法操作");
    match ModelManager::<ManualTableTest>::find(vec![], None).await {
        Ok(records) => {
            if records.is_empty() {
                println!("   ✅ 确认表已无法操作（返回0条记录，符合预期）");
            } else {
                println!("   ⚠️  意外：仍然可以查询已删除的表，找到 {} 条记录", records.len());
            }
        },
        Err(e) => {
            println!("   ✅ 确认表已无法操作（符合预期）: {}", e);
        }
    }

    println!("\n=== 手动表管理演示完成 ===");
    println!("✅ 所有手动表管理操作演示成功");
    println!("✅ 遵循了框架的模型宏设计原则");
    println!("✅ 展示了表存在性检查和直接表管理操作");
    println!("✅ 使用了ModelManager::<T>::create_table()直接创建表");

    // 关闭连接池
    shutdown().await?;

    Ok(())
}