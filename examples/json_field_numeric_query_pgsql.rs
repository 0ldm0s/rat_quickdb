//! PostgreSQL JSON 字段数值类型查询示例
//!
//! 专门测试 JSON 字段中数值类型（Int / UInt / Float / Bool）的查询能力。
//! 这些场景使用 `Eq` 操作符 + 数值类型 `DataValue`，内部走
//! `build_json_query_condition()` 生成 `@>` JSONB 包含查询。
//!
//! 覆盖场景：
//! - Int 数值查询（如 age = 28）
//! - Float 数值查询（如 score = 95.5）
//! - Bool 布尔查询（如 is_active = true）
//! - JsonContains 字符串查询（如 {"city": "北京"}）
//! - 组合查询（数值 + 布尔 + 字符串混合）

use rat_logger::{LevelFilter, LoggerBuilder, handler::term::TermConfig};
use rat_quickdb::types::{
    ConnectionConfig, DatabaseType, LogicalOperator, PoolConfig, QueryConditionGroup,
};
use rat_quickdb::*;
use rat_quickdb::{DataValue, ModelManager, ModelOperations, QueryCondition, QueryOperator, json_field};
use serde_json::json;

define_model! {
    struct Sensor {
        id: String,
        name: String,
        metadata: serde_json::Value,
    }
    collection = "json_numeric_test",
    database = "main",
    fields = {
        id: string_field(None, None, None).required().unique(),
        name: string_field(None, None, None).required(),
        metadata: json_field(),
    }
}

fn display_result(index: usize, sensor: &Sensor) {
    let meta = serde_json::to_string_pretty(&sensor.metadata).unwrap_or_default();
    println!("  {}. {} | metadata: {}", index + 1, sensor.name, meta);
}

