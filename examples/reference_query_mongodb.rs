//! RatQuickDB MongoDB 关联查询完整演示
//!
//! 本示例专门展示关联字段（reference_field）的各种查询方式，包括：
//! - 等值查询（=）：查询关联到特定记录的数据
//! - 不等查询（!=）：查询不关联到特定记录的数据
//! - IN 查询：查询关联到多个记录中任一个的数据
//! - NOT IN 查询：查询不关联到多个记录中任一个的数据
//! - IS NULL 查询：查询未关联任何记录的数据
//! - IS NOT NULL 查询：查询已关联记录的数据
//!
//! 📊 关联查询场景：
//! - 部门（Department）↔ 员工（Employee）：一个部门有多个员工
//! - 项目（Project）↔ 任务（Task）：一个项目有多个任务
//!
//! 🎯 重点：仅关注关联字段的查询操作

use chrono::Utc;
use rat_logger::{LevelFilter, LoggerBuilder, handler::term::TermConfig};
use rat_quickdb::types::*;
use rat_quickdb::*;
use rat_quickdb::{
    ModelManager, ModelOperations, boolean_field, datetime_field, float_field, integer_field,
    string_field, uuid_field,
};
use serde::{Deserialize, Serialize};

// 部门模型 - 被关联的模型
define_model! {
    struct Department {
        id: String,
        name: String,
        code: String,
        description: String,
        budget: f64,
        established_date: chrono::DateTime<chrono::Utc>,
    }
    collection = "ref_demo_departments",
    database = "main",
    fields = {
        id: uuid_field().required().unique(),
        name: string_field(None, None, None).required(),
        code: string_field(None, None, None).required().unique(),
        description: string_field(None, None, None).required(),
        budget: float_field(None, None).required(),
        established_date: datetime_field().required(),
    }
    indexes = [
        { fields: ["name"], unique: false, name: "idx_dept_name" },
        { fields: ["code"], unique: true, name: "idx_dept_code" },
    ],
}

// 员工模型 - 包含关联字段的模型
define_model! {
    struct Employee {
        id: String,
        name: String,
        email: String,
        department_id: String,  // 关联字段：指向 Department.id
        position: String,
        salary: f64,
        is_active: bool,
        hire_date: chrono::DateTime<chrono::Utc>,
    }
    collection = "ref_demo_employees",
    database = "main",
    fields = {
        id: uuid_field().required().unique(),
        name: string_field(None, None, None).required(),
        email: string_field(None, None, None).required().unique(),
        department_id: uuid_field().required(),  // 关联字段使用 uuid_field
        position: string_field(None, None, None).required(),
        salary: float_field(None, None).required(),
        is_active: boolean_field().required(),
        hire_date: datetime_field().required(),
    }
    indexes = [
        { fields: ["name"], unique: false, name: "idx_emp_name" },
        { fields: ["email"], unique: true, name: "idx_emp_email" },
        { fields: ["department_id"], unique: false, name: "idx_emp_dept" },
        { fields: ["is_active"], unique: false, name: "idx_emp_active" },
    ],
}

// 项目模型
define_model! {
    struct Project {
        id: String,
        name: String,
        department_id: String,  // 关联字段：指向 Department.id
        status: String,
        priority: i32,
        start_date: chrono::DateTime<chrono::Utc>,
    }
    collection = "ref_demo_projects",
    database = "main",
    fields = {
        id: uuid_field().required().unique(),
        name: string_field(None, None, None).required(),
        department_id: reference_field("Department".to_string()).required(),
        status: string_field(None, None, None).required(),
        priority: integer_field(None, None).required(),
        start_date: datetime_field().required(),
    }
    indexes = [
        { fields: ["department_id"], unique: false, name: "idx_proj_dept" },
        { fields: ["status"], unique: false, name: "idx_proj_status" },
    ],
}

