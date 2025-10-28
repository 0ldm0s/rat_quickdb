//! 复杂JOIN查询测试示例
//! 验证多表混合JOIN类型的虚拟表格宏功能

use rat_quickdb::*;
use rat_quickdb::define_join_table;
use rat_quickdb::types::*;
use rat_quickdb::join_macro::{JoinDefinition, JoinType};
use rat_quickdb::adapter::OrderClause;
use rat_quickdb::types::query::SortDirection;

// 复杂的虚拟表格：电商订单详细信息（用户+订单+商品+分类+供应商+配送）
define_join_table! {
    /// 电商订单详细信息（包含用户、订单、商品、分类、供应商、配送信息）
    virtual_table ECommerceOrderDetail {
        base_table: "orders",
        joins: [
            // 内连接用户表（订单必须有用户）
            JoinDefinition {
                table: "users".to_string(),
                on_condition: "orders.user_id = users.id".to_string(),
                join_type: JoinType::Inner
            },
            // 左连接订单项表（订单可能有多个项目）
            JoinDefinition {
                table: "order_items".to_string(),
                on_condition: "orders.id = order_items.order_id".to_string(),
                join_type: JoinType::Left
            },
            // 左连接商品表（订单项必须有商品，但为了测试Left Join）
            JoinDefinition {
                table: "products".to_string(),
                on_condition: "order_items.product_id = products.id".to_string(),
                join_type: JoinType::Left
            },
            // 左连接商品分类表（商品可能没有分类）
            JoinDefinition {
                table: "categories".to_string(),
                on_condition: "products.category_id = categories.id".to_string(),
                join_type: JoinType::Left
            },
            // 右连接供应商表（测试Right Join，商品必须有供应商）
            JoinDefinition {
                table: "suppliers".to_string(),
                on_condition: "products.supplier_id = suppliers.id".to_string(),
                join_type: JoinType::Right
            },
            // 全连接配送表（测试Full Outer Join）
            JoinDefinition {
                table: "shipments".to_string(),
                on_condition: "orders.id = shipments.order_id".to_string(),
                join_type: JoinType::Full
            }
        ],
        fields: {
            // 订单信息
            order_id: "orders.id as order_id",
            order_number: "orders.order_number as order_number",
            order_date: "orders.created_at as order_date",
            order_status: "orders.status as order_status",
            order_total: "orders.total_amount as order_total",

            // 用户信息
            user_id: "users.id as user_id",
            user_name: "users.name as user_name",
            user_email: "users.email as user_email",
            user_phone: "users.phone as user_phone",

            // 订单项信息
            item_id: "order_items.id as item_id",
            item_quantity: "order_items.quantity as item_quantity",
            item_price: "order_items.unit_price as item_price",

            // 商品信息
            product_id: "products.id as product_id",
            product_name: "products.name as product_name",
            product_description: "products.description as product_description",

            // 分类信息
            category_id: "categories.id as category_id",
            category_name: "categories.name as category_name",

            // 供应商信息
            supplier_id: "suppliers.id as supplier_id",
            supplier_name: "suppliers.name as supplier_name",
            supplier_contact: "suppliers.contact_email as supplier_contact",

            // 配送信息
            shipment_id: "shipments.id as shipment_id",
            shipment_status: "shipments.status as shipment_status",
            tracking_number: "shipments.tracking_number as tracking_number"
        }
    }
}