#[tokio::main]
async fn main() -> QuickDbResult<()> {
    LoggerBuilder::new()
        .with_level(LevelFilter::Warn)
        .add_terminal_with_config(TermConfig::default())
        .init()
        .expect("日志初始化失败");

    println!("=== PostgreSQL JSON 字段数值类型查询测试 ===\n");

    // 配置数据库（builder 模式）
    let db_config = DatabaseConfig::builder()
        .db_type(DatabaseType::PostgreSQL)
        .connection(ConnectionConfig::PostgreSQL {
            host: "172.16.0.96".to_string(),
            port: 5432,
            database: "testdb".to_string(),
            username: "testdb".to_string(),
            password: "testdb".to_string(),
            ssl_mode: Some("prefer".to_string()),
            tls_config: Some(rat_quickdb::types::TlsConfig {
                enabled: true,
                ca_cert_path: None,
                client_cert_path: None,
                client_key_path: None,
                verify_server_cert: false,
                verify_hostname: false,
                min_tls_version: None,
                cipher_suites: None,
            }),
        })
        .pool(
            PoolConfig::builder()
                .max_connections(1)
                .min_connections(1)
                .connection_timeout(30)
                .idle_timeout(300)
                .max_lifetime(3600)
                .max_retries(3)
                .retry_interval_ms(1000)
                .keepalive_interval_sec(60)
                .health_check_timeout_sec(10)
                .build()?,
        )
        .alias("main")
        .id_strategy(IdStrategy::Uuid)
        .build()?;

    add_database(db_config).await?;
    println!("数据库连接成功\n");

    // 清理旧数据
    let _ = drop_table("main", "json_numeric_test").await;

    // 创建测试数据
    let test_data = vec![
        Sensor {
            id: String::new(),
            name: "温度传感器-A".to_string(),
            metadata: json!({
                "temperature": 28,
                "humidity": 65,
                "score": 95.5,
                "is_active": true,
                "location": "北京"
            }),
        },
        Sensor {
            id: String::new(),
            name: "温度传感器-B".to_string(),
            metadata: json!({
                "temperature": 35,
                "humidity": 80,
                "score": 72.0,
                "is_active": true,
                "location": "上海"
            }),
        },
        Sensor {
            id: String::new(),
            name: "温度传感器-C".to_string(),
            metadata: json!({
                "temperature": 28,
                "humidity": 40,
                "score": 88.3,
                "is_active": false,
                "location": "深圳"
            }),
        },
        Sensor {
            id: String::new(),
            name: "压力传感器-D".to_string(),
            metadata: json!({
                "pressure": 101325,
                "unit": "hPa",
                "score": 99.9,
                "is_active": true,
                "location": "北京"
            }),
        },
    ];

    for item in &test_data {
        item.save().await?;
    }
    println!("已创建 {} 条测试数据\n", test_data.len());

    // =========================================================
    // 测试 1: Int 数值查询 — temperature = 28
    // 内部生成: metadata @> $1  参数: DataValue::Json(28)
    // =========================================================
    println!("--- 测试 1: Int 查询 (temperature = 28) ---");
    let results = ModelManager::<Sensor>::find(
        vec![QueryCondition {
            field: "metadata".to_string(),
            operator: QueryOperator::Eq,
            value: DataValue::Int(28),
        }],
        None,
    )
    .await?;
    println!("找到 {} 条:", results.len());
    for (i, r) in results.iter().enumerate() {
        display_result(i, r);
    }
    assert_eq!(results.len(), 2, "应找到 2 条 temperature=28 的记录");

    // =========================================================
    // 测试 2: UInt 数值查询 — pressure = 101325
    // =========================================================
    println!("\n--- 测试 2: UInt 查询 (pressure = 101325) ---");
    let results = ModelManager::<Sensor>::find(
        vec![QueryCondition {
            field: "metadata".to_string(),
            operator: QueryOperator::Eq,
            value: DataValue::UInt(101325),
        }],
        None,
    )
    .await?;
    println!("找到 {} 条:", results.len());
    for (i, r) in results.iter().enumerate() {
        display_result(i, r);
    }
    assert_eq!(results.len(), 1, "应找到 1 条 pressure=101325 的记录");

    // =========================================================
    // 测试 3: Float 数值查询 — score = 95.5
    // =========================================================
    println!("\n--- 测试 3: Float 查询 (score = 95.5) ---");
    let results = ModelManager::<Sensor>::find(
        vec![QueryCondition {
            field: "metadata".to_string(),
            operator: QueryOperator::Eq,
            value: DataValue::Float(95.5),
        }],
        None,
    )
    .await?;
    println!("找到 {} 条:", results.len());
    for (i, r) in results.iter().enumerate() {
        display_result(i, r);
    }
    assert_eq!(results.len(), 1, "应找到 1 条 score=95.5 的记录");

    // =========================================================
    // 测试 4: Bool 布尔查询 — is_active = true
    // =========================================================
    println!("\n--- 测试 4: Bool 查询 (is_active = true) ---");
    let results = ModelManager::<Sensor>::find(
        vec![QueryCondition {
            field: "metadata".to_string(),
            operator: QueryOperator::Eq,
            value: DataValue::Bool(true),
        }],
        None,
    )
    .await?;
    println!("找到 {} 条:", results.len());
    for (i, r) in results.iter().enumerate() {
        display_result(i, r);
    }
    assert_eq!(results.len(), 3, "应找到 3 条 is_active=true 的记录");

    // =========================================================
    // 测试 5: JsonContains 字符串查询 — {"location": "北京"}
    // =========================================================
    println!("\n--- 测试 5: JsonContains 字符串查询 (location = 北京) ---");
    let results = ModelManager::<Sensor>::find(
        vec![QueryCondition {
            field: "metadata".to_string(),
            operator: QueryOperator::JsonContains,
            value: DataValue::String(r#"{"location": "北京"}"#.to_string()),
        }],
        None,
    )
    .await?;
    println!("找到 {} 条:", results.len());
    for (i, r) in results.iter().enumerate() {
        display_result(i, r);
    }
    assert_eq!(results.len(), 2, "应找到 2 条 location=北京 的记录");

    // =========================================================
    // 测试 6: 组合查询 — (temperature = 28) AND (is_active = false)
    // =========================================================
    println!("\n--- 测试 6: 组合查询 (temperature=28 AND is_active=false) ---");
    let group = QueryConditionGroup::Group {
        operator: LogicalOperator::And,
        conditions: vec![
            QueryConditionGroup::Single(QueryCondition {
                field: "metadata".to_string(),
                operator: QueryOperator::Eq,
                value: DataValue::Int(28),
            }),
            QueryConditionGroup::Single(QueryCondition {
                field: "metadata".to_string(),
                operator: QueryOperator::Eq,
                value: DataValue::Bool(false),
            }),
        ],
    };
    let results = ModelManager::<Sensor>::find_with_groups(vec![group], None).await?;
    println!("找到 {} 条:", results.len());
    for (i, r) in results.iter().enumerate() {
        display_result(i, r);
    }
    assert_eq!(results.len(), 1, "应找到 1 条同时满足条件的记录");
    assert_eq!(results[0].name, "温度传感器-C");

    println!("\n=== 全部测试通过 ===");
    Ok(())
}