// 任务模型 - 可选关联字段
define_model! {
    struct Task {
        id: String,
        title: String,
        project_id: Option<String>,  // 可选关联字段
        assignee_id: String,  // 关联到 Employee.id
        status: String,
        due_date: chrono::DateTime<chrono::Utc>,
    }
    collection = "ref_demo_tasks",
    database = "main",
    fields = {
        id: uuid_field().required().unique(),
        title: string_field(None, None, None).required(),
        project_id: uuid_field(),  // 可选关联字段
        assignee_id: uuid_field().required(),  // 关联字段使用 uuid_field
        status: string_field(None, None, None).required(),
        due_date: datetime_field().required(),
    }
    indexes = [
        { fields: ["project_id"], unique: false, name: "idx_task_project" },
        { fields: ["assignee_id"], unique: false, name: "idx_task_assignee" },
    ],
}

// 清理测试数据
async fn cleanup_test_data() {
    println!("清理测试数据...");

    let tables = vec![
        ("main", "ref_demo_departments"),
        ("main", "ref_demo_employees"),
        ("main", "ref_demo_projects"),
        ("main", "ref_demo_tasks"),
    ];

    for (db, table) in tables {
        if let Err(e) = rat_quickdb::drop_table(db, table).await {
            println!("清理表 {} 失败: {}", table, e);
        }
    }
}

// 创建测试数据
async fn create_test_data() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    println!("创建测试数据...");

    let mut department_ids = Vec::new();

    // 创建部门
    let departments = vec![
        ("技术部", "TECH", "负责技术研发和创新", 1000000.0),
        ("销售部", "SALES", "负责产品销售和客户关系", 500000.0),
        ("人事部", "HR", "负责人力资源管理", 300000.0),
        ("财务部", "FINANCE", "负责财务管理", 400000.0),
        ("市场部", "MARKETING", "负责市场营销", 600000.0),
    ];

    for (name, code, desc, budget) in departments {
        let dept = Department {
            id: String::new(),
            name: name.to_string(),
            code: code.to_string(),
            description: desc.to_string(),
            budget,
            established_date: Utc::now(),
        };
        let dept_id = dept.save().await?;
        println!("  创建部门: {} (ID: {})", name, dept_id);
        department_ids.push(dept_id);
    }

    // 为每个部门创建员工
    println!("  创建员工...");
    for (i, dept_id) in department_ids.iter().enumerate() {
        for j in 0..3 {
            let emp = Employee {
                id: String::new(),
                name: format!("员工 {}-{}", i, j),
                email: format!("emp{}_{}@example.com", i, j),
                department_id: dept_id.clone(),
                position: format!("职位_{}", j),
                salary: 10000.0 + (j as f64 * 1000.0),
                is_active: true,
                hire_date: Utc::now(),
            };
            emp.save().await?;
        }
    }

    // 创建项目
    println!("  创建项目...");
    for (i, dept_id) in department_ids.iter().enumerate() {
        for j in 0..2 {
            let project = Project {
                id: String::new(),
                name: format!("项目 {}-{}", i, j),
                department_id: dept_id.clone(),
                status: if j % 2 == 0 { "进行中" } else { "已完成" }.to_string(),
                priority: (j + 1) * 10,
                start_date: Utc::now(),
            };
            project.save().await?;
        }
    }

    // 获取所有员工ID用于任务分配
    let all_employees = ModelManager::<Employee>::find(vec![], None).await?;
    let all_projects = ModelManager::<Project>::find(vec![], None).await?;

    // 创建任务（部分关联到项目，部分不关联）
    println!("  创建任务...");
    for (i, emp) in all_employees.iter().enumerate() {
        let task = Task {
            id: String::new(),
            title: format!("任务 {}", i),
            project_id: if i < all_projects.len() {
                Some(all_projects[i].id.clone())
            } else {
                None  // 部分任务不关联项目
            },
            assignee_id: emp.id.clone(),
            status: "待处理".to_string(),
            due_date: Utc::now(),
        };
        task.save().await?;
    }

    println!("✅ 测试数据创建完成");
    Ok(department_ids)
}

