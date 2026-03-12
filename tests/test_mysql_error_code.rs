//! 测试 MySQL 错误码检测
//!
//! 验证 MySQL 是否支持错误码检测（用于表不存在判断）

#[tokio::test]
#[cfg(feature = "mysql-support")]
async fn test_mysql_error_code() {
    rat_logger::LoggerBuilder::new()
        .with_level(rat_logger::LevelFilter::Debug)
        .add_terminal_with_config(rat_logger::handler::term::TermConfig::default())
        .init()
        .expect("日志初始化失败");

    println!("🔍 开始测试 MySQL 错误码检测...");

    // 创建 MySQL 连接（使用与 query_operations_mysql.rs 相同的配置）
    let pool = sqlx::mysql::MySqlPoolOptions::new()
        .connect("mysql://testdb:testdb123456@172.16.0.97:3306/testdb")
        .await;

    if let Err(e) = &pool {
        println!("   ⚠️  无法连接到 MySQL: {}", e);
        println!("   💡 跳过 MySQL 测试（如果没有配置数据库）");
        println!("\n✅ 测试跳过！");
        return;
    }

    let pool = pool.unwrap();
    println!("✅ 数据库连接成功");

    // 测试查询不存在的表
    println!("\n📋 测试: 查询不存在的表");
    let fake_table = "this_table_does_not_exist_test";
    let sql = format!("SELECT * FROM {}", fake_table);

    let result = sqlx::query(&sql).fetch_all(&pool).await;

    match result {
        Err(e) => {
            println!("   ✅ 成功捕获错误");

            // 尝试获取数据库错误信息
            if let Some(db_err) = e.as_database_error() {
                let code = db_err.code();
                let message = db_err.message();

                println!("   📌 错误码: {:?}", code);
                println!("   📝 错误消息: {}", message);

                // MySQL 错误码 1146 = Table doesn't exist
                // SQLSTATE: 42S02
                if let Some(code_str) = code {
                    if code_str.as_ref() == "42S02" {
                        println!("   ✅ SQLSTATE 错误码正确: 42S02 (base table or view not found)");
                        println!("   💡 可以使用 code() 检测表不存在");
                    } else if code_str.as_ref() == "1146" {
                        println!("   ✅ MySQL 错误码正确: 1146 (Table doesn't exist)");
                    } else {
                        println!("   ℹ️  错误码: {:?}", code_str);
                    }
                } else {
                    println!("   ⚠️  无法获取错误码");
                }
            }
        }
        Ok(_) => {
            println!("   ❌ 查询成功（不符合预期）");
        }
    }

    println!("\n✅ 测试完成！");
}
