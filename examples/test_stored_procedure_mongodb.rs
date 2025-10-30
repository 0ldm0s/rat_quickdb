//! MongoDB存储过程创建测试

#[cfg(feature = "mongodb-support")]
use rat_quickdb::*;
#[cfg(feature = "mongodb-support")]
use rat_logger::{debug, LoggerBuilder, LevelFilter, handler::term};

// 用户集合
#[cfg(feature = "mongodb-support")]
define_model! {
    struct User {
        id: String,
        username: String,
        email: String,
    }
    collection = "users",
    database = "test_db",
    fields = {
        id: string_field(None, None, None).required().unique(),
        username: string_field(None, None, None).required(),
        email: string_field(None, None, None).required().unique(),
    }
    indexes = [
        { fields: ["email"], unique: true, name: "idx_email" },
    ],
}

// 订单集合
#[cfg(feature = "mongodb-support")]
define_model! {
    struct Order {
        id: String,
        user_id: String,
        total: f64,
    }
    collection = "orders",
    database = "test_db",
    fields = {
        id: string_field(None, None, None).required().unique(),
        user_id: string_field(None, None, None).required(),
        total: float_field(None, None).required(),
    }
    indexes = [
        { fields: ["user_id"], unique: false, name: "idx_user_id" },
    ],
}

#[cfg(feature = "mongodb-support")]
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
        .db_type(DatabaseType::MongoDB)
        .connection(ConnectionConfig::MongoDB {
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
        })
        .pool(pool_config)
        .alias("test_db")
        .id_strategy(IdStrategy::Uuid)
        .build()?;

    add_database(db_config).await?;
    set_default_alias("test_db").await?;

    let config = StoredProcedureConfig::builder("get_users_with_orders", "test_db")
        .with_dependency::<User>()
        .with_dependency::<Order>()
        .with_mongo_aggregation()
            .project(vec![
                ("user_id", crate::stored_procedure::types::MongoFieldExpression::field("_id")),
                ("user_name", crate::stored_procedure::types::MongoFieldExpression::field("username")),
                ("user_email", crate::stored_procedure::types::MongoFieldExpression::field("email")),
            ])
            .lookup("orders", "_id", "user_id", "orders_joined")
            .unwind("orders_joined")
            .group(
                crate::stored_procedure::types::MongoGroupKey::Field("_id".to_string()),
                vec![
                    ("user_id", crate::stored_procedure::types::MongoAccumulator::Push { field: "_id".to_string() }),
                    ("user_name", crate::stored_procedure::types::MongoAccumulator::Push { field: "username".to_string() }),
                    ("user_email", crate::stored_procedure::types::MongoAccumulator::Push { field: "email".to_string() }),
                    ("order_count", crate::stored_procedure::types::MongoAccumulator::Count),
                    ("total_spent", crate::stored_procedure::types::MongoAccumulator::Sum { field: "total".to_string() }),
                ],
            )
            .add_fields(vec![
                ("order_count", crate::stored_procedure::types::MongoFieldExpression::if_null(
                    "orders_joined",
                    crate::stored_procedure::types::MongoFieldExpression::constant(crate::types::DataValue::Int(0))
                )),
            ])
            .project(vec![
                ("user_id", crate::stored_procedure::types::MongoFieldExpression::field("user_id")),
                ("user_name", crate::stored_procedure::types::MongoFieldExpression::field("user_name")),
                ("user_email", crate::stored_procedure::types::MongoFieldExpression::field("user_email")),
                ("order_count", crate::stored_procedure::types::MongoFieldExpression::field("order_count")),
                ("total_spent", crate::stored_procedure::types::MongoFieldExpression::field("total_spent")),
            ])
            .with_common_placeholders()  // 添加常用占位符
            .build();

    match create_stored_procedure(config).await {
        Ok(result) => println!("结果: {:?}", result),
        Err(e) => println!("错误: {}", e),
    }

    Ok(())
}

#[cfg(not(feature = "mongodb-support"))]
fn main() {
    println!("需要 mongodb-support 特性");
}