//! RatQuickDB MySQL 查询操作完整演示
//!
//! 本示例展示了 RatQuickDB 的完整查询操作能力，包括：
//! - 高效批量操作（批量创建、更新、删除）
//! - 复杂查询演示（AND/OR逻辑、嵌套查询）
//! - 查询性能优化和基准测试
//! - 条件查询、排序、分页组合
//! - 数据更新和删除策略
//!
//! 📊 MySQL 查询优化特点：
//! - 原生批量操作，高性能处理
//! - 事务ACID保证数据一致性
//! - 索引优化和查询计划缓存
//! - 连接池管理，高并发支持
//! - 存储引擎选择（InnoDB/MyISAM）

use chrono::Utc;
use rat_logger::{LevelFilter, LoggerBuilder, debug, handler::term::TermConfig};
use rat_quickdb::types::UpdateOperation;
use rat_quickdb::types::*;
use rat_quickdb::types::{LogicalOperator, QueryConditionGroup, QueryConditionWithConfig};
use rat_quickdb::*;
use rat_quickdb::{
    ModelManager, ModelOperations, boolean_field, datetime_field, float_field, integer_field,
    string_field, uuid_field,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Instant, SystemTime};

// 用户模型 - 用于批量操作演示
define_model! {
    struct User {
        id: String,
        username: String,
        email: String,
        full_name: String,
        age: Option<i32>,
        department: String,
        is_active: bool,
        salary: Option<f64>,
        created_at: chrono::DateTime<chrono::Utc>,
        updated_at: Option<chrono::DateTime<chrono::Utc>>,
    }
    collection = "query_demo_users",
    database = "main",
    fields = {
        id: uuid_field().required().unique(),
        username: string_field(None, None, None).required().unique(),
        email: string_field(None, None, None).required().unique(),
        full_name: string_field(None, None, None).required(),
        age: integer_field(None, None),
        department: string_field(None, None, None).required(),
        is_active: boolean_field().required(),
        salary: float_field(None, None),
        created_at: datetime_field().required(),
        updated_at: datetime_field(),
    }
    indexes = [
        { fields: ["username"], unique: true, name: "idx_username" },
        { fields: ["email"], unique: true, name: "idx_email" },
        { fields: ["department"], unique: false, name: "idx_department" },
        { fields: ["is_active"], unique: false, name: "idx_active" },
        { fields: ["age"], unique: false, name: "idx_age" },
        { fields: ["salary"], unique: false, name: "idx_salary" },
    ],
}

// 产品模型 - 用于复杂查询演示
define_model! {
    struct Product {
        id: String,
        name: String,
        category: String,
        price: f64,
        stock: i32,
        is_available: bool,
        rating: Option<f64>,
        tags: Vec<String>,
        created_at: chrono::DateTime<chrono::Utc>,
        updated_at: Option<chrono::DateTime<chrono::Utc>>,
    }
    collection = "query_demo_products",
    database = "main",
    fields = {
        id: uuid_field().required().unique(),
        name: string_field(None, None, None).required(),
        category: string_field(None, None, None).required(),
        price: float_field(None, None).required(),
        stock: integer_field(None, None).required(),
        is_available: boolean_field().required(),
        rating: float_field(None, None),
        tags: array_field(field_types!(string), Some(50), Some(0)),
        created_at: datetime_field().required(),
        updated_at: datetime_field(),
    }
    indexes = [
        { fields: ["name"], unique: false, name: "idx_name" },
        { fields: ["category"], unique: false, name: "idx_category" },
        { fields: ["price"], unique: false, name: "idx_price" },
        { fields: ["stock"], unique: false, name: "idx_stock" },
        { fields: ["is_available"], unique: false, name: "idx_available" },
        { fields: ["rating"], unique: false, name: "idx_rating" },
        // MySQL索引长度限制：移除可能导致超长警告的复合索引
        // { fields: ["category", "is_available"], unique: false, name: "idx_category_available" },
    ],
}

// 简单操作统计
#[derive(Debug)]
struct QueryStats {
    batch_operations: u64,
    complex_queries: u64,
    total_time_ms: u64,
    successful_operations: u64,
}

