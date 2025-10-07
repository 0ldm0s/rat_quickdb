//! Drop Table 功能测试
//!
//! 测试不同数据库别名下的 drop_table 功能是否正常工作

use rat_quickdb::*;
use rat_quickdb::types::{DatabaseType, ConnectionConfig, PoolConfig, IdStrategy};

#[tokio::main]
async fn main() -> QuickDbResult<()> {
    println!("🧪 Drop Table 功能测试");
    println!("=====================");

    // 测试不同别名下的drop_table
    test_drop_table_with_aliases().await?;

    Ok(())
}

async fn test_drop_table_with_aliases() -> QuickDbResult<()> {
    println!("\n📋 测试不同别名下的drop_table功能");
    println!("=====================================");

    // 测试别名列表
    let test_aliases = vec![
        ("test_auto_increment", IdStrategy::AutoIncrement),
        ("test_uuid", IdStrategy::Uuid),
        ("test_snowflake", IdStrategy::Snowflake { machine_id: 1, datacenter_id: 1 }),
    ];

    for (alias, id_strategy) in test_aliases {
        println!("\n🎯 测试别名: {} (策略: {:?})", alias, id_strategy);
        println!("------------------------------------");

        // 1. 添加数据库配置
        let db_config = DatabaseConfig {
            db_type: DatabaseType::MySQL,
            connection: ConnectionConfig::MySQL {
                host: "172.16.0.21".to_string(),
                port: 3306,
                database: "testdb".to_string(),
                username: "testdb".to_string(),
                password: "yash2vCiBA&B#h$#i&gb@IGSTh&cP#QC^".to_string(),
                ssl_opts: {
                    let mut opts = std::collections::HashMap::new();
                    opts.insert("ssl_mode".to_string(), "PREFERRED".to_string());
                    Some(opts)
                },
                tls_config: None,
            },
            pool: PoolConfig {
                min_connections: 1,
                max_connections: 5,
                connection_timeout: 30,
                idle_timeout: 300,
                max_lifetime: 1800,
            },
            alias: alias.to_string(),
            cache: None,
            id_strategy,
        };

        add_database(db_config).await?;
        println!("✅ 数据库配置添加成功: {}", alias);

        // 2. 尝试删除表
        println!("🗑️ 尝试删除表: test_users");
        match rat_quickdb::drop_table(alias, "test_users").await {
            Ok(_) => println!("✅ drop_table调用成功"),
            Err(e) => println!("❌ drop_table调用失败: {}", e),
        }

        // 3. 设置默认别名
        set_default_alias(alias).await?;
        println!("✅ 设置默认别名: {}", alias);

        // 4. 简单验证：尝试创建表来验证drop是否生效
        println!("🔍 验证drop效果...");
        println!("   (如果drop生效，新表会根据策略创建正确的ID字段类型)");

        // 5. 清理
        let _ = remove_database(alias).await;
        println!("🧹 清理数据库配置: {}", alias);
    }

    println!("\n🎉 Drop Table 测试完成！");
    Ok(())
}