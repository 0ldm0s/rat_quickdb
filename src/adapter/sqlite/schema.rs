use crate::adapter::SqliteAdapter;
use crate::adapter::{DatabaseAdapter, SqlQueryBuilder};
use crate::error::{QuickDbError, QuickDbResult};
use crate::types::*;
use crate::model::{FieldDefinition, FieldType};
use crate::pool::DatabaseConnection;
use async_trait::async_trait;
use rat_logger::debug;
use sqlx::{sqlite::SqliteRow, Row, Column};
use std::collections::HashMap;

/// SQLite创建表操作
pub(crate) async fn create_table(
    adapter: &SqliteAdapter,
    connection: &DatabaseConnection,
    table: &str,
    fields: &HashMap<String, FieldDefinition>,
    id_strategy: &IdStrategy,
) -> QuickDbResult<()> {
    let pool = match connection {
        DatabaseConnection::SQLite(pool) => pool,
        _ => return Err(QuickDbError::ConnectionError {
            message: "Invalid connection type for SQLite".to_string(),
        }),
    };
    {
        let mut sql = format!("CREATE TABLE IF NOT EXISTS {} (", table);
        let mut has_fields = false;

        // 检查是否已经有id字段，如果没有则添加默认的id主键
        if !fields.contains_key("id") {
            sql.push_str("id INTEGER PRIMARY KEY AUTOINCREMENT");
            has_fields = true;
        }

        for (field_name, field_definition) in fields {
            if has_fields {
                sql.push_str(", ");
            }

            let sql_type = match &field_definition.field_type {
                FieldType::String { max_length, .. } => {
                    if let Some(max_len) = max_length {
                        format!("VARCHAR({})", max_len)
                    } else {
                        "TEXT".to_string()
                    }
                },
                FieldType::Integer { .. } => "INTEGER".to_string(),
                FieldType::BigInteger => "INTEGER".to_string(), // SQLite只有INTEGER类型
                FieldType::Float { .. } => "REAL".to_string(),
                FieldType::Double => "REAL".to_string(), // SQLite只有REAL类型
                FieldType::Text => "TEXT".to_string(),
                FieldType::Boolean => "INTEGER".to_string(),
                FieldType::DateTime => "TEXT".to_string(),
                FieldType::Date => "TEXT".to_string(),
                FieldType::Time => "TEXT".to_string(),
                FieldType::Json => "TEXT".to_string(),
                FieldType::Uuid => "TEXT".to_string(),
                FieldType::Binary => "BLOB".to_string(),
                FieldType::Decimal { precision: _, scale: _ } => "REAL".to_string(), // SQLite没有DECIMAL，使用REAL
                FieldType::Array { .. } => "TEXT".to_string(), // 存储为JSON
                FieldType::Object { .. } => "TEXT".to_string(), // 存储为JSON
                FieldType::Reference { .. } => "TEXT".to_string(), // 存储引用ID
            };

            // 如果是id字段，添加主键约束
            // 添加NULL或NOT NULL约束
            let null_constraint = if field_definition.required {
                "NOT NULL"
            } else {
                ""
            };

            if field_name == "id" {
                sql.push_str(&format!("{} {} PRIMARY KEY", field_name, sql_type));
            } else {
                sql.push_str(&format!("{} {} {}", field_name, sql_type, null_constraint));
            }
            has_fields = true;
        }

        sql.push(')');

        sqlx::query(&sql).execute(pool).await
            .map_err(|e| QuickDbError::QueryError {
                message: format!("创建SQLite表失败: {}", e),
            })?;

        Ok(())
    }
}

/// SQLite创建索引操作
pub(crate) async fn create_index(
    adapter: &SqliteAdapter,
    connection: &DatabaseConnection,
    table: &str,
    index_name: &str,
    fields: &[String],
    unique: bool,
) -> QuickDbResult<()> {
    let pool = match connection {
        DatabaseConnection::SQLite(pool) => pool,
        _ => return Err(QuickDbError::ConnectionError {
            message: "Invalid connection type for SQLite".to_string(),
        }),
    };
    {
        let unique_keyword = if unique { "UNIQUE " } else { "" };
        let fields_str = fields.join(", ");
        let sql = format!(
            "CREATE {}INDEX IF NOT EXISTS {} ON {} ({})",
            unique_keyword, index_name, table, fields_str
        );

        sqlx::query(&sql).execute(pool).await
            .map_err(|e| QuickDbError::QueryError {
                message: format!("创建SQLite索引失败: {}", e),
            })?;

        Ok(())
    }
}

/// SQLite表存在检查操作
pub(crate) async fn table_exists(
    adapter: &SqliteAdapter,
    connection: &DatabaseConnection,
    table: &str,
) -> QuickDbResult<bool> {
    let pool = match connection {
        DatabaseConnection::SQLite(pool) => pool,
        _ => return Err(QuickDbError::ConnectionError {
            message: "Invalid connection type for SQLite".to_string(),
        }),
    };
    {
        let sql = "SELECT name FROM sqlite_master WHERE type='table' AND name=?";
        let row = sqlx::query(sql)
            .bind(table)
            .fetch_optional(pool)
            .await
            .map_err(|e| QuickDbError::QueryError {
                message: format!("检查SQLite表是否存在失败: {}", e),
            })?;

        Ok(row.is_some())
    }
}

/// SQLite删除表操作
pub(crate) async fn drop_table(
    adapter: &SqliteAdapter,
    connection: &DatabaseConnection,
    table: &str,
) -> QuickDbResult<()> {
    let pool = match connection {
        DatabaseConnection::SQLite(pool) => pool,
        _ => return Err(QuickDbError::ConnectionError {
            message: "Invalid connection type for SQLite".to_string(),
        }),
    };

    let sql = format!("DROP TABLE IF EXISTS {}", table);

    debug!("执行SQLite删除表SQL: {}", sql);

    sqlx::query(&sql)
        .execute(pool)
        .await
        .map_err(|e| QuickDbError::QueryError {
            message: format!("删除SQLite表失败: {}", e),
        })?;

    debug!("成功删除SQLite表: {}", table);
    Ok(())
}

/// SQLite获取服务器版本操作
pub(crate) async fn get_server_version(
    adapter: &SqliteAdapter,
    connection: &DatabaseConnection,
) -> QuickDbResult<String> {
    let pool = match connection {
        DatabaseConnection::SQLite(pool) => pool,
        _ => return Err(QuickDbError::ConnectionError {
            message: "Invalid connection type for SQLite".to_string(),
        }),
    };

    let sql = "SELECT sqlite_version()";

    debug!("执行SQLite版本查询SQL: {}", sql);

    let row = sqlx::query(sql)
        .fetch_one(pool)
        .await
        .map_err(|e| QuickDbError::QueryError {
            message: format!("查询SQLite版本失败: {}", e),
        })?;

    let version: String = row.try_get(0)
        .map_err(|e| QuickDbError::QueryError {
            message: format!("解析SQLite版本结果失败: {}", e),
        })?;

    debug!("成功获取SQLite版本: {}", version);
    Ok(version)
}