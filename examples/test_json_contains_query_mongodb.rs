//! MongoDB JSON 字段 JsonContains 查询功能测试示例
//!
//! 测试 JSON 字段的存储和 JsonContains 查询功能

use rat_quickdb::*;
use rat_quickdb::types::{DatabaseType, ConnectionConfig, PoolConfig, QueryConditionGroup, LogicalOperator, QueryOptions, SortConfig, SortDirection};
use rat_quickdb::model::FieldType;
use rat_quickdb::manager::health_check;
use rat_quickdb::{ModelManager, ModelOperations, QueryCondition, QueryOperator, DataValue, json_field, field_types};
use rat_logger::{LoggerBuilder, LevelFilter, handler::term::TermConfig};
use serde_json::json;

/// 显示结果的详细信息，包括JSON字段的JSON格式
fn display_json_test_result(index: usize, result: &JsonTestModel) {
    // 将JSON字段转换为JSON字符串显示
    let profile_json = serde_json::to_string_pretty(&result.profile).unwrap_or_else(|_| "null".to_string());
    let settings_json = serde_json::to_string_pretty(&result.settings).unwrap_or_else(|_| "null".to_string());

    println!("  {}. {}", index + 1, result.name);
    println!("     profile: {}", profile_json);
    println!("     settings: {}", settings_json);
    println!();
}

// 定义测试模型
define_model! {
    /// JSON 字段测试模型
    struct JsonTestModel {
        id: String,
        name: String,
        profile: serde_json::Value,    // 用户配置JSON
        settings: serde_json::Value,   // 应用设置JSON
    }
    collection = "json_test",
    database = "main",
    fields = {
        id: string_field(None, None, None).required().unique(),
        name: string_field(None, None, None).required(),
        profile: json_field(),
        settings: json_field(),
    }
}

