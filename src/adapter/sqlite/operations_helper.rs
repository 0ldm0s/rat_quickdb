//! SQLite operations 辅助函数 - 处理 QueryConditionGroupWithConfig 转换

use crate::types::*;

/// 将 QueryConditionGroupWithConfig 转换为 QueryConditionGroup
pub fn convert_condition_group_with_config(
    condition_groups: &[QueryConditionGroupWithConfig],
) -> Vec<QueryConditionGroup> {
    condition_groups
        .iter()
        .map(|g| -> QueryConditionGroup {
            match g {
                QueryConditionGroupWithConfig::Single(c) => {
                    QueryConditionGroup::Single(QueryCondition {
                        field: c.field.clone(),
                        operator: c.operator.clone(),
                        value: c.value.clone(),
                    })
                }
                QueryConditionGroupWithConfig::Group { operator, conditions } => {
                    QueryConditionGroup::Group {
                        operator: operator.clone(),
                        conditions: conditions
                            .iter()
                            .map(|c| -> QueryCondition {
                                match c {
                                    QueryConditionWithConfig { field, operator, value, .. } => {
                                        QueryCondition {
                                            field: field.clone(),
                                            operator: operator.clone(),
                                            value: value.clone(),
                                        }
                                    }
                                })
                            })
                            .collect(),
                    }
                }
            }
        })
        .collect()
}

/// 简化 find_with_groups_with_cache_control 方法中的复杂逻辑
pub fn find_with_groups_simplified(
    connection: &crate::pool::DatabaseConnection,
    table: &str,
    condition_groups: &[QueryConditionGroupWithConfig],
    options: &crate::types::QueryOptions,
    alias: &str,
    bypass_cache: bool,
) -> crate::error::QuickDbResult<Vec<crate::types::DataValue>> {
    let simple_groups = convert_condition_group_with_config(condition_groups);
    crate::adapter::sqlite::query::sqlite_query::find_with_groups_with_cache_control(
        connection, table, &simple_groups, options, alias, bypass_cache
    ).await
}
