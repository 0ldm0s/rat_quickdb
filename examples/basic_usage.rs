//! RatQuickDB 基本使用示例
//! 
//! 本示例展示了如何使用 RatQuickDB 进行基本的数据库操作,
//! 包括连接配置、CRUD操作、多数据库管理等功能。

use rat_quickdb::*;
use rat_quickdb::manager::{health_check, shutdown, get_cache_manager, get_cache_stats};
use rat_quickdb::types::{CacheConfig, CacheStrategy, TtlConfig, L1CacheConfig, L2CacheConfig, CompressionConfig, CompressionAlgorithm};
use std::collections::HashMap;
use chrono::Utc;
use uuid::Uuid;

#[tokio::main]
async fn main() -> QuickDbResult<()> {
    // 初始化日志系统
    rat_quickdb::init();
    println!("=== RatQuickDB 基本使用示例 ===");
    println!("库版本: {}", rat_quickdb::get_info());

    // 清理旧的数据库文件
    let db_files = ["/tmp/test_basic_usage.db"];
    for db_path in &db_files {
        if std::path::Path::new(db_path).exists() {
            std::fs::remove_file(db_path).unwrap_or_else(|e| {
                eprintln!("警告：删除数据库文件失败 {}: {}", db_path, e);
            });
            println!("✅ 已清理旧的数据库文件: {}", db_path);
        }
    }

    // 1. 配置SQLite数据库
    println!("\n1. 配置SQLite数据库...");
    let sqlite_config = DatabaseConfig::builder()
        .db_type(DatabaseType::SQLite)
        .connection(ConnectionConfig::SQLite {
            path: "/tmp/test_basic_usage.db".to_string(),
            create_if_missing: true,
        })
        .pool(PoolConfig::builder()
            .min_connections(2)
            .max_connections(10)
            .connection_timeout(30)
            .idle_timeout(300)
            .max_lifetime(3600)
            .build()?)
        .alias("default".to_string())
        .id_strategy(IdStrategy::AutoIncrement) // 添加ID生成策略
        // 添加真正的内存缓存配置
        .cache(CacheConfig {
            enabled: true,
            strategy: CacheStrategy::Lru,
            ttl_config: TtlConfig {
                default_ttl_secs: 300, // 5分钟缓存
                max_ttl_secs: 3600,    // 最大1小时
                check_interval_secs: 60,
            },
            l1_config: L1CacheConfig {
                max_capacity: 1000,   // 最多1000个条目
                max_memory_mb: 64,    // L1缓存64MB内存
                enable_stats: true,   // 启用统计
            },
            l2_config: None, // 不使用L2磁盘缓存
            compression_config: CompressionConfig {
                enabled: false,
                algorithm: CompressionAlgorithm::Lz4,
                threshold_bytes: 1024,
            },
            version: "1".to_string(),
        })
        .build()?;
    
    // 添加数据库到连接池管理器
    add_database(sqlite_config).await?;
    println!("SQLite数据库配置完成");
    
    // 2. 获取ODM管理器并进行基本操作
    println!("\n2. 开始数据库操作...");
    let odm = get_odm_manager().await;
    
    // 创建users表（如果不存在）
    create_users_table(&odm).await?;
    
    // 创建用户数据
    println!("\n2.1 创建用户数据");
    let users_data = vec![
        create_user_data(&format!("张三_{}", uuid::Uuid::new_v4().simple()), &format!("zhangsan_{}@example.com", uuid::Uuid::new_v4().simple()), 25, "技术部"),
        create_user_data(&format!("李四_{}", uuid::Uuid::new_v4().simple()), &format!("lisi_{}@example.com", uuid::Uuid::new_v4().simple()), 30, "产品部"),
        create_user_data(&format!("王五_{}", uuid::Uuid::new_v4().simple()), &format!("wangwu_{}@example.com", uuid::Uuid::new_v4().simple()), 28, "技术部"),
        create_user_data(&format!("赵六_{}", uuid::Uuid::new_v4().simple()), &format!("zhaoliu_{}@example.com", uuid::Uuid::new_v4().simple()), 32, "市场部"),
    ];
    
    for (i, user_data) in users_data.iter().enumerate() {
        let result = rat_quickdb::create("users", user_data.clone(), None).await?;
        println!("创建用户 {}: {}", i + 1, result);
    }
    
    // 查询所有用户
    println!("\n2.2 查询所有用户");
    let all_users = rat_quickdb::find("users", vec![], None, None).await?;
    println!("所有用户: {:?}", all_users);
    
    // 条件查询
    println!("\n2.3 条件查询 - 查找技术部员工");
    let tech_conditions = vec![
        QueryCondition {
            field: "department".to_string(),
            operator: QueryOperator::Eq,
            value: DataValue::String("技术部".to_string()),
        }
    ];
    
    let tech_users = rat_quickdb::find("users", tech_conditions.clone(), None, None).await?;
    println!("技术部员工: {:?}", tech_users);
    
    // 范围查询
    println!("\n2.4 范围查询 - 查找年龄在25-30之间的员工");
    let age_conditions = vec![
        QueryCondition {
            field: "age".to_string(),
            operator: QueryOperator::Gte,
            value: DataValue::Int(25),
        },
        QueryCondition {
            field: "age".to_string(),
            operator: QueryOperator::Lte,
            value: DataValue::Int(30),
        }
    ];
    
    let age_filtered_users = rat_quickdb::find("users", age_conditions, None, None).await?;
    println!("年龄25-30的员工: {:?}", age_filtered_users);
    
    // 排序和分页查询
    println!("\n2.5 排序和分页查询");
    let query_options = QueryOptions {
        conditions: vec![],
        sort: vec![
            SortConfig {
                field: "age".to_string(),
                direction: SortDirection::Desc,
            },
            SortConfig {
                field: "name".to_string(),
                direction: SortDirection::Asc,
            },
        ],
        pagination: Some(PaginationConfig {
            skip: 0,
            limit: 2,
        }),
        fields: vec![],
    };
    
    let sorted_users = rat_quickdb::find("users", vec![], Some(query_options), None).await?;
    println!("排序分页结果: {:?}", sorted_users);
    
    // 更新操作
    println!("\n2.6 更新操作 - 给技术部员工加薪");
    let mut salary_update = HashMap::new();
    salary_update.insert("salary".to_string(), DataValue::Float(8000.0));
    salary_update.insert("updated_at".to_string(), DataValue::DateTime(Utc::now()));
    
    let updated_count = rat_quickdb::update("users", tech_conditions.clone(), salary_update, None).await?;
    println!("更新了 {} 条技术部员工记录", updated_count);
    
    // 统计操作
    println!("\n2.7 统计操作");
    let total_count = rat_quickdb::count("users", vec![], None).await?;
    println!("总用户数: {}", total_count);
    
    let tech_count = rat_quickdb::count("users", tech_conditions, None).await?;
    println!("技术部员工数: {}", tech_count);
    
    // 检查记录是否存在
    let conditions = vec![
        QueryCondition {
            field: "name".to_string(),
            operator: QueryOperator::Eq,
            value: DataValue::String("Alice".to_string()),
        }
    ];
    let exists = rat_quickdb::exists("users", conditions, None).await?;
    println!("用户 Alice 是否存在: {}", exists);
    
    // 3. 连接池监控
    println!("\n3. 连接池监控");
    let aliases = get_aliases();
    println!("已配置的数据库别名: {:?}", aliases);
    
    let health_map = health_check().await;
    let health = health_map.get("default").unwrap_or(&false);
    println!("数据库健康状态: {}", if *health { "正常" } else { "异常" });
    
    // 4. JSON序列化示例
    println!("\n4. JSON序列化示例");
    demonstrate_serialization().await?;
    
    // 5. 缓存功能演示
    println!("\n5. 缓存功能演示");
    demonstrate_caching().await?;
    
    // 删除记录
    let delete_conditions = vec![
        QueryCondition {
            field: "name".to_string(),
            operator: QueryOperator::Eq,
            value: DataValue::String("Alice".to_string()),
        }
    ];
    let deleted = rat_quickdb::delete("users", delete_conditions, None).await?;
    println!("删除的记录数: {}", deleted);
    
    // 6. 清理操作
    println!("\n6. 清理操作");
    let cleanup_conditions = vec![
        QueryCondition {
            field: "department".to_string(),
            operator: QueryOperator::Eq,
            value: DataValue::String("市场部".to_string()),
        }
    ];
    
    let deleted_count = rat_quickdb::delete("users", cleanup_conditions, None).await?;
    println!("删除了 {} 条市场部记录", deleted_count);
    
    // 关闭连接池
    shutdown().await?;
    println!("\n=== 示例执行完成 ===");
    
    Ok(())
}

