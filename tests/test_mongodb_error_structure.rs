//! 测试 MongoDB 错误类型结构
//!
//! 探索 MongoDB Rust driver 的错误类型，看看是否有类似 SQL 的错误码

#[tokio::test]
#[cfg(feature = "mongodb-support")]
async fn test_mongodb_error_structure() {
    rat_logger::LoggerBuilder::new()
        .with_level(rat_logger::LevelFilter::Debug)
        .add_terminal_with_config(rat_logger::handler::term::TermConfig::default())
        .init()
        .expect("日志初始化失败");

    println!("🔍 开始探索 MongoDB 错误类型结构...");

    // 连接 MongoDB（使用与 query_operations_mongodb.rs 相同的配置）
    let connection_string = "mongodb://testdb:testdb123456@172.16.0.94:27017/testdb?authSource=testdb&directConnection=true";
    let client = mongodb::Client::with_uri_str(connection_string)
        .await
        .expect("连接 MongoDB 失败");

    let db = client.database("testdb");
    println!("✅ 数据库连接成功");

    // 测试查询不存在的集合
    println!("\n📋 测试: 查询不存在的集合");
    let collection: mongodb::Collection<mongodb::bson::Document> = db.collection("this_collection_does_not_exist_test");

    let result = collection.count_documents(None, None).await;

    match result {
        Err(e) => {
            println!("   ✅ 成功捕获错误");
            println!("   📝 错误消息: {}", e);
            println!("   💡 MongoDB 查询不存在的集合通常返回 0，不会抛出错误");
            println!("   💡 只有特定操作（如 drop）才会抛出 NamespaceNotFound 错误");
        }
        Ok(count) => {
            println!("   ℹ️  查询成功，返回计数: {}", count);
            println!("   💡 MongoDB 返回 0 表示集合为空或不存在");
        }
    }

    println!("\n✅ 测试完成！");
    println!("\n📋 结论: MongoDB 的表不存在处理与 SQL 数据库不同");
    println!("   - SQL 数据库: 查询不存在的表会抛出错误（可用错误码检测）");
    println!("   - MongoDB: 查询不存在的集合返回空结果（无需错误检测）");
    println!("   - 当前实现: MongoDB 将空结果视为 TableNotExistError 以保持接口一致性");
}
