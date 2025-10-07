//! 分页查询演示示例
//!
//! 本示例演示如何使用 ModelManager 进行分页查询，包括：
//! - 基础分页查询
//! - 排序 + 分页组合
//! - 条件过滤 + 分页
//! - 分页导航信息计算

use rat_quickdb::*;
use rat_quickdb::types::{QueryCondition, QueryOperator, DataValue, QueryOptions, SortConfig, SortDirection, PaginationConfig};
use rat_quickdb::{ModelManager, ModelOperations, string_field, integer_field, float_field, boolean_field, datetime_field, field_types};
use serde::{Deserialize, Serialize};
use chrono::Utc;

/// 用户模型
define_model! {
    struct User {
        id: i32,
        name: String,
        email: String,
        age: i32,
        department: String,
        salary: f64,
        created_at: chrono::DateTime<chrono::Utc>,
    }
    collection = "users",
    fields = {
        id: integer_field(None, None).required().unique(),
        name: string_field(None, None, None).required(),
        email: string_field(None, None, None).required(),
        age: integer_field(None, None).required(),
        department: string_field(None, None, None).required(),
        salary: float_field(None, None).required(),
        created_at: datetime_field().required(),
    }
    indexes = [
        { fields: ["department"], unique: false, name: "idx_department" },
        { fields: ["age"], unique: false, name: "idx_age" },
        { fields: ["salary"], unique: false, name: "idx_salary" },
    ],
}

impl User {
    /// 创建测试用户
    fn create_test_user(index: usize) -> Self {
        let departments = ["技术部", "销售部", "市场部", "人事部", "财务部"];
        let names = ["张三", "李四", "王五", "赵六", "孙七", "周八", "吴九", "郑十"];

        User {
            id: (index + 1) as i32,
            name: format!("{}{}", names[index % names.len()], index + 1),
            email: format!("user{}@example.com", index + 1),
            age: ((index % 35) + 22) as i32, // 22-56岁
            department: departments[index % departments.len()].to_string(),
            salary: 5000.0 + (index as f64 * 1000.0) + ((index % 10) as f64 * 500.0),
            created_at: Utc::now(),
        }
    }
}

/// 分页信息结构
#[derive(Debug)]
struct PageInfo {
    page: usize,
    page_size: usize,
    total_count: usize,
    total_pages: usize,
    has_prev: bool,
    has_next: bool,
}

impl PageInfo {
    fn new(page: usize, page_size: usize, total_count: usize) -> Self {
        let total_pages = (total_count + page_size - 1) / page_size;
        Self {
            page,
            page_size,
            total_count,
            total_pages,
            has_prev: page > 1,
            has_next: page < total_pages,
        }
    }

    fn display(&self) {
        println!("📄 分页信息: 第 {}/{} 页 | 每页 {} 条 | 共 {} 条 | 上一页: {} | 下一页: {}",
            self.page, self.total_pages, self.page_size, self.total_count,
            if self.has_prev { "✓" } else { "✗" },
            if self.has_next { "✓" } else { "✗" }
        );
    }
}