/// 创建users表（如果不存在）
async fn create_users_table(odm: &AsyncOdmManager) -> QuickDbResult<()> {
    use rat_quickdb::model::FieldType;
    use std::collections::HashMap;
    
    // 定义users表的字段结构
    let mut fields = HashMap::new();
    
    // name字段：字符串类型，最大长度100
    fields.insert("name".to_string(), FieldType::String {
        max_length: Some(100),
        min_length: Some(1),
        regex: None,
    });
    
    // email字段：字符串类型，最大长度255
    fields.insert("email".to_string(), FieldType::String {
        max_length: Some(255),
        min_length: Some(5),
        regex: None,
    });
    
    // age字段：整数类型，范围0-150
    fields.insert("age".to_string(), FieldType::Integer {
        min_value: Some(0),
        max_value: Some(150),
    });
    
    // department字段：字符串类型，最大长度50
    fields.insert("department".to_string(), FieldType::String {
        max_length: Some(50),
        min_length: Some(1),
        regex: None,
    });
    
    // salary字段：浮点数类型，最小值0
    fields.insert("salary".to_string(), FieldType::Float {
        min_value: Some(0.0),
        max_value: None,
    });
    
    // created_at字段：日期时间类型
    fields.insert("created_at".to_string(), FieldType::DateTime);
    
    // updated_at字段：日期时间类型
    fields.insert("updated_at".to_string(), FieldType::DateTime);
    
    // status字段：字符串类型，最大长度20
    fields.insert("status".to_string(), FieldType::String {
        max_length: Some(20),
        min_length: Some(1),
        regex: None,
    });
    
    // 注意：在ODM设计下，表会自动创建，无需手动操作
    println!("📝 注意：rat_quickdb会在首次插入数据时自动创建表");
    println!("users表结构已定义，包含以下字段：");
    for (field_name, field_type) in &fields {
        println!("  - {}: {:?}", field_name, field_type);
    }
    Ok(())
}