impl QueryStats {
    fn new() -> Self {
        Self {
            batch_operations: 0,
            complex_queries: 0,
            total_time_ms: 0,
            successful_operations: 0,
        }
    }

    fn add_operation(&mut self, duration_ms: u64, success: bool, is_batch: bool) {
        self.total_time_ms += duration_ms;
        self.successful_operations += if success { 1 } else { 0 };
        if is_batch {
            self.batch_operations += 1;
        } else {
            self.complex_queries += 1;
        }
    }

    fn get_summary(&self) -> String {
        format!(
            "查询操作统计:\n\
             ├─ 批量操作: {} 次\n\
             ├─ 复杂查询: {} 次\n\
             ├─ 总操作数: {} 次\n\
             ├─ 成功率: {:.1}%\n\
             └─ 平均耗时: {:.1}ms/次",
            self.batch_operations,
            self.complex_queries,
            self.batch_operations + self.complex_queries,
            if self.batch_operations + self.complex_queries > 0 {
                self.successful_operations as f64
                    / (self.batch_operations + self.complex_queries) as f64
                    * 100.0
            } else {
                0.0
            },
            if self.batch_operations + self.complex_queries > 0 {
                self.total_time_ms as f64 / (self.batch_operations + self.complex_queries) as f64
            } else {
                0.0
            }
        )
    }
}

// 清理测试数据
async fn cleanup_test_data() {
    println!("清理测试数据...");

    if let Err(e) = rat_quickdb::drop_table("main", "query_demo_users").await {
        debug!("清理用户表失败: {}", e);
    }

    if let Err(e) = rat_quickdb::drop_table("main", "query_demo_products").await {
        debug!("清理产品表失败: {}", e);
    }
}

// 创建测试数据
async fn create_test_data(count: usize) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    println!("创建 {} 条测试用户数据...", count);
    let mut user_ids = Vec::new();

    for i in 0..count {
        let user = User {
            id: String::new(),
            username: format!("user_{}", i),
            email: format!("user{}@example.com", i),
            full_name: format!("测试用户 {}", i),
            age: Some((20 + (i % 50)) as i32),
            department: match i % 4 {
                0 => "技术部".to_string(),
                1 => "销售部".to_string(),
                2 => "人事部".to_string(),
                _ => "财务部".to_string(),
            },
            is_active: i % 5 != 0,
            salary: Some(30000.0 + (i as f64 * 1000.0)),
            created_at: Utc::now(),
            updated_at: None,
        };

        let user_id = user.save().await?;
        user_ids.push(user_id);
    }

    Ok(user_ids)
}

// 创建产品测试数据（仅用于复杂查询演示）
async fn create_product_test_data(count: usize) -> Result<(), Box<dyn std::error::Error>> {
    println!("创建 {} 条测试产品数据...", count);

    for i in 0..count {
        // 前5个产品使用英文名称，用于测试大小写不敏感查询
        let (name, category) = if i < 5 {
            let cat = match i % 3 {
                0 => "Electronics",
                1 => "Office",
                _ => "Lifestyle",
            };
            (format!("PRODUCT {}", i), cat.to_string())
        } else {
            (
                format!("产品 {}", i),
                match i % 3 {
                    0 => "电子产品".to_string(),
                    1 => "办公用品".to_string(),
                    _ => "生活用品".to_string(),
                },
            )
        };

        let product = Product {
            id: String::new(),
            name,
            category,
            price: (100 + i) as f64 * 1.5,
            stock: (10 + i * 2) as i32,
            is_available: i % 3 != 0,
            rating: Some((3.0 + (i % 10) as f64 * 0.5).min(5.0)),
            tags: vec![
                match i % 5 {
                    0 => "热销".to_string(),
                    1 => "新品".to_string(),
                    2 => "推荐".to_string(),
                    3 => "特价".to_string(),
                    _ => "限量".to_string(),
                },
                format!("类别{}", i % 3),
            ],
            created_at: Utc::now(),
            updated_at: None,
        };

        product.save().await?;
    }

    Ok(())
}

