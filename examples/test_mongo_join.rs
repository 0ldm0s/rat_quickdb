//! MongoDB虚拟表格宏测试示例
//! 验证define_join_table宏在MongoDB聚合管道下的功能

use rat_quickdb::types::*;
use rat_quickdb::*;
use serde_json::{Value as JsonValue, json};

// MongoDB专用的虚拟表格定义
// 我们需要增强宏来支持不同的数据库类型输出

/// MongoDB JOIN操作定义
#[derive(Debug, Clone)]
pub struct MongoJoin {
    pub from: String,
    pub local_field: String,
    pub foreign_field: String,
    pub as_field: String,
    pub join_type: MongoJoinType,
}

/// MongoDB JOIN类型（对应SQL的JOIN类型）
#[derive(Debug, Clone, PartialEq)]
pub enum MongoJoinType {
    Left,  // $lookup + $unwind with preserveNullAndEmptyArrays: true
    Inner, // $lookup + $unwind with preserveNullAndEmptyArrays: false
    Right, // 需要特殊处理，或者交换lookup方向
    Full,  // 需要union处理，复杂度较高
}

/// MongoDB虚拟表格宏（POC版本）
macro_rules! define_mongo_join_table {
    (
        $(#[$attr:meta])*
        $vis:vis virtual_table $struct_name:ident {
            base_collection: $base_collection:expr,
            joins: [$($join:expr),* $(,)?],
            fields: {
                $($field:ident: $pipeline:expr),* $(,)?
            }
        }
    ) => {
        // 生成字段结构体
        #[derive(Debug, Clone)]
        $vis struct $struct_name {
            $(
                $field: DataValue,
            )*
        }

        impl $struct_name {
            /// 生成MongoDB聚合管道
            pub fn to_mongo_pipeline(&self, conditions: &[QueryCondition], options: &QueryOptions) -> (Vec<JsonValue>, Vec<DataValue>) {
                let mut pipeline = Vec::new();
                let mut params = Vec::new();

                // 1. 添加JOIN阶段
                let joins = vec![$($join),*];
                for join in &joins {
                    let lookup_stage = json!({
                        "$lookup": {
                            "from": join.from,
                            "localField": join.local_field,
                            "foreignField": join.foreign_field,
                            "as": join.as_field
                        }
                    });
                    pipeline.push(lookup_stage);

                    // 根据JOIN类型决定如何处理unwind
                    let preserve_nulls = match join.join_type {
                        MongoJoinType::Left => true,
                        MongoJoinType::Inner => false,
                        MongoJoinType::Right => {
                            // Right JOIN需要特殊处理，暂时当作Left处理
                            true
                        },
                        MongoJoinType::Full => {
                            // Full JOIN需要更复杂的处理，暂时当作Left处理
                            true
                        }
                    };

                    let unwind_stage = json!({
                        "$unwind": {
                            "path": format!("${}", join.as_field),
                            "preserveNullAndEmptyArrays": preserve_nulls
                        }
                    });
                    pipeline.push(unwind_stage);
                }

                // 2. 添加字段映射阶段
                let mut project_fields = serde_json::Map::new();
                let fields = vec![$($pipeline),*];
                for field_expr in &fields {
                    // 解析字段表达式，转换为MongoDB字段映射
                    if let Some((field_name, expression)) = self.parse_field_expression(field_expr) {
                        project_fields.insert(field_name, expression);
                    }
                }

                let project_stage = json!({
                    "$project": project_fields
                });
                pipeline.push(project_stage);

                // 3. 添加查询条件阶段
                if !conditions.is_empty() {
                    let mut match_conditions = serde_json::Map::new();
                    for condition in conditions {
                        let (field_path, mongo_op) = self.convert_condition_to_mongo(condition, &mut params);
                        match_conditions.insert(field_path, mongo_op);
                    }

                    let match_stage = json!({
                        "$match": match_conditions
                    });
                    pipeline.push(match_stage);
                }

                // 4. 添加排序阶段
                if !options.sort.is_empty() {
                    let mut sort_fields = serde_json::Map::new();
                    for sort_config in &options.sort {
                        let sort_value = match sort_config.direction {
                            crate::types::query::SortDirection::Asc => 1,
                            crate::types::query::SortDirection::Desc => -1,
                        };
                        sort_fields.insert(sort_config.field.clone(), JsonValue::Number(sort_value.into()));
                    }

                    let sort_stage = json!({
                        "$sort": sort_fields
                    });
                    pipeline.push(sort_stage);
                }

                // 5. 添加分页阶段
                if let Some(pagination) = &options.pagination {
                    if pagination.skip > 0 {
                        pipeline.push(json!({
                            "$skip": pagination.skip
                        }));
                    }

                    pipeline.push(json!({
                        "$limit": pagination.limit
                    }));
                }

                (pipeline, params)
            }

            /// 解析字段表达式（简化版）
            fn parse_field_expression(&self, expr: &str) -> Option<(String, JsonValue)> {
                // 简单解析：将 "table.field as alias" 转换为MongoDB字段引用
                if let Some((field_expr, alias)) = expr.split_once(" as ") {
                    let field_ref = if field_expr.contains('.') {
                        // 嵌套字段引用
                        format!("${}", field_expr)
                    } else {
                        // 基础字段引用
                        format!("${}", field_expr)
                    };
                    Some((alias.to_string(), JsonValue::String(field_ref)))
                } else if expr.contains('.') {
                    // 嵌套字段
                    Some((expr.to_string(), JsonValue::String(format!("${}", expr))))
                } else {
                    // 基础字段
                    Some((expr.to_string(), JsonValue::String(format!("${}", expr))))
                }
            }

            /// 将查询条件转换为MongoDB格式
            fn convert_condition_to_mongo(&self, condition: &QueryCondition, params: &mut Vec<DataValue>) -> (String, JsonValue) {
                let operator = match condition.operator {
                    QueryOperator::Eq => "$eq",
                    QueryOperator::Ne => "$ne",
                    QueryOperator::Gt => "$gt",
                    QueryOperator::Gte => "$gte",
                    QueryOperator::Lt => "$lt",
                    QueryOperator::Lte => "$lte",
                    QueryOperator::In => "$in",
                    QueryOperator::NotIn => "$nin",
                    QueryOperator::Contains => "$regex",
                    QueryOperator::StartsWith => "$regex",
                    QueryOperator::EndsWith => "$regex",
                    QueryOperator::Regex => "$regex",
                    QueryOperator::Exists => "$exists",
                    QueryOperator::IsNull => "$eq",
                    QueryOperator::IsNotNull => "$ne",
                };

                let value = match &condition.value {
                    DataValue::String(s) => {
                        if matches!(condition.operator, QueryOperator::Contains) {
                            JsonValue::String(format!(".*{}.*", s))
                        } else if matches!(condition.operator, QueryOperator::StartsWith) {
                            JsonValue::String(format!("^{}.*", s))
                        } else if matches!(condition.operator, QueryOperator::EndsWith) {
                            JsonValue::String(format!(".*{}$", s))
                        } else {
                            JsonValue::String(s.clone())
                        }
                    },
                    DataValue::Int(i) => JsonValue::Number((*i).into()),
                    DataValue::Float(f) => JsonValue::Number(serde_json::Number::from_f64(*f).unwrap_or(0.into())),
                    DataValue::Bool(b) => JsonValue::Bool(*b),
                    DataValue::Null => JsonValue::Null,
                    _ => JsonValue::String(format!("{:?}", condition.value)),
                };

                if matches!(condition.operator, QueryOperator::IsNull) {
                    (condition.field.clone(), JsonValue::Null)
                } else if matches!(condition.operator, QueryOperator::IsNotNull) {
                    (condition.field.clone(), json!({"$ne": null}))
                } else if matches!(condition.operator, QueryOperator::Exists) {
                    (condition.field.clone(), json!({"$exists": true}))
                } else {
                    params.push(condition.value.clone());
                    (condition.field.clone(), json!({operator: value}))
                }
            }

            /// 生成聚合管道的可读表示
            pub fn pipeline_to_string(&self, pipeline: &[JsonValue]) -> String {
                serde_json::to_string_pretty(pipeline).unwrap_or_else(|_| "Invalid JSON".to_string())
            }
        }
    };
}

// 使用示例：用户配置信息的虚拟表格（MongoDB版本）
define_mongo_join_table! {
    /// 用户配置信息（MongoDB版本）
    virtual_table MongoUserProfileInfo {
        base_collection: "users",
        joins: [
            MongoJoin {
                from: "profiles".to_string(),
                local_field: "_id".to_string(),
                foreign_field: "user_id".to_string(),
                as_field: "profile".to_string(),
                join_type: MongoJoinType::Left
            }
        ],
        fields: {
            user_id: "$_id",
            user_name: "$name",
            user_email: "$email",
            user_age: "$age",
            profile_name: "$profile.name",
            profile_bio: "$profile.bio",
            profile_avatar: "$profile.avatar_url"
        }
    }
}

// 复杂示例：电商订单详细信息（MongoDB版本）
define_mongo_join_table! {
    /// 电商订单详细信息（MongoDB版本）
    virtual_table MongoECommerceOrderDetail {
        base_collection: "orders",
        joins: [
            MongoJoin {
                from: "users".to_string(),
                local_field: "user_id".to_string(),
                foreign_field: "_id".to_string(),
                as_field: "user".to_string(),
                join_type: MongoJoinType::Inner
            },
            MongoJoin {
                from: "order_items".to_string(),
                local_field: "_id".to_string(),
                foreign_field: "order_id".to_string(),
                as_field: "items".to_string(),
                join_type: MongoJoinType::Left
            },
            MongoJoin {
                from: "products".to_string(),
                local_field: "items.product_id".to_string(),
                foreign_field: "_id".to_string(),
                as_field: "product".to_string(),
                join_type: MongoJoinType::Left
            },
            MongoJoin {
                from: "categories".to_string(),
                local_field: "product.category_id".to_string(),
                foreign_field: "_id".to_string(),
                as_field: "category".to_string(),
                join_type: MongoJoinType::Left
            }
        ],
        fields: {
            // 订单信息
            order_id: "$_id",
            order_number: "$order_number",
            order_date: "$created_at",
            order_status: "$status",
            order_total: "$total_amount",

            // 用户信息
            user_id: "$user._id",
            user_name: "$user.name",
            user_email: "$user.email",
            user_phone: "$user.phone",

            // 订单项信息
            item_quantity: "$items.quantity",
            item_price: "$items.unit_price",

            // 商品信息
            product_name: "$product.name",
            product_description: "$product.description",

            // 分类信息
            category_name: "$category.name"
        }
    }
}

fn main() {
    println!("🚀 MongoDB虚拟表格宏测试");
    println!("=========================");
    println!("验证MongoDB聚合管道生成能力");
    println!();

    // 测试基础用户配置信息
    test_mongo_user_profile_info();

    // 测试复杂电商订单
    test_mongo_ecommerce_order_detail();

    println!();
    println!("✅ MongoDB POC测试完成！");
}

fn test_mongo_user_profile_info() {
    println!("👤 测试MongoDB用户配置信息");
    println!("===========================");

    // 创建虚拟表格实例
    let profile = MongoUserProfileInfo {
        user_id: DataValue::String("507f1f77bcf86cd799439011".to_string()),
        user_name: DataValue::String("张三".to_string()),
        user_email: DataValue::String("zhangsan@example.com".to_string()),
        user_age: DataValue::Int(28),
        profile_name: DataValue::String("张三的配置".to_string()),
        profile_bio: DataValue::String("软件工程师".to_string()),
        profile_avatar: DataValue::String("https://example.com/avatar.jpg".to_string()),
    };

    // 生成查询条件
    let conditions = vec![
        QueryCondition {
            field: "user_age".to_string(),
            operator: QueryOperator::Gt,
            value: DataValue::Int(25),
        },
        QueryCondition {
            field: "profile_bio".to_string(),
            operator: QueryOperator::Contains,
            value: DataValue::String("工程师".to_string()),
        },
    ];

    let options = QueryOptions {
        pagination: Some(crate::types::query::PaginationConfig { skip: 0, limit: 10 }),
        sort: vec![crate::types::query::SortConfig {
            field: "user_name".to_string(),
            direction: crate::types::query::SortDirection::Asc,
        }],
        ..Default::default()
    };

    // 生成MongoDB聚合管道
    let (pipeline, params) = profile.to_mongo_pipeline(&conditions, &options);

    println!("生成的聚合管道阶段数: {}", pipeline.len());
    println!("参数数量: {} 个", params.len());
    println!("聚合管道:");
    println!("{}", profile.pipeline_to_string(&pipeline));
    println!("参数: {:?}", params);
    println!();
}

fn test_mongo_ecommerce_order_detail() {
    println!("📦 测试MongoDB电商订单详细信息");
    println!("==============================");

    // 创建虚拟表格实例
    let order_detail = MongoECommerceOrderDetail {
        // 订单信息
        order_id: DataValue::String("507f1f77bcf86cd799439012".to_string()),
        order_number: DataValue::String("ORD-2023-1001".to_string()),
        order_date: DataValue::String("2023-12-01T10:30:00Z".to_string()),
        order_status: DataValue::String("shipped".to_string()),
        order_total: DataValue::Float(299.99),

        // 用户信息
        user_id: DataValue::String("507f1f77bcf86cd799439013".to_string()),
        user_name: DataValue::String("张小明".to_string()),
        user_email: DataValue::String("zhangxiaoming@example.com".to_string()),
        user_phone: DataValue::String("13800138000".to_string()),

        // 订单项信息
        item_quantity: DataValue::Int(2),
        item_price: DataValue::Float(149.99),

        // 商品信息
        product_name: DataValue::String("智能手表 Pro".to_string()),
        product_description: DataValue::String("高端智能手表，支持健康监测".to_string()),

        // 分类信息
        category_name: DataValue::String("电子产品".to_string()),
    };

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
        },
    ];

    let options = QueryOptions {
        pagination: Some(crate::types::query::PaginationConfig { skip: 0, limit: 20 }),
        sort: vec![crate::types::query::SortConfig {
            field: "order_date".to_string(),
            direction: crate::types::query::SortDirection::Desc,
        }],
        ..Default::default()
    };

    // 生成MongoDB聚合管道
    let (pipeline, params) = order_detail.to_mongo_pipeline(&conditions, &options);

    println!("生成的聚合管道阶段数: {}", pipeline.len());
    println!("参数数量: {} 个", params.len());
    println!("聚合管道:");
    println!("{}", order_detail.pipeline_to_string(&pipeline));
    println!("参数: {:?}", params);
    println!();
}
