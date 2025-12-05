//! 测试全局操作锁机制
//! 验证查询开始后不能再添加数据库的功能

use rat_quickdb::types::*;
use rat_quickdb::*;
use tokio::time::{Duration, sleep};

// 在测试结束时清理测试文件
use std::fs;

#[tokio::main]
async fn main() -> QuickDbResult<()> {
    println!("🔒 测试全局操作锁机制");
    println!("========================");

    // 测试1：正常添加数据库（查询操作开始前）
    test_normal_database_addition().await?;

    // 测试2：查询操作后禁止添加数据库
    test_database_addition_after_queries().await?;

    // 测试3：查询操作后禁止添加数据库（使用表操作）
    test_database_addition_after_table_ops().await?;

    // 清理测试文件
    let _ = fs::remove_file("test_global_lock.db");
    let _ = fs::remove_file("test_should_fail.db");
    let _ = fs::remove_file("test_should_also_fail.db");

    println!("\n✅ 全局操作锁机制测试完成！");
    Ok(())
}

async fn test_normal_database_addition() -> QuickDbResult<()> {
    println!("\n📋 测试1：正常添加数据库（查询操作开始前）");
    println!("===========================================");

    // 创建数据库配置
    let config = DatabaseConfig {
        db_type: DatabaseType::SQLite,
        connection: ConnectionConfig::SQLite {
            path: "test_global_lock.db".to_string(),
            create_if_missing: true,
        },
        pool: PoolConfig::default(),
        alias: "test_normal".to_string(),
        cache: None,
        id_strategy: IdStrategy::Uuid,
    };

    // 在查询操作开始前添加数据库应该成功
    match add_database(config).await {
        Ok(()) => println!("✅ 查询前添加数据库成功"),
        Err(e) => println!("❌ 查询前添加数据库失败: {}", e),
    }

    Ok(())
}

async fn test_database_addition_after_queries() -> QuickDbResult<()> {
    println!("\n🔍 测试2：查询操作后禁止添加数据库");
    println!("=================================");

    // 执行一个查询操作（这会锁定全局操作）
    println!("执行查询操作以触发全局锁...");

    // 创建一个简单的查询条件
    let conditions = vec![QueryCondition {
        field: "id".to_string(),
        operator: QueryOperator::Eq,
        value: DataValue::String("test".to_string()),
    }];

    // 执行查询（应该触发全局锁）
    match find(
        "test_collection",
        conditions.clone(),
        None,
        Some("test_normal"),
    )
    .await
    {
        Ok(_) => println!("✅ 查询操作执行成功（已触发全局锁）"),
        Err(e) => println!("⚠️ 查询操作执行失败: {}（但可能已触发全局锁）", e),
    }

    // 等待一小段时间确保全局锁已设置
    sleep(Duration::from_millis(100)).await;

    // 尝试添加新数据库（应该失败）
    let new_config = DatabaseConfig {
        db_type: DatabaseType::SQLite,
        connection: ConnectionConfig::SQLite {
            path: "test_should_fail.db".to_string(),
            create_if_missing: true,
        },
        pool: PoolConfig::default(),
        alias: "should_fail".to_string(),
        cache: None,
        id_strategy: IdStrategy::Uuid,
    };

    match add_database(new_config).await {
        Ok(()) => println!("❌ 查询后添加数据库成功（这不应该发生！）"),
        Err(e) => {
            println!("✅ 查询后添加数据库被正确阻止: {}", e);
            // 检查错误消息是否包含预期的内容
            if e.to_string().contains("系统已开始执行查询操作") {
                println!("✅ 错误消息符合预期");
            } else {
                println!("⚠️ 错误消息不符合预期: {}", e);
            }
        }
    }

    Ok(())
}

async fn test_database_addition_after_table_ops() -> QuickDbResult<()> {
    println!("\n📊 测试3：表操作后禁止添加数据库");
    println!("===============================");

    // 执行表操作（这也会锁定全局操作）
    println!("执行表检查操作以触发全局锁...");

    match table_exists("test_normal", "some_table").await {
        Ok(exists) => println!("✅ 表检查操作执行成功（已触发全局锁），表存在: {}", exists),
        Err(e) => println!("⚠️ 表检查操作执行失败: {}（但可能已触发全局锁）", e),
    }

    // 等待一小段时间确保全局锁已设置
    sleep(Duration::from_millis(100)).await;

    // 尝试添加另一个新数据库（应该失败）
    let another_config = DatabaseConfig {
        db_type: DatabaseType::SQLite,
        connection: ConnectionConfig::SQLite {
            path: "test_should_also_fail.db".to_string(),
            create_if_missing: true,
        },
        pool: PoolConfig::default(),
        alias: "should_also_fail".to_string(),
        cache: None,
        id_strategy: IdStrategy::Uuid,
    };

    match add_database(another_config).await {
        Ok(()) => println!("❌ 表操作后添加数据库成功（这不应该发生！）"),
        Err(e) => {
            println!("✅ 表操作后添加数据库被正确阻止: {}", e);
            // 检查错误消息是否包含预期的内容
            if e.to_string().contains("系统已开始执行查询操作") {
                println!("✅ 错误消息符合预期");
            } else {
                println!("⚠️ 错误消息不符合预期: {}", e);
            }
        }
    }

    Ok(())
}
