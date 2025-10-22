//! 批量操作演示示例（MongoDB版本）
//!
//! 演示如何使用 rat_quickdb 进行批量更新和批量删除操作

use rat_quickdb::*;
use rat_quickdb::model::{ModelManager, Model, string_field, integer_field, float_field, boolean_field, datetime_field};
use rat_quickdb::types::{UpdateOperation, QueryOperator, QueryCondition, DataValue, DatabaseType, ConnectionConfig};
use rat_logger::debug;
use chrono::Utc;
use std::collections::HashMap;

/// 定义用户模型
define_model! {
    /// 用户模型
    struct User {
        id: String,
        username: String,
        email: String,
        full_name: String,
        age: Option<i32>,
        department: String,
        is_active: bool,
        salary: Option<f64>,
        created_at: chrono::DateTime<chrono::Utc>,
        updated_at: Option<chrono::DateTime<chrono::Utc>>,
    }
    collection = "batch_demo_users",
    fields = {
        id: string_field(None, None, None).required().unique(),
        username: string_field(None, None, None).required().unique(),
        email: string_field(None, None, None).required().unique(),
        full_name: string_field(None, None, None).required(),
        age: integer_field(None, None),
        department: string_field(None, None, None).required(),
        is_active: boolean_field().required(),
        salary: float_field(None, None),
        created_at: datetime_field().required(),
        updated_at: datetime_field(),
    }
    indexes = [
        { fields: ["username"], unique: true, name: "idx_username" },
        { fields: ["email"], unique: true, name: "idx_email" },
        { fields: ["department"], unique: false, name: "idx_department" },
        { fields: ["is_active"], unique: false, name: "idx_active" },
    ],
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    rat_logger::init();

    println!("=== rat_quickdb 批量操作演示（MongoDB版本）===\n");

    // 1. 初始化数据库连接（使用MongoDB）
    let db_config = DatabaseConfig {
        alias: "mongodb_default".to_string(),
        db_type: DatabaseType::MongoDB,
        connection: ConnectionConfig::MongoDB {
            host: "db0.0ldm0s.net".to_string(),
            port: 27017,
            database: "testdb".to_string(),
            username: Some("testdb".to_string()),
            password: Some("yash2vCiBA&B#h$#i&gb@IGSTh&cP#QC^".to_string()),
            auth_source: Some("testdb".to_string()),
            direct_connection: true,
            tls_config: Some(rat_quickdb::types::TlsConfig {
                enabled: true,
                ca_cert_path: None,
                client_cert_path: None,
                client_key_path: None,
                verify_server_cert: false,
                verify_hostname: false,
                min_tls_version: None,
                cipher_suites: None,
            }),
            zstd_config: Some(rat_quickdb::types::ZstdConfig {
                enabled: true,
                compression_level: Some(3),
                compression_threshold: Some(1024),
            }),
            options: {
                let mut opts = std::collections::HashMap::new();
                opts.insert("retryWrites".to_string(), "true".to_string());
                opts.insert("w".to_string(), "majority".to_string());
                Some(opts)
            },
        },
        pool: PoolConfig::default(),
        id_strategy: IdStrategy::Uuid,
        cache: None,
    };

    // 添加数据库连接
    add_database(db_config).await?;

    // 设置默认数据库别名
    rat_quickdb::set_default_alias("mongodb_default").await?;

    // 清理旧数据表
    println!("清理旧数据表...");
    let _ = rat_quickdb::drop_table("mongodb_default", "batch_demo_users").await;

    println!("✅ 数据库连接已建立");

    // 2. 创建测试数据
    create_test_data().await?;

    // 3. 演示批量更新操作
    demonstrate_batch_update().await?;

    // 4. 演示批量删除操作
    demonstrate_batch_delete().await?;

    // 5. 清理演示数据
    cleanup_demo_data().await?;

    println!("\n✅ 批量操作演示完成！");
    Ok(())
}

