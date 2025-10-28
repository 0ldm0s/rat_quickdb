//! 测试模型宏的数据库别名功能
//! 验证跨库操作的正确性

use rat_quickdb::*;
use rat_quickdb::types::*;
use rat_quickdb::{ModelOperations, ModelManager, set_default_alias, add_database};
#[cfg(debug_assertions)]
use rat_logger::debug;
use chrono::{DateTime, Utc};

// 定义带有数据库别名的用户模型
define_model! {
    /// 用户模型（主数据库）
    struct MainUser {
        id: String,
        name: String,
        email: String,
        age: Option<i32>,  // 修复：改为Option<i32>以匹配现有表结构
    }
    collection = "users",
    database = "main_db",  // 指定数据库别名
    fields = {
        id: string_field(None, None, None).required().unique(),
        name: string_field(Some(100), Some(1), None).required(),
        email: string_field(Some(255), Some(1), None).required(),
        age: integer_field(None, None),  // 移除required约束，与现有表结构匹配
    }
}

// 定义带有不同数据库别名的用户模型
define_model! {
    /// 用户模型（归档数据库）
    struct ArchiveUser {
        id: String,
        name: String,
        email: String,
        archived_at: DateTime<Utc>,
    }
    collection = "users",
    database = "archive_db",  // 不同的数据库别名
    fields = {
        id: string_field(None, None, None).required().unique(),
        name: string_field(Some(100), Some(1), None).required(),
        email: string_field(Some(255), Some(1), None).required(),
        archived_at: datetime_field().required(),  // 使用正确的DateTime类型
    }
}

// 定义没有指定数据库别名的模型（应该使用默认别名）
define_model! {
    /// 日志模型（默认数据库）
    struct LogEntry {
        id: String,
        message: String,
        level: String,
        timestamp: DateTime<Utc>,
    }
    collection = "logs",
    // 没有指定 database，应该使用默认别名
    fields = {
        id: string_field(None, None, None).required().unique(),
        message: string_field(None, None, None).required(),
        level: string_field(Some(20), Some(1), None).required(),
        timestamp: datetime_field().required(),  // 使用正确的DateTime类型
    }
}

#[tokio::main]
async fn main() -> QuickDbResult<()> {

    println!("🚀 测试模型数据库别名功能");
    println!("===========================");

    // 创建测试数据库配置
    setup_test_databases().await?;

    // 测试1：验证模型的数据库别名获取
    test_model_database_alias();

    // 测试2：验证跨库操作
    test_cross_database_operations().await?;

    // 测试3：验证默认别名回退
    test_default_alias_fallback().await?;

    // 清理测试环境
    cleanup_test_databases().await?;

    println!("\n✅ 所有测试完成！");
    Ok(())
}

async fn setup_test_databases() -> QuickDbResult<()> {
    println!("\n📋 设置测试数据库...");

    // 删除可能存在的旧数据库文件，确保测试环境干净
    let old_files = ["test_main.db", "test_archive.db", "test_default.db"];
    for file in &old_files {
        if std::path::Path::new(file).exists() {
            if let Err(e) = std::fs::remove_file(file) {
                println!("⚠️ 删除旧文件 {} 失败: {}", file, e);
            } else {
                println!("🗑️ 删除旧文件: {}", file);
            }
        }
    }

    // 创建主数据库配置
    let main_config = DatabaseConfig {
        db_type: DatabaseType::SQLite,
        connection: ConnectionConfig::SQLite {
            path: "test_main.db".to_string(),
            create_if_missing: true,
        },
        pool: PoolConfig::default(),
        alias: "main_db".to_string(),
        cache: None,
        id_strategy: IdStrategy::Uuid,
    };

    // 创建归档数据库配置
    let archive_config = DatabaseConfig {
        db_type: DatabaseType::SQLite,
        connection: ConnectionConfig::SQLite {
            path: "test_archive.db".to_string(),
            create_if_missing: true,
        },
        pool: PoolConfig::default(),
        alias: "archive_db".to_string(),
        cache: None,
        id_strategy: IdStrategy::Uuid,
    };

    // 创建默认数据库配置
    let default_config = DatabaseConfig {
        db_type: DatabaseType::SQLite,
        connection: ConnectionConfig::SQLite {
            path: "test_default.db".to_string(),
            create_if_missing: true,
        },
        pool: PoolConfig::default(),
        alias: "default".to_string(),
        cache: None,
        id_strategy: IdStrategy::Uuid,
    };

    // 添加数据库
    add_database(main_config).await?;
    add_database(archive_config).await?;
    add_database(default_config).await?;

    println!("✅ 测试数据库设置完成");
    println!("📁 数据库文件路径：test_main.db, test_archive.db, test_default.db");
    Ok(())
}

