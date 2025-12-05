//! 虚拟表格宏定义
//! 用于简化复杂JOIN查询和缓存管理
//! 支持SQL和MongoDB双模式

use crate::adapter::DatabaseAdapter;
use crate::pool::DatabaseConnection;
use crate::types::*;
use async_trait::async_trait;
use serde_json::Value as JsonValue;
use std::collections::HashMap;

/// JOIN子句定义
#[derive(Debug, Clone)]
pub struct JoinDefinition {
    pub table: String,
    pub database: Option<String>, // 数据库别名，None表示使用默认数据库
    pub on_condition: String,
    pub join_type: JoinType,
}

/// JOIN类型
#[derive(Debug, Clone, PartialEq)]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Full,
}

/// 数据库类型枚举
#[derive(Debug, Clone, PartialEq)]
pub enum VirtualTableDbType {
    Sql,
    Mongo,
}

/// MongoDB JOIN操作定义
#[derive(Debug, Clone)]
pub struct MongoJoinDefinition {
    pub from_collection: String,
    pub local_field: String,
    pub foreign_field: String,
    pub as_field: String,
    pub join_type: JoinType,
}

/// 虚拟表格的trait定义
pub trait VirtualTable {
    /// 获取基础表/集合名称
    fn get_base_name() -> &'static str;

    /// 获取数据库类型
    fn get_database_type() -> VirtualTableDbType;

    /// 获取JOIN定义（SQL模式）
    fn get_sql_joins() -> Option<&'static [JoinDefinition]>;

    /// 获取JOIN定义（MongoDB模式）
    fn get_mongo_joins() -> Option<&'static [MongoJoinDefinition]>;

    /// 获取字段映射
    fn get_field_mappings() -> &'static [(&'static str, &'static str)];
}

/// 虚拟表格定义宏
///
/// # 语法
/// ```rust
/// define_join_table! {
///     /// 用户配置信息
///     virtual_table UserProfileInfo {
///         base_table: "users",
///         joins: [
///             Join {
///                 table: "profiles",
///                 on: "users.id = profiles.user_id",
///                 join_type: Left
///             }
///         ],
///         fields: {
///             user_id: "users.id as user_id",
///             user_name: "users.name as user_name",
///             profile_name: "profiles.name as profile_name",
///             profile_age: "profiles.age as profile_age"
///         }
///     }
/// }
/// ```
#[macro_export]
macro_rules! define_join_table {
    (
        $(#[$attr:meta])*
        $vis:vis virtual_table $struct_name:ident {
            base_table: $base_table:expr,
            $(database: $database:expr,)?
            joins: [$($join:expr),* $(,)?],
            fields: {
                $($field:ident: $expr:expr),* $(,)?
            }
        }
    ) => {
        // 生成字段结构体
        #[derive(Debug)]
        $vis struct $struct_name {
            $(
                $field: DataValue,
            )*
        }

        impl $struct_name {
            /// 获取基础表名称
            pub fn get_base_name() -> &'static str {
                $base_table
            }

            /// 获取数据库别名
            pub fn get_database_alias() -> Option<String> {
                None $(.or(Some($database.to_string())))?
            }

            /// 生成SQL查询
            pub fn to_sql(&self, conditions: &[QueryCondition], options: &QueryOptions) -> (String, Vec<DataValue>) {
                let fields = vec![$($expr),*];

                // 构建JOIN子句 - 直接从静态定义构造
                let mut join_clauses = Vec::new();
                $(
                    let join_str = match $join.join_type {
                        crate::join_macro::JoinType::Inner => "INNER JOIN",
                        crate::join_macro::JoinType::Left => "LEFT JOIN",
                        crate::join_macro::JoinType::Right => "RIGHT JOIN",
                        crate::join_macro::JoinType::Full => "FULL OUTER JOIN",
                    };
                    join_clauses.push(format!(" {} {} ON {}", join_str, $join.table, $join.on_condition));
                )*

                // 构建WHERE子句
                let (mut where_clause, mut params) = (String::new(), Vec::new());
                if !conditions.is_empty() {
                    let mut clause_parts = Vec::new();
                    for condition in conditions {
                        let placeholder = format!("${}", params.len() + 1);
                        let op_str = match condition.operator {
                            crate::types::query::QueryOperator::Eq => "=",
                            crate::types::query::QueryOperator::Ne => "!=",
                            crate::types::query::QueryOperator::Gt => ">",
                            crate::types::query::QueryOperator::Gte => ">=",
                            crate::types::query::QueryOperator::Lt => "<",
                            crate::types::query::QueryOperator::Lte => "<=",
                            crate::types::query::QueryOperator::Contains => "LIKE",
                            crate::types::query::QueryOperator::StartsWith => "LIKE",
                            crate::types::query::QueryOperator::EndsWith => "LIKE",
                            crate::types::query::QueryOperator::In => "IN",
                            crate::types::query::QueryOperator::NotIn => "NOT IN",
                            crate::types::query::QueryOperator::Regex => "REGEX",
                            crate::types::query::QueryOperator::Exists => "IS NOT NULL",
                            crate::types::query::QueryOperator::IsNull => "IS NULL",
                            crate::types::query::QueryOperator::IsNotNull => "IS NOT NULL",
                        };

                        if matches!(condition.operator, crate::types::query::QueryOperator::IsNull | crate::types::query::QueryOperator::IsNotNull | crate::types::query::QueryOperator::Exists) {
                            clause_parts.push(format!("{} {}", condition.field, op_str));
                        } else {
                            clause_parts.push(format!("{} {} {}", condition.field, op_str, placeholder));
                            params.push(condition.value.clone());
                        }
                    }
                    where_clause = format!("WHERE {}", clause_parts.join(" AND "));
                }

                // 构建完整的SQL
                let fields_str = fields.join(", ");
                let joins_str = join_clauses.join(" ");
                let sql = format!(
                    "SELECT {} FROM {} {} {}",
                    fields_str,
                    $base_table,
                    joins_str,
                    where_clause
                );

                (sql, params)
            }
        }
    };
}