/// 创建测试数据
async fn create_test_data() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📝 创建测试数据...");

    let test_users = vec![
        User {
            id: String::new(),
            username: "alice_dev".to_string(),
            email: "alice@company.com".to_string(),
            full_name: "Alice Johnson".to_string(),
            age: Some(28),
            department: "Engineering".to_string(),
            is_active: true,
            salary: Some(80000.0),
            created_at: Utc::now(),
            updated_at: Some(Utc::now()),
        },
        User {
            id: String::new(),
            username: "bob_dev".to_string(),
            email: "bob@company.com".to_string(),
            full_name: "Bob Smith".to_string(),
            age: Some(32),
            department: "Engineering".to_string(),
            is_active: true,
            salary: Some(95000.0),
            created_at: Utc::now(),
            updated_at: Some(Utc::now()),
        },
        User {
            id: String::new(),
            username: "charlie_hr".to_string(),
            email: "charlie@company.com".to_string(),
            full_name: "Charlie Brown".to_string(),
            age: Some(35),
            department: "Human Resources".to_string(),
            is_active: true,
            salary: Some(65000.0),
            created_at: Utc::now(),
            updated_at: Some(Utc::now()),
        },
        User {
            id: String::new(),
            username: "diana_sales".to_string(),
            email: "diana@company.com".to_string(),
            full_name: "Diana Wilson".to_string(),
            age: Some(29),
            department: "Sales".to_string(),
            is_active: true,
            salary: Some(70000.0),
            created_at: Utc::now(),
            updated_at: Some(Utc::now()),
        },
        User {
            id: String::new(),
            username: "eve_dev".to_string(),
            email: "eve@company.com".to_string(),
            full_name: "Eve Davis".to_string(),
            age: Some(26),
            department: "Engineering".to_string(),
            is_active: true,
            salary: Some(75000.0),
            created_at: Utc::now(),
            updated_at: Some(Utc::now()),
        },
    ];

    let mut created_count = 0;
    for user in test_users {
        match user.save().await {
            Ok(_) => {
                created_count += 1;
                println!("  ✅ 创建用户成功");
            },
            Err(e) => println!("  ❌ 创建用户失败: {}", e),
        }
    }

    println!("📊 测试数据创建完成，共创建 {} 个用户", created_count);
    Ok(())
}

