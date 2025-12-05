//! 测试读取数据库中的时间戳数据

use chrono::{DateTime, Utc};
use rat_quickdb::ModelOperations;
use rat_quickdb::manager::shutdown;
use rat_quickdb::types::QueryCondition;
use rat_quickdb::*;

#[tokio::main]
async fn main() -> QuickDbResult<()> {
    rat_quickdb::init();

    // 创建数据库配置，使用已有的数据库文件
    let config = DatabaseConfig::builder()
        .db_type(DatabaseType::SQLite)
        .connection(ConnectionConfig::SQLite {
            path: "./timezone_range_demo.db".to_string(),
            create_if_missing: false, // 使用已有的数据库
        })
        .pool(
            PoolConfig::builder()
                .min_connections(1)
                .max_connections(5)
                .build()?,
        )
        .alias("default".to_string())
        .id_strategy(IdStrategy::Uuid)
        .build()?;

    add_database(config).await?;

    println!("=== 测试读取user_activities表中的数据 ===");

    // 使用正确的模型查询
    match ModelManager::<UserActivity>::find(vec![], None).await {
        Ok(activities) => {
            println!("✅ 查询成功！找到 {} 条记录", activities.len());

            for (i, activity) in activities.iter().enumerate() {
                println!("记录 {}:", i + 1);
                println!("  id: {}", activity.id);
                println!("  username: {}", activity.username);
                println!("  beijing_time: {}", activity.beijing_time);
                println!("  duration_minutes: {}", activity.duration_minutes);
                println!("  created_at: {}", activity.created_at);
                println!();
            }
        }
        Err(e) => {
            println!("❌ 查询失败: {}", e);
        }
    }

    shutdown().await?;
    Ok(())
}

// 使用与示例中相同的模型定义
define_model! {
    /// 用户活动日志模型
    struct UserActivity {
        id: String,
        user_id: String,
        username: String,
        activity_type: String,
        description: String,
        beijing_time: String,
        duration_minutes: i32,
        ip_address: String,
        created_at: String,
    }
    collection = "user_activities",
    database = "default",
    fields = {
        id: string_field(None, None, None).required().unique(),
        user_id: string_field(None, None, None).required(),
        username: string_field(Some(100), Some(1), None).required(),
        activity_type: string_field(Some(50), Some(1), None).required(),
        description: string_field(Some(500), Some(1), None).required(),
        beijing_time: datetime_field(Some("+08:00".to_string())).required(),
        duration_minutes: integer_field(Some(0), Some(1440)).required(),
        ip_address: string_field(Some(45), Some(1), None).required(),
        created_at: datetime_field(Some("+00:00".to_string())).required(),
    }
    indexes = [
        { fields: ["user_id"], unique: false, name: "idx_user_id" },
        { fields: ["beijing_time"], unique: false, name: "idx_beijing_time" },
        { fields: ["activity_type"], unique: false, name: "idx_activity_type" },
        { fields: ["user_id", "beijing_time"], unique: false, name: "idx_user_time" },
    ],
}
