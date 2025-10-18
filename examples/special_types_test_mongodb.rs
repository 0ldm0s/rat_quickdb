//! MongoDB 特殊字段类型验证示例
//!
//! 验证MongoDB适配器对特殊字段类型的处理，包括：
//! - 时间戳字段的null值处理（模拟cleanup.rs遇到的问题）
//! - 数组类型（原生数组支持）
//! - JSON类型（BSON）
//! - 其他复杂类型的反序列化
//!
//! 使用正确的ODM（Object-Document Mapper）方式进行测试

use rat_quickdb::*;
use rat_quickdb::types::{QueryCondition, QueryConditionGroup, LogicalOperator, QueryOperator, DataValue, QueryOptions, SortConfig, SortDirection, PaginationConfig};
use rat_quickdb::manager::shutdown;
use rat_quickdb::{ModelOperations, string_field, integer_field, float_field, datetime_field, boolean_field, json_field, array_field, FieldType};
use std::collections::HashMap;
use chrono::{Utc, DateTime, Duration};
use rand::Rng;
use rat_logger::{LoggerBuilder, handler::term::TermConfig, debug};
use serde_json::{json, Value};

// 定义特殊字段测试模型
define_model! {
    /// 特殊字段测试模型 - 用于验证MongoDB特殊字段类型的处理
    struct SpecialFieldsTest {
        id: String,
        title: String,
        description: Option<String>,
        expires_at: Option<chrono::DateTime<chrono::Utc>>,  // 可能null的时间戳字段
        read_at: Option<chrono::DateTime<chrono::Utc>>,    // 可能null的时间戳字段
        is_read: bool,
        tags: Vec<String>,                    // 原生数组类型
        metadata: serde_json::Value,          // BSON类型
        priority: i32,
        created_at: chrono::DateTime<chrono::Utc>,
        updated_at: Option<chrono::DateTime<chrono::Utc>>,
    }
    collection = "special_fields_test",
    fields = {
        id: string_field(None, None, None).required().unique(),
        title: string_field(Some(200), Some(1), None).required(),
        description: string_field(Some(1000), Some(0), None),
        expires_at: datetime_field(),  // 可选的时间戳字段
        read_at: datetime_field(),    // 可选的时间戳字段
        is_read: boolean_field().required(),
        tags: array_field(
            FieldType::String {
                max_length: Some(100),
                min_length: Some(0),
                regex: None,
            },
            Some(50), // 最多50个标签
            None,    // 可以为空数组
        ),  // 数组字段
        metadata: json_field(),       // JSON字段
        priority: integer_field(Some(1), Some(10)).required(),
        created_at: datetime_field().required(),
        updated_at: datetime_field(),
    }
    indexes = [
        { fields: ["expires_at"], unique: false, name: "idx_expires_at" },
        { fields: ["read_at"], unique: false, name: "idx_read_at" },
        { fields: ["is_read"], unique: false, name: "idx_is_read" },
        { fields: ["priority"], unique: false, name: "idx_priority" },
        { fields: ["expires_at", "is_read"], unique: false, name: "idx_expires_read" },
    ],
}