// 演示批量操作
async fn demonstrate_batch_operations() -> Result<QueryStats, Box<dyn std::error::Error>> {
    println!("\n=== 批量操作演示 ===");

    let mut stats = QueryStats::new();
    let user_ids = create_test_data(100).await?;

    // 批量更新操作
    println!("\n执行批量更新...");
    let start = Instant::now();

    let mut update_data = HashMap::new();
    update_data.insert(
        "department".to_string(),
        DataValue::String("升级部门".to_string()),
    );
    update_data.insert("salary".to_string(), DataValue::Float(50000.0));

    let update_conditions = vec![QueryCondition {
        field: "age".to_string(),
        operator: QueryOperator::Gte,
        value: DataValue::Int(40),
    }];

    // 执行批量更新
    let operations = vec![
        UpdateOperation::set("department", DataValue::String("升级部门".to_string())),
        UpdateOperation::set("salary", DataValue::Float(50000.0)),
        UpdateOperation::set("updated_at", DataValue::from(Utc::now())),
    ];

    let update_result = User::update_many_with_operations(update_conditions, operations).await;
    let update_time = start.elapsed().as_millis() as u64;

    match update_result {
        Ok(updated_count) => {
            println!(
                "✅ 批量更新成功: {} 条记录，耗时 {}ms",
                updated_count, update_time
            );
            stats.add_operation(update_time, true, true);
        }
        Err(e) => {
            println!("❌ 批量更新失败: {}，耗时 {}ms", e, update_time);
            stats.add_operation(update_time, false, true);
        }
    }

    // 批量删除操作
    println!("\n执行批量删除...");
    let start = Instant::now();

    let delete_conditions = vec![QueryCondition {
        field: "is_active".to_string(),
        operator: QueryOperator::Eq,
        value: DataValue::Bool(false),
    }];

    let delete_result = User::delete_many(delete_conditions).await;
    let delete_time = start.elapsed().as_millis() as u64;

    match delete_result {
        Ok(deleted_count) => {
            println!(
                "✅ 批量删除成功: {} 条记录，耗时 {}ms",
                deleted_count, delete_time
            );
            stats.add_operation(delete_time, true, true);
        }
        Err(e) => {
            println!("❌ 批量删除失败: {}，耗时 {}ms", e, delete_time);
            stats.add_operation(delete_time, false, true);
        }
    }

    Ok(stats)
}

