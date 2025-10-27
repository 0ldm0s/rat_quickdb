//! PostgreSQL查询相关操作

use crate::adapter::postgres::PostgresAdapter;
use crate::adapter::DatabaseAdapter;
use crate::pool::DatabaseConnection;
use crate::error::{QuickDbError, QuickDbResult};
use crate::types::*;
use crate::adapter::query_builder::SqlQueryBuilder;
use rat_logger::debug;

/// PostgreSQL删除操作
pub(crate) async fn delete(
    adapter: &PostgresAdapter,
    connection: &DatabaseConnection,
    table: &str,
    conditions: &[QueryCondition],
) -> QuickDbResult<u64> {
    if let DatabaseConnection::PostgreSQL(pool) = connection {
        let (sql, params) = SqlQueryBuilder::new()
            .database_type(crate::types::DatabaseType::PostgreSQL)
            .delete()
            .from(table)
            .where_conditions(conditions)
            .build()?;

        debug!("执行PostgreSQL删除: {}", sql);

        super::utils::execute_update(adapter, pool, &sql, &params).await
    } else {
        Err(QuickDbError::ConnectionError {
            message: "连接类型不匹配，期望PostgreSQL连接".to_string(),
        })
    }
}

/// PostgreSQL根据ID删除操作
pub(crate) async fn delete_by_id(
    adapter: &PostgresAdapter,
    connection: &DatabaseConnection,
    table: &str,
    id: &DataValue,
) -> QuickDbResult<bool> {
    let conditions = vec![QueryCondition {
        field: "id".to_string(),
        operator: QueryOperator::Eq,
        value: id.clone(),
    }];

    let affected = delete(adapter, connection, table, &conditions).await?;
    Ok(affected > 0)
}

/// PostgreSQL计数操作
pub(crate) async fn count(
    adapter: &PostgresAdapter,
    connection: &DatabaseConnection,
    table: &str,
    conditions: &[QueryCondition],
) -> QuickDbResult<u64> {
    if let DatabaseConnection::PostgreSQL(pool) = connection {
        let (sql, params) = SqlQueryBuilder::new()
            .database_type(crate::types::DatabaseType::PostgreSQL)
            .select(&["COUNT(*) as count"])
            .from(table)
            .where_conditions(conditions)
            .build()?;

        debug!("执行PostgreSQL计数: {}", sql);

        let results = super::utils::execute_query(adapter, pool, &sql, &params).await?;
        if let Some(result) = results.first() {
            if let DataValue::Object(obj) = result {
                if let Some(DataValue::Int(count)) = obj.get("count") {
                    return Ok(*count as u64);
                }
            }
        }

        Ok(0)
    } else {
        Err(QuickDbError::ConnectionError {
            message: "连接类型不匹配，期望PostgreSQL连接".to_string(),
        })
    }
}

/// PostgreSQL存在检查操作
pub(crate) async fn exists(
    adapter: &PostgresAdapter,
    connection: &DatabaseConnection,
    table: &str,
    conditions: &[QueryCondition],
) -> QuickDbResult<bool> {
    let count = count(adapter, connection, table, conditions).await?;
    Ok(count > 0)
}