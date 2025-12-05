//! MongoDB Array 字段的 IN 查询功能测试示例
//!
//! 测试 Array 字段的存储、查询和类型转换功能

use rat_logger::{LevelFilter, LoggerBuilder, handler::term::TermConfig};
use rat_quickdb::manager::health_check;
use rat_quickdb::model::FieldType;
use rat_quickdb::types::{
    ConnectionConfig, DatabaseType, LogicalOperator, PoolConfig, QueryConditionGroup, QueryOptions,
    SortConfig, SortDirection,
};
use rat_quickdb::*;
use rat_quickdb::{
    DataValue, ModelManager, ModelOperations, QueryCondition, QueryOperator, array_field,
    field_types,
};

/// 显示结果的详细信息，包括Array字段的JSON格式
fn display_array_test_result(index: usize, result: &ArrayTestModel) {
    // 将Array字段转换为JSON字符串显示
    let tags_json = serde_json::to_string(&result.tags).unwrap_or_else(|_| "[]".to_string());
    let category_ids_json =
        serde_json::to_string(&result.category_ids).unwrap_or_else(|_| "[]".to_string());
    let ratings_json = serde_json::to_string(&result.ratings).unwrap_or_else(|_| "[]".to_string());

    println!("  {}. {}", index + 1, result.name);
    println!("     tags: {}", tags_json);
    println!("     category_ids: {}", category_ids_json);
    println!("     ratings: {}", ratings_json);
    println!();
}

// 定义测试模型
define_model! {
    /// Array 字段测试模型
    struct ArrayTestModel {
        id: String,
        name: String,
        tags: Vec<String>,        // 字符串数组
        category_ids: Vec<i64>,   // 整数数组
        ratings: Vec<f64>,        // 浮点数数组
    }
    collection = "array_test",
    database = "main",
    fields = {
        id: string_field(None, None, None).required().unique(),
        name: string_field(None, None, None).required(),
        tags: array_field(field_types!(string), Some(10), Some(0)),
        category_ids: array_field(field_types!(integer), Some(20), Some(0)),
        ratings: array_field(field_types!(float), Some(15), Some(0)),
    }
}

