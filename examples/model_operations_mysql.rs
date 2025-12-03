//! RatQuickDB MySQL 模型操作演示
//!
//! 展示完整的模型操作：
//! - 模型定义和验证
//! - CRUD操作
//! - 并发测试
//! - 分页查询

use rat_quickdb::*;
use rat_quickdb::types::*;
use rat_quickdb::{ModelManager, ModelOperations,
    string_field, integer_field, float_field, boolean_field, datetime_field, json_field, array_field, uuid_field};
use rat_logger::{LoggerBuilder, LevelFilter, handler::term::TermConfig};
use std::collections::HashMap;
use std::time::Instant;
use tokio::join;
use chrono::Utc;

// 用户模型
define_model! {
    struct User {
        id: String,
        username: String,
        email: String,
        password_hash: String,
        full_name: String,
        age: Option<i32>,
        is_active: bool,
        created_at: chrono::DateTime<chrono::Utc>,
        tags: Option<Vec<String>>,
    }
    collection = "users",
    database = "main",
    fields = {
        id: uuid_field().required().unique(),
        username: string_field(None, None, None).required().unique(),
        email: string_field(None, None, None).required().unique(),
        password_hash: string_field(None, None, None).required(),
        full_name: string_field(None, None, None).required(),
        age: integer_field(None, None),
        is_active: boolean_field().required(),
        created_at: datetime_field().required(),
        tags: array_field(field_types!(string), None, None),
    }
    indexes = [
        { fields: ["username"], unique: true, name: "idx_username" },
        { fields: ["email"], unique: true, name: "idx_email" },
        { fields: ["is_active", "created_at"], unique: false, name: "idx_active_created" },
    ],
}

// 员工模型 - 用于分页测试
define_model! {
    struct Employee {
        id: String,
        employee_id: String,
        name: String,
        department: String,
        salary: f64,
        is_active: bool,
        created_at: chrono::DateTime<chrono::Utc>,
    }
    collection = "employees",
    database = "main",
    fields = {
        id: uuid_field().required().unique(),
        employee_id: string_field(None, None, None).required().unique(),
        name: string_field(None, None, None).required(),
        department: string_field(None, None, None).required(),
        salary: float_field(None, None).required(),
        is_active: boolean_field().required(),
        created_at: datetime_field().required(),
    }
    indexes = [
        { fields: ["employee_id"], unique: true, name: "idx_employee_id" },
        { fields: ["department"], unique: false, name: "idx_department" },
        { fields: ["salary"], unique: false, name: "idx_salary" },
    ],
}

// 简单性能统计
#[derive(Debug)]
struct SimpleStats {
    total_operations: u64,
    successful_operations: u64,
    total_time_ms: u64,
}

impl SimpleStats {
    fn new() -> Self {
        Self {
            total_operations: 0,
            successful_operations: 0,
            total_time_ms: 0,
        }
    }

    fn add_operation(&mut self, duration_ms: u64, success: bool) {
        self.total_operations += 1;
        self.total_time_ms += duration_ms;
        if success {
            self.successful_operations += 1;
        }
    }

    fn average_time(&self) -> f64 {
        if self.total_operations == 0 {
            0.0
        } else {
            self.total_time_ms as f64 / self.total_operations as f64
        }
    }

    fn success_rate(&self) -> f64 {
        if self.total_operations == 0 {
            0.0
        } else {
            self.successful_operations as f64 / self.total_operations as f64 * 100.0
        }
    }
}

// 清理测试数据
async fn cleanup_test_data() {
    println!("清理测试数据...");

    // 清理用户表
    if let Err(e) = rat_quickdb::drop_table("main", "users").await {
        println!("清理用户表失败: {}", e);
    }

    // 清理员工表
    if let Err(e) = rat_quickdb::drop_table("main", "employees").await {
        println!("清理员工表失败: {}", e);
    }
}

