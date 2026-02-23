use super::SqlQueryBuilder;
use crate::adapter::{DatabaseAdapter, SqliteAdapter};
use crate::error::{QuickDbError, QuickDbResult};
use crate::pool::DatabaseConnection;
use crate::types::*;
use rat_logger::debug;
use sqlx::{Column, Row, sqlite::SqliteRow};

/// 检查SQLite错误是否为表不存在错误
fn check_table_not_exist_error(error: &sqlx::Error, table: &str) -> bool {
    let error_string = error.to_string().to_lowercase();
    error_string.contains("no such table") ||
    error_string.contains(&format!("no such table: {}", table.to_lowercase())) ||
    error_string.contains("table") && error_string.contains("not found")
}

/// SQLite删除操作
pub(crate) async fn delete(
    adapter: &SqliteAdapter,
    connection: &DatabaseConnection,
    table: &str,
    conditions: &[QueryConditionWithConfig],
    alias: &str,
) -> QuickDbResult<u64> {
    let pool = match connection {
        DatabaseConnection::SQLite(pool) => pool,
        _ => {
            return Err(QuickDbError::ConnectionError {
                message: "Invalid connection type for SQLite".to_string(),
            });
        }
    };
    {
        let (sql, params) = SqlQueryBuilder::new()
            .delete()
            .where_conditions(conditions)
            .build(table, alias)?;

        let mut query = sqlx::query(&sql);
        for param in &params {
            match param {
                DataValue::String(s) => {
                    query = query.bind(s);
                }
                DataValue::Int(i) => {
                    query = query.bind(i);
                }
                DataValue::Float(f) => {
                    query = query.bind(f);
                }
                DataValue::Bool(b) => {
                    query = query.bind(b);
                }
                _ => {
                    query = query.bind(param.to_string());
                }
            }
        }

        let result = query
            .execute(pool)
            .await
            .map_err(|e| {
                if check_table_not_exist_error(&e, table) {
                    QuickDbError::TableNotExistError {
                        table: table.to_string(),
                        message: format!("SQLite表 '{}' 不存在", table),
                    }
                } else {
                    QuickDbError::QueryError {
                        message: format!("执行SQLite删除失败: {}", e),
                    }
                }
            })?;

        Ok(result.rows_affected())
    }
}

/// SQLite根据ID删除操作
pub(crate) async fn delete_by_id(
    adapter: &SqliteAdapter,
    connection: &DatabaseConnection,
    table: &str,
    id: &DataValue,
    alias: &str,
) -> QuickDbResult<bool> {
    let condition = QueryConditionWithConfig {
        field: "id".to_string(),
        operator: QueryOperator::Eq,
        value: id.clone(),
        case_insensitive: false,
    };

    let affected_rows = delete(adapter, connection, table, &[condition], alias).await?;
    Ok(affected_rows > 0)
}

/// SQLite统计操作
pub(crate) async fn count(
    adapter: &SqliteAdapter,
    connection: &DatabaseConnection,
    table: &str,
    conditions: &[QueryConditionWithConfig],
    alias: &str,
) -> QuickDbResult<u64> {
    let pool = match connection {
        DatabaseConnection::SQLite(pool) => pool,
        _ => {
            return Err(QuickDbError::ConnectionError {
                message: "Invalid connection type for SQLite".to_string(),
            });
        }
    };
    {
        let (sql, params) = SqlQueryBuilder::new()
            .select(&["COUNT(*) as count"])
            .where_conditions(conditions)
            .build(table, alias)?;

        let mut query = sqlx::query(&sql);
        for param in &params {
            match param {
                DataValue::String(s) => {
                    query = query.bind(s);
                }
                DataValue::Int(i) => {
                    query = query.bind(i);
                }
                DataValue::Float(f) => {
                    query = query.bind(f);
                }
                DataValue::Bool(b) => {
                    query = query.bind(b);
                }
                _ => {
                    query = query.bind(param.to_string());
                }
            }
        }

        let row = query
            .fetch_one(pool)
            .await
            .map_err(|e| {
                if check_table_not_exist_error(&e, table) {
                    QuickDbError::TableNotExistError {
                        table: table.to_string(),
                        message: format!("SQLite表 '{}' 不存在", table),
                    }
                } else {
                    QuickDbError::QueryError {
                        message: format!("执行SQLite统计失败: {}", e),
                    }
                }
            })?;

        let count: i64 = row.try_get("count").map_err(|e| QuickDbError::QueryError {
            message: format!("获取统计结果失败: {}", e),
        })?;

        Ok(count as u64)
    }
}

/// SQLite条件组合统计操作
pub(crate) async fn count_with_groups(
    adapter: &SqliteAdapter,
    connection: &DatabaseConnection,
    table: &str,
    condition_groups: &[QueryConditionGroupWithConfig],
    alias: &str,
) -> QuickDbResult<u64> {
    let pool = match connection {
        DatabaseConnection::SQLite(pool) => pool,
        _ => {
            return Err(QuickDbError::ConnectionError {
                message: "Invalid connection type for SQLite".to_string(),
            });
        }
    };

    // SQLite 不支持 case_insensitive，将 QueryConditionGroupWithConfig 转换为 QueryConditionGroup
    fn convert_group(group: &QueryConditionGroupWithConfig) -> QueryConditionGroup {
        match group {
            QueryConditionGroupWithConfig::Single(c) => {
                QueryConditionGroup::Single(QueryCondition {
                    field: c.field.clone(),
                    operator: c.operator.clone(),
                    value: c.value.clone(),
                })
            }
            QueryConditionGroupWithConfig::GroupWithConfig { operator, conditions } => {
                QueryConditionGroup::Group {
                    operator: operator.clone(),
                    conditions: conditions.iter().map(convert_group).collect(),
                }
            }
        }
    }

    let simple_groups: Vec<QueryConditionGroup> =
        condition_groups.iter().map(convert_group).collect();

    let (sql, params) = SqlQueryBuilder::new()
        .select(&["COUNT(*) as count"])
        .where_condition_groups(&simple_groups)
        .build(table, alias)?;

    debug!("执行SQLite条件组合统计: {}", sql);

    let mut query = sqlx::query(&sql);
    for param in &params {
        match param {
            DataValue::String(s) => {
                query = query.bind(s);
            }
            DataValue::Int(i) => {
                query = query.bind(i);
            }
            DataValue::Float(f) => {
                query = query.bind(f);
            }
            DataValue::Bool(b) => {
                query = query.bind(b);
            }
            _ => {
                query = query.bind(param.to_string());
            }
        }
    }

    let row = query
        .fetch_one(pool)
        .await
        .map_err(|e| {
            if check_table_not_exist_error(&e, table) {
                QuickDbError::TableNotExistError {
                    table: table.to_string(),
                    message: format!("SQLite表 '{}' 不存在", table),
                }
            } else {
                QuickDbError::QueryError {
                    message: format!("执行SQLite条件组合统计失败: {}", e),
                }
            }
        })?;

    let count: i64 = row.try_get("count").map_err(|e| QuickDbError::QueryError {
        message: format!("获取统计结果失败: {}", e),
    })?;

    Ok(count as u64)
}