/// 创建用户数据的辅助函数
fn create_user_data(name: &str, email: &str, age: i32, department: &str) -> HashMap<String, DataValue> {
    let mut user_data = HashMap::new();
    user_data.insert("name".to_string(), DataValue::String(name.to_string()));
    user_data.insert("email".to_string(), DataValue::String(email.to_string()));
    user_data.insert("age".to_string(), DataValue::Int(age as i64));
    user_data.insert("department".to_string(), DataValue::String(department.to_string()));
    user_data.insert("salary".to_string(), DataValue::Float(5000.0 + (age as f64 * 100.0))); // 基于年龄的薪资
    user_data.insert("created_at".to_string(), DataValue::DateTime(Utc::now()));
    user_data.insert("updated_at".to_string(), DataValue::DateTime(Utc::now()));
    user_data.insert("status".to_string(), DataValue::String("active".to_string()));
    user_data
}

/// 演示JSON序列化功能
async fn demonstrate_serialization() -> QuickDbResult<()> {
    use rat_quickdb::serializer::*;
    
    // 创建测试数据
    let mut test_data = HashMap::new();
    test_data.insert("id".to_string(), DataValue::String("user_001".to_string()));
    test_data.insert("name".to_string(), DataValue::String("测试用户".to_string()));
    test_data.insert("score".to_string(), DataValue::Float(95.67));
    test_data.insert("active".to_string(), DataValue::Bool(true));
    test_data.insert("tags".to_string(), DataValue::Array(vec![
        DataValue::String("VIP".to_string()),
        DataValue::String("高级用户".to_string()),
    ]));
    
    // 默认序列化
    let default_serializer = DataSerializer::default();
    let result = default_serializer.serialize_record(test_data.clone())?;
    let json_string = result.to_json_string()?;
    println!("默认序列化: {}", json_string);
    
    // 创建美化输出的序列化器
    let mut pretty_config = SerializerConfig::new();
    pretty_config.pretty = true;
    pretty_config.include_null = false;
    let pretty_serializer = DataSerializer::new(pretty_config);
    
    // 序列化单个记录
    let pretty_result = pretty_serializer.serialize_record(test_data.clone())?;
    let pretty_json = pretty_result.to_json_string()?;
    println!("美化JSON输出: {}", pretty_json);
    
    // 使用PyO3兼容序列化器
    let pyo3_serializer = DataSerializer::new(SerializerConfig::for_pyo3());
    let pyo3_result = pyo3_serializer.serialize_record(test_data)?;
    let pyo3_json = pyo3_result.to_json_string()?;
    println!("PyO3兼容序列化: {}", pyo3_json);
    
    Ok(())
}