// 演示基础CRUD操作
async fn demonstrate_crud() -> Result<SimpleStats, Box<dyn std::error::Error>> {
    println!("\n=== CRUD操作演示 ===");

    let mut stats = SimpleStats::new();

    // 创建用户
    let user = User {
        id: String::new(),
        username: format!("test_user_{}", uuid::Uuid::new_v4().simple()),
        email: format!("test_{}@example.com", uuid::Uuid::new_v4().simple()),
        password_hash: "hashed_password".to_string(),
        full_name: "测试用户".to_string(),
        age: Some(25),
        is_active: true,
        created_at: Utc::now(),
        tags: Some(vec!["测试".to_string(), "用户".to_string()]),
    };

    // 插入
    let start = Instant::now();
    let user_id = match user.save().await {
        Ok(id) => {
            println!("✅ 用户创建成功: {}", id);
            id
        }
        Err(e) => {
            println!("❌ 用户创建失败: {}", e);
            return Ok(stats);
        }
    };
    stats.add_operation(start.elapsed().as_millis() as u64, true);

    // 查询
    let start = Instant::now();
    match ModelManager::<User>::find_by_id(&user_id).await {
        Ok(Some(found_user)) => {
            println!("✅ 用户查询成功: {}", found_user.username);
            stats.add_operation(start.elapsed().as_millis() as u64, true);

            // 更新
            let start = Instant::now();
            let mut update_data = HashMap::new();
            update_data.insert("age".to_string(), DataValue::Int(26));

            match found_user.update(update_data).await {
                Ok(_) => {
                    println!("✅ 用户更新成功");
                    stats.add_operation(start.elapsed().as_millis() as u64, true);
                }
                Err(e) => {
                    println!("❌ 用户更新失败: {}", e);
                    stats.add_operation(start.elapsed().as_millis() as u64, false);
                }
            }

            // 删除
            let start = Instant::now();
            match found_user.delete().await {
                Ok(_) => {
                    println!("✅ 用户删除成功");
                    stats.add_operation(start.elapsed().as_millis() as u64, true);
                }
                Err(e) => {
                    println!("❌ 用户删除失败: {}", e);
                    stats.add_operation(start.elapsed().as_millis() as u64, false);
                }
            }
        }
        Ok(None) => {
            println!("❌ 用户未找到");
            stats.add_operation(start.elapsed().as_millis() as u64, false);
        }
        Err(e) => {
            println!("❌ 查询失败: {}", e);
            stats.add_operation(start.elapsed().as_millis() as u64, false);
        }
    }

    Ok(stats)
}

// 演示并发操作
async fn demonstrate_concurrency() -> Result<SimpleStats, Box<dyn std::error::Error>> {
    println!("\n=== 并发操作演示 ===");

    let mut stats = SimpleStats::new();
    let concurrent_count = 10;

    // 并发插入
    let mut tasks = Vec::new();
    for i in 0..concurrent_count {
        let task = tokio::spawn(async move {
            let user = User {
                id: String::new(),
                username: format!("concurrent_user_{}_{}", i, uuid::Uuid::new_v4().simple()),
                email: format!("concurrent_{}_{}@example.com", i, uuid::Uuid::new_v4().simple()),
                password_hash: "hashed_password".to_string(),
                full_name: format!("并发用户 {}", i),
                age: Some(20 + i),
                is_active: true,
                created_at: Utc::now(),
                tags: Some(vec!["并发".to_string(), "测试".to_string()]),
            };

            let start = Instant::now();
            match user.save().await {
                Ok(id) => {
                    println!("  并发创建用户 {}: 成功 ({})", i, id);
                    (start.elapsed().as_millis() as u64, true)
                }
                Err(e) => {
                    println!("  并发创建用户 {}: 失败 - {}", i, e);
                    (start.elapsed().as_millis() as u64, false)
                }
            }
        });
        tasks.push(task);
    }

    // 等待所有任务完成
    for (i, task) in tasks.into_iter().enumerate() {
        match task.await {
            Ok((duration, success)) => {
                stats.add_operation(duration, success);
            }
            Err(e) => {
                println!("任务 {} 执行失败: {}", i, e);
                stats.add_operation(0, false);
            }
        }
    }

    println!("并发操作完成 - 成功率: {:.1}%, 平均耗时: {:.1}ms",
             stats.success_rate(), stats.average_time());

    Ok(stats)
}

// 演示分页查询
async fn demonstrate_pagination() -> Result<SimpleStats, Box<dyn std::error::Error>> {
    println!("\n=== 分页查询演示 ===");

    let mut stats = SimpleStats::new();

    // 先创建一些测试数据
    println!("创建测试员工数据...");
    let mut created_ids = Vec::new();
    for i in 0..50 {
        let employee = Employee {
            id: String::new(),
            employee_id: format!("EMP{:04}", i),
            name: format!("员工 {}", i),
            department: match i % 3 {
                0 => "技术部".to_string(),
                1 => "销售部".to_string(),
                _ => "人事部".to_string(),
            },
            salary: 5000.0 + (i as f64 * 100.0),
            is_active: i % 5 != 0, // 80%活跃
            created_at: Utc::now(),
        };

        match employee.save().await {
            Ok(id) => created_ids.push(id),
            Err(e) => println!("创建员工 {} 失败: {}", i, e),
        }
    }

    // 分页查询
    let page_size = 10;
    for page in 0..5 {
        let start = Instant::now();
        let pagination = PaginationConfig {
            limit: page_size,
            skip: page * page_size,
        };

        let options = QueryOptions {
            pagination: Some(pagination),
            ..Default::default()
        };

        match ModelManager::<Employee>::find(vec![], Some(options)).await {
            Ok(employees) => {
                stats.add_operation(start.elapsed().as_millis() as u64, true);
                println!("第 {} 页: {} 条记录", page + 1, employees.len());

                // 显示前几条记录
                for (i, emp) in employees.iter().take(3).enumerate() {
                    println!("  {}. {} - {} - ${:.0}",
                             (page as u64 * page_size as u64 + i as u64 + 1), emp.name, emp.department, emp.salary);
                }
                if employees.len() > 3 {
                    println!("     ... 还有 {} 条", employees.len() - 3);
                }
            }
            Err(e) => {
                stats.add_operation(start.elapsed().as_millis() as u64, false);
                println!("第 {} 页查询失败: {}", page + 1, e);
            }
        }
    }

    // 清理测试数据
    println!("清理测试数据...");
    for id in created_ids {
        if let Ok(Some(employee)) = ModelManager::<Employee>::find_by_id(&id).await {
            let _ = employee.delete().await;
        }
    }

    println!("分页查询完成 - 成功率: {:.1}%, 平均耗时: {:.1}ms",
             stats.success_rate(), stats.average_time());

    Ok(stats)
}

