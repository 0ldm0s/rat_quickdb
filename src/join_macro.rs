//! 虚拟表格宏定义
//! 用于简化复杂JOIN查询和缓存管理

use std::collections::HashMap;
use crate::types::*;
use crate::adapter::DatabaseAdapter;
use crate::pool::DatabaseConnection;
use async_trait::async_trait;

/// JOIN子句定义
#[derive(Debug, Clone)]
pub struct JoinDefinition {
    pub table: String,
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
            /// 生成SQL查询
            pub fn to_sql(&self, conditions: &[QueryCondition], options: &QueryOptions) -> (String, Vec<DataValue>) {
                let mut fields = vec![$($expr),*];
                let mut joins = vec![$($join),*];

                // 构建JOIN子句
                let mut join_clauses = Vec::new();
                for join in &joins {
                    let join_str = match join.join_type {
                        crate::join_macro::JoinType::Inner => "INNER JOIN",
                        crate::join_macro::JoinType::Left => "LEFT JOIN",
                        crate::join_macro::JoinType::Right => "RIGHT JOIN",
                        crate::join_macro::JoinType::Full => "FULL OUTER JOIN",
                    };
                    join_clauses.push(format!(" {} {} ON {}", join_str, join.table, join.on_condition));
                }

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

