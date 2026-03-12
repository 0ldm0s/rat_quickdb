//! 测试 PostgreSQL 错误码检测
//!
//! 本测试验证能否正确获取 PostgreSQL 的错误码（42P01 = UNDEFINED_TABLE）
//! 用于优化 TableNotExistError 的判断逻辑，从字符串匹配改为错误码检测

#[tokio::test]
#[cfg(feature = "postgres-support")]
async fn test_postgresql_error_code_undefined_table() {
    // 初始化日志
    rat_logger::LoggerBuilder::new()
        .with_level(rat_logger::LevelFilter::Debug)
        .add_terminal_with_config(rat_logger::handler::term::TermConfig::default())
        .init()
        .expect("日志初始化失败");

    println!("🔍 开始测试 PostgreSQL 错误码检测...");

    // 直接创建 sqlx PostgreSQL 连接池
    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect("postgres://testdb:testdb@172.16.0.96:5432/testdb?sslmode=prefer")
        .await
        .expect("连接 PostgreSQL 失败");

    println!("✅ 数据库连接成功");

    // 测试1: 尝试查询一个不存在的表，验证错误码
    println!("\n📋 测试1: 查询不存在的表，检查错误码");
    let fake_table = "this_table_does_not_exist_test_12345";
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

                // 验证错误码是否为 42P01 (UNDEFINED_TABLE)
                // sqlx 返回的是 Option<Cow<'_, str>>
                if let Some(code_str) = code {
                    if code_str.as_ref() == "42P01" {
                        println!("   ✅ 错误码正确: 42P01 (UNDEFINED_TABLE)");
                        println!("   💡 可以使用 code() 检测表不存在");
                        println!("   💡 示例代码:");
                        println!("   ```rust");
                        println!("   if let Some(db_err) = e.as_database_error() {{");
                        println!("       if let Some(code) = db_err.code() {{");
                        println!("           if code.as_ref() == \"42P01\" {{");
                        println!("               // 表不存在");
                        println!("           }}");
                        println!("       }}");
                        println!("   }}");
                        println!("   ```");
                    } else {
                        println!("   ⚠️  错误码不是预期的 42P01");
                        println!("   实际错误码: {:?}", code_str);
                    }
                } else {
                    println!("   ❌ 无法获取错误码");
                }
            } else {
                println!("   ❌ 无法获取数据库错误信息");
                println!("   错误类型: {:?}", std::any::type_name::<sqlx::Error>());
                println!("   实际错误: {}", e);
            }
        }
        Ok(_) => {
            println!("   ❌ 查询成功（不符合预期，表不应该存在）");
        }
    }

    // 测试2: 测试其他常见错误码
    println!("\n📋 测试2: 测试其他常见错误码");

    // 测试语法错误（42601）
    let syntax_error_sql = "SELEKT * FROM query_demo_users LIMIT 1"; // SELEKT 是错误的
    let result = sqlx::query(syntax_error_sql).fetch_one(&pool).await;

    match result {
        Err(e) => {
            if let Some(db_err) = e.as_database_error() {
                if let Some(code) = db_err.code() {
                    println!("   语法错误码: {:?}", code);
                    println!("   错误消息: {}", db_err.message());
                }
            }
        }
        Ok(_) => {
            println!("   ❌ 查询成功（不符合预期）");
        }
    }

    println!("\n✅ 测试完成！");
}
