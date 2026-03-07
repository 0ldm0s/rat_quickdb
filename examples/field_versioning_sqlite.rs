//! 字段版本管理演示 (SQLite)
//!
//! 演示如何使用字段版本管理功能：
//! 1. 注册模型初始版本
//! 2. 升级模型到新版本
//! 3. 回滚到上一版本
//! 4. 查看 DDL 文件

use rat_quickdb::field_versioning::FieldVersionManager;
use rat_quickdb::model::field_types::{FieldDefinition, FieldType, ModelMeta};
use rat_quickdb::types::DatabaseType;
use std::collections::HashMap;

/// 创建版本 1 的用户模型
fn create_user_model_v1() -> ModelMeta {
    let mut fields = HashMap::new();

    fields.insert(
        "id".to_string(),
        FieldDefinition {
            field_type: FieldType::String {
                max_length: Some(36),
                min_length: None,
                regex: None,
            },
            required: true,
            unique: true,
            default: None,
            indexed: false,
            description: Some("用户ID".to_string()),
            validator: None,
            sqlite_compatibility: false,
        },
    );

    fields.insert(
        "username".to_string(),
        FieldDefinition {
            field_type: FieldType::String {
                max_length: Some(50),
                min_length: Some(1),
                regex: None,
            },
            required: true,
            unique: true,
            default: None,
            indexed: false,
            description: Some("用户名".to_string()),
            validator: None,
            sqlite_compatibility: false,
        },
    );

    fields.insert(
        "created_at".to_string(),
        FieldDefinition {
            field_type: FieldType::DateTime,
            required: true,
            unique: false,
            default: None,
            indexed: false,
            description: Some("创建时间".to_string()),
            validator: None,
            sqlite_compatibility: false,
        },
    );

    ModelMeta {
        collection_name: "users".to_string(),
        database_alias: Some("main".to_string()),
        fields,
        indexes: vec![],
        description: Some("用户表".to_string()),
        version: Some(1),
    }
}

/// 创建版本 2 的用户模型（新增 email 和 age 字段）
fn create_user_model_v2() -> ModelMeta {
    let mut fields = HashMap::new();

    // 保留原有字段
    fields.insert(
        "id".to_string(),
        FieldDefinition {
            field_type: FieldType::String {
                max_length: Some(36),
                min_length: None,
                regex: None,
            },
            required: true,
            unique: true,
            default: None,
            indexed: false,
            description: Some("用户ID".to_string()),
            validator: None,
            sqlite_compatibility: false,
        },
    );

    fields.insert(
        "username".to_string(),
        FieldDefinition {
            field_type: FieldType::String {
                max_length: Some(50),
                min_length: Some(1),
                regex: None,
            },
            required: true,
            unique: true,
            default: None,
            indexed: false,
            description: Some("用户名".to_string()),
            validator: None,
            sqlite_compatibility: false,
        },
    );

    fields.insert(
        "created_at".to_string(),
        FieldDefinition {
            field_type: FieldType::DateTime,
            required: true,
            unique: false,
            default: None,
            indexed: false,
            description: Some("创建时间".to_string()),
            validator: None,
            sqlite_compatibility: false,
        },
    );

    // 新增字段
    fields.insert(
        "email".to_string(),
        FieldDefinition {
            field_type: FieldType::String {
                max_length: Some(100),
                min_length: None,
                regex: None,
            },
            required: true,
            unique: true,
            default: None,
            indexed: false,
            description: Some("邮箱地址".to_string()),
            validator: None,
            sqlite_compatibility: false,
        },
    );

    fields.insert(
        "age".to_string(),
        FieldDefinition {
            field_type: FieldType::Integer {
                min_value: Some(0),
                max_value: Some(150),
            },
            required: false,
            unique: false,
            default: None,
            indexed: false,
            description: Some("年龄".to_string()),
            validator: None,
            sqlite_compatibility: false,
        },
    );

    ModelMeta {
        collection_name: "users".to_string(),
        database_alias: Some("main".to_string()),
        fields,
        indexes: vec![],
        description: Some("用户表".to_string()),
        version: Some(2),
    }
}

/// 创建版本 3 的用户模型（新增 is_active 字段）
fn create_user_model_v3() -> ModelMeta {
    let mut model = create_user_model_v2();
    model.version = Some(3);

    model.fields.insert(
        "is_active".to_string(),
        FieldDefinition {
            field_type: FieldType::Boolean,
            required: false,
            unique: false,
            default: None,
            indexed: false,
            description: Some("是否激活".to_string()),
            validator: None,
            sqlite_compatibility: false,
        },
    );

    model
}