#[tokio::main]
async fn main() -> QuickDbResult<()> {
    // 初始化日志
    LoggerBuilder::new()
        .add_terminal_with_config(TermConfig::default())
        .init()
        .expect("日志初始化失败");

    println!("🚀 测试 MongoDB JSON 字段 JsonContains 查询功能");
    println!("==========================================\n");

    // 1. 配置数据库
    println!("1. 配置MongoDB数据库...");
    let db_config = DatabaseConfig {
        alias: "main".to_string(),
        db_type: DatabaseType::MongoDB,
        connection: ConnectionConfig::MongoDB {
            host: "db0.0ldm0s.net".to_string(),
            port: 27017,
            database: "testdb".to_string(),
            username: Some("testdb".to_string()),
            password: Some("testdb123456".to_string()),
            auth_source: Some("testdb".to_string()),
            direct_connection: true,
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
            zstd_config: Some(rat_quickdb::types::ZstdConfig {
                enabled: true,
                compression_level: Some(3),
                compression_threshold: Some(1024),
            }),
            options: {
                let mut opts = std::collections::HashMap::new();
                opts.insert("retryWrites".to_string(), "true".to_string());
                opts.insert("w".to_string(), "majority".to_string());
                Some(opts)
            },
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
        id_strategy: IdStrategy::ObjectId,
        cache: None,
    };

    // 添加数据库配置
    add_database(db_config).await?;
    println!("✓ MongoDB数据库配置完成");

    // 清理之前的测试数据
    println!("\n清理之前的测试数据...");
    match drop_table("main", "json_test").await {
        Ok(_) => println!("✓ 清理完成"),
        Err(e) => println!("注意: 清理失败或表不存在: {}", e),
    }

    // 2. 创建测试数据
    println!("\n2. 创建测试数据...");
    let test_data = vec![
        JsonTestModel {
            id: generate_object_id(),
            name: "张三".to_string(),
            profile: json!({
                "name": "张三",
                "age": 28,
                "city": "北京",
                "hobbies": ["读书", "游泳", "编程"],
                "contact": {
                    "email": "zhangsan@example.com",
                    "phone": "13800138001"
                }
            }),
            settings: json!({
                "theme": "dark",
                "language": "zh-CN",
                "notifications": {
                    "email": true,
                    "sms": false,
                    "push": true
                },
                "features": {
                    "auto_save": true,
                    "analytics": false
                }
            }),
        },
        JsonTestModel {
            id: generate_object_id(),
            name: "李四".to_string(),
            profile: json!({
                "name": "李四",
                "age": 32,
                "city": "上海",
                "hobbies": ["旅行", "摄影", "美食"],
                "contact": {
                    "email": "lisi@example.com",
                    "phone": "13800138002"
                }
            }),
            settings: json!({
                "theme": "light",
                "language": "zh-CN",
                "notifications": {
                    "email": false,
                    "sms": true,
                    "push": true
                },
                "features": {
                    "auto_save": true,
                    "analytics": true
                }
            }),
        },
        JsonTestModel {
            id: generate_object_id(),
            name: "王五".to_string(),
            profile: json!({
                "name": "王五",
                "age": 25,
                "city": "深圳",
                "hobbies": ["游戏", "音乐", "运动"],
                "contact": {
                    "email": "wangwu@example.com",
                    "phone": "13800138003"
                }
            }),
            settings: json!({
                "theme": "dark",
                "language": "en-US",
                "notifications": {
                    "email": true,
                    "sms": true,
                    "push": false
                },
                "features": {
                    "auto_save": false,
                    "analytics": true
                }
            }),
        },
        JsonTestModel {
            id: generate_object_id(),
            name: "赵六".to_string(),
            profile: json!({
                "name": "赵六",
                "age": 30,
                "city": "广州",
                "hobbies": ["编程", "阅读", "写作"],
                "contact": {
                    "email": "zhaoliu@example.com",
                    "phone": "13800138004"
                }
            }),
            settings: json!({
                "theme": "light",
                "language": "zh-CN",
                "notifications": {
                    "email": true,
                    "sms": true,
                    "push": true
                },
                "features": {
                    "auto_save": true,
                    "analytics": false
                }
            }),
        },
    ];

    for (i, item) in test_data.iter().enumerate() {
        match item.save().await {
            Ok(_) => println!("✓ 创建测试数据 {}: {}", i + 1, item.name),
            Err(e) => {
                eprintln!("❌ 创建测试数据失败 {}: {}", i + 1, e);
                return Err(e);
            }
        }
    }

    // 3. JSON 字段 JsonContains 查询测试

    // 测试1: 在 profile 中查找特定城市
    println!("\n3.1 查找 profile 中城市为 '北京' 的用户:");
    match ModelManager::<JsonTestModel>::find(
        vec![QueryCondition {
            field: "profile".to_string(),
            operator: QueryOperator::JsonContains,
            value: DataValue::String(r#"{"city": "北京"}"#.to_string()),
        }],
        None,
    ).await {
        Ok(results) => {
            println!("✓ 找到 {} 个用户:", results.len());
            for (i, result) in results.iter().enumerate() {
                display_json_test_result(i, result);
            }
        },
        Err(e) => {
            eprintln!("❌ 查询失败: {}", e);
        }
    }

    // 测试2: 在 settings 中查找特定功能设置
    println!("\n3.2 查找 settings 中启用了自动保存的用户:");
    match ModelManager::<JsonTestModel>::find(
        vec![QueryCondition {
            field: "settings".to_string(),
            operator: QueryOperator::JsonContains,
            value: DataValue::String(r#"{"features": {"auto_save": true}}"#.to_string()),
        }],
        None,
    ).await {
        Ok(results) => {
            println!("✓ 找到 {} 个用户:", results.len());
            for (i, result) in results.iter().enumerate() {
                display_json_test_result(i, result);
            }
        },
        Err(e) => {
            eprintln!("❌ 查询失败: {}", e);
        }
    }

  
    // 4. 复杂 JSON 查询测试

    // 测试4: 复杂组合查询 - (profile城市为'北京' OR profile城市为'上海') AND (settings主题为'dark')
    println!("\n4.1 复杂组合查询: (城市为'北京'或'上海') AND (主题为'dark')");
    let complex_condition = QueryConditionGroup::Group {
        operator: LogicalOperator::And,
        conditions: vec![
            // 城市条件组 (OR)
            QueryConditionGroup::Group {
                operator: LogicalOperator::Or,
                conditions: vec![
                    QueryConditionGroup::Single(QueryCondition {
                        field: "profile".to_string(),
                        operator: QueryOperator::JsonContains,
                        value: DataValue::String(r#"{"city": "北京"}"#.to_string()),
                    }),
                    QueryConditionGroup::Single(QueryCondition {
                        field: "profile".to_string(),
                        operator: QueryOperator::JsonContains,
                        value: DataValue::String(r#"{"city": "上海"}"#.to_string()),
                    }),
                ],
            },
            // 主题条件 (Single)
            QueryConditionGroup::Single(QueryCondition {
                field: "settings".to_string(),
                operator: QueryOperator::JsonContains,
                value: DataValue::String(r#"{"theme": "dark"}"#.to_string()),
            }),
        ],
    };

    match ModelManager::<JsonTestModel>::find_with_groups(
        vec![complex_condition],
        None,
    ).await {
        Ok(results) => {
            println!("✓ 找到 {} 个用户:", results.len());
            for (i, result) in results.iter().enumerate() {
                display_json_test_result(i, result);
            }
        },
        Err(e) => {
            eprintln!("❌ 复杂查询失败: {}", e);
        }
    }

    println!("\n✅ JSON 字段 JsonContains 查询测试完成！");
    println!("🗄️ MongoDB数据库集合: json_test（可用于验证数据正确性）");

    Ok(())
}