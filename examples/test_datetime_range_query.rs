//! 测试DateTime字段的范围查询功能
//!
//! 验证带时区的DateTime字段范围查询是否正常工作

use chrono::{DateTime, Duration, Utc};
use rat_logger::{LevelFilter, LoggerBuilder, handler::term::TermConfig};
use rat_quickdb::manager::health_check;
use rat_quickdb::types::{ConnectionConfig, DatabaseType, PoolConfig};
use rat_quickdb::types::{QueryCondition, QueryOperator};
use rat_quickdb::*;
use rat_quickdb::{ModelManager, ModelOperations, datetime_with_tz_field};

// 定义测试模型
define_model! {
    struct TimeRangeTestModel {
        id: String,
        name: String,
        event_time: chrono::DateTime<chrono::Utc>,
    }
    collection = "time_range_test",
    database = "main",
    fields = {
        id: string_field(None, None, None).required().unique(),
        name: string_field(None, None, None).required(),
        event_time: datetime_field(),  // 普通DateTime字段
    }
}

#[tokio::main]
async fn main() -> QuickDbResult<()> {
    // 初始化日志
    LoggerBuilder::new()
        .add_terminal_with_config(TermConfig::default())
        .init()
        .expect("日志初始化失败");

    println!("🕒 测试DateTime范围查询功能");
    println!("========================\n");

    // 清理之前的测试文件
    cleanup_test_files().await;

    // 1. 配置数据库
    println!("1. 配置SQLite数据库...");
    let db_config = DatabaseConfig {
        alias: "main".to_string(),
        db_type: DatabaseType::SQLite,
        connection: ConnectionConfig::SQLite {
            path: "./time_range_test.db".to_string(),
            create_if_missing: true,
        },
        pool: PoolConfig::builder()
            .max_connections(10)
            .min_connections(1)
            .connection_timeout(10)
            .idle_timeout(300)
            .max_lifetime(1800)
            .max_retries(3)
            .retry_interval_ms(1000)
            .keepalive_interval_sec(60)
            .health_check_timeout_sec(10)
            .build()
            .unwrap(),
        cache: None,
        id_strategy: Default::default(),
    };

    add_database(db_config).await?;
    let health_status = health_check().await;
    if !health_status.get("main").unwrap_or(&false) {
        return Err(QuickDbError::ConnectionError {
            message: "数据库连接失败".to_string(),
        });
    }
    println!("✅ 数据库配置完成\n");

    // 2. 创建测试数据
    println!("2. 创建不同时间点的测试数据...");
    let base_time = Utc::now();

    // 创建5个不同时间点的记录
    for i in 0..5 {
        let event_time = base_time + Duration::hours(i * 2); // 每2小时一个事件
        let model = TimeRangeTestModel {
            id: String::new(),
            name: format!("事件_{}", i + 1),
            event_time,
        };

        match model.save().await {
            Ok(_) => println!(
                "✅ 创建事件_{}: {}",
                i + 1,
                event_time.format("%Y-%m-%d %H:%M:%S UTC")
            ),
            Err(e) => println!("❌ 创建事件_{}失败: {}", i + 1, e),
        }
    }
    println!();

    // 3. 测试范围查询
    println!("3. 测试时间范围查询...");

    // 查询第2个事件到第4个事件之间（4-8小时后）
    let start_time = base_time + Duration::hours(4);
    let end_time = base_time + Duration::hours(8);

    let conditions = vec![
        QueryCondition {
            field: "event_time".to_string(),
            operator: QueryOperator::Gte,
            value: rat_quickdb::types::DataValue::DateTime(start_time),
        },
        QueryCondition {
            field: "event_time".to_string(),
            operator: QueryOperator::Lte,
            value: rat_quickdb::types::DataValue::DateTime(end_time),
        },
    ];

    match ModelManager::<TimeRangeTestModel>::find(conditions, None).await {
        Ok(results) => {
            println!("✅ 范围查询成功，找到 {} 条记录", results.len());
            for model in results {
                println!(
                    "  📋 {}: {}",
                    model.name,
                    model.event_time.format("%Y-%m-%d %H:%M:%S UTC")
                );
            }
        }
        Err(e) => {
            println!("❌ 范围查询失败: {}", e);
        }
    }
    println!();

    // 4. 测试大于查询
    println!("4. 测试大于查询（6小时后）...");
    let after_time = base_time + Duration::hours(6);
    let gt_condition = vec![QueryCondition {
        field: "event_time".to_string(),
        operator: QueryOperator::Gt,
        value: rat_quickdb::types::DataValue::DateTime(after_time),
    }];

    match ModelManager::<TimeRangeTestModel>::find(gt_condition, None).await {
        Ok(results) => {
            println!("✅ 大于查询成功，找到 {} 条记录", results.len());
            for model in results {
                println!(
                    "  📋 {}: {}",
                    model.name,
                    model.event_time.format("%Y-%m-%d %H:%M:%S UTC")
                );
            }
        }
        Err(e) => {
            println!("❌ 大于查询失败: {}", e);
        }
    }
    println!();

    // 5. 测试小于查询
    println!("5. 测试小于查询（4小时前）...");
    let before_time = base_time + Duration::hours(4);
    let lt_condition = vec![QueryCondition {
        field: "event_time".to_string(),
        operator: QueryOperator::Lt,
        value: rat_quickdb::types::DataValue::String(before_time.to_rfc3339()),
    }];

    match ModelManager::<TimeRangeTestModel>::find(lt_condition, None).await {
        Ok(results) => {
            println!("✅ 小于查询成功，找到 {} 条记录", results.len());
            for model in results {
                println!(
                    "  📋 {}: {}",
                    model.name,
                    model.event_time.format("%Y-%m-%d %H:%M:%S UTC")
                );
            }
        }
        Err(e) => {
            println!("❌ 小于查询失败: {}", e);
        }
    }
    println!();

    println!("🎉 DateTime范围查询测试完成！");
    println!("💾 数据库文件已保留: ./time_range_test.db");

    Ok(())
}

async fn cleanup_test_files() {
    let _ = tokio::fs::remove_file("./time_range_test.db").await;
}