#[tokio::main]
async fn main() -> QuickDbResult<()> {
    // 初始化日志
    LoggerBuilder::new()
        .add_terminal_with_config(TermConfig::default())
        .init()
        .expect("日志初始化失败");

    println!("🚀 测试 MongoDB Array 字段 IN 查询功能");
    println!("===============================\n");

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
    match drop_table("main", "array_test").await {
        Ok(_) => println!("✓ 清理完成"),
        Err(e) => println!("注意: 清理失败或表不存在: {}", e),
    }

    // 2. 创建测试数据
    println!("\n2. 创建测试数据...");
    let test_data = vec![
        ArrayTestModel {
            id: generate_object_id(),
            name: "iPhone 15".to_string(),
            tags: vec![
                "apple".to_string(),
                "smartphone".to_string(),
                "premium".to_string(),
            ],
            category_ids: vec![1, 5, 10],
            ratings: vec![4.5, 4.8, 4.2],
        },
        ArrayTestModel {
            id: generate_object_id(),
            name: "Samsung Galaxy S24".to_string(),
            tags: vec![
                "samsung".to_string(),
                "smartphone".to_string(),
                "android".to_string(),
            ],
            category_ids: vec![1, 5, 11],
            ratings: vec![4.3, 4.6, 4.1],
        },
        ArrayTestModel {
            id: generate_object_id(),
            name: "MacBook Pro".to_string(),
            tags: vec![
                "apple".to_string(),
                "laptop".to_string(),
                "premium".to_string(),
            ],
            category_ids: vec![2, 5, 12],
            ratings: vec![4.7, 4.9, 4.8],
        },
        ArrayTestModel {
            id: generate_object_id(),
            name: "Dell XPS 13".to_string(),
            tags: vec![
                "dell".to_string(),
                "laptop".to_string(),
                "business".to_string(),
            ],
            category_ids: vec![2, 5, 13],
            ratings: vec![4.2, 4.4, 4.3],
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

    // 3. Array 字段 IN 查询测试
    println!("\n3. Array 字段 IN 查询测试...");

    // 测试1: 字符串数组的 IN 查询（单个值）
    println!("\n3.1 查找标签包含 'apple' 的产品:");
    match ModelManager::<ArrayTestModel>::find(
        vec![QueryCondition {
            field: "tags".to_string(),
            operator: QueryOperator::In,
            value: DataValue::Array(vec![DataValue::String("apple".to_string())]),
        }],
        None,
    )
    .await
    {
        Ok(results) => {
            println!("✓ 找到 {} 个产品:", results.len());
            for (i, result) in results.iter().enumerate() {
                display_array_test_result(i, result);
            }
        }
        Err(e) => {
            eprintln!("❌ 查询失败: {}", e);
        }
    }

    // 测试2: 字符串数组的 IN 查询（多个值）
    println!("\n3.2 查找标签包含 'laptop' 或 'smartphone' 的产品:");
    match ModelManager::<ArrayTestModel>::find(
        vec![QueryCondition {
            field: "tags".to_string(),
            operator: QueryOperator::In,
            value: DataValue::Array(vec![
                DataValue::String("laptop".to_string()),
                DataValue::String("smartphone".to_string()),
            ]),
        }],
        None,
    )
    .await
    {
        Ok(results) => {
            println!("✓ 找到 {} 个产品:", results.len());
            for (i, result) in results.iter().enumerate() {
                display_array_test_result(i, result);
            }
        }
        Err(e) => {
            eprintln!("❌ 查询失败: {}", e);
        }
    }

    // 测试3: 整数数组的 IN 查询
    println!("\n3.3 查找分类ID包含 1 的产品:");
    match ModelManager::<ArrayTestModel>::find(
        vec![QueryCondition {
            field: "category_ids".to_string(),
            operator: QueryOperator::In,
            value: DataValue::Array(vec![DataValue::Int(1)]),
        }],
        None,
    )
    .await
    {
        Ok(results) => {
            println!("✓ 找到 {} 个产品:", results.len());
            for (i, result) in results.iter().enumerate() {
                display_array_test_result(i, result);
            }
        }
        Err(e) => {
            eprintln!("❌ 查询失败: {}", e);
        }
    }

    // 测试4: 浮点数数组的 IN 查询
    println!("\n3.4 查找评分包含 4.8 的产品:");
    match ModelManager::<ArrayTestModel>::find(
        vec![QueryCondition {
            field: "ratings".to_string(),
            operator: QueryOperator::In,
            value: DataValue::Array(vec![DataValue::Float(4.8)]),
        }],
        None,
    )
    .await
    {
        Ok(results) => {
            println!("✓ 找到 {} 个产品:", results.len());
            for (i, result) in results.iter().enumerate() {
                display_array_test_result(i, result);
            }
        }
        Err(e) => {
            eprintln!("❌ 查询失败: {}", e);
        }
    }

    // 测试5: NOT IN 查询
    println!("\n3.5 测试 Array 字段的 NOT IN 查询（查找不包含 'apple' 的标签）:");
    match ModelManager::<ArrayTestModel>::find(
        vec![QueryCondition {
            field: "tags".to_string(),
            operator: QueryOperator::NotIn,
            value: DataValue::Array(vec![DataValue::String("apple".to_string())]),
        }],
        None,
    )
    .await
    {
        Ok(results) => {
            println!("✓ 找到 {} 个不包含'apple'标签的产品:", results.len());
            for (i, result) in results.iter().enumerate() {
                display_array_test_result(i, result);
            }
        }
        Err(e) => {
            eprintln!("❌ 查询失败: {}", e);
        }
    }

    // 测试6: 不支持类型的 IN 查询（应该报错）
    println!("\n4.6 测试不支持类型的 IN 查询（应该报错）:");
    match ModelManager::<ArrayTestModel>::find(
        vec![QueryCondition {
            field: "tags".to_string(),
            operator: QueryOperator::In,
            value: DataValue::Array(vec![DataValue::Bool(true)]),
        }],
        None,
    )
    .await
    {
        Ok(_) => {
            eprintln!("❌ 意外成功，应该报错");
        }
        Err(e) => {
            println!("✓ 正确报错: {}", e);
        }
    }

    // 4. 复杂 Array 查询测试
    println!("\n4. 复杂 Array 查询测试...");

    // 测试7: 复杂组合查询 - (tags IN ['apple'] OR tags IN ['samsung']) AND (category_ids IN [1])
    println!("\n4.1 复杂组合查询: (tags包含'apple'或'samsung') AND (category_ids包含1)");
    let complex_condition = QueryConditionGroup::Group {
        operator: LogicalOperator::And,
        conditions: vec![
            // 标签条件组 (OR)
            QueryConditionGroup::Group {
                operator: LogicalOperator::Or,
                conditions: vec![
                    QueryConditionGroup::Single(QueryCondition {
                        field: "tags".to_string(),
                        operator: QueryOperator::In,
                        value: DataValue::Array(vec![DataValue::String("apple".to_string())]),
                    }),
                    QueryConditionGroup::Single(QueryCondition {
                        field: "tags".to_string(),
                        operator: QueryOperator::In,
                        value: DataValue::Array(vec![DataValue::String("samsung".to_string())]),
                    }),
                ],
            },
            // 分类ID条件 (Single)
            QueryConditionGroup::Single(QueryCondition {
                field: "category_ids".to_string(),
                operator: QueryOperator::In,
                value: DataValue::Array(vec![DataValue::Int(1)]),
            }),
        ],
    };

    match ModelManager::<ArrayTestModel>::find_with_groups(vec![complex_condition], None).await {
        Ok(results) => {
            println!("✓ 找到 {} 个产品:", results.len());
            for (i, result) in results.iter().enumerate() {
                display_array_test_result(i, result);
            }
        }
        Err(e) => {
            eprintln!("❌ 复杂查询失败: {}", e);
        }
    }

    println!("\n✅ Array 字段复杂查询测试完成！");
    println!("🗄️ MongoDB数据库集合: array_test（可用于验证数据正确性）");

    Ok(())
}
