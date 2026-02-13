
//! MySQL查询相关操作

use crate::adapter::DatabaseAdapter;
use crate::adapter::MysqlAdapter;
use crate::adapter::mysql::query_builder::SqlQueryBuilder;
use crate::error::{QuickDbError, QuickDbResult};
use crate::pool::DatabaseConnection;
use crate::types::*;
use rat_logger::debug;

/// MySQL删除操作
pub(crate) async fn delete(
    adapter: &MysqlAdapter,
    connection: &DatabaseConnection,
    table: &str,
    conditions: &[QueryConditionWithConfig],
    alias: &str,
) -> QuickDbResult<u64> {
    if let DatabaseConnection::MySQL(pool) = connection {
        let (sql, params) = SqlQueryBuilder::new()
            .delete()
            .where_conditions(conditions)
            .build(table, alias)?;

        adapter.execute_update(pool, &sql, &params, table).await
    } else {
        Err(QuickDbError::ConnectionError {
            message: "连接类型不匹配，期望MySQL连接".to_string(),
        })
    }
}

pub(crate) async fn delete_by_id(
    adapter: &MysqlAdapter,
    connection: &DatabaseConnection,
    table: &str,
    id: &DataValue,
    alias: &str,
) -> QuickDbResult<bool> {
    if let DatabaseConnection::MySQL(pool) = connection {
        let condition = QueryConditionWithConfig {
            field: "id".to_string(),
            operator: QueryOperator::Eq,
            value: id.clone(),
            case_insensitive: false,
        };

        let (sql, params) = SqlQueryBuilder::new()
            .delete()
            .where_condition(condition)
            .build(table, alias)?;

        let affected_rows = adapter.execute_update(pool, &sql, &params, table).await?;
        Ok(affected_rows > 0)
    } else {
        Err(QuickDbError::ConnectionError {
            message: "连接类型不匹配，期望MySQL连接".to_string(),
        })
    }
}

pub(crate) async fn count(
    adapter: &MysqlAdapter,
    connection: &DatabaseConnection,
    table: &str,
    conditions: &[QueryConditionWithConfig],
    alias: &str,
) -> QuickDbResult<u64> {
    if let DatabaseConnection::MySQL(pool) = connection {
        let (sql, params) = SqlQueryBuilder::new()
            .select(&["COUNT(*) as count"])
            .where_conditions(conditions)
            .build(table, alias)?;

        let results = adapter.execute_query(pool, &sql, &params, table).await?;

        if let Some(result) = results.first() {
            if let DataValue::Object(map) = result {
                if let Some(DataValue::Int(count)) = map.get("count") {
                    return Ok(*count as u64);
                }
            }
        }

        Ok(0)
    } else {
        Err(QuickDbError::ConnectionError {
            message: "连接类型不匹配，期望MySQL连接".to_string(),
        })
    }
}