// 复杂的虚拟表格：社交媒体内容分析（用户+内容+评论+点赞+标签+媒体）
define_join_table! {
    /// 社交媒体内容分析
    virtual_table SocialMediaAnalytics {
        base_table: "posts",
        joins: [
            // 内连接用户（帖子必须有作者）
            JoinDefinition {
                table: "users".to_string(),
                on_condition: "posts.user_id = users.id".to_string(),
                join_type: JoinType::Inner
            },
            // 左连接评论（帖子可能没有评论）
            JoinDefinition {
                table: "comments".to_string(),
                on_condition: "posts.id = comments.post_id".to_string(),
                join_type: JoinType::Left
            },
            // 右连接点赞（测试Right Join，点赞必须有帖子）
            JoinDefinition {
                table: "likes".to_string(),
                on_condition: "posts.id = likes.post_id".to_string(),
                join_type: JoinType::Right
            },
            // 全连接标签（测试Full Outer Join，标签可能没有帖子）
            JoinDefinition {
                table: "tags".to_string(),
                on_condition: "posts.id = tags.post_id".to_string(),
                join_type: JoinType::Full
            },
            // 左连接媒体文件（帖子可能没有媒体）
            JoinDefinition {
                table: "media".to_string(),
                on_condition: "posts.id = media.post_id".to_string(),
                join_type: JoinType::Left
            }
        ],
        fields: {
            // 帖子信息
            post_id: "posts.id as post_id",
            post_title: "posts.title as post_title",
            post_content: "posts.content as post_content",
            post_created: "posts.created_at as post_created",
            post_type: "posts.type as post_type",

            // 用户信息
            author_id: "users.id as author_id",
            author_name: "users.name as author_name",
            author_username: "users.username as author_username",
            author_avatar: "users.avatar_url as author_avatar",

            // 评论信息
            comment_id: "comments.id as comment_id",
            comment_content: "comments.content as comment_content",
            comment_created: "comments.created_at as comment_created",

            // 点赞信息
            like_id: "likes.id as like_id",
            like_user_id: "likes.user_id as like_user_id",
            like_created: "likes.created_at as like_created",

            // 标签信息
            tag_id: "tags.id as tag_id",
            tag_name: "tags.name as tag_name",
            tag_category: "tags.category as tag_category",

            // 媒体信息
            media_id: "media.id as media_id",
            media_type: "media.type as media_type",
            media_url: "media.url as media_url"
        }
    }
}

fn main() {
    println!("🚀 复杂JOIN查询测试");
    println!("==================");
    println!("测试多表混合JOIN类型的虚拟表格");
    println!();

    // 测试电商订单详细信息
    test_ecommerce_order_detail();

    // 测试社交媒体内容分析
    test_social_media_analytics();

    println!();
    println!("✅ 所有复杂测试完成！");
}

fn test_ecommerce_order_detail() {
    println!("📦 测试电商订单详细信息");
    println!("===========================");

    // 创建虚拟表格实例
    let order_detail = ECommerceOrderDetail {
        // 订单信息
        order_id: DataValue::Int(1001),
        order_number: DataValue::String("ORD-2023-1001".to_string()),
        order_date: DataValue::String("2023-12-01T10:30:00Z".to_string()),
        order_status: DataValue::String("shipped".to_string()),
        order_total: DataValue::Float(299.99),

        // 用户信息
        user_id: DataValue::Int(501),
        user_name: DataValue::String("张小明".to_string()),
        user_email: DataValue::String("zhangxiaoming@example.com".to_string()),
        user_phone: DataValue::String("13800138000".to_string()),

        // 订单项信息
        item_id: DataValue::Int(2001),
        item_quantity: DataValue::Int(2),
        item_price: DataValue::Float(149.99),

        // 商品信息
        product_id: DataValue::Int(301),
        product_name: DataValue::String("智能手表 Pro".to_string()),
        product_description: DataValue::String("高端智能手表，支持健康监测".to_string()),

        // 分类信息
        category_id: DataValue::Int(401),
        category_name: DataValue::String("电子产品".to_string()),

        // 供应商信息
        supplier_id: DataValue::Int(601),
        supplier_name: DataValue::String("科技供应商有限公司".to_string()),
        supplier_contact: DataValue::String("contact@tecsupplier.com".to_string()),

        // 配送信息
        shipment_id: DataValue::Int(701),
        shipment_status: DataValue::String("in_transit".to_string()),
        tracking_number: DataValue::String("SF1234567890".to_string()),
    };

    println!("虚拟表格实例创建成功，包含 {} 个字段", 23);

    // 测试复杂查询条件
    let conditions = vec![
        QueryCondition {
            field: "order_status".to_string(),
            operator: QueryOperator::Eq,
            value: DataValue::String("shipped".to_string()),
        },
        QueryCondition {
            field: "order_total".to_string(),
            operator: QueryOperator::Gt,
            value: DataValue::Float(100.0),
        },
        QueryCondition {
            field: "category_name".to_string(),
            operator: QueryOperator::Eq,
            value: DataValue::String("电子产品".to_string()),
        }
    ];

    let options = QueryOptions {
        pagination: Some(crate::types::query::PaginationConfig {
            skip: 0,
            limit: 20,
        }),
        sort: vec![
            crate::types::query::SortConfig {
                field: "order_date".to_string(),
                direction: SortDirection::Desc,
            },
            crate::types::query::SortConfig {
                field: "order_total".to_string(),
                direction: SortDirection::Desc,
            }
        ],
        ..Default::default()
    };

    // 生成SQL查询
    let (sql, params) = order_detail.to_sql(&conditions, &options);

    println!("生成的SQL长度: {} 字符", sql.len());
    println!("参数数量: {} 个", params.len());
    println!("SQL预览（前200字符）: {}...", &sql[..200]);
    println!("完整SQL:");
    println!("{}", sql);
    println!("参数: {:?}", params);
    println!();
}