// 1. 等值查询（=）：查询关联到特定记录的数据
async fn demo_equal_reference_query() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n【1】等值查询（=）- 查询特定部门的员工");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // 获取第一个部门
    let departments = ModelManager::<Department>::find(vec![], None).await?;
    let target_dept = &departments[0];

    println!("📌 查询条件: department_id = '{}'", target_dept.id);
    println!("   （查询属于 {} 的员工）", target_dept.name);

    let conditions = vec![QueryCondition {
        field: "department_id".to_string(),
        operator: QueryOperator::Eq,
        value: DataValue::String(target_dept.id.clone()),
    }];

    let employees = ModelManager::<Employee>::find(conditions, None).await?;

    println!("📊 查询结果: 找到 {} 名员工", employees.len());
    for (i, emp) in employees.iter().enumerate() {
        println!("   {}. {} - {} - 薪资: ¥{:.2}",
            i + 1, emp.name, emp.position, emp.salary);
    }

    Ok(())
}

// 2. 不等查询（!=）：查询不关联到特定记录的数据
async fn demo_not_equal_reference_query() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n【2】不等查询（!=）- 查询不属于特定部门的员工");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let departments = ModelManager::<Department>::find(vec![], None).await?;
    let exclude_dept = &departments[0];

    println!("📌 查询条件: department_id != '{}'", exclude_dept.id);
    println!("   （查询不属于 {} 的员工）", exclude_dept.name);

    let conditions = vec![QueryCondition {
        field: "department_id".to_string(),
        operator: QueryOperator::Ne,
        value: DataValue::String(exclude_dept.id.clone()),
    }];

    let employees = ModelManager::<Employee>::find(conditions, None).await?;

    println!("📊 查询结果: 找到 {} 名员工", employees.len());

    // 按部门分组显示
    let mut dept_groups: std::collections::HashMap<String, Vec<&Employee>> = std::collections::HashMap::new();
    for emp in &employees {
        dept_groups.entry(emp.department_id.clone())
            .or_insert_with(Vec::new)
            .push(emp);
    }

    for (dept_id, emps) in dept_groups {
        // 获取部门名称
        if let Ok(depts) = ModelManager::<Department>::find(vec![
            QueryCondition {
                field: "id".to_string(),
                operator: QueryOperator::Eq,
                value: DataValue::String(dept_id.clone()),
            }
        ], None).await {
            if let Some(dept) = depts.first() {
                println!("   {}:", dept.name);
                for emp in emps {
                    println!("     - {}", emp.name);
                }
            }
        }
    }

    Ok(())
}

// 3. IN 查询：查询关联到多个记录中任一个的数据
async fn demo_in_reference_query() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n【3】IN 查询 - 查询属于多个指定部门的员工");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let departments = ModelManager::<Department>::find(vec![], None).await?;

    // 选择前3个部门
    let target_depts: Vec<&Department> = departments.iter().take(3).collect();
    let target_dept_ids: Vec<String> = target_depts.iter().map(|d| d.id.clone()).collect();
    let target_dept_names: Vec<&str> = target_depts.iter().map(|d| d.name.as_str()).collect();

    println!("📌 查询条件: department_id IN [...]");
    println!("   （查询属于以下部门的员工）");
    for (i, dept) in target_depts.iter().enumerate() {
        println!("     {}. {}", i + 1, dept.name);
    }

    let conditions = vec![QueryCondition {
        field: "department_id".to_string(),
        operator: QueryOperator::In,
        value: DataValue::Array(
            target_dept_ids.iter().map(|id| DataValue::String(id.clone())).collect()
        ),
    }];

    let employees = ModelManager::<Employee>::find(conditions, None).await?;

    println!("📊 查询结果: 找到 {} 名员工", employees.len());

    // 按部门分组显示
    let mut dept_groups: std::collections::HashMap<String, Vec<&Employee>> = std::collections::HashMap::new();
    for emp in &employees {
        dept_groups.entry(emp.department_id.clone())
            .or_insert_with(Vec::new)
            .push(emp);
    }

    for dept_id in target_dept_ids {
        if let Some(emps) = dept_groups.get(&dept_id) {
            if let Ok(depts) = ModelManager::<Department>::find(vec![
                QueryCondition {
                    field: "id".to_string(),
                    operator: QueryOperator::Eq,
                    value: DataValue::String(dept_id.clone()),
                }
            ], None).await {
                if let Some(dept) = depts.first() {
                    println!("   {} ({} 名员工):", dept.name, emps.len());
                    for emp in emps {
                        println!("     - {}", emp.name);
                    }
                }
            }
        }
    }

    Ok(())
}