// 演示复杂查询
async fn demonstrate_complex_queries() -> Result<QueryStats, Box<dyn std::error::Error>> {
    println!("\n=== 复杂查询演示 ===");

    let mut stats = QueryStats::new();

    // 创建产品测试数据（用户数据在batch operations中已经创建）
    create_product_test_data(50).await?;

    // 1. 简单条件查询
    println!("\n1. 简单条件查询...");
    let start = Instant::now();

    let simple_conditions = vec![QueryCondition {
        field: "category".to_string(),
        operator: QueryOperator::Eq,
        value: DataValue::String("电子产品".to_string()),
    }];

    let simple_result = ModelManager::<Product>::find(simple_conditions, None).await?;
    let simple_time = start.elapsed().as_millis() as u64;
    println!(
        "✅ 电子产品查询: {} 条记录，耗时 {}ms",
        simple_result.len(),
        simple_time
    );
    stats.add_operation(simple_time, true, false);

    // 2. 复杂AND/OR混用查询 - (is_available = true AND (category = '电子产品' OR price >= 180.0))
    println!("\n2. 复杂AND/OR混用查询...");
    let start = Instant::now();

    let complex_condition = QueryConditionGroup::Group {
        operator: LogicalOperator::And,
        conditions: vec![
            QueryConditionGroup::Single(QueryCondition {
                field: "is_available".to_string(),
                operator: QueryOperator::Eq,
                value: DataValue::Bool(true),
            }),
            QueryConditionGroup::Group {
                operator: LogicalOperator::Or,
                conditions: vec![
                    QueryConditionGroup::Single(QueryCondition {
                        field: "category".to_string(),
                        operator: QueryOperator::Eq,
                        value: DataValue::String("电子产品".to_string()),
                    }),
                    QueryConditionGroup::Single(QueryCondition {
                        field: "price".to_string(),
                        operator: QueryOperator::Gte,
                        value: DataValue::Float(180.0),
                    }),
                ],
            },
        ],
    };

    let complex_result =
        ModelManager::<Product>::find_with_groups(vec![complex_condition.clone()], None).await?;
    let complex_time = start.elapsed().as_millis() as u64;
    println!(
        "✅ AND/OR混用查询: {} 条记录，耗时 {}ms",
        complex_result.len(),
        complex_time
    );
    stats.add_operation(complex_time, true, false);

    // 2.5. 使用 count_with_groups 统计符合条件的记录数（与上面相同的条件）
    println!("\n2.5. 使用 count_with_groups 统计...");
    let start = Instant::now();

    // 使用与上面相同的条件进行计数
    let count_result =
        ModelManager::<Product>::count_with_groups(vec![complex_condition]).await?;
    let count_time = start.elapsed().as_millis() as u64;
    println!(
        "✅ count_with_groups: {} 条记录，耗时 {}ms",
        count_result, count_time
    );
    println!(
        "   验证: find_with_groups 返回 {} 条，count_with_groups 返回 {} 条",
        complex_result.len(),
        count_result
    );
    stats.add_operation(count_time, true, false);

    // 3. 排序查询
    println!("\n3. 排序查询...");
    let start = Instant::now();

    let sort_options = QueryOptions {
        sort: vec![
            SortConfig {
                field: "price".to_string(),
                direction: SortDirection::Desc,
            },
            SortConfig {
                field: "name".to_string(),
                direction: SortDirection::Asc,
            },
        ],
        ..Default::default()
    };

    let sort_result = ModelManager::<Product>::find(vec![], Some(sort_options)).await?;
    let sort_time = start.elapsed().as_millis() as u64;
    println!(
        "✅ 排序查询: {} 条记录，耗时 {}ms",
        sort_result.len(),
        sort_time
    );
    println!("   最贵3个产品:");
    for (i, product) in sort_result.iter().take(3).enumerate() {
        println!(
            "   {}. {} - ${:.2} - {}",
            i + 1,
            product.name,
            product.price,
            product.category
        );
    }
    stats.add_operation(sort_time, true, false);

    // 4. 分页查询
    println!("\n4. 分页查询...");
    let start = Instant::now();

    let pagination_options = QueryOptions {
        pagination: Some(PaginationConfig { skip: 10, limit: 5 }),
        sort: vec![SortConfig {
            field: "rating".to_string(),
            direction: SortDirection::Desc,
        }],
        ..Default::default()
    };

    let page_result = ModelManager::<Product>::find(vec![], Some(pagination_options)).await?;
    let page_time = start.elapsed().as_millis() as u64;
    println!(
        "✅ 分页查询: {} 条记录，耗时 {}ms",
        page_result.len(),
        page_time
    );
    println!("   第2页产品 (按评分排序):");
    for (i, product) in page_result.iter().enumerate() {
        println!(
            "   {}. {} - 评分: {:.1} - ${:.2}",
            i + 11,
            product.name,
            product.rating.unwrap_or(0.0),
            product.price
        );
    }
    stats.add_operation(page_time, true, false);

    // 5. 字段选择查询
    println!("\n5. 字段选择查询...");
    let start = Instant::now();

    let field_options = QueryOptions {
        fields: vec![
            "name".to_string(),
            "price".to_string(),
            "category".to_string(),
        ],
        ..Default::default()
    };

    let field_result = ModelManager::<Product>::find(vec![], Some(field_options)).await?;
    let field_time = start.elapsed().as_millis() as u64;
    println!(
        "✅ 字段选择查询: {} 条记录，耗时 {}ms",
        field_result.len(),
        field_time
    );
    println!("   前5个产品 (仅显示名称、价格、类别):");
    for (i, product) in field_result.iter().take(5).enumerate() {
        println!(
            "   {}. {} - ${:.2} - {}",
            i + 1,
            product.name,
            product.price,
            product.category
        );
    }
    stats.add_operation(field_time, true, false);

    // 6. IN查询 - 查询tags数组包含特定值的记录
    println!("\n6. IN查询 (数组字段)...");
    let start = Instant::now();

    let in_conditions = vec![QueryCondition {
        field: "tags".to_string(),
        operator: QueryOperator::In,
        value: DataValue::Array(vec![
            DataValue::String("热销".to_string()),
            DataValue::String("新品".to_string()),
            DataValue::String("推荐".to_string()),
            DataValue::String("特价".to_string()),
            DataValue::String("限量".to_string()),
        ]),
    }];

    let in_result = ModelManager::<Product>::find(in_conditions, None).await?;
    let in_time = start.elapsed().as_millis() as u64;
    println!("✅ IN查询: {} 条记录，耗时 {}ms", in_result.len(), in_time);
    println!("   包含'热销'或'新品'标签的产品:");
    for (i, product) in in_result.iter().take(3).enumerate() {
        println!("   {}. {} - 标签: {:?}", i + 1, product.name, product.tags);
    }
    stats.add_operation(in_time, true, false);

    // 7. 大小写不敏感查询测试
    println!("\n7. 大小写不敏感查询测试...");
    let start = Instant::now();

    // 先测试大小写敏感查询（默认行为）
    let case_sensitive_conditions = vec![QueryCondition {
        field: "category".to_string(),
        operator: QueryOperator::Eq,
        value: DataValue::String("ELECTRONICS".to_string()),
    }];

    let case_sensitive_result =
        ModelManager::<Product>::find(case_sensitive_conditions, None).await?;
    println!(
        "   大小写敏感查询 'ELECTRONICS': {} 条记录",
        case_sensitive_result.len()
    );

    // 测试大小写不敏感查询
    let case_insensitive_conditions = vec![QueryConditionWithConfig {
        field: "category".to_string(),
        operator: QueryOperator::Eq,
        value: DataValue::String("electronics".to_string()),
        case_insensitive: true,
    }];

    let case_insensitive_result =
        ModelManager::<Product>::find_with_config(case_insensitive_conditions, None).await?;
    let ci_time = start.elapsed().as_millis() as u64;
    println!(
        "   大小写不敏感查询 'electronics': {} 条记录，耗时 {}ms",
        case_insensitive_result.len(),
        ci_time
    );

    // 验证结果
    if case_insensitive_result.len() > 0 {
        println!("   ✅ 大小写不敏感查询成功");
        println!("   找到的产品: {}", case_insensitive_result[0].name);
    } else {
        println!("   ⚠️  大小写不敏感查询未找到结果");
    }

    stats.add_operation(ci_time, true, false);

    // ============================================================================
    // 8. UUID字段查询 - 重要：MySQL/SQLite 使用 uuid_field() 时必须使用字符串查询
    // ============================================================================
    println!("\n8. UUID字段查询（重要场景）...");
    println!("   📝 说明: 在 MySQL/SQLite/MongoDB 中，uuid_field() 存储为字符串");
    println!("   📝 查询时传入 DataValue::String 即可正常查询，框架会自动处理类型转换");

    // 先创建一个带 UUID 字段的测试记录
    let test_user = User {
        id: String::new(),
        username: "uuid_test_user".to_string(),
        email: "uuid_test@example.com".to_string(),
        full_name: "UUID测试用户".to_string(),
        age: Some(25),
        department: "技术部".to_string(),
        is_active: true,
        salary: Some(10000.0),
        created_at: Utc::now(),
        updated_at: None,
    };
    let test_user_id = test_user.save().await?;
    println!("   创建测试用户 ID: {}", test_user_id);

    // 使用字符串值查询 UUID 字段（这是正确的方式）
    let start = Instant::now();
    let uuid_query_conditions = vec![QueryCondition {
        field: "id".to_string(),
        operator: QueryOperator::Eq,
        value: DataValue::String(test_user_id.clone()),
    }];
    let uuid_result = ModelManager::<User>::find(uuid_query_conditions, None).await?;
    let uuid_query_time = start.elapsed().as_millis() as u64;

    if !uuid_result.is_empty() {
        println!("   ✅ UUID字段查询成功: 找到用户 '{}', 耗时 {}ms",
            uuid_result[0].username, uuid_query_time);
    } else {
        println!("   ❌ UUID字段查询失败: 未找到用户");
    }

    // 测试使用字符串查询另一个 UUID 字段 - department_id 场景
    println!("   测试外键 UUID 查询场景...");

    // 使用 username 查询（username 也是 string_field）
    let username_query = vec![QueryCondition {
        field: "username".to_string(),
        operator: QueryOperator::Eq,
        value: DataValue::String("uuid_test_user".to_string()),
    }];
    let username_result = ModelManager::<User>::find(username_query, None).await?;
    if !username_result.is_empty() {
        println!("   ✅ 普通字符串字段查询成功: 找到用户 '{}'", username_result[0].full_name);
    }

    stats.add_operation(uuid_query_time, true, false);

    Ok(stats)
}

