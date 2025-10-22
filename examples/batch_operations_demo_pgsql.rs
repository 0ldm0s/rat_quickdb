//! 批量操作演示示例（PostgreSQL版本）
//!
//! 演示如何使用 rat_quickdb 进行批量更新和批量删除操作

use rat_quickdb::*;
use rat_quickdb::model::{ModelManager, Model, string_field, integer_field, float_field, boolean_field, datetime_field};
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

    println!("=== rat_quickdb 批量操作演示（PostgreSQL版本）===\n");

    // 1. 初始化数据库连接（使用PostgreSQL）
    let db_config = DatabaseConfig {
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
        pool: PoolConfig::default(),
        alias: "default".to_string(),
        id_strategy: IdStrategy::Uuid,
        cache: None,
    };

    // 添加数据库连接
    add_database(db_config).await?;

    // 清理旧数据表
    println!("清理旧数据表...");
    let _ = rat_quickdb::drop_table("default", "batch_demo_users").await;

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

    // 1. 演示按部门批量加薪
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

            // 批量更新薪资
            let mut update_data = HashMap::new();
            update_data.insert("updated_at".to_string(), DataValue::DateTime(Utc::now()));

            let mut updated_count = 0;
            for mut engineer in engineers {
                if let Some(current_salary) = engineer.salary {
                    let new_salary = current_salary * 1.1; // 增加10%
                    update_data.insert("salary".to_string(), DataValue::Float(new_salary));

                    match engineer.update(update_data.clone()).await {
                        Ok(_) => {
                            updated_count += 1;
                            println!("    ✅ 更新 {} 薪资: ${:.2} -> ${:.2}",
                                   engineer.username, current_salary, new_salary);
                        },
                        Err(e) => println!("    ❌ 更新 {} 失败: {}", engineer.username, e),
                    }
                }
            }
            println!("  📈 Engineering部门批量加薪完成，更新了 {} 个员工", updated_count);
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

    // 3. 演示复杂条件的批量操作
    println!("\n3️⃣ 复杂条件批量更新（特定部门且薪资低于某个值的员工）");
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

    let mut update_data = HashMap::new();
    update_data.insert("updated_at".to_string(), DataValue::DateTime(Utc::now()));

    match ModelManager::<User>::find(complex_conditions.clone(), None).await {
        Ok(target_users) => {
            println!("  找到 {} 个符合条件的Sales部门员工", target_users.len());

            for user in target_users {
                println!("    - 更新 {}: 部门={}, 薪资=${:.2}",
                       user.username, user.department, user.salary.unwrap_or(0.0));

                match user.update(update_data.clone()).await {
                    Ok(_) => println!("      ✅ 更新成功"),
                    Err(e) => println!("      ❌ 更新失败: {}", e),
                }
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

    // 2. 按部门批量删除
    println!("\n2️⃣ 按部门批量删除（删除Temp部门的所有用户）");
    let temp_dept_conditions = vec![
        QueryCondition {
            field: "department".to_string(),
            operator: QueryOperator::Eq,
            value: DataValue::String("Temp".to_string()),
        },
    ];

    // 先查询要删除的用户
    match ModelManager::<User>::find(temp_dept_conditions.clone(), None).await {
        Ok(temp_dept_users) => {
            println!("  找到 {} 个Temp部门的用户待删除", temp_dept_users.len());

            let mut deleted_count = 0;
            for user in temp_dept_users {
                println!("    - 删除用户: {} ({})", user.username, user.full_name);
                match user.delete().await {
                    Ok(_) => {
                        deleted_count += 1;
                        println!("      ✅ 删除成功");
                    },
                    Err(e) => println!("      ❌ 删除失败: {}", e),
                }
            }
            println!("  🗑️ Temp部门批量删除完成，删除了 {} 个用户", deleted_count);
        },
        Err(e) => println!("  ❌ 查询Temp部门失败: {}", e),
    }

    // 3. 按状态批量删除
    println!("\n3️⃣ 按状态批量删除（删除非活跃用户）");
    let inactive_conditions = vec![
        QueryCondition {
            field: "is_active".to_string(),
            operator: QueryOperator::Eq,
            value: DataValue::Bool(false),
        },
    ];

    match ModelManager::<User>::find(inactive_conditions.clone(), None).await {
        Ok(inactive_users) => {
            println!("  找到 {} 个非活跃用户待删除", inactive_users.len());

            let mut deleted_count = 0;
            for user in inactive_users {
                println!("    - 删除非活跃用户: {} ({})", user.username, user.full_name);
                match user.delete().await {
                    Ok(_) => {
                        deleted_count += 1;
                        println!("      ✅ 删除成功");
                    },
                    Err(e) => println!("      ❌ 删除失败: {}", e),
                }
            }
            println!("  🔒 非活跃用户批量删除完成，删除了 {} 个用户", deleted_count);
        },
        Err(e) => println!("  ❌ 查询非活跃用户失败: {}", e),
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