// 4. NOT IN 查询：查询不关联到多个记录中任一个的数据
async fn demo_not_in_reference_query() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n【4】NOT IN 查询 - 查询不属于多个指定部门的员工");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let departments = ModelManager::<Department>::find(vec![], None).await?;

    // 排除前2个部门
    let exclude_depts: Vec<&Department> = departments.iter().take(2).collect();
    let exclude_dept_ids: Vec<String> = exclude_depts.iter().map(|d| d.id.clone()).collect();
    let exclude_dept_names: Vec<&str> = exclude_depts.iter().map(|d| d.name.as_str()).collect();

    println!("📌 查询条件: department_id NOT IN [...]");
    println!("   （查询不属于以下部门的员工）");
    for (i, dept) in exclude_depts.iter().enumerate() {
        println!("     {}. {}", i + 1, dept.name);
    }

    // MongoDB 不直接支持 NotIn 操作，使用组合条件实现
    // 方式1：多次 != 条件组合
    let mut conditions = Vec::new();
    for dept_id in &exclude_dept_ids {
        conditions.push(QueryCondition {
            field: "department_id".to_string(),
            operator: QueryOperator::Ne,
            value: DataValue::String(dept_id.clone()),
        });
    }

    let employees = ModelManager::<Employee>::find(conditions, None).await?;

    println!("📊 查询结果: 找到 {} 名员工", employees.len());

    // 按部门分组显示
    let mut dept_groups: std::collections::HashMap<String, Vec<&Employee>> = std::collections::HashMap::new();
    for emp in &employees {
        dept_groups.entry(emp.department_id.clone())
            .or_insert_with(Vec::new)
            .push(emp);
    }

    for (dept_id, emps) in dept_groups {
        if let Ok(depts) = ModelManager::<Department>::find(vec![
            QueryCondition {
                field: "id".to_string(),
                operator: QueryOperator::Eq,
                value: DataValue::String(dept_id.clone()),
            }
        ], None).await {
            if let Some(dept) = depts.first() {
                println!("   {} ({} 名员工):", dept.name, emps.len());
                for emp in emps {
                    println!("     - {}", emp.name);
                }
            }
        }
    }

    Ok(())
}

// 5. IS NULL 查询：查询未关联任何记录的数据
async fn demo_is_null_reference_query() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n【5】IS NULL 查询 - 查询未关联项目的任务");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    println!("📌 查询条件: project_id IS NULL");
    println!("   （查询未分配到任何项目的任务）");

    let conditions = vec![QueryCondition {
        field: "project_id".to_string(),
        operator: QueryOperator::IsNull,
        value: DataValue::Null,
    }];

    let tasks = ModelManager::<Task>::find(conditions, None).await?;

    println!("📊 查询结果: 找到 {} 个未关联项目的任务", tasks.len());

    for (i, task) in tasks.iter().enumerate() {
        println!("   {}. {} - 状态: {} - 负责人ID: {}",
            i + 1, task.title, task.status, task.assignee_id);
    }

    Ok(())
}

