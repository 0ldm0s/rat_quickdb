//! 测试 SQLite 错误码检测
//!
//! 验证 SQLite 是否支持错误码检测（用于表不存在判断）

#[tokio::test]
#[cfg(feature = "sqlite-support")]
async fn test_sqlite_error_code() {
    rat_logger::LoggerBuilder::new()
        .with_level(rat_logger::LevelFilter::Debug)
        .add_terminal_with_config(rat_logger::handler::term::TermConfig::default())
        .init()
        .expect("日志初始化失败");

    println!("🔍 开始测试 SQLite 错误码检测...");

    // 创建临时数据库
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .connect("sqlite::memory:")
        .await
        .expect("连接 SQLite 失败");

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

                // SQLite 使用扩展错误码
                // SQLITE_ERROR = 1, "no such table"
                if let Some(code_str) = code {
                    if code_str.as_ref() == "1" {
                        println!("   ✅ 错误码正确: 1 (SQLITE_ERROR)");
                        println!("   💡 SQLite 使用错误码，但需要结合消息判断");
                    } else {
                        println!("   ℹ️  错误码: {:?}", code_str);
                    }
                } else {
                    println!("   ⚠️  无法获取错误码");
                }

                // 检查扩展错误码
                println!("   📌 扩展错误码: {:?}", db_err.constraint());
            }
        }
        Ok(_) => {
            println!("   ❌ 查询成功（不符合预期）");
        }
    }

    println!("\n✅ 测试完成！");
}
