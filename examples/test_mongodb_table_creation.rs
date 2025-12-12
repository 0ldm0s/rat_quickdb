//! 测试MongoDB集合创建行为和TableNotExistError

use chrono::Utc;
use rat_logger::{LevelFilter, LoggerBuilder, handler::term::TermConfig};
use rat_quickdb::types::*;
use rat_quickdb::*;
use rat_quickdb::{
    ModelManager, ModelOperations, string_field, uuid_field,
};

// 数据库别名常量
const DATABASE_ALIAS: &str = "main";

// 用户模型
define_model! {
    struct User {
        id: String,
        username: String,
    }
    collection = "users",
    database = DATABASE_ALIAS,
    fields = {
        id: uuid_field().required().unique(),
        username: string_field(None, None, None).required(),
    }
}

// 测试MongoDB集合创建行为
async fn test_collection_creation() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== 测试MongoDB集合创建行为和TableNotExistError ===");

    // 1. 测试查询不存在的集合
    println!("1. 测试查询不存在的集合...");
    match ModelManager::<User>::find_by_id("00000000-0000-0000-0000-000000000000").await {
        Err(QuickDbError::TableNotExistError { table, message }) => {
            println!("   ✅ 成功识别表不存在错误:");
            println!("     表名: {}", table);
            println!("     错误信息: {}", message);
        }
        Err(e) => {
            println!("   ⚠️  识别到其他错误: {}", e);
        }
        Ok(result) => {
            println!("   ℹ️  查询成功，返回: {:?}", result);
            println!("   ⚠️  MongoDB查询不存在的集合返回None而不是错误");
        }
    }

    // 2. 创建一个空集合（先插入数据，再删除）
    println!("\n2. 创建空集合用于对比测试...");
    let user = User {
        id: String::new(),
        username: "temp_user".to_string(),
    };

    match user.save().await {
        Ok(id) => {
            println!("   ✅ 临时数据插入成功: {}", id);

            // 删除刚插入的数据，留下空集合
            if let Ok(Some(saved_user)) = ModelManager::<User>::find_by_id(&id).await {
                match saved_user.delete().await {
                    Ok(_) => println!("   ✅ 临时数据已删除，留下空集合"),
                    Err(e) => println!("   ⚠️  删除临时数据失败: {}", e),
                }
            }
        }
        Err(e) => {
            println!("   ❌ 创建临时数据失败: {}", e);
            return Ok(());
        }
    }

    // 3. 测试查询存在的空集合
    println!("\n3. 测试查询存在的空集合...");
    match ModelManager::<User>::find_by_id("00000000-0000-0000-0000-000000000000").await {
        Err(QuickDbError::TableNotExistError { table, message }) => {
            println!("   ✅ 符合预期：空集合也返回TableNotExistError");
            println!("     表名: {}", table);
            println!("     错误信息: {}", message);
        }
        Err(e) => {
            println!("   ⚠️  识别到其他错误: {}", e);
        }
        Ok(result) => {
            println!("   ℹ️  查询成功，返回: {:?}", result);
            println!("   ⚠️  这种情况不应该发生");
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== MongoDB集合创建行为测试 ===");

    // 初始化日志
    LoggerBuilder::new()
        .with_level(LevelFilter::Warn)
        .add_terminal_with_config(TermConfig::default())
        .init()?;

    // 初始化数据库
    // TLS配置
    let tls_config = rat_quickdb::types::TlsConfig {
        enabled: true,
        ca_cert_path: None,
        client_cert_path: None,
        client_key_path: None,
        verify_server_cert: false,
        verify_hostname: false,
        min_tls_version: Some("1.2".to_string()),
        cipher_suites: None,
    };

    // ZSTD压缩配置
    let zstd_config = rat_quickdb::types::ZstdConfig {
        enabled: true,
        compression_level: Some(3),
        compression_threshold: Some(1024),
    };

    let db_config = DatabaseConfig::builder()
        .db_type(DatabaseType::MongoDB)
        .connection(ConnectionConfig::MongoDB {
            host: "db0.0ldm0s.net".to_string(),
            port: 27017,
            database: "testdb".to_string(),
            username: Some("testdb".to_string()),
            password: Some("testdb123456".to_string()),
            auth_source: Some("testdb".to_string()),
            direct_connection: true,
            options: None,
            tls_config: Some(tls_config),
            zstd_config: Some(zstd_config),
        })
        .pool(
            PoolConfig::builder()
                .max_connections(25)
                .min_connections(5)
                .connection_timeout(10)
                .idle_timeout(30)
                .max_lifetime(1200)
                .max_retries(6)
                .retry_interval_ms(250)
                .keepalive_interval_sec(20)
                .health_check_timeout_sec(3)
                .build()?,
        )
        .alias(DATABASE_ALIAS)
        .id_strategy(IdStrategy::Uuid)
        .build()?;

    add_database(db_config).await?;
    println!("数据库连接成功");

    // 执行测试
    test_collection_creation().await?;

    println!("\n测试完成");

    Ok(())
}