#[tokio::main]
async fn main() -> QuickDbResult<()> {
    // 初始化日志系统
    LoggerBuilder::new()
        .add_terminal_with_config(TermConfig::default())
        .init()
        .expect("日志初始化失败");

    rat_quickdb::init();
    println!("=== MongoDB 特殊字段类型验证示例 ===");

    // 创建数据库配置 - 从model_definition_mongodb.rs复制
    let config = DatabaseConfig {
        db_type: DatabaseType::MongoDB,
        connection: ConnectionConfig::MongoDB {
            host: "db0.0ldm0s.net".to_string(),
            port: 27017,
            database: "testdb".to_string(),
            username: Some("testdb".to_string()),
            password: Some("yash2vCiBA&B#h$#i&gb@IGSTh&cP#QC^".to_string()),
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
        pool: PoolConfig {
            min_connections: 2,
            max_connections: 10,
            connection_timeout: 30,
            idle_timeout: 300,
            max_lifetime: 1800,
        },
        alias: "mongodb_default".to_string(),
        cache: None,
        id_strategy: IdStrategy::Uuid,
    };

    // 初始化数据库
    add_database(config).await?;

    // 设置默认数据库别名
    rat_quickdb::set_default_alias("mongodb_default").await?;

    // 清理旧的测试表（确保每次都是干净的状态）
    println!("清理旧的测试表...");
    match drop_table("mongodb_default", "special_fields_test").await {
        Ok(_) => println!("✅ 已清理旧的special_fields_test表"),
        Err(e) => println!("   注意：清理表失败（可能表不存在）: {}", e),
    }

    // 创建测试表
    create_test_table().await?;

    // 插入测试数据（包含极端场景）
    insert_test_data_with_extreme_scenarios().await?;

    println!("\n=== 开始特殊字段类型验证测试 ===\n");

    // 测试1: 验证null时间戳字段的反序列化（模拟cleanup.rs遇到的问题）
    println!("1. 验证null时间戳字段的反序列化（模拟cleanup.rs遇到的问题）");

    let null_timestamp_condition = QueryConditionGroup::Group {
        operator: LogicalOperator::And,
        conditions: vec![
            QueryConditionGroup::Single(QueryCondition {
                field: "expires_at".to_string(),
                operator: QueryOperator::IsNull,
                value: DataValue::Null,
            }),
        ],
    };

    match ModelManager::<SpecialFieldsTest>::find_with_groups(
        vec![null_timestamp_condition],
        None,
    ).await {
        Ok(records) => {
            println!("   ✅ 成功查询到 {} 个expires_at为null的记录", records.len());
            for record in &records {
                println!("   - 记录ID: {}, 标题: {}, expires_at: {:?}, read_at: {:?}",
                    record.id, record.title, record.expires_at, record.read_at);
            }
        },
        Err(e) => {
            println!("   ❌ 查询null时间戳记录失败: {}", e);
            println!("   这表明时间戳null值反序列化存在问题");
        }
    }

    println!();

    // 测试2: 验证非null时间戳字段的反序列化
    println!("2. 验证非null时间戳字段的反序列化");

    let not_null_timestamp_condition = QueryConditionGroup::Group {
        operator: LogicalOperator::And,
        conditions: vec![
            QueryConditionGroup::Single(QueryCondition {
                field: "expires_at".to_string(),
                operator: QueryOperator::IsNotNull,
                value: DataValue::Null,
            }),
        ],
    };

    match ModelManager::<SpecialFieldsTest>::find_with_groups(
        vec![not_null_timestamp_condition],
        None,
    ).await {
        Ok(records) => {
            println!("   ✅ 成功查询到 {} 个expires_at不为null的记录", records.len());
            for record in &records {
                if let Some(expires_at) = record.expires_at {
                    println!("   - 记录ID: {}, 标题: {}, expires_at: {}",
                        record.id, record.title, expires_at.format("%Y-%m-%d %H:%M:%S UTC"));
                }
            }
        },
        Err(e) => {
            println!("   ❌ 查询非null时间戳记录失败: {}", e);
        }
    }

    println!();

    // 测试3: 验证数组字段的处理
    println!("3. 验证数组字段的处理");

    let array_condition = QueryConditionGroup::Group {
        operator: LogicalOperator::And,
        conditions: vec![
            QueryConditionGroup::Single(QueryCondition {
                field: "tags".to_string(),
                operator: QueryOperator::IsNotNull,
                value: DataValue::Null,
            }),
        ],
    };

    match ModelManager::<SpecialFieldsTest>::find_with_groups(
        vec![array_condition],
        None,
    ).await {
        Ok(records) => {
            println!("   ✅ 成功查询到 {} 个有tags数组的记录", records.len());
            for record in &records {
                println!("   - 记录ID: {}, 标题: {}, tags数量: {}",
                    record.id, record.title, record.tags.len());
                println!("     标签: {}", record.tags.join(", "));
            }
        },
        Err(e) => {
            println!("   ❌ 查询数组记录失败: {}", e);
            println!("   这表明MongoDB数组类型处理存在问题");
        }
    }

    println!();

    // 测试4: 验证JSON字段的处理
    println!("4. 验证JSON字段的处理");

    let json_condition = QueryConditionGroup::Group {
        operator: LogicalOperator::And,
        conditions: vec![
            QueryConditionGroup::Single(QueryCondition {
                field: "metadata".to_string(),
                operator: QueryOperator::IsNotNull,
                value: DataValue::Null,
            }),
        ],
    };

    match ModelManager::<SpecialFieldsTest>::find_with_groups(
        vec![json_condition],
        None,
    ).await {
        Ok(records) => {
            println!("   ✅ 成功查询到 {} 个有metadata JSON的记录", records.len());
            for record in &records {
                println!("   - 记录ID: {}, 标题: {}, metadata: {}",
                    record.id, record.title, record.metadata);
            }
        },
        Err(e) => {
            println!("   ❌ 查询JSON记录失败: {}", e);
            println!("   这表明JSON类型处理存在问题");
        }
    }

    println!();

    // 测试5: 验证复杂的嵌套查询（类似cleanup.rs的场景）
    println!("5. 验证复杂的嵌套查询（类似cleanup.rs的场景）");

    let complex_condition = QueryConditionGroup::Group {
        operator: LogicalOperator::And,
        conditions: vec![
            // 模拟cleanup.rs的查询条件：expires_at不为null且小于当前时间
            QueryConditionGroup::Single(QueryCondition {
                field: "expires_at".to_string(),
                operator: QueryOperator::IsNotNull,
                value: DataValue::Null,
            }),
            QueryConditionGroup::Single(QueryCondition {
                field: "expires_at".to_string(),
                operator: QueryOperator::Lt,
                value: DataValue::DateTime(Utc::now()),
            }),
            // is_read为true
            QueryConditionGroup::Single(QueryCondition {
                field: "is_read".to_string(),
                operator: QueryOperator::Eq,
                value: DataValue::Bool(true),
            }),
            // read_at不为null
            QueryConditionGroup::Single(QueryCondition {
                field: "read_at".to_string(),
                operator: QueryOperator::IsNotNull,
                value: DataValue::Null,
            }),
            // read_at小于30天前
            QueryConditionGroup::Single(QueryCondition {
                field: "read_at".to_string(),
                operator: QueryOperator::Lt,
                value: DataValue::DateTime(Utc::now() - Duration::days(30)),
            }),
        ],
    };

    match ModelManager::<SpecialFieldsTest>::find_with_groups(
        vec![complex_condition],
        None,
    ).await {
        Ok(records) => {
            println!("   ✅ 成功执行复杂查询，找到 {} 个匹配记录", records.len());
            for record in &records {
                println!("   - 记录ID: {}, 标题: {}, is_read: {}, expires_at: {:?}, read_at: {:?}",
                    record.id, record.title, record.is_read, record.expires_at, record.read_at);
            }
        },
        Err(e) => {
            println!("   ❌ 复杂查询失败: {}", e);
            println!("   这表明复杂条件下的时间戳处理存在问题");
        }
    }

    println!();

    // 测试6: 验证所有数据的完整性（检查数据是否正确保存和读取）
    println!("6. 验证所有数据的完整性（检查数据是否正确保存和读取）");

    match ModelManager::<SpecialFieldsTest>::find_with_groups(
        vec![],
        None,
    ).await {
        Ok(all_records) => {
            println!("   ✅ 数据库中共有 {} 条记录", all_records.len());

            let mut null_expires_count = 0;
            let mut not_null_expires_count = 0;
            let mut null_read_count = 0;
            let mut not_null_read_count = 0;
            let mut with_tags_count = 0;
            let mut with_metadata_count = 0;

            for record in &all_records {
                if record.expires_at.is_none() {
                    null_expires_count += 1;
                } else {
                    not_null_expires_count += 1;
                }

                if record.read_at.is_none() {
                    null_read_count += 1;
                } else {
                    not_null_read_count += 1;
                }

                if !record.tags.is_empty() {
                    with_tags_count += 1;
                }

                if !record.metadata.is_null() {
                    with_metadata_count += 1;
                }
            }

            println!("   统计结果:");
            println!("     - expires_at为null: {} 条", null_expires_count);
            println!("     - expires_at不为null: {} 条", not_null_expires_count);
            println!("     - read_at为null: {} 条", null_read_count);
            println!("     - read_at不为null: {} 条", not_null_read_count);
            println!("     - 有tags数组: {} 条", with_tags_count);
            println!("     - 有metadata JSON: {} 条", with_metadata_count);
        },
        Err(e) => {
            println!("   ❌ 查询所有记录失败: {}", e);
        }
    }

    println!("\n=== 特殊字段类型验证示例完成 ===");

    // 关闭连接池
    shutdown().await?;

    Ok(())
}

/// 创建测试表（现在由模型自动处理）
async fn create_test_table() -> QuickDbResult<()> {
    // 模型会自动创建表和索引，无需手动操作
    println!("✅ 特殊字段测试表定义完成（通过模型自动创建）");
    Ok(())
}

/// 插入包含极端场景的测试数据
async fn insert_test_data_with_extreme_scenarios() -> QuickDbResult<()> {
    println!("插入特殊字段测试数据（包含极端场景）...");

    let base_time = Utc::now();
    let test_records = vec![
        // 场景1: 所有时间戳字段都有值
        create_test_record(
            "完全有效的记录",
            Some("所有字段都有值的完整记录"),
            Some(base_time + Duration::hours(1)),
            Some(base_time - Duration::minutes(30)),
            true,
            vec!["完整".to_string(), "有效".to_string(), "测试".to_string()],
            json!({"type": "complete", "status": "active", "score": 95}),
            8,
        ),

        // 场景2: expires_at为null，read_at有值（模拟未设置过期时间的已读消息）
        create_test_record(
            "无过期时间的已读记录",
            Some("expires_at为null但read_at有值"),
            None,
            Some(base_time - Duration::hours(2)),
            true,
            vec!["已读".to_string(), "无过期".to_string()],
            json!({"type": "read_no_expire", "status": "processed"}),
            5,
        ),

        // 场景3: expires_at有值，read_at为null（模拟未读消息）
        create_test_record(
            "未读的有效记录",
            Some("expires_at有值但read_at为null的未读记录"),
            Some(base_time + Duration::hours(24)),
            None,
            false,
            vec!["未读".to_string(), "待处理".to_string()],
            json!({"type": "unread", "priority": "high"}),
            9,
        ),

        // 场景4: 所有时间戳字段都为null（模拟草稿或临时记录）
        create_test_record(
            "草稿记录",
            Some("所有时间戳字段都为null的草稿记录"),
            None,
            None,
            false,
            vec!["草稿".to_string(), "临时".to_string()],
            json!({"type": "draft", "editable": true}),
            1,
        ),

        // 场景5: 过期记录（模拟cleanup.rs需要清理的记录）
        create_test_record(
            "过期记录",
            Some("已过期且已读的记录，应该被cleanup清理"),
            Some(base_time - Duration::hours(5)),  // 5小时前过期
            Some(base_time - Duration::days(40)), // 40天前读取（满足30天前的条件）
            true,
            vec!["过期".to_string(), "已读".to_string(), "需清理".to_string()],
            json!({"type": "expired", "cleanup_needed": true, "expired_days": 40}),
            3,
        ),

        // 场景6: 复杂JSON数据
        create_test_record(
            "复杂JSON记录",
            Some("包含复杂嵌套JSON结构的记录"),
            Some(base_time + Duration::days(7)),
            Some(base_time - Duration::minutes(15)),
            true,
            vec!["复杂".to_string(), "JSON".to_string(), "嵌套".to_string()],
            json!({
                "type": "complex",
                "nested": {
                    "level1": {
                        "level2": {
                            "data": ["array", "elements", 123],
                            "boolean": true,
                            "null_value": null
                        }
                    }
                },
                "metadata": {
                    "created_by": "test_system",
                    "tags": ["auto-generated", "test-data"],
                    "statistics": {
                        "views": 42,
                        "interactions": 7
                    }
                }
            }),
            7,
        ),

        // 场景7: 大数组数据
        create_test_record(
            "大数组记录",
            Some("包含大量标签的记录"),
            Some(base_time + Duration::hours(12)),
            Some(base_time - Duration::minutes(5)),
            true,
            (1..=20).map(|i| format!("标签_{}", i)).collect(), // 20个标签
            json!({"type": "large_array", "tag_count": 20}),
            6,
        ),

        // 场景8: 空数组和空JSON
        create_test_record(
            "空容器记录",
            Some("数组为空，JSON为空的记录"),
            Some(base_time + Duration::hours(6)),
            None,
            false,
            vec![], // 空数组
            json!({}), // 空JSON对象
            2,
        ),
    ];

    for (i, record_data) in test_records.iter().enumerate() {
        let record = SpecialFieldsTest {
            id: String::new(), // 框架会自动生成ID
            title: if let Some(DataValue::String(title)) = record_data.get("title") {
                title.clone()
            } else {
                "".to_string()
            },
            description: if let Some(DataValue::String(desc)) = record_data.get("description") {
                Some(desc.clone())
            } else {
                None
            },
            expires_at: if let Some(expires_data) = record_data.get("expires_at") {
                match expires_data {
                    DataValue::DateTime(dt) => Some(*dt),
                    _ => None,
                }
            } else {
                None
            },
            read_at: if let Some(read_data) = record_data.get("read_at") {
                match read_data {
                    DataValue::DateTime(dt) => Some(*dt),
                    _ => None,
                }
            } else {
                None
            },
            is_read: if let Some(DataValue::Bool(is_read)) = record_data.get("is_read") {
                *is_read
            } else {
                false
            },
            tags: if let Some(DataValue::Array(tags)) = record_data.get("tags") {
                tags.iter().filter_map(|tag| {
                    if let DataValue::String(s) = tag {
                        Some(s.clone())
                    } else {
                        None
                    }
                }).collect()
            } else {
                vec![]
            },
            metadata: if let Some(DataValue::Object(meta)) = record_data.get("metadata") {
                // 将DataValue::Object转换为serde_json::Value
                let mut map = serde_json::Map::new();
                for (k, v) in meta {
                    map.insert(k.clone(), data_value_to_json_value(v));
                }
                Value::Object(map)
            } else {
                json!({})
            },
            priority: if let Some(DataValue::Int(priority)) = record_data.get("priority") {
                *priority as i32
            } else {
                1
            },
            created_at: if let Some(DataValue::DateTime(dt)) = record_data.get("created_at") {
                *dt
            } else {
                Utc::now()
            },
            updated_at: Some(Utc::now()),
        };

        let result = record.save().await?;
        println!("   创建记录 {}: {}", i + 1, result);
    }

    println!("✅ 特殊字段测试数据插入完成");
    Ok(())
}

/// 创建测试记录的辅助函数
fn create_test_record(
    title: &str,
    description: Option<&str>,
    expires_at: Option<DateTime<Utc>>,
    read_at: Option<DateTime<Utc>>,
    is_read: bool,
    tags: Vec<String>,
    metadata: Value,
    priority: i32,
) -> HashMap<String, DataValue> {
    let mut record_data = HashMap::new();

    record_data.insert("title".to_string(), DataValue::String(title.to_string()));

    if let Some(desc) = description {
        record_data.insert("description".to_string(), DataValue::String(desc.to_string()));
    }

    if let Some(expires) = expires_at {
        record_data.insert("expires_at".to_string(), DataValue::DateTime(expires));
    }
    // 注意：如果不设置expires_at，就不插入这个键，让数据库处理为null

    if let Some(read) = read_at {
        record_data.insert("read_at".to_string(), DataValue::DateTime(read));
    }

    record_data.insert("is_read".to_string(), DataValue::Bool(is_read));

    // 将Vec<String>转换为DataValue::Array
    let tags_array: Vec<DataValue> = tags.into_iter().map(DataValue::String).collect();
    record_data.insert("tags".to_string(), DataValue::Array(tags_array));

    // 将serde_json::Value转换为DataValue::Object
    let meta_object = json_value_to_data_value(metadata);
    record_data.insert("metadata".to_string(), meta_object);

    record_data.insert("priority".to_string(), DataValue::Int(priority as i64));
    record_data.insert("created_at".to_string(), DataValue::DateTime(Utc::now()));

    record_data
}

/// 将serde_json::Value转换为DataValue（需要实现这个转换函数）
fn json_value_to_data_value(json_val: Value) -> DataValue {
    match json_val {
        Value::String(s) => DataValue::String(s),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                DataValue::Int(i)
            } else if let Some(f) = n.as_f64() {
                DataValue::Float(f)
            } else {
                DataValue::String(n.to_string())
            }
        },
        Value::Bool(b) => DataValue::Bool(b),
        Value::Null => DataValue::Null,
        Value::Array(arr) => {
            let data_array: Vec<DataValue> = arr.into_iter().map(json_value_to_data_value).collect();
            DataValue::Array(data_array)
        },
        Value::Object(obj) => {
            let mut data_map = HashMap::new();
            for (k, v) in obj {
                data_map.insert(k, json_value_to_data_value(v));
            }
            DataValue::Object(data_map)
        }
    }
}

/// 将DataValue转换为serde_json::Value（反向转换函数）
fn data_value_to_json_value(data_val: &DataValue) -> Value {
    match data_val {
        DataValue::String(s) => Value::String(s.clone()),
        DataValue::Int(i) => Value::Number(serde_json::Number::from(*i)),
        DataValue::Float(f) => Value::Number(serde_json::Number::from_f64(*f).unwrap_or(serde_json::Number::from(0))),
        DataValue::Bool(b) => Value::Bool(*b),
        DataValue::Null => Value::Null,
        DataValue::Array(arr) => {
            let json_array: Vec<Value> = arr.iter().map(data_value_to_json_value).collect();
            Value::Array(json_array)
        },
        DataValue::Object(obj) => {
            let mut json_map = serde_json::Map::new();
            for (k, v) in obj {
                json_map.insert(k.clone(), data_value_to_json_value(v));
            }
            Value::Object(json_map)
        },
        DataValue::DateTime(dt) => Value::String(dt.to_rfc3339()),
        // 其他类型根据需要转换
        _ => Value::Null,
    }
}