/// 演示真正的缓存功能
async fn demonstrate_caching() -> QuickDbResult<()> {
    println!("演示 rat_memcache 缓存功能...");

    // 获取缓存管理器
    let cache_manager = match get_cache_manager("default") {
        Ok(manager) => manager,
        Err(e) => {
            println!("获取缓存管理器失败: {}", e);
            return Ok(());
        }
    };

    // 测试数据
    let mut test_user = HashMap::new();
    test_user.insert("name".to_string(), DataValue::String("缓存测试用户".to_string()));
    test_user.insert("email".to_string(), DataValue::String("cache_test@example.com".to_string()));
    test_user.insert("age".to_string(), DataValue::Int(25));

    // 创建用户并获取ID
    let user_id = rat_quickdb::create("users", test_user.clone(), None).await?;
    println!("创建用户成功，ID: {:?}", user_id);

    // 第一次查询（应该从数据库获取）
    println!("第一次查询用户（从数据库获取）...");
    let start_time = std::time::Instant::now();
    let first_query = rat_quickdb::find_by_id("users", &user_id.to_string(), None).await?;
    let first_duration = start_time.elapsed();
    println!("第一次查询耗时: {:?}, 结果: {:?}", first_duration, first_query);

    // 第二次查询（应该从缓存获取）
    println!("第二次查询用户（从缓存获取）...");
    let start_time = std::time::Instant::now();
    let second_query = rat_quickdb::find_by_id("users", &user_id.to_string(), None).await?;
    let second_duration = start_time.elapsed();
    println!("第二次查询耗时: {:?}, 结果: {:?}", second_duration, second_query);

    // 比较查询性能
    if second_duration < first_duration {
        println!("✅ 缓存生效！第二次查询比第一次快 {:?}", first_duration - second_duration);
    } else {
        println!("⚠️  缓存效果不明显，两次查询耗时相近");
    }

    // 显示缓存统计信息
    if let Ok(stats) = get_cache_stats("default").await {
        println!("缓存统计信息:");
        println!("  命中次数: {}", stats.hits);
        println!("  未命中次数: {}", stats.misses);
        println!("  命中率: {:.2}%", stats.hit_rate * 100.0);
        println!("  缓存条目数: {}", stats.entries);
        println!("  内存使用: {} KB", stats.memory_usage_bytes / 1024);
    }

    // 测试查询结果缓存
    println!("\n测试查询结果缓存...");
    let conditions = vec![
        QueryCondition {
            field: "department".to_string(),
            operator: QueryOperator::Eq,
            value: DataValue::String("技术部".to_string()),
        }
    ];

    // 第一次条件查询
    println!("第一次条件查询技术部员工...");
    let start_time = std::time::Instant::now();
    let first_result = rat_quickdb::find("users", conditions.clone(), None, None).await?;
    let first_duration = start_time.elapsed();
    println!("第一次条件查询耗时: {:?}, 找到 {} 条记录", first_duration, first_result.len());

    // 第二次相同条件查询（应该从缓存获取）
    println!("第二次相同条件查询...");
    let start_time = std::time::Instant::now();
    let second_result = rat_quickdb::find("users", conditions.clone(), None, None).await?;
    let second_duration = start_time.elapsed();
    println!("第二次条件查询耗时: {:?}, 找到 {} 条记录", second_duration, second_result.len());

    if second_duration < first_duration {
        println!("✅ 查询缓存生效！第二次查询比第一次快 {:?}", first_duration - second_duration);
    }

    Ok(())
}