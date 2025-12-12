//! PostgreSQL查询相关操作

use crate::adapter::DatabaseAdapter;
use crate::adapter::postgres::PostgresAdapter;
use crate::adapter::postgres::query_builder::SqlQueryBuilder;
use crate::error::{QuickDbError, QuickDbResult};
use crate::pool::DatabaseConnection;
use crate::types::*;
use rat_logger::debug;


/// PostgreSQL删除操作
pub(crate) async fn delete(
    adapter: &PostgresAdapter,
    connection: &DatabaseConnection,
    table: &str,
    conditions: &[QueryCondition],
    alias: &str,
) -> QuickDbResult<u64> {
    if let DatabaseConnection::PostgreSQL(pool) = connection {
        let (sql, params) = SqlQueryBuilder::new()
            .delete()
            .where_conditions(conditions)
            .build(table, alias)?;

        debug!("执行PostgreSQL删除: {}", sql);

        super::utils::execute_update(adapter, pool, &sql, &params, table).await
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
    alias: &str,
) -> QuickDbResult<bool> {
    let conditions = vec![QueryCondition {
        field: "id".to_string(),
        operator: QueryOperator::Eq,
        value: id.clone(),
    }];

    let affected = delete(adapter, connection, table, &conditions, alias).await?;
    Ok(affected > 0)
}

/// PostgreSQL计数操作
pub(crate) async fn count(
    adapter: &PostgresAdapter,
    connection: &DatabaseConnection,
    table: &str,
    conditions: &[QueryCondition],
    alias: &str,
) -> QuickDbResult<u64> {
    if let DatabaseConnection::PostgreSQL(pool) = connection {
        let (sql, params) = SqlQueryBuilder::new()
            .select(&["COUNT(*) as count"])
            .where_conditions(conditions)
            .build(table, alias)?;

        debug!("执行PostgreSQL计数: {}", sql);

        let results = super::utils::execute_query(adapter, pool, &sql, &params, table).await?;
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