// 性能基准测试
async fn performance_benchmark() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== 性能基准测试 ===");

    let test_count = 100;
    let start = Instant::now();

    // 批量创建测试
    let mut successful = 0;
    for i in 0..test_count {
        let user = User {
            id: String::new(),
            username: format!("perf_user_{}", i),
            email: format!("perf_{}@test.com", i),
            password_hash: "hash".to_string(),
            full_name: format!("性能用户 {}", i),
            age: Some(25 + i),
            is_active: true,
            created_at: Utc::now(),
            tags: Some(vec!["性能测试".to_string()]),
        };

        if user.save().await.is_ok() {
            successful += 1;
        }
    }

    let create_time = start.elapsed();
    println!("创建 {} 条记录: 成功 {} 条, 耗时 {:?}, 平均 {:.1}ms/条",
             test_count, successful, create_time,
             create_time.as_millis() as f64 / test_count as f64);

    // 批量查询测试
    let start = Instant::now();
    let mut found = 0;

    match ModelManager::<User>::find(vec![], None).await {
        Ok(users) => {
            found = users.len();
        }
        Err(e) => println!("批量查询失败: {}", e),
    }

    let query_time = start.elapsed();
    println!("查询 {} 条记录: 找到 {} 条, 耗时 {:?}", test_count, found, query_time);

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== RatQuickDB MySQL 模型操作演示 ===");

    // 初始化日志
    LoggerBuilder::new()
        .with_level(LevelFilter::Warn)  // 减少输出
        .add_terminal_with_config(TermConfig::default())
        .init()?;

    // 初始化数据库
    let db_config = DatabaseConfig::builder()
        .db_type(DatabaseType::MySQL)
        .connection(ConnectionConfig::MySQL {
            host: "172.16.0.21".to_string(),
            port: 3306,
            database: "testdb".to_string(),
            username: "testdb".to_string(),
            password: "testdb123456".to_string(),
            ssl_opts: Default::default(),
            tls_config: None,
        })
        .pool(PoolConfig::builder()
            .max_connections(15)
            .min_connections(3)
            .connection_timeout(15)
            .idle_timeout(120)
            .max_lifetime(2400)
            .max_retries(4)
            .retry_interval_ms(750)
            .keepalive_interval_sec(45)
            .health_check_timeout_sec(8)
            .build()?)
        .alias("main")
        .id_strategy(IdStrategy::Uuid)
        .build()?;

    add_database(db_config).await?;
    println!("数据库连接成功");

    // 清理测试数据
    cleanup_test_data().await;
    println!("清理完成");

    // 执行演示
    let crud_stats = demonstrate_crud().await?;
    let concurrent_stats = demonstrate_concurrency().await?;
    let pagination_stats = demonstrate_pagination().await?;

    performance_benchmark().await?;

    // 输出统计
    println!("\n=== 操作统计 ===");
    println!("CRUD操作: {} 次, 成功率 {:.1}%, 平均 {:.1}ms",
             crud_stats.total_operations, crud_stats.success_rate(), crud_stats.average_time());
    println!("并发操作: {} 次, 成功率 {:.1}%, 平均 {:.1}ms",
             concurrent_stats.total_operations, concurrent_stats.success_rate(), concurrent_stats.average_time());
    println!("分页操作: {} 次, 成功率 {:.1}%, 平均 {:.1}ms",
             pagination_stats.total_operations, pagination_stats.success_rate(), pagination_stats.average_time());

    // 健康检查
    println!("\n=== 健康检查 ===");
    let health = health_check().await;
    for (alias, is_healthy) in health {
        let status = if is_healthy { "✅" } else { "❌" };
        println!("{}: {}", alias, status);
    }

    // 清理
    cleanup_test_data().await;
    println!("\n演示完成");

    Ok(())
}