//! 虚拟表格宏测试示例
//! 验证define_join_table宏的功能和SQL生成

use rat_quickdb::*;
use rat_quickdb::define_join_table;
use rat_quickdb::types::*;
use rat_quickdb::join_macro::{JoinDefinition, JoinType};
use rat_quickdb::adapter::OrderClause;
use rat_quickdb::types::query::SortDirection;

// 定义虚拟表格：用户配置信息（用户表 + 配置表）
define_join_table! {
    /// 用户配置信息（包含用户基本信息和配置详情）
    virtual_table UserProfileInfo {
        base_table: "users",
        joins: [
            JoinDefinition {
                table: "profiles".to_string(),
                on_condition: "users.id = profiles.user_id".to_string(),
                join_type: JoinType::Left
            }
        ],
        fields: {
            user_id: "users.id as user_id",
            user_name: "users.name as user_name",
            user_email: "users.email as user_email",
            user_age: "users.age as user_age",
            profile_name: "profiles.name as profile_name",
            profile_bio: "profiles.bio as profile_bio",
            profile_avatar: "profiles.avatar_url as profile_avatar"
        }
    }
}

// 定义虚拟表格：文章统计信息（文章表 + 用户表 + 分类表）
define_join_table! {
    /// 文章统计信息
    virtual_table ArticleStats {
        base_table: "articles",
        joins: [
            JoinDefinition {
                table: "users".to_string(),
                on_condition: "articles.author_id = users.id".to_string(),
                join_type: JoinType::Inner
            },
            JoinDefinition {
                table: "categories".to_string(),
                on_condition: "articles.category_id = categories.id".to_string(),
                join_type: JoinType::Inner
            }
        ],
        fields: {
            article_id: "articles.id as article_id",
            article_title: "articles.title as article_title",
            author_name: "users.name as author_name",
            author_email: "users.email as author_email",
            category_name: "categories.name as category_name",
            article_views: "articles.views as article_views",
            article_created: "articles.created_at as article_created"
        }
    }
}

fn main() {
    println!("🚀 虚拟表格宏测试");
    println!("================");

    // 测试UserProfileInfo虚拟表格
    test_user_profile_info();

    // 测试ArticleStats虚拟表格
    test_article_stats();

    println!("✅ 所有测试完成！");
}

fn test_user_profile_info() {
    println!("\n📋 测试 UserProfileInfo 虚拟表格");
    println!("===============================");

    // 创建虚拟表格实例
    let profile = UserProfileInfo {
        user_id: DataValue::Int(1),
        user_name: DataValue::String("张三".to_string()),
        user_email: DataValue::String("zhangsan@example.com".to_string()),
        user_age: DataValue::Int(28),
        profile_name: DataValue::String("张三的配置".to_string()),
        profile_bio: DataValue::String("软件工程师".to_string()),
        profile_avatar: DataValue::String("https://example.com/avatar.jpg".to_string()),
    };

    println!("虚拟表格实例: {:?}", profile);

    // 生成查询条件
    let conditions = vec![
        QueryCondition {
            field: "user_age".to_string(),
            operator: QueryOperator::Gt,
            value: DataValue::Int(25),
        }
    ];

    let options = QueryOptions {
        pagination: Some(crate::types::query::PaginationConfig {
            skip: 0,
            limit: 10,
        }),
        sort: vec![crate::types::query::SortConfig {
            field: "user_created_at".to_string(),
            direction: SortDirection::Desc,
        }],
        ..Default::default()
    };

    // 生成SQL查询
    let (sql, params) = profile.to_sql(&conditions, &options);

    println!("生成的SQL: {}", sql);
    println!("参数: {:?}", params);
}

fn test_article_stats() {
    println!("\n📚 测试 ArticleStats 虚拟表格");
    println!("=========================");

    // 创建虚拟表格实例
    let stats = ArticleStats {
        article_id: DataValue::Int(1),
        article_title: DataValue::String("如何使用Rust开发数据库应用".to_string()),
        author_name: DataValue::String("李四".to_string()),
        author_email: DataValue::String("lisi@example.com".to_string()),
        category_name: DataValue::String("技术分享".to_string()),
        article_views: DataValue::Int(1500),
        article_created: DataValue::String("2023-12-01T10:00:00Z".to_string()),
    };

    println!("虚拟表格实例: {:?}", stats);

    // 生成查询条件
    let conditions = vec![
        QueryCondition {
            field: "category_name".to_string(),
            operator: QueryOperator::Eq,
            value: DataValue::String("技术分享".to_string()),
        },
        QueryCondition {
            field: "article_views".to_string(),
            operator: QueryOperator::Gt,
            value: DataValue::Int(100),
        }
    ];

    let options = QueryOptions {
        pagination: Some(crate::types::query::PaginationConfig {
            skip: 0,
            limit: 5,
        }),
        sort: vec![crate::types::query::SortConfig {
            field: "article_views".to_string(),
            direction: SortDirection::Desc,
        }],
        ..Default::default()
    };

    // 生成SQL查询
    let (sql, params) = stats.to_sql(&conditions, &options);

    println!("生成的SQL: {}", sql);
    println!("参数: {:?}", params);
}