/// 演示批量更新操作
async fn demonstrate_batch_update() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔄 批量更新操作演示");

    // 1. 演示按部门批量加薪 - 使用新的update_many_with_operations方法
    println!("\n1️⃣ 按部门批量加薪（Engineering部门薪资增加10%）");
    let engineering_conditions = vec![
        QueryCondition {
            field: "department".to_string(),
            operator: QueryOperator::Eq,
            value: DataValue::String("Engineering".to_string()),
        },
    ];

    // 先查询Engineering部门的用户
    match ModelManager::<User>::find(engineering_conditions.clone(), None).await {
        Ok(engineers) => {
            println!("  找到 {} 个Engineering部门的员工", engineers.len());
            for eng in &engineers {
                println!("    - {}: 当前薪资 ${:.2}", eng.username, eng.salary.unwrap_or(0.0));
            }

            // 使用新的批量操作方法进行原子性更新！
            println!("  🔥 使用新的update_many_with_operations方法进行高效批量更新...");
            let operations = vec![
                // 更新时间戳
                UpdateOperation::set("updated_at", DataValue::DateTime(Utc::now())),
                // 真正的百分比增加！直接在SQL中计算salary = salary * (1.0 + 10.0/100.0)
                UpdateOperation::percent_increase("salary", 10.0), // 增加10%
            ];

            match User::update_many_with_operations(engineering_conditions.clone(), operations).await {
                Ok(affected_rows) => {
                    println!("  ✅ 高效批量加薪完成！影响了 {} 条记录", affected_rows);
                    println!("  🎉 这是真正的高效SQL操作：UPDATE users SET updated_at = ?, salary = salary * (1.0 + 10.0/100.0) WHERE department = ?");
                },
                Err(e) => println!("  ❌ 批量加薪失败: {}", e),
            }

            // 查询更新后的结果验证
            println!("  🔍 验证更新结果...");
            match ModelManager::<User>::find(engineering_conditions.clone(), None).await {
                Ok(updated_engineers) => {
                    for eng in &updated_engineers {
                        println!("    - {}: 更新后薪资 ${:.2}", eng.username, eng.salary.unwrap_or(0.0));
                    }
                },
                Err(e) => println!("  ❌ 验证失败: {}", e),
            }
        },
        Err(e) => println!("  ❌ 查询Engineering部门失败: {}", e),
    }

    // 2. 演示基于年龄的批量状态更新
    println!("\n2️⃣ 批量状态更新（年龄>=30的用户标记为资深员工）");
    let senior_conditions = vec![
        QueryCondition {
            field: "age".to_string(),
            operator: QueryOperator::Gte,
            value: DataValue::Int(30),
        },
    ];

    let mut update_data = HashMap::new();
    update_data.insert("updated_at".to_string(), DataValue::DateTime(Utc::now()));

    // 查询资深员工
    match ModelManager::<User>::find(senior_conditions.clone(), None).await {
        Ok(senior_users) => {
            println!("  找到 {} 个年龄>=30的用户", senior_users.len());

            let mut updated_count = 0;
            for user in senior_users {
                println!("    - 标记 {} 为资深用户 (年龄: {})", user.username, user.age.unwrap_or(0));

                match user.update(update_data.clone()).await {
                    Ok(_) => updated_count += 1,
                    Err(e) => println!("    ❌ 更新用户 {} 失败: {}", user.username, e),
                }
            }
            println!("  🏆 资深用户标记完成，更新了 {} 个用户", updated_count);
        },
        Err(e) => println!("  ❌ 查询资深用户失败: {}", e),
    }

    // 3. 演示复杂条件的批量操作 - 使用多种新操作类型
    println!("\n3️⃣ 复杂条件批量更新（Sales部门低薪员工多重调整）");
    let complex_conditions = vec![
        QueryCondition {
            field: "department".to_string(),
            operator: QueryOperator::Eq,
            value: DataValue::String("Sales".to_string()),
        },
        QueryCondition {
            field: "salary".to_string(),
            operator: QueryOperator::Lt,
            value: DataValue::Float(75000.0),
        },
    ];

    match ModelManager::<User>::find(complex_conditions.clone(), None).await {
        Ok(target_users) => {
            println!("  找到 {} 个符合条件的Sales部门低薪员工", target_users.len());
            for user in &target_users {
                println!("    - 调整前 {}: 薪资=${:.2}, 活跃={}",
                       user.username, user.salary.unwrap_or(0.0), user.is_active);
            }

            // 使用多种新操作类型进行复杂的批量更新！
            println!("  🔥 使用多种新操作类型进行复杂批量更新...");
            let operations = vec![
                // 更新时间戳
                UpdateOperation::set("updated_at", DataValue::DateTime(Utc::now())),
                // 薪资增加37.5% (合并25%加薪 + 10%奖金，1.25 * 1.1 = 1.375，即增加37.5%)
                UpdateOperation::percent_increase("salary", 37.5),
                // 年龄加1岁 (模拟生日批量更新)
                UpdateOperation::increment("age", DataValue::Int(1)),
                // 设置为活跃用户
                UpdateOperation::set("is_active", DataValue::Bool(true)),
            ];

            match User::update_many_with_operations(complex_conditions.clone(), operations).await {
                Ok(affected_rows) => {
                    println!("  ✅ 复杂批量更新完成！影响了 {} 条记录", affected_rows);
                    println!("  🎉 生成的复杂SQL操作包含多个原子操作！");
                },
                Err(e) => println!("  ❌ 复杂批量更新失败: {}", e),
            }

            // 验证更新结果
            println!("  🔍 验证复杂更新结果...");
            match ModelManager::<User>::find(complex_conditions.clone(), None).await {
                Ok(updated_users) => {
                    for user in &updated_users {
                        println!("    - 调整后 {}: 薪资=${:.2}, 活跃={}, 年龄={}",
                               user.username, user.salary.unwrap_or(0.0), user.is_active, user.age.unwrap_or(0));
                    }
                },
                Err(e) => println!("  ❌ 验证失败: {}", e),
            }
        },
        Err(e) => println!("  ❌ 查询失败: {}", e),
    }

    Ok(())
}

