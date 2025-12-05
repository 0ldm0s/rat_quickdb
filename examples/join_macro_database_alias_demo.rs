//! join宏数据库别名功能演示
//! 展示如何为虚拟表指定数据库别名

#[cfg(debug_assertions)]
use rat_logger::debug;
use rat_quickdb::join_macro::*;
use rat_quickdb::join_macro::{JoinDefinition, JoinType};
use rat_quickdb::*;

// 定义用户模型（用于创建users表）
define_model! {
    /// 用户模型
    struct User {
        id: String,
        name: String,
        email: String,
    }
    collection = "users",
    database = "main_db",
    fields = {
        id: string_field(None, None, None).required().unique(),
        name: string_field(Some(100), Some(1), None).required(),
        email: string_field(Some(255), Some(1), None).required(),
    }
}

// 定义订单模型（用于创建orders表）
define_model! {
    /// 订单模型
    struct Order {
        id: String,
        user_id: String,
        product_name: String,
        amount: f64,
    }
    collection = "orders",
    database = "main_db",
    fields = {
        id: string_field(None, None, None).required().unique(),
        user_id: string_field(None, None, None).required(),
        product_name: string_field(Some(200), Some(1), None).required(),
        amount: float_field(None, None).required(),
    }
}

// 定义带有数据库别名的虚拟表（基于已存在的users和orders表）
define_join_table! {
    /// 用户订单详情虚拟表
    virtual_table UserOrderDetail {
        base_table: "users",
        database: "main_db",  // 指定数据库别名
        joins: [
            JoinDefinition {
                table: "orders".to_string(),
                database: None,  // 使用同一个数据库
                on_condition: "users.id = orders.user_id".to_string(),
                join_type: JoinType::Left,
            }
        ],
        fields: {
            user_id: "users.id as user_id",
            user_name: "users.name as user_name",
            user_email: "users.email as user_email",
            order_id: "orders.id as order_id",
            product_name: "orders.product_name as product_name",
            order_amount: "orders.amount as order_amount"
        }
    }
}

#[tokio::main]
async fn main() -> QuickDbResult<()> {
    println!("🚀 join宏数据库别名功能演示");
    println!("============================");

    // 设置测试数据库
    setup_database().await?;

    // 测试1：获取虚拟表的数据库别名信息
    let alias = UserOrderDetail::get_database_alias();
    let base = UserOrderDetail::get_base_name();
    println!("UserOrderDetail 虚拟表信息:");
    println!("  - 数据库别名: {:?}", alias);
    println!("  - 基础表: {}", base);
    assert_eq!(alias, Some("main_db".to_string()));
    assert_eq!(base, "users");

    // 测试2：验证SQL生成功能
    test_sql_generation();

    // 清理
    cleanup_database().await?;

    println!("\n✅ join宏数据库别名功能演示完成！");
    println!("现在join宏可以像define_model宏一样指定数据库别名了。");
    Ok(())
}

async fn setup_database() -> QuickDbResult<()> {
    // 删除旧文件
    if std::path::Path::new("join_demo.db").exists() {
        std::fs::remove_file("join_demo.db").ok();
    }

    // 创建数据库配置
    let config = DatabaseConfig {
        db_type: DatabaseType::SQLite,
        connection: ConnectionConfig::SQLite {
            path: "join_demo.db".to_string(),
            create_if_missing: true,
        },
        pool: PoolConfig::default(),
        alias: "main_db".to_string(),
        cache: None,
        id_strategy: IdStrategy::Uuid,
    };

    add_database(config).await?;
    set_default_alias("main_db").await?;

    println!("✅ 数据库设置完成");
    Ok(())
}

fn test_sql_generation() {
    println!("\n🔍 测试SQL生成功能:");

    // 创建虚拟表实例（使用UUID格式的ID）
    let virtual_table = UserOrderDetail {
        user_id: DataValue::String("550e8400-e29b-41d4-a716-446655440001".to_string()),
        user_name: DataValue::String("张三".to_string()),
        user_email: DataValue::String("zhang@example.com".to_string()),
        order_id: DataValue::String("550e8400-e29b-41d4-a716-446655440002".to_string()),
        product_name: DataValue::String("产品A".to_string()),
        order_amount: DataValue::Float(199.99),
    };

    // 创建查询条件（使用UUID格式的ID）
    let conditions = vec![QueryCondition {
        field: "users.id".to_string(),
        operator: QueryOperator::Eq,
        value: DataValue::String("550e8400-e29b-41d4-a716-446655440001".to_string()),
    }];

    let options = QueryOptions::default();

    // 生成SQL
    let (sql, params) = virtual_table.to_sql(&conditions, &options);

    println!("生成的SQL:");
    println!("{}", sql);
    println!("参数: {:?}", params);

    // 验证SQL包含预期内容
    assert!(sql.contains("SELECT"));
    assert!(sql.contains("FROM users"));
    assert!(sql.contains("LEFT JOIN orders ON users.id = orders.user_id"));
    assert!(sql.contains("WHERE"));

    println!("✅ SQL生成功能正常");
}

async fn cleanup_database() -> QuickDbResult<()> {
    // 保留测试文件以便检查
    println!("📁 保留测试文件：join_demo.db");
    Ok(())
}