fn test_social_media_analytics() {
    println!("📱 测试社交媒体内容分析");
    println!("==========================");

    // 创建虚拟表格实例
    let analytics = SocialMediaAnalytics {
        // 帖子信息
        post_id: DataValue::Int(8001),
        post_title: DataValue::String("Rust数据库开发心得".to_string()),
        post_content: DataValue::String("最近在开发一个Rust数据库库，收获很多...".to_string()),
        post_created: DataValue::String("2023-12-01T15:45:00Z".to_string()),
        post_type: DataValue::String("article".to_string()),

        // 用户信息
        author_id: DataValue::Int(9001),
        author_name: DataValue::String("李技术".to_string()),
        author_username: DataValue::String("tech_li".to_string()),
        author_avatar: DataValue::String("https://example.com/avatar.jpg".to_string()),

        // 评论信息
        comment_id: DataValue::Int(10001),
        comment_content: DataValue::String("很实用的分享！学到了很多".to_string()),
        comment_created: DataValue::String("2023-12-01T16:00:00Z".to_string()),

        // 点赞信息
        like_id: DataValue::Int(11001),
        like_user_id: DataValue::Int(12001),
        like_created: DataValue::String("2023-12-01T16:15:00Z".to_string()),

        // 标签信息
        tag_id: DataValue::Int(13001),
        tag_name: DataValue::String("Rust".to_string()),
        tag_category: DataValue::String("编程语言".to_string()),

        // 媒体信息
        media_id: DataValue::Int(14001),
        media_type: DataValue::String("image".to_string()),
        media_url: DataValue::String("https://example.com/media.jpg".to_string()),
    };

    println!("虚拟表格实例创建成功，包含 {} 个字段", 20);

    // 测试社交媒体查询条件
    let conditions = vec![
        QueryCondition {
            field: "post_type".to_string(),
            operator: QueryOperator::Eq,
            value: DataValue::String("article".to_string()),
        },
        QueryCondition {
            field: "tag_name".to_string(),
            operator: QueryOperator::Eq,
            value: DataValue::String("Rust".to_string()),
        },
        QueryCondition {
            field: "post_created".to_string(),
            operator: QueryOperator::Gte,
            value: DataValue::String("2023-12-01T00:00:00Z".to_string()),
        }
    ];

    let options = QueryOptions {
        pagination: Some(crate::types::query::PaginationConfig {
            skip: 0,
            limit: 50,
        }),
        sort: vec![
            crate::types::query::SortConfig {
                field: "post_created".to_string(),
                direction: SortDirection::Desc,
            }
        ],
        ..Default::default()
    };

    // 生成SQL查询
    let (sql, params) = analytics.to_sql(&conditions, &options);

    println!("生成的SQL长度: {} 字符", sql.len());
    println!("参数数量: {} 个", params.len());
    println!("SQL预览（前200字符）: {}...", &sql[..200]);
    println!("完整SQL:");
    println!("{}", sql);
    println!("参数: {:?}", params);
    println!();
}