// 6. IS NOT NULL 查询：查询已关联记录的数据
async fn demo_is_not_null_reference_query() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n【6】IS NOT NULL 查询 - 查询已关联项目的任务");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    println!("📌 查询条件: project_id IS NOT NULL");
    println!("   （查询已分配到项目的任务）");

    let conditions = vec![QueryCondition {
        field: "project_id".to_string(),
        operator: QueryOperator::IsNotNull,
        value: DataValue::Null,
    }];

    let tasks = ModelManager::<Task>::find(conditions, None).await?;

    println!("📊 查询结果: 找到 {} 个已关联项目的任务", tasks.len());

    for (i, task) in tasks.iter().take(5).enumerate() {
        let project_id = task.project_id.as_ref().map(|id| id.as_str()).unwrap_or("无");
        println!("   {}. {} - 项目ID: {} - 状态: {}",
            i + 1, task.title, project_id, task.status);
    }

    if tasks.len() > 5 {
        println!("   ... 还有 {} 个任务", tasks.len() - 5);
    }

    Ok(())
}

// 7. 组合查询：关联字段 + 其他字段条件
async fn demo_combined_reference_query() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n【7】组合查询 - 关联字段 + 其他条件");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let departments = ModelManager::<Department>::find(vec![], None).await?;
    let target_dept = &departments[0];

    println!("📌 查询条件:");
    println!("   department_id = '{}' (属于 {})", target_dept.id, target_dept.name);
    println!("   AND");
    println!("   salary >= 11000.0 (薪资 >= 11000)");

    let conditions = vec![
        QueryCondition {
            field: "department_id".to_string(),
            operator: QueryOperator::Eq,
            value: DataValue::String(target_dept.id.clone()),
        },
        QueryCondition {
            field: "salary".to_string(),
            operator: QueryOperator::Gte,
            value: DataValue::Float(11000.0),
        },
    ];

    let employees = ModelManager::<Employee>::find(conditions, None).await?;

    println!("📊 查询结果: 找到 {} 名员工", employees.len());

    for (i, emp) in employees.iter().enumerate() {
        println!("   {}. {} - {} - 薪资: ¥{:.2}",
            i + 1, emp.name, emp.position, emp.salary);
    }

    Ok(())
}

// 8. 多级关联查询：通过中间表查询
async fn demo_multi_level_reference_query() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n【8】多级关联查询 - 查询特定部门的任务");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let departments = ModelManager::<Department>::find(vec![], None).await?;
    let target_dept = &departments[0];

    println!("📌 查询目标: 查询 {} 的所有任务", target_dept.name);
    println!("   （通过员工作为中间关联）");

    // 第一步：查询该部门的所有员工
    println!("\n   步骤1: 查询 {} 的员工", target_dept.name);
    let emp_conditions = vec![QueryCondition {
        field: "department_id".to_string(),
        operator: QueryOperator::Eq,
        value: DataValue::String(target_dept.id.clone()),
    }];
    let employees = ModelManager::<Employee>::find(emp_conditions, None).await?;
    println!("   找到 {} 名员工", employees.len());

    // 第二步：查询这些员工的任务
    println!("\n   步骤2: 查询这些员工负责的任务");
    let emp_ids: Vec<String> = employees.iter().map(|e| e.id.clone()).collect();

    let task_conditions = vec![QueryCondition {
        field: "assignee_id".to_string(),
        operator: QueryOperator::In,
        value: DataValue::Array(
            emp_ids.iter().map(|id| DataValue::String(id.clone())).collect()
        ),
    }];
    let tasks = ModelManager::<Task>::find(task_conditions, None).await?;

    println!("📊 查询结果: 找到 {} 个任务", tasks.len());

    for (i, task) in tasks.iter().take(5).enumerate() {
        println!("   {}. {} - 状态: {}", i + 1, task.title, task.status);
    }

    if tasks.len() > 5 {
        println!("   ... 还有 {} 个任务", tasks.len() - 5);
    }

    Ok(())
}