#[tokio::main]
async fn main() -> QuickDbResult<()> {
    // 初始化日志系统
    rat_quickdb::init();
    println!("=== 分页查询演示 ===\n");

    // 清理旧的数据库文件
    let db_files = ["/tmp/pagination_demo.db"];
    for db_path in &db_files {
        if std::path::Path::new(db_path).exists() {
            std::fs::remove_file(db_path).unwrap_or_else(|e| {
                eprintln!("警告：删除数据库文件失败 {}: {}", db_path, e);
            });
            println!("✅ 已清理旧的数据库文件: {}", db_path);
        }
    }

    // 1. 配置数据库
    println!("1. 配置数据库...");
    let config = DatabaseConfig::builder()
        .db_type(DatabaseType::SQLite)
        .connection(ConnectionConfig::SQLite {
            path: "/tmp/pagination_demo.db".to_string(),
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
        .id_strategy(IdStrategy::AutoIncrement)
        .build()?;

    add_database(config).await?;
    println!("✅ 数据库配置完成\n");

    // 2. 插入测试数据
    println!("2. 插入测试数据...");

    let users: Vec<User> = (0..50).map(|i| User::create_test_user(i)).collect();
    let mut created_count = 0;

    for user in users {
        match user.save().await {
            Ok(_) => created_count += 1,
            Err(e) => println!("❌ 创建用户失败: {}", e),
        }
    }

    println!("✅ 成功创建 {} 个用户\n", created_count);

    // 3. 演示基础分页查询
    println!("3. 🔍 基础分页查询");
    println!("==================");

    let page_size = 5;
    let total_count = ModelManager::<User>::count(vec![]).await?;
    let total_pages = (total_count + page_size - 1) / page_size;

    println!("总共 {} 条记录，每页 {} 条，共 {} 页\n", total_count, page_size, total_pages);

    for page in 1..=std::cmp::min(3, total_pages) {
        let skip = (page - 1) * page_size;

        let query_options = QueryOptions {
            conditions: vec![],
            sort: vec![],
            pagination: Some(PaginationConfig {
                skip,
                limit: page_size,
            }),
            fields: vec![],
        };

        let page_users = ModelManager::<User>::find(vec![], Some(query_options)).await?;

        let page_info = PageInfo::new(page as usize, page_size as usize, total_count as usize);

        println!("--- 第 {} 页 ---", page);
        page_info.display();
        println!("📋 用户列表:");

        for (index, user) in page_users.iter().enumerate() {
            println!("   {}. {} ({}岁, {})", index + 1, user.name, user.age, user.department);
        }
        println!();
    }

    if total_pages > 3 {
        println!("... 还有 {} 页数据 ...\n", total_pages - 3);
    }

    // 4. 演示排序 + 分页
    println!("4. 🔄 排序 + 分页查询");
    println!("===================");

    let sort_query_options = QueryOptions {
        conditions: vec![],
        sort: vec![
            SortConfig {
                field: "salary".to_string(),
                direction: SortDirection::Desc,
            },
            SortConfig {
                field: "name".to_string(),
                direction: SortDirection::Asc,
            },
        ],
        pagination: Some(PaginationConfig {
            skip: 0,
            limit: 8,
        }),
        fields: vec![],
    };

    let high_salary_users = ModelManager::<User>::find(vec![], Some(sort_query_options)).await?;

    println!("📊 按薪资降序、姓名升序排列的前8名用户:");
    for (index, user) in high_salary_users.iter().enumerate() {
        println!("   {}. {} - 薪资: {:.2} - {}", index + 1, user.name, user.salary, user.department);
    }
    println!();

    // 5. 演示条件过滤 + 分页
    println!("5. 🔍 条件过滤 + 分页查询");
    println!("=======================");

    let filter_conditions = vec![
        QueryCondition {
            field: "age".to_string(),
            operator: QueryOperator::Gte,
            value: DataValue::Int(30),
        },
    ];

    let filter_page_size = 6;
    let filtered_count = ModelManager::<User>::count(filter_conditions.clone()).await?;
    let filtered_total_pages = (filtered_count + filter_page_size - 1) / filter_page_size;

    println!("30岁以上的用户: {} 条记录\n", filtered_count);

    if filtered_count > 0 {
        for page in 1..=std::cmp::min(2, filtered_total_pages) {
            let skip = (page - 1) * filter_page_size;

            let filter_query_options = QueryOptions {
                conditions: filter_conditions.clone(),
                sort: vec![
                    SortConfig {
                        field: "age".to_string(),
                        direction: SortDirection::Desc,
                    }
                ],
                pagination: Some(PaginationConfig {
                    skip,
                    limit: filter_page_size,
                }),
                fields: vec![],
            };

            let filtered_users = ModelManager::<User>::find(filter_conditions.clone(), Some(filter_query_options)).await?;

            println!("--- 30岁以上用户 - 第 {} 页 ---", page);
            for (index, user) in filtered_users.iter().enumerate() {
                println!("   {}. {} - {}岁 - {} - 薪资: {:.2}",
                    index + 1, user.name, user.age, user.department, user.salary);
            }
            println!();
        }
    }

    // 6. 演示复杂分页导航
    println!("6. 🧭 复杂分页导航演示");
    println!("=====================");

    let nav_page_size = 7;
    let nav_total_count = ModelManager::<User>::count(vec![]).await?;
    let nav_total_pages = (nav_total_count + nav_page_size - 1) / nav_page_size;

    // 模拟跳转到第3页
    let current_page = 3;
    if current_page <= nav_total_pages {
        let skip = (current_page - 1) * nav_page_size;

        let nav_query_options = QueryOptions {
            conditions: vec![],
            sort: vec![],
            pagination: Some(PaginationConfig {
                skip,
                limit: nav_page_size,
            }),
            fields: vec![],
        };

        let nav_users = ModelManager::<User>::find(vec![], Some(nav_query_options)).await?;
        let nav_page_info = PageInfo::new(current_page as usize, nav_page_size as usize, nav_total_count as usize);

        println!("跳转到第 {} 页的显示结果:", current_page);
        nav_page_info.display();

        // 显示分页导航条
        print!("📑 导航: ");
        if nav_page_info.has_prev {
            print!("<上一页> ");
        }

        let start_page = if current_page > 2 { current_page - 2 } else { 1 };
        let end_page = std::cmp::min(start_page + 4, nav_total_pages);

        for page in start_page..=end_page {
            if page == current_page {
                print!("[{}] ", page);
            } else {
                print!("{} ", page);
            }
        }

        if nav_page_info.has_next {
            print!("<下一页>");
        }
        println!("\n");

        println!("当前页用户列表:");
        for (index, user) in nav_users.iter().enumerate() {
            println!("   {}. {} - {} - {}岁", index + 1, user.name, user.department, user.age);
        }
    }

    println!("\n=== 分页查询演示完成 ===");
    Ok(())
}