fn test_model_database_alias() {
    println!("\n🔍 测试1：验证模型的数据库别名获取");
    println!("=========================================");

    // 测试MainUser的数据库别名
    let main_alias = MainUser::database_alias();
    println!("MainUser 数据库别名: {:?}", main_alias);
    assert_eq!(main_alias, Some("main_db".to_string()));

    // 测试ArchiveUser的数据库别名
    let archive_alias = ArchiveUser::database_alias();
    println!("ArchiveUser 数据库别名: {:?}", archive_alias);
    assert_eq!(archive_alias, Some("archive_db".to_string()));

    // 测试LogEntry的数据库别名（应该为None，使用默认别名）
    let log_alias = LogEntry::database_alias();
    println!("LogEntry 数据库别名: {:?}", log_alias);
    assert_eq!(log_alias, None);

    println!("✅ 模型数据库别名获取测试通过");
}

async fn test_cross_database_operations() -> QuickDbResult<()> {
    println!("\n🔄 测试2：验证跨库操作");
    println!("========================");

    // 在主数据库创建用户（save会自动创建表）
    let main_user = MainUser {
        id: "main_user_1".to_string(),
        name: "主库用户".to_string(),
        email: "main@example.com".to_string(),
        age: Some(25),  // Option<i32>类型
    };

    match main_user.save().await {
        Ok(id) => println!("✅ 主数据库用户创建成功: {}", id),
        Err(e) => println!("❌ 主数据库用户创建失败: {}", e),
    }

    // 在归档数据库创建用户（save会自动创建表）
    let archive_user = ArchiveUser {
        id: "archive_user_1".to_string(),
        name: "归档用户".to_string(),
        email: "archive@example.com".to_string(),
        archived_at: DateTime::parse_from_rfc3339("2023-01-01T00:00:00Z").unwrap().with_timezone(&Utc),
    };

    match archive_user.save().await {
        Ok(id) => println!("✅ 归档数据库用户创建成功: {}", id),
        Err(e) => println!("❌ 归档数据库用户创建失败: {}", e),
    }

    // 从主数据库查询用户
    match ModelManager::<MainUser>::find_by_id("main_user_1").await {
        Ok(Some(user)) => println!("✅ 从主数据库查询到用户: {}", user.name),
        Ok(None) => println!("⚠️ 主数据库中未找到用户"),
        Err(e) => println!("❌ 主数据库查询失败: {}", e),
    }

    // 从归档数据库查询用户
    match ModelManager::<ArchiveUser>::find_by_id("archive_user_1").await {
        Ok(Some(user)) => println!("✅ 从归档数据库查询到用户: {}", user.name),
        Ok(None) => println!("⚠️ 归档数据库中未找到用户"),
        Err(e) => println!("❌ 归档数据库查询失败: {}", e),
    }

    println!("✅ 跨库操作测试完成");
    Ok(())
}

async fn test_default_alias_fallback() -> QuickDbResult<()> {
    println!("\n🔄 测试3：验证默认别名回退");
    println!("==========================");

    // 设置默认数据库别名
    set_default_alias("default").await?;

    // 创建日志条目（应该使用默认数据库）
    let log_entry = LogEntry {
        id: "log_1".to_string(),
        message: "测试日志消息".to_string(),
        level: "INFO".to_string(),
        timestamp: DateTime::parse_from_rfc3339("2023-01-01T12:00:00Z").unwrap().with_timezone(&Utc),
    };

    match log_entry.save().await {
        Ok(id) => println!("✅ 默认数据库日志创建成功: {}", id),
        Err(e) => println!("❌ 默认数据库日志创建失败: {}", e),
    }

    // 从默认数据库查询日志
    match ModelManager::<LogEntry>::find_by_id("log_1").await {
        Ok(Some(log)) => println!("✅ 从默认数据库查询到日志: {}", log.message),
        Ok(None) => println!("⚠️ 默认数据库中未找到日志"),
        Err(e) => println!("❌ 默认数据库查询失败: {}", e),
    }

    println!("✅ 默认别名回退测试完成");
    Ok(())
}

async fn cleanup_test_databases() -> QuickDbResult<()> {
    println!("\n🧹 清理测试数据库...");

    // 注意：remove_database已被移除，不再支持动态移除数据库
    // 这是设计上的安全考虑，防止运行时危险操作
    // 保留测试文件以便检查
    println!("📁 保留测试文件以便检查：test_main.db, test_archive.db, test_default.db");

    // 检查文件是否存在
    for file in ["test_main.db", "test_archive.db", "test_default.db"] {
        if std::path::Path::new(file).exists() {
            println!("✅ 数据库文件存在: {}", file);
        } else {
            println!("❌ 数据库文件不存在: {}", file);
        }
    }

    Ok(())
}