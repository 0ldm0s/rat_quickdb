//! 测试DateTime字段是否能接受String输入

use rat_quickdb::*;
use rat_quickdb::types::{DatabaseType, ConnectionConfig};
use rat_quickdb::{ModelManager, ModelOperations};

// 定义测试模型
define_model! {
    struct StringInputTestModel {
        id: String,
        name: String,
        event_time: chrono::DateTime<chrono::Utc>,
    }
    collection = "string_input_test",
    database = "main",
    fields = {
        id: string_field(None, None, None).required().unique(),
        name: string_field(None, None, None).required(),
        event_time: datetime_field(),
    }
}

#[tokio::main]
async fn main() -> QuickDbResult<()> {
    // 配置数据库
    let db_config = DatabaseConfig {
        alias: "main".to_string(),
        db_type: DatabaseType::SQLite,
        connection: ConnectionConfig::SQLite {
            path: "./string_input_test.db".to_string(),
            create_if_missing: true,
        },
        pool: Default::default(),
        cache: None,
        id_strategy: Default::default(),
    };

    add_database(db_config).await?;

    // 测试1: 尝试传入String类型的DateTime字段
    println!("测试1: 传入String类型到DateTime字段");
    let model = StringInputTestModel {
        id: String::new(),
        name: "测试String输入".to_string(),
        event_time: "2025-11-28T12:00:00Z".to_string(),  // 这里传入String而不是DateTime<Utc>
    };

    match model.save().await {
        Ok(id) => println!("✅ 成功保存，ID: {}", id),
        Err(e) => println!("❌ 保存失败: {}", e),
    }

    Ok(())
}