// 9. 排序和分页 + 关联查询
async fn demo_sorted_paginated_reference_query() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n【9】排序和分页 + 关联查询");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let departments = ModelManager::<Department>::find(vec![], None).await?;
    let target_dept = &departments[0];

    println!("📌 查询条件: department_id = '{}' (属于 {})", target_dept.id, target_dept.name);
    println!("   排序: salary DESC (薪资从高到低)");
    println!("   分页: limit 3 (只显示前3条)");

    let conditions = vec![QueryCondition {
        field: "department_id".to_string(),
        operator: QueryOperator::Eq,
        value: DataValue::String(target_dept.id.clone()),
    }];

    let options = QueryOptions {
        sort: vec![SortConfig {
            field: "salary".to_string(),
            direction: SortDirection::Desc,
        }],
        pagination: Some(PaginationConfig {
            skip: 0,
            limit: 3,
        }),
        ..Default::default()
    };

    let employees = ModelManager::<Employee>::find(conditions, Some(options)).await?;

    println!("📊 查询结果: 该部门薪资最高的 3 名员工");

    for (i, emp) in employees.iter().enumerate() {
        println!("   {}. {} - {} - 薪资: ¥{:.2}",
            i + 1, emp.name, emp.position, emp.salary);
    }

    Ok(())
}

// 10. 统计查询：按关联字段分组统计
async fn demo_count_by_reference() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n【10】统计查询 - 统计每个部门的员工数");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let departments = ModelManager::<Department>::find(vec![], None).await?;

    println!("📊 各部门员工统计:");

    for dept in departments {
        let conditions = vec![QueryCondition {
            field: "department_id".to_string(),
            operator: QueryOperator::Eq,
            value: DataValue::String(dept.id.clone()),
        }];

        let count = ModelManager::<Employee>::count(conditions).await?;
        println!("   {} - {} 名员工", dept.name, count);
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║  RatQuickDB MongoDB 关联查询完整演示                      ║");
    println!("║  专注展示 reference_field 的各种查询方式                  ║");
    println!("╚════════════════════════════════════════════════════════════╝");

    // 初始化日志
    LoggerBuilder::new()
        .with_level(LevelFilter::Warn)
        .add_terminal_with_config(TermConfig::default())
        .init()?;

    // 初始化数据库
    println!("\n📡 连接数据库...");
    let db_config = DatabaseConfig::builder()
        .db_type(DatabaseType::MongoDB)
        .connection(ConnectionConfig::MongoDB {
            host: "172.16.0.94".to_string(),
            port: 27017,
            database: "testdb".to_string(),
            username: Some("testdb".to_string()),
            password: Some("testdb123456".to_string()),
            auth_source: Some("testdb".to_string()),
            direct_connection: true,
            tls_config: None,
            zstd_config: None,
            options: None,
        })
        .pool(
            PoolConfig::builder()
                .max_connections(5)
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
    println!("✅ 数据库连接成功");

    // 清理并创建测试数据
    cleanup_test_data().await;
    create_test_data().await?;

    // 执行各种关联查询演示
    demo_equal_reference_query().await?;
    demo_not_equal_reference_query().await?;
    demo_in_reference_query().await?;
    demo_not_in_reference_query().await?;
    demo_is_null_reference_query().await?;
    demo_is_not_null_reference_query().await?;
    demo_combined_reference_query().await?;
    demo_multi_level_reference_query().await?;
    demo_sorted_paginated_reference_query().await?;
    demo_count_by_reference().await?;

    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║  演示完成！                                                 ║");
    println!("║  所有数据保留在数据库中，可手动检查                        ║");
    println!("╚════════════════════════════════════════════════════════════╝");

    Ok(())
}
