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
        .connection_timeout(5000)
        .idle_timeout(300000)
        .max_lifetime(1800000)
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
        .with_dependency("users")
        .with_dependency("orders")
        .with_field("user_id", "users.id")
        .with_field("user_name", "users.name")
        .with_field("user_email", "users.email")
        .with_field("order_count", "COUNT(orders.id)")
        .with_field("total_spent", "SUM(orders.total)")
        .with_join("orders", "users.id", "orders.user_id", JoinType::Left)
        .build();

    match create_stored_procedure(config).await {
        Ok(result) => println!("结果: {:?}", result),
        Err(e) => println!("错误: {}", e),
    }

    Ok(())
}

#[cfg(not(feature = "sqlite-support"))]
fn main() {
    println!("需要 sqlite-support 特性");
}