/// 演示批量删除操作
async fn demonstrate_batch_delete() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🗑️ 批量删除操作演示");

    // 1. 先创建一些用于删除演示的临时数据
    println!("\n1️⃣ 创建临时数据用于删除演示");
    let temp_users = vec![
        ("temp_user_1", "temp1@test.com", "Temp User 1", "Temp"),
        ("temp_user_2", "temp2@test.com", "Temp User 2", "Temp"),
        ("temp_user_3", "temp3@test.com", "Temp User 3", "Temp"),
    ];

    let mut temp_ids = Vec::new();
    for (username, email, full_name, department) in temp_users {
        let temp_user = User {
            id: String::new(),
            username: username.to_string(),
            email: email.to_string(),
            full_name: full_name.to_string(),
            age: Some(25),
            department: department.to_string(),
            is_active: false, // 默认非活跃
            salary: Some(50000.0),
            created_at: Utc::now(),
            updated_at: Some(Utc::now()),
        };

        match temp_user.save().await {
            Ok(id) => {
                temp_ids.push(id);
                println!("  ✅ 创建临时用户: {}", username);
            },
            Err(e) => println!("  ❌ 创建临时用户失败: {}", e),
        }
    }

    // 2. 按部门批量删除 - 测试高效delete_many方法
    println!("\n2️⃣ 按部门批量删除（删除Temp部门的所有用户）");
    let temp_dept_conditions = vec![
        QueryCondition {
            field: "department".to_string(),
            operator: QueryOperator::Eq,
            value: DataValue::String("Temp".to_string()),
        },
    ];

    // 先显示要删除的用户（仅用于演示）
    match ModelManager::<User>::find(temp_dept_conditions.clone(), None).await {
        Ok(temp_dept_users) => {
            println!("  找到 {} 个Temp部门的用户待删除", temp_dept_users.len());
            for user in &temp_dept_users {
                println!("    - 将删除: {} ({})", user.username, user.full_name);
            }
        },
        Err(e) => {
            println!("  ❌ 查询Temp部门失败: {}", e);
        }
    }

    // 🔥 测试真正的高效批量删除！
    println!("  🔥 测试User::delete_many高效批量删除...");
    match User::delete_many(temp_dept_conditions.clone()).await {
        Ok(affected_rows) => {
            println!("  ✅ 高效批量删除成功！删除了 {} 条记录", affected_rows);
            println!("  🎉 一次SQL操作：DELETE FROM users WHERE department = 'Temp'");
        },
        Err(e) => {
            println!("  ❌ User::delete_many失败: {}", e);
            println!("  🔄 降级使用逐个删除方式...");

            // 降级方案：使用原来的逐个删除方式
            match ModelManager::<User>::find(temp_dept_conditions.clone(), None).await {
                Ok(temp_dept_users) => {
                    let mut deleted_count = 0;
                    for user in temp_dept_users {
                        match user.delete().await {
                            Ok(_) => {
                                deleted_count += 1;
                                println!("    - 逐个删除成功: {}", user.username);
                            },
                            Err(e) => println!("    ❌ 逐个删除失败 {}: {}", user.username, e),
                        }
                    }
                    println!("  📊 逐个删除完成，删除了 {} 个用户", deleted_count);
                },
                Err(e) => println!("  ❌ 降级删除也失败: {}", e),
            }
        }
    }

    // 3. 按状态批量删除 - 使用高效delete_many方法
    println!("\n3️⃣ 按状态批量删除（删除非活跃用户）");
    let inactive_conditions = vec![
        QueryCondition {
            field: "is_active".to_string(),
            operator: QueryOperator::Eq,
            value: DataValue::Bool(false),
        },
    ];

    // 🔥 使用高效的批量删除！
    println!("  🔥 使用User::delete_many删除非活跃用户...");
    match User::delete_many(inactive_conditions.clone()).await {
        Ok(affected_rows) => {
            println!("  ✅ 非活跃用户批量删除成功！删除了 {} 条记录", affected_rows);
            println!("  🎉 一次SQL操作：DELETE FROM users WHERE is_active = false");
        },
        Err(e) => {
            println!("  ❌ User::delete_many失败: {}", e);
            println!("  🔄 降级使用逐个删除方式...");

            // 降级方案
            match ModelManager::<User>::find(inactive_conditions.clone(), None).await {
                Ok(inactive_users) => {
                    println!("  找到 {} 个非活跃用户待删除", inactive_users.len());
                    let mut deleted_count = 0;
                    for user in inactive_users {
                        match user.delete().await {
                            Ok(_) => {
                                deleted_count += 1;
                                println!("    - 逐个删除非活跃用户: {}", user.username);
                            },
                            Err(e) => println!("    ❌ 逐个删除失败 {}: {}", user.username, e),
                        }
                    }
                    println!("  🔒 逐个删除完成，删除了 {} 个用户", deleted_count);
                },
                Err(e) => println!("  ❌ 降级删除也失败: {}", e),
            }
        }
    }

    Ok(())
}

/// 清理演示数据
async fn cleanup_demo_data() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🧹 清理演示数据...");

    // 删除所有演示数据
    let all_conditions = vec![]; // 无条件，匹配所有记录

    match ModelManager::<User>::find(all_conditions, None).await {
        Ok(all_users) => {
            println!("  找到 {} 个用户待清理", all_users.len());

            let mut deleted_count = 0;
            for user in all_users {
                match user.delete().await {
                    Ok(_) => deleted_count += 1,
                    Err(e) => println!("  ❌ 删除用户 {} 失败: {}", user.username, e),
                }
            }
            println!("  🧹 演示数据清理完成，删除了 {} 个用户", deleted_count);
        },
        Err(e) => println!("  ❌ 查询演示数据失败: {}", e),
    }

    Ok(())
}