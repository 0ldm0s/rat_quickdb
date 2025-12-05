use super::SqlQueryBuilder;
use crate::adapter::{DatabaseAdapter, SqliteAdapter};
use crate::error::{QuickDbError, QuickDbResult};
use crate::pool::DatabaseConnection;
use crate::types::*;
use rat_logger::debug;
use sqlx::{Column, Row, sqlite::SqliteRow};

/// SQLite删除操作
pub(crate) async fn delete(
    adapter: &SqliteAdapter,
    connection: &DatabaseConnection,
    table: &str,
    conditions: &[QueryCondition],
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
            .map_err(|e| QuickDbError::QueryError {
                message: format!("执行SQLite删除失败: {}", e),
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
    let condition = QueryCondition {
        field: "id".to_string(),
        operator: QueryOperator::Eq,
        value: id.clone(),
    };

    let affected_rows = delete(adapter, connection, table, &[condition], alias).await?;
    Ok(affected_rows > 0)
}

/// SQLite统计操作
pub(crate) async fn count(
    adapter: &SqliteAdapter,
    connection: &DatabaseConnection,
    table: &str,
    conditions: &[QueryCondition],
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
            .map_err(|e| QuickDbError::QueryError {
                message: format!("执行SQLite统计失败: {}", e),
            })?;

        let count: i64 = row.try_get("count").map_err(|e| QuickDbError::QueryError {
            message: format!("获取统计结果失败: {}", e),
        })?;

        Ok(count as u64)
    }
}
