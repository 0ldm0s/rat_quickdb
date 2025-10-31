//! 存储过程创建测试

#[cfg(feature = "sqlite-support")]
use rat_quickdb::*;
#[cfg(feature = "sqlite-support")]
use rat_logger::{debug, LoggerBuilder, LevelFilter, handler::term};

// 用户表
#[cfg(feature = "sqlite-support")]
define_model! {
    struct User {
        id: i32,
        name: String,
        email: String,
    }
    collection = "users",
    database = "test_db",
    fields = {
        id: integer_field(None, None).required().unique(),
        name: string_field(None, None, None).required(),
        email: string_field(None, None, None).required().unique(),
    }
    indexes = [
        { fields: ["email"], unique: true, name: "idx_email" },
    ],
}

// 订单表
#[cfg(feature = "sqlite-support")]
define_model! {
    struct Order {
        id: i32,
        user_id: i32,
        total: f64,
    }
    collection = "orders",
    database = "test_db",
    fields = {
        id: integer_field(None, None).required().unique(),
        user_id: integer_field(None, None).required(),
        total: float_field(None, None).required(),
    }
    indexes = [
        { fields: ["user_id"], unique: false, name: "idx_user_id" },
    ],
}

#[cfg(feature = "sqlite-support")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志系统
    LoggerBuilder::new()
        .with_level(LevelFilter::Info)
        .add_terminal_with_config(term::TermConfig::default())
        .init()?;

    let pool_config = PoolConfig::builder()
        .max_connections(10)
        .min_connections(1)
        .connection_timeout(30)
        .idle_timeout(300)
        .max_lifetime(1800)
        .max_retries(3)
        .retry_interval_ms(1000)
        .keepalive_interval_sec(60)
        .health_check_timeout_sec(10)
        .build()?;

    let db_config = DatabaseConfig::builder()
        .db_type(DatabaseType::SQLite)
        .connection(ConnectionConfig::SQLite {
            path: ":memory:".to_string(),
            create_if_missing: true,
        })
        .pool(pool_config)
        .alias("test_db")
        .id_strategy(IdStrategy::AutoIncrement)
        .build()?;

    add_database(db_config).await?;
    set_default_alias("test_db").await?;

    let config = StoredProcedureConfig::builder("get_users_with_orders", "test_db")
        .with_dependency::<User>()
        .with_dependency::<Order>()
        .with_field("user_id", "users.id")
        .with_field("user_name", "users.name")
        .with_field("user_email", "users.email")
        .with_field("order_count", "COUNT(orders.id)")
        .with_field("total_spent", "SUM(orders.total)")
        .with_join::<Order>("users.id", "orders.user_id", JoinType::Left)
        .build();

    // 0. 准备测试数据
    println!("0. 准备测试数据...");

    // 创建一些测试用户
    let test_users = vec![
        User {
            id: 1,
            name: "张三".to_string(),
            email: "zhangsan@example.com".to_string(),
        },
        User {
            id: 2,
            name: "李四".to_string(),
            email: "lisi@example.com".to_string(),
        },
        User {
            id: 3,
            name: "王五".to_string(),
            email: "wangwu@example.com".to_string(),
        },
    ];

    // 创建一些测试订单
    let test_orders = vec![
        Order {
            id: 1,
            user_id: 1,
            total: 100.50,
        },
        Order {
            id: 2,
            user_id: 1,
            total: 200.75,
        },
        Order {
            id: 3,
            user_id: 2,
            total: 150.00,
        },
    ];

    // 插入测试用户
    for user in &test_users {
        let user_instance = User {
            id: user.id,
            name: user.name.clone(),
            email: user.email.clone(),
        };
        match user_instance.save().await {
            Ok(_) => println!("✅ 创建用户: {} ({})", user.name, user.email),
            Err(e) => println!("❌ 创建用户失败: {}", e),
        }
    }

    // 插入测试订单
    for order in &test_orders {
        let order_instance = Order {
            id: order.id,
            user_id: order.user_id,
            total: order.total,
        };
        match order_instance.save().await {
            Ok(_) => println!("✅ 创建订单: 用户ID={}, 总金额={}", order.user_id, order.total),
            Err(e) => println!("❌ 创建订单失败: {}", e),
        }
    }

    // 1. 创建存储过程
    println!("\n1. 创建存储过程...");
    match ModelManager::<User>::create_stored_procedure(config).await {
        Ok(result) => {
            println!("✅ SQLite存储过程创建成功: {:?}", result);
        },
        Err(e) => {
            println!("❌ SQLite存储过程创建失败: {}", e);
            return Ok(());
        }
    }

    // 2. 执行存储过程（独立操作）
    println!("\n2. 执行存储过程查询...");

    // 无参数查询
    println!("2.1 无参数查询:");
    match ModelManager::<User>::execute_stored_procedure("get_users_with_orders", None).await {
        Ok(results) => {
            println!("✅ 查询成功，返回 {} 条记录", results.len());
            for (i, row) in results.iter().enumerate() {
                if let (Some(user_id), Some(user_name), Some(user_email)) = (
                    row.get("user_id"),
                    row.get("user_name"),
                    row.get("user_email")
                ) {
                    println!("  {}. {} - {} ({})", i+1,
                        user_name.to_string(),
                        user_email.to_string(),
                        user_id.to_string()
                    );
                }
            }
        },
        Err(e) => println!("❌ 查询失败: {}", e),
    }

    // 带参数查询
    println!("\n2.2 带参数查询 (LIMIT 2):");
    let mut params = std::collections::HashMap::new();
    params.insert("LIMIT".to_string(), DataValue::Int(2));

    match ModelManager::<User>::execute_stored_procedure("get_users_with_orders", Some(params)).await {
        Ok(results) => {
            println!("✅ 参数查询成功，返回 {} 条记录", results.len());
            for (i, row) in results.iter().enumerate() {
                if let (Some(user_id), Some(user_name)) = (
                    row.get("user_id"),
                    row.get("user_name")
                ) {
                    println!("  {}. {} ({})", i+1,
                        user_name.to_string(),
                        user_id.to_string()
                    );
                }
            }
        },
        Err(e) => println!("❌ 参数查询失败: {}", e),
    }

    Ok(())
}

#[cfg(not(feature = "sqlite-support"))]
fn main() {
    println!("需要 sqlite-support 特性");
}