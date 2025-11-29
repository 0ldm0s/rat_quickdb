//! 测试RFC3339字符串DateTime查询

use rat_quickdb::*;
use rat_quickdb::types::{DatabaseType, ConnectionConfig};
use rat_quickdb::{ModelManager, ModelOperations};
use rat_quickdb::types::{QueryCondition, QueryOperator};

// 定义测试模型
define_model! {
    struct StringDateTimeTestModel {
        id: String,
        name: String,
        event_time: chrono::DateTime<chrono::Utc>,
    }
    collection = "string_datetime_test",
    database = "main",
    fields = {
        id: string_field(None, None, None).required().unique(),
        name: string_field(None, None, None).required(),
        event_time: datetime_field(),
    }
}

#[tokio::main]
async fn main() -> QuickDbResult<()> {
    println!("🧪 测试RFC3339字符串DateTime查询");
    println!("===============================\n");

    // 清理之前的测试文件
    let _ = tokio::fs::remove_file("./string_datetime_test.db").await;

    // 配置数据库
    let db_config = DatabaseConfig {
        alias: "main".to_string(),
        db_type: DatabaseType::SQLite,
        connection: ConnectionConfig::SQLite {
            path: "./string_datetime_test.db".to_string(),
            create_if_missing: true,
        },
        pool: Default::default(),
        cache: None,
        id_strategy: Default::default(),
    };

    add_database(db_config).await?;

    // 创建测试数据
    let base_time = chrono::Utc::now();
    for i in 0..3 {
        let event_time = base_time + chrono::Duration::hours(i * 4);
        let model = StringDateTimeTestModel {
            id: String::new(),
            name: format!("事件_{}", i + 1),
            event_time,
        };

        model.save().await?;
        println!("✅ 创建事件_{}: {}", i + 1, event_time.format("%Y-%m-%d %H:%M:%S UTC"));
    }
    println!();

    // 测试：使用RFC3339字符串查询
    println!("🔍 测试RFC3339字符串范围查询...");
    let start_str = (base_time + chrono::Duration::hours(4)).to_rfc3339();
    let end_str = (base_time + chrono::Duration::hours(8)).to_rfc3339();

    println!("查询范围: {} 到 {}", start_str, end_str);

    let conditions = vec![
        QueryCondition {
            field: "event_time".to_string(),
            operator: QueryOperator::Gte,
            value: rat_quickdb::types::DataValue::String(start_str),
        },
        QueryCondition {
            field: "event_time".to_string(),
            operator: QueryOperator::Lte,
            value: rat_quickdb::types::DataValue::String(end_str),
        },
    ];

    match ModelManager::<StringDateTimeTestModel>::find(conditions, None).await {
        Ok(results) => {
            println!("✅ RFC3339字符串查询成功，找到 {} 条记录", results.len());
            for model in results {
                println!("  📋 {}: {}", model.name, model.event_time.format("%Y-%m-%d %H:%M:%S UTC"));
            }
        },
        Err(e) => println!("❌ RFC3339字符串查询失败: {}", e),
    }

    println!("\n🎉 RFC3339字符串DateTime查询测试完成！");
    println!("💾 数据库文件已保留，可以检查: ./string_datetime_test.db");

    Ok(())
}