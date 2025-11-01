//! 复杂查询示例
//!
//! 展示如何使用AND/OR逻辑组合进行复杂查询

use rat_quickdb::*;
use rat_quickdb::types::{QueryCondition, QueryConditionGroup, LogicalOperator, QueryOperator, DataValue, QueryOptions, SortConfig, SortDirection, PaginationConfig};
use rat_quickdb::manager::shutdown;
use rat_quickdb::{ModelOperations, string_field, integer_field, float_field, datetime_field};
use std::collections::HashMap;
use chrono::Utc;
use rat_logger::{LoggerBuilder, handler::term::TermConfig, debug};

#[tokio::main]
async fn main() -> QuickDbResult<()> {
    // 初始化日志系统
    LoggerBuilder::new()
        .add_terminal_with_config(TermConfig::default())
        .init()
        .expect("日志初始化失败");

    rat_quickdb::init();
    println!("=== 复杂查询示例 ===");

    // 创建数据库配置 - PostgreSQL配置（从id_strategy_test_pgsql.rs复制）
    let config = DatabaseConfig {
        db_type: DatabaseType::PostgreSQL,
        connection: ConnectionConfig::PostgreSQL {
            host: "172.16.0.23".to_string(),
            port: 5432,
            database: "testdb".to_string(),
            username: "testdb".to_string(),
            password: "testdb123456".to_string(),
            ssl_mode: Some("prefer".to_string()),
            tls_config: None,
        },
        pool: PoolConfig::builder()
            .min_connections(2)
            .max_connections(10)
            .connection_timeout(30)
            .idle_timeout(300)
            .max_lifetime(1800)
            .max_retries(3)
            .retry_interval_ms(1000)
            .keepalive_interval_sec(60)
            .health_check_timeout_sec(10)
            .build()?,
        alias: "default".to_string(),
        cache: None,
        id_strategy: IdStrategy::Uuid,
    };

    // 初始化数据库
    add_database(config).await?;

    // 清理旧的测试表（确保每次都是干净的状态）
    println!("清理旧的测试表...");
    match drop_table("default", "users").await {
        Ok(_) => println!("✅ 已清理旧的users表"),
        Err(e) => println!("   注意：清理表失败（可能表不存在）: {}", e),
    }

    // 创建测试表
    create_test_table().await?;

    // 插入测试数据
    insert_test_data().await?;

    println!("\n=== 开始复杂查询测试 ===\n");

    // 示例1: 简单的OR查询 - 查找年龄为25岁或者姓名为"张三"的用户
    println!("1. OR查询示例: (age = 25 OR name = '张三')");

    let or_condition = QueryConditionGroup::Group {
        operator: LogicalOperator::Or,
        conditions: vec![
            QueryConditionGroup::Single(QueryCondition {
                field: "age".to_string(),
                operator: QueryOperator::Eq,
                value: DataValue::Int(25),
            }),
            QueryConditionGroup::Single(QueryCondition {
                field: "name".to_string(),
                operator: QueryOperator::Eq,
                value: DataValue::String("张三".to_string()),
            }),
        ],
    };

    let or_result = ModelManager::<User>::find_with_groups(
        vec![or_condition],
        None,
    ).await;

    match or_result {
        Ok(users) => {
            println!("   找到 {} 个用户", users.len());
            for user in &users {
                println!("   - {} ({}岁, {}, {})", user.name, user.age, user.city, user.job);
            }
        },
        Err(e) => println!("   查询失败: {}", e),
    }

    println!();

    // 示例2: 复杂的AND和OR组合 - 查找年龄在18-30岁之间，并且(居住在北京或者职业为"开发者")的用户
    println!("2. 复杂组合查询: (age >= 18 AND age <= 30) AND (city = '北京' OR job = '开发者')");

    let complex_condition = QueryConditionGroup::Group {
        operator: LogicalOperator::And,
        conditions: vec![
            // 年龄条件组
            QueryConditionGroup::Group {
                operator: LogicalOperator::And,
                conditions: vec![
                    QueryConditionGroup::Single(QueryCondition {
                        field: "age".to_string(),
                        operator: QueryOperator::Gte,
                        value: DataValue::Int(18),
                    }),
                    QueryConditionGroup::Single(QueryCondition {
                        field: "age".to_string(),
                        operator: QueryOperator::Lte,
                        value: DataValue::Int(30),
                    }),
                ],
            },
            // 地点和职业条件组
            QueryConditionGroup::Group {
                operator: LogicalOperator::Or,
                conditions: vec![
                    QueryConditionGroup::Single(QueryCondition {
                        field: "city".to_string(),
                        operator: QueryOperator::Eq,
                        value: DataValue::String("北京".to_string()),
                    }),
                    QueryConditionGroup::Single(QueryCondition {
                        field: "job".to_string(),
                        operator: QueryOperator::Eq,
                        value: DataValue::String("开发者".to_string()),
                    }),
                ],
            },
        ],
    };

    let complex_result = ModelManager::<User>::find_with_groups(
        vec![complex_condition],
        None,
    ).await;

    match complex_result {
        Ok(users) => {
            println!("   找到 {} 个用户", users.len());
            for user in &users {
                println!("   - {} ({}岁, {}, {})", user.name, user.age, user.city, user.job);
            }
        },
        Err(e) => println!("   查询失败: {}", e),
    }

    println!();

    // 示例3: 嵌套的复杂条件 - 三层嵌套: AND(OR(AND), AND)
    println!("3. 嵌套查询示例: (name = '张三' OR name = '李四') AND (age > 25) AND (city = '上海' OR city = '深圳')");

    let nested_condition = QueryConditionGroup::Group {
        operator: LogicalOperator::And,
        conditions: vec![
            // 姓名条件组 (OR)
            QueryConditionGroup::Group {
                operator: LogicalOperator::Or,
                conditions: vec![
                    QueryConditionGroup::Single(QueryCondition {
                        field: "name".to_string(),
                        operator: QueryOperator::Eq,
                        value: DataValue::String("张三".to_string()),
                    }),
                    QueryConditionGroup::Single(QueryCondition {
                        field: "name".to_string(),
                        operator: QueryOperator::Eq,
                        value: DataValue::String("李四".to_string()),
                    }),
                ],
            },
            // 年龄条件 (Single)
            QueryConditionGroup::Single(QueryCondition {
                field: "age".to_string(),
                operator: QueryOperator::Gt,
                value: DataValue::Int(25),
            }),
            // 城市条件组 (OR)
            QueryConditionGroup::Group {
                operator: LogicalOperator::Or,
                conditions: vec![
                    QueryConditionGroup::Single(QueryCondition {
                        field: "city".to_string(),
                        operator: QueryOperator::Eq,
                        value: DataValue::String("上海".to_string()),
                    }),
                    QueryConditionGroup::Single(QueryCondition {
                        field: "city".to_string(),
                        operator: QueryOperator::Eq,
                        value: DataValue::String("深圳".to_string()),
                    }),
                ],
            },
        ],
    };

    let nested_result = ModelManager::<User>::find_with_groups(
        vec![nested_condition],
        None,
    ).await;

    match nested_result {
        Ok(users) => {
            println!("   找到 {} 个用户", users.len());
            for user in &users {
                println!("   - {} ({}岁, {}, {})", user.name, user.age, user.city, user.job);
            }
        },
        Err(e) => println!("   查询失败: {}", e),
    }

    println!();

    // 示例4: 带排序和分页的复杂查询
    println!("4. 带排序和分页的复杂查询: (status = 'active' AND (score > 80 OR level > 5)) 按年龄降序，限制2条");

    let sorted_condition = QueryConditionGroup::Group {
        operator: LogicalOperator::And,
        conditions: vec![
            QueryConditionGroup::Single(QueryCondition {
                field: "status".to_string(),
                operator: QueryOperator::Eq,
                value: DataValue::String("active".to_string()),
            }),
            QueryConditionGroup::Group {
                operator: LogicalOperator::Or,
                conditions: vec![
                    QueryConditionGroup::Single(QueryCondition {
                        field: "score".to_string(),
                        operator: QueryOperator::Gt,
                        value: DataValue::Float(80.0),
                    }),
                    QueryConditionGroup::Single(QueryCondition {
                        field: "level".to_string(),
                        operator: QueryOperator::Gt,
                        value: DataValue::Int(5),
                    }),
                ],
            },
        ],
    };

    let options = QueryOptions {
        conditions: vec![],  // 条件在condition_groups中处理
        sort: vec![
            SortConfig {
                field: "age".to_string(),
                direction: SortDirection::Desc,
            },
        ],
        pagination: Some(PaginationConfig {
            limit: 2,
            skip: 0,
        }),
        fields: vec![],
    };

    let sorted_result = ModelManager::<User>::find_with_groups(
        vec![sorted_condition],
        Some(options),
    ).await;

    match sorted_result {
        Ok(users) => {
            println!("   找到 {} 个用户", users.len());
            for user in &users {
                println!("   - {} ({}岁, {}, {})", user.name, user.age, user.city, user.job);
            }
        },
        Err(e) => println!("   查询失败: {}", e),
    }

    println!();

    // 示例5: 组合模糊查询 - 使用包含、开始于、结束于和正则表达式
    println!("5. 组合模糊查询示例: (name 包含 '张' OR job 以 '开发' 开始) AND (city 包含 '京' OR city 以 '海' 结束)");

    let fuzzy_condition = QueryConditionGroup::Group {
        operator: LogicalOperator::And,
        conditions: vec![
            // 姓名和职业条件组 (OR)
            QueryConditionGroup::Group {
                operator: LogicalOperator::Or,
                conditions: vec![
                    QueryConditionGroup::Single(QueryCondition {
                        field: "name".to_string(),
                        operator: QueryOperator::Contains,
                        value: DataValue::String("张".to_string()),
                    }),
                    QueryConditionGroup::Single(QueryCondition {
                        field: "job".to_string(),
                        operator: QueryOperator::StartsWith,
                        value: DataValue::String("开发".to_string()),
                    }),
                ],
            },
            // 城市模糊条件组 (OR)
            QueryConditionGroup::Group {
                operator: LogicalOperator::Or,
                conditions: vec![
                    QueryConditionGroup::Single(QueryCondition {
                        field: "city".to_string(),
                        operator: QueryOperator::Contains,
                        value: DataValue::String("京".to_string()),
                    }),
                    QueryConditionGroup::Single(QueryCondition {
                        field: "city".to_string(),
                        operator: QueryOperator::EndsWith,
                        value: DataValue::String("海".to_string()),
                    }),
                ],
            },
        ],
    };

    let fuzzy_result = ModelManager::<User>::find_with_groups(
        vec![fuzzy_condition],
        None,
    ).await;

    match fuzzy_result {
        Ok(users) => {
            println!("   找到 {} 个用户", users.len());
            for user in &users {
                println!("   - {} ({}岁, {}, {})", user.name, user.age, user.city, user.job);
            }
        },
        Err(e) => println!("   查询失败: {}", e),
    }

    println!();

    // 示例6: 复杂的组合模糊查询 - 混合精确和模糊匹配
    println!("6. 复杂组合模糊查询: (status = 'active' AND (name 包含 '张' OR job 包含 '设计')) AND age >= 25");

    let complex_fuzzy_condition = QueryConditionGroup::Group {
        operator: LogicalOperator::And,
        conditions: vec![
            // 精确条件：status = 'active'
            QueryConditionGroup::Single(QueryCondition {
                field: "status".to_string(),
                operator: QueryOperator::Eq,
                value: DataValue::String("active".to_string()),
            }),
            // 模糊条件组：name 包含 '张' OR job 包含 '设计'
            QueryConditionGroup::Group {
                operator: LogicalOperator::Or,
                conditions: vec![
                    QueryConditionGroup::Single(QueryCondition {
                        field: "name".to_string(),
                        operator: QueryOperator::Contains,
                        value: DataValue::String("张".to_string()),
                    }),
                    QueryConditionGroup::Single(QueryCondition {
                        field: "job".to_string(),
                        operator: QueryOperator::Contains,
                        value: DataValue::String("设计".to_string()),
                    }),
                ],
            },
            // 精确条件：age >= 25
            QueryConditionGroup::Single(QueryCondition {
                field: "age".to_string(),
                operator: QueryOperator::Gte,
                value: DataValue::Int(25),
            }),
        ],
    };

    let complex_fuzzy_result = ModelManager::<User>::find_with_groups(
        vec![complex_fuzzy_condition],
        None,
    ).await;

    match complex_fuzzy_result {
        Ok(users) => {
            println!("   找到 {} 个用户", users.len());
            for user in &users {
                println!("   - {} ({}岁, {}, {})", user.name, user.age, user.city, user.job);
            }
        },
        Err(e) => println!("   查询失败: {}", e),
    }

    println!("\n=== 复杂查询示例完成 ===");

    // 关闭连接池
    shutdown().await?;

    Ok(())
}

