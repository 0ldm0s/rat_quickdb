    //! MySQL查询相关操作

use crate::adapter::MysqlAdapter;
use crate::adapter::DatabaseAdapter;
use crate::pool::DatabaseConnection;
use crate::error::{QuickDbError, QuickDbResult};
use crate::types::*;
use crate::adapter::query_builder::SqlQueryBuilder;
use rat_logger::debug;

/// MySQL删除操作
pub(crate) async fn delete(
    adapter: &MysqlAdapter,
    connection: &DatabaseConnection,
    table: &str,
    conditions: &[QueryCondition],
    ) -> QuickDbResult<u64> {
        if let DatabaseConnection::MySQL(pool) = connection {
            let (sql, params) = SqlQueryBuilder::new()
                .database_type(crate::types::DatabaseType::MySQL)
                .delete()
                .from(table)
                .where_conditions(conditions)
                .build()?;
            
            adapter.execute_update(pool, &sql, &params).await
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
    ) -> QuickDbResult<bool> {
        if let DatabaseConnection::MySQL(pool) = connection {
            let condition = QueryCondition {
                field: "id".to_string(),
                operator: QueryOperator::Eq,
                value: id.clone(),
            };
            
            let (sql, params) = SqlQueryBuilder::new()
                .database_type(crate::types::DatabaseType::MySQL)
                .delete()
                .from(table)
                .where_condition(condition)
                .build()?;
            
            let affected_rows = adapter.execute_update(pool, &sql, &params).await?;
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
        conditions: &[QueryCondition],
    ) -> QuickDbResult<u64> {
        if let DatabaseConnection::MySQL(pool) = connection {
            let (sql, params) = SqlQueryBuilder::new()
                .database_type(crate::types::DatabaseType::MySQL)
                .select(&["COUNT(*) as count"])
                .from(table)
                .where_conditions(conditions)
                .build()?;
            
            let results = adapter.execute_query(pool, &sql, &params).await?;
            
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

    