fn main() {
    println!("=== 字段版本管理演示 (SQLite) ===\n");

    // 使用临时目录存储版本数据
    let temp_dir = std::env::temp_dir().join("rat_quickdb_versioning_demo");
    println!("📁 版本存储路径: {:?}\n", temp_dir);

    // 清理旧数据（如果存在）
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    // 创建版本管理器
    let manager = FieldVersionManager::new(temp_dir.clone(), DatabaseType::SQLite)
        .expect("无法创建版本管理器");

    // ========== 步骤 1: 注册初始版本 ==========
    println!("📝 步骤 1: 注册模型 v1");
    let model_v1 = create_user_model_v1();

    match manager.register_model(&model_v1) {
        Ok(_) => {
            println!("✅ 模型 v1 注册成功");

            // 查看生成的 DDL
            let ddl_path = manager.get_ddl_path("users", true);
            println!("📄 DDL 文件路径: {:?}", ddl_path);

            if let Ok(ddl) = manager.read_ddl("users", true) {
                println!("📄 生成的 DDL:\n{}", ddl);
            }
        }
        Err(e) => {
            println!("❌ 注册失败: {}", e);
        }
    }

    // ========== 步骤 2: 升级到 v2 ==========
    println!("\n📝 步骤 2: 升级模型到 v2 (新增 email, age 字段)");
    let model_v2 = create_user_model_v2();

    match manager.upgrade_model("users", &model_v2) {
        Ok(result) => {
            println!("✅ 升级成功: v{} -> v{}", result.old_version, result.new_version);
            println!("\n📄 升级 DDL:\n{}", result.upgrade_ddl);
            println!("📄 降级 DDL:\n{}", result.downgrade_ddl);
        }
        Err(e) => {
            println!("❌ 升级失败: {}", e);
        }
    }

    // ========== 步骤 3: 升级到 v3 ==========
    println!("\n📝 步骤 3: 升级模型到 v3 (新增 is_active 字段)");
    let model_v3 = create_user_model_v3();

    match manager.upgrade_model("users", &model_v3) {
        Ok(result) => {
            println!("✅ 升级成功: v{} -> v{}", result.old_version, result.new_version);
            println!("\n📄 升级 DDL:\n{}", result.upgrade_ddl);
            println!("📄 降级 DDL:\n{}", result.downgrade_ddl);
        }
        Err(e) => {
            println!("❌ 升级失败: {}", e);
        }
    }

    // ========== 步骤 4: 查看当前版本 ==========
    println!("\n📝 步骤 4: 查看当前版本");
    match manager.get_version("users") {
        Ok(Some(version)) => println!("✅ 当前版本: {}", version),
        Ok(None) => println!("⚠️ 模型不存在"),
        Err(e) => println!("❌ 查询失败: {}", e),
    }

    // ========== 步骤 5: 查看变更历史 ==========
    println!("\n📝 步骤 5: 查看变更历史");
    match manager.get_changes("users") {
        Ok(changes) => {
            if changes.is_empty() {
                println!("⚠️ 没有变更记录");
            } else {
                for change in changes {
                    println!(
                        "  - {} -> {} ({:?}) at {}",
                        change.from_version,
                        change.to_version,
                        change.change_type,
                        change.timestamp.format("%Y-%m-%d %H:%M:%S")
                    );
                }
            }
        }
        Err(e) => println!("❌ 查询失败: {}", e),
    }

    // ========== 步骤 6: 回滚到上一版本 ==========
    println!("\n📝 步骤 6: 回滚到上一版本");
    match manager.rollback_model("users") {
        Ok(result) => {
            println!("✅ 回滚成功: v{} -> v{}", result.old_version, result.new_version);
            println!("\n📄 升级 DDL (用于回滚后如需再次升级):\n{}", result.upgrade_ddl);
            println!("📄 降级 DDL:\n{}", result.downgrade_ddl);
        }
        Err(e) => {
            println!("❌ 回滚失败: {}", e);
        }
    }

    // ========== 步骤 7: 再次查看当前版本 ==========
    println!("\n📝 步骤 7: 确认回滚后的版本");
    match manager.get_version("users") {
        Ok(Some(version)) => println!("✅ 当前版本: {}", version),
        Ok(None) => println!("⚠️ 模型不存在"),
        Err(e) => println!("❌ 查询失败: {}", e),
    }

    // ========== 步骤 8: 查看 DDL 文件 ==========
    println!("\n📝 步骤 8: 查看存储的 DDL 文件");
    let upgrade_path = manager.get_ddl_path("users", true);
    let downgrade_path = manager.get_ddl_path("users", false);

    println!("📄 升级 DDL 文件: {:?}", upgrade_path);
    println!("📄 降级 DDL 文件: {:?}", downgrade_path);

    // 清理
    println!("\n🧹 清理临时文件...");
    std::fs::remove_dir_all(&temp_dir).ok();
    println!("✅ 演示完成!");
}