// 定义用户模型
define_model! {
    /// 用户模型
    struct User {
        id: String,
        name: String,
        age: i32,
        city: String,
        job: String,
        status: String,
        score: f64,
        level: i32,
        created_at: chrono::DateTime<chrono::Utc>,
    }
    collection = "users",
    fields = {
        id: string_field(None, None, None).required().unique(),
        name: string_field(Some(100), Some(1), None).required(),
        age: integer_field(Some(0), Some(150)).required(),
        city: string_field(Some(50), Some(1), None).required(),
        job: string_field(Some(50), Some(1), None).required(),
        status: string_field(Some(20), Some(1), None).required(),
        score: float_field(Some(0.0), Some(100.0)).required(),
        level: integer_field(Some(1), Some(10)).required(),
        created_at: datetime_field().required(),
    }
    indexes = [
        { fields: ["name"], unique: false, name: "idx_name" },
        { fields: ["age"], unique: false, name: "idx_age" },
        { fields: ["city"], unique: false, name: "idx_city" },
        { fields: ["job"], unique: false, name: "idx_job" },
        { fields: ["status"], unique: false, name: "idx_status" },
        { fields: ["status", "age"], unique: false, name: "idx_status_age" },
        { fields: ["created_at"], unique: false, name: "idx_created_at" },
    ],
}

