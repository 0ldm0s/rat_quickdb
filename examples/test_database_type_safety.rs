//! 数据库类型安全验证测试

use rat_quickdb::*;
use rat_logger::{LoggerBuilder, LevelFilter, handler::term, debug};

#[cfg(feature = "sqlite-support")]
define_model! {
    struct TestUser {
        id: String,
        name: String,
    }
    collection = "test_users",
    database = "test_db",
    fields = {
        id: string_field(None, None, None).required().unique(),
        name: string_field(None, None, None).required(),
    }
    indexes = [],
}

#[cfg(feature = "sqlite-support")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志系统
    LoggerBuilder::new()
        .with_level(LevelFilter::Info)
        .add_terminal_with_config(term::TermConfig::default())
        .init()?;

    // 创建SQLite数据库配置
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
        .alias("sqlite_test")
        .id_strategy(IdStrategy::AutoIncrement)
        .build()?;

    add_database(db_config).await?;

    println!("测试1: SQLite数据库错误使用MongoDB聚合管道API");

    // 测试错误情况：在SQLite中使用MongoDB聚合管道
    let invalid_config = StoredProcedureConfig::builder("test_proc", "sqlite_test")
        .with_dependency::<TestUser>()
        .with_mongo_aggregation()  // 这个方法不应该在SQLite中使用
            .project(vec![
                ("user_id", crate::stored_procedure::types::MongoFieldExpression::field("_id")),
                ("user_name", crate::stored_procedure::types::MongoFieldExpression::field("name")),
            ])
            .build();

    match invalid_config.validate() {
        Ok(_) => println!("❌ 错误：应该阻止SQLite使用MongoDB聚合管道"),
        Err(e) => println!("✅ 正确阻止了错误配置：{}", e),
    }

    println!("\n测试2: SQLite数据库正确使用传统方式");

    // 测试正确情况：在SQLite中使用传统方式
    let valid_config = StoredProcedureConfig::builder("test_proc2", "sqlite_test")
        .with_dependency::<TestUser>()
        .with_field("user_id", "id")
        .with_field("user_name", "name")
        .build();

    match valid_config.validate() {
        Ok(_) => println!("✅ SQLite传统配置验证通过"),
        Err(e) => println!("❌ 意外错误：{}", e),
    }

    Ok(())
}

#[cfg(not(feature = "sqlite-support"))]
fn main() {
    println!("需要 sqlite-support 特性");
}