// 性能基准测试
async fn performance_benchmark() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== 性能基准测试 ===");

    // 创建大量测试数据
    println!("创建性能测试数据...");
    let start = Instant::now();
    create_performance_test_data(500).await?;
    let create_time = start.elapsed();

    // 测试不同查询类型的性能
    let test_queries = vec![
        (
            "单字段查询",
            vec![QueryCondition {
                field: "is_available".to_string(),
                operator: QueryOperator::Eq,
                value: DataValue::Bool(true),
            }],
        ),
        (
            "双字段AND查询",
            vec![
                QueryCondition {
                    field: "is_available".to_string(),
                    operator: QueryOperator::Eq,
                    value: DataValue::Bool(true),
                },
                QueryCondition {
                    field: "price".to_string(),
                    operator: QueryOperator::Gte,
                    value: DataValue::Float(300.0),
                },
            ],
        ),
        (
            "范围查询",
            vec![
                QueryCondition {
                    field: "price".to_string(),
                    operator: QueryOperator::Gte,
                    value: DataValue::Float(100.0),
                },
                QueryCondition {
                    field: "price".to_string(),
                    operator: QueryOperator::Lte,
                    value: DataValue::Float(500.0),
                },
            ],
        ),
    ];

    for (name, conditions) in test_queries {
        let start = Instant::now();
        let result = ModelManager::<Product>::find(conditions, None).await?;
        let query_time = start.elapsed();

        println!("{}: {} 条记录, 耗时 {:?}", name, result.len(), query_time);
    }

    println!("数据创建耗时: {:?}", create_time);

    Ok(())
}

// 创建性能测试专用数据
async fn create_performance_test_data(count: usize) -> Result<(), Box<dyn std::error::Error>> {
    // 创建新的用户数据，确保email唯一
    for i in 0..count {
        let user = User {
            id: String::new(),
            username: format!("perf_user_{}", i),
            email: format!("perf{}@test.com", i),
            full_name: format!("性能用户 {}", i),
            age: Some((25 + i) as i32),
            department: match i % 4 {
                0 => "技术部".to_string(),
                1 => "销售部".to_string(),
                2 => "人事部".to_string(),
                _ => "财务部".to_string(),
            },
            is_active: true,
            salary: Some(30000.0 + (i as f64 * 100.0)),
            created_at: Utc::now(),
            updated_at: None,
        };

        user.save().await?;
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== RatQuickDB MySQL 查询操作完整演示 ===");

    // 初始化日志
    LoggerBuilder::new()
        .with_level(LevelFilter::Warn)
        .add_terminal_with_config(TermConfig::default())
        .init()?;

    // 初始化数据库
    let db_config = DatabaseConfig::builder()
        .db_type(DatabaseType::MySQL)
        .connection(ConnectionConfig::MySQL {
            host: "172.16.0.24".to_string(),
            port: 3306,
            database: "testdb".to_string(),
            username: "testdb".to_string(),
            password: "testdb123456".to_string(),
            ssl_opts: Default::default(),
            tls_config: None,
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
    println!("数据库连接成功");

    // 清理测试数据
    cleanup_test_data().await;
    println!("清理完成");

    // 执行演示
    let batch_stats = demonstrate_batch_operations().await?;
    let query_stats = demonstrate_complex_queries().await?;

    // 性能基准测试
    performance_benchmark().await?;

    // 输出统计
    println!("\n=== 操作统计 ===");
    println!("{}", batch_stats.get_summary());
    println!("{}", query_stats.get_summary());

    // 健康检查
    println!("\n=== 健康检查 ===");
    let health = health_check().await;
    for (alias, is_healthy) in health {
        let status = if is_healthy { "✅" } else { "❌" };
        println!("{}: {}", alias, status);
    }

    // 清理（注释掉以便检查数据）
    // cleanup_test_data().await;
    println!("\n演示完成 - 数据保留在数据库中以便检查");

    Ok(())
}