/// 创建测试表（现在由模型自动处理）
async fn create_test_table() -> QuickDbResult<()> {
    // 模型会自动创建表和索引，无需手动操作
    println!("✅ 表定义完成（通过模型自动创建）");
    Ok(())
}

/// 插入测试数据
async fn insert_test_data() -> QuickDbResult<()> {
    println!("插入测试数据...");

    let test_data = vec![
        create_user("张三", 25, "北京", "开发者", "active", 85.5, 6),
        create_user("李四", 30, "上海", "设计师", "active", 92.0, 8),
        create_user("王五", 28, "深圳", "开发者", "active", 78.0, 5),
        create_user("赵六", 32, "北京", "产品经理", "inactive", 88.0, 7),
        create_user("张三", 25, "广州", "测试", "active", 76.5, 4),
        create_user("李四", 35, "上海", "开发者", "active", 95.0, 9),
        create_user("钱七", 22, "深圳", "设计师", "active", 82.0, 6),
        create_user("孙八", 29, "北京", "开发者", "inactive", 91.5, 8),
    ];

    for (i, user_data) in test_data.iter().enumerate() {
        // 从HashMap数据创建User结构体实例
        let user = User {
            id: String::new(), // 框架会自动生成ID
            name: if let Some(DataValue::String(name)) = user_data.get("name") {
                name.clone()
            } else {
                "".to_string()
            },
            age: if let Some(DataValue::Int(age)) = user_data.get("age") {
                *age as i32
            } else {
                0
            },
            city: if let Some(DataValue::String(city)) = user_data.get("city") {
                city.clone()
            } else {
                "".to_string()
            },
            job: if let Some(DataValue::String(job)) = user_data.get("job") {
                job.clone()
            } else {
                "".to_string()
            },
            status: if let Some(DataValue::String(status)) = user_data.get("status") {
                status.clone()
            } else {
                "".to_string()
            },
            score: if let Some(DataValue::Float(score)) = user_data.get("score") {
                *score
            } else {
                0.0
            },
            level: if let Some(DataValue::Int(level)) = user_data.get("level") {
                *level as i32
            } else {
                0
            },
            created_at: if let Some(DataValue::DateTime(dt)) = user_data.get("created_at") {
                *dt
            } else {
                Utc::now()
            },
        };

        let result = user.save().await?;
        println!("   创建用户 {}: {}", i + 1, result);
    }

    println!("✅ 测试数据插入完成");
    Ok(())
}

/// 创建用户数据的辅助函数
fn create_user(name: &str, age: i32, city: &str, job: &str, status: &str, score: f64, level: i32) -> HashMap<String, DataValue> {
    let mut user_data = HashMap::new();
    user_data.insert("name".to_string(), DataValue::String(name.to_string()));
    user_data.insert("age".to_string(), DataValue::Int(age as i64));
    user_data.insert("city".to_string(), DataValue::String(city.to_string()));
    user_data.insert("job".to_string(), DataValue::String(job.to_string()));
    user_data.insert("status".to_string(), DataValue::String(status.to_string()));
    user_data.insert("score".to_string(), DataValue::Float(score));
    user_data.insert("level".to_string(), DataValue::Int(level as i64));
    user_data.insert("created_at".to_string(), DataValue::DateTime(Utc::now()));
    user_data
}