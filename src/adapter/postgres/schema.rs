//! PostgreSQL表和索引管理操作

use crate::adapter::postgres::PostgresAdapter;
use crate::pool::DatabaseConnection;
use crate::error::{QuickDbError, QuickDbResult};
use crate::types::*;
use crate::model::{FieldType, FieldDefinition};
use rat_logger::debug;
use sqlx::Row;
use std::collections::HashMap;

/// PostgreSQL创建表操作
pub(crate) async fn create_table(
    adapter: &PostgresAdapter,
    connection: &DatabaseConnection,
    table: &str,
    fields: &HashMap<String, FieldDefinition>,
    id_strategy: &IdStrategy,
    alias: &str,
) -> QuickDbResult<()> {
    if let DatabaseConnection::PostgreSQL(pool) = connection {
        let mut field_definitions = Vec::new();

        // 根据ID策略创建ID字段
        if !fields.contains_key("id") {
            let id_definition = match id_strategy {
                IdStrategy::AutoIncrement => "id SERIAL PRIMARY KEY".to_string(),
                IdStrategy::Uuid => "id UUID PRIMARY KEY".to_string(), // 使用原生UUID类型，返回时转换为字符串
                IdStrategy::Snowflake { .. } => "id BIGINT PRIMARY KEY".to_string(),
                IdStrategy::ObjectId => "id TEXT PRIMARY KEY".to_string(),
                IdStrategy::Custom(_) => "id TEXT PRIMARY KEY".to_string(), // 自定义策略使用TEXT
            };
            field_definitions.push(id_definition);
        }

        for (name, field_definition) in fields {
            let sql_type = match &field_definition.field_type {
                FieldType::String { max_length, .. } => {
                    if let Some(max_len) = max_length {
                        format!("VARCHAR({})", max_len)
                    } else {
                        "TEXT".to_string()
                    }
                },
                FieldType::Integer { .. } => "INTEGER".to_string(),
                FieldType::BigInteger => "BIGINT".to_string(),
                FieldType::Float { .. } => "REAL".to_string(),
                FieldType::Double => "DOUBLE PRECISION".to_string(),
                FieldType::Text => "TEXT".to_string(),
                FieldType::Boolean => "BOOLEAN".to_string(),
                FieldType::DateTime => {
                    debug!("🔍 字段 {} 类型为 DateTime，required: {}", name, field_definition.required);
                    "TIMESTAMPTZ".to_string()
                },
                FieldType::DateTimeWithTz { .. } => {
                    debug!("🔍 字段 {} 类型为 DateTimeWithTz，required: {}", name, field_definition.required);
                    "TIMESTAMPTZ".to_string()
                },
                FieldType::Date => "DATE".to_string(),
                FieldType::Time => "TIME".to_string(),
                FieldType::Uuid => "UUID".to_string(),
                FieldType::Json => "JSONB".to_string(),
                FieldType::Binary => "BYTEA".to_string(),
                FieldType::Decimal { precision, scale } => format!("DECIMAL({},{})", precision, scale),
                FieldType::Array { item_type: _, max_items: _, min_items: _ } => "JSONB".to_string(),
                FieldType::Object { .. } => "JSONB".to_string(),
                FieldType::Reference { target_collection: _ } => "TEXT".to_string(),
            };

            // 如果是id字段，根据ID策略创建正确的字段类型
            if name == "id" {
                let id_definition = match id_strategy {
                    IdStrategy::AutoIncrement => "id SERIAL PRIMARY KEY".to_string(),
                    IdStrategy::Uuid => "id UUID PRIMARY KEY".to_string(), // 使用原生UUID类型
                    IdStrategy::Snowflake { .. } => "id BIGINT PRIMARY KEY".to_string(),
                    IdStrategy::ObjectId => "id TEXT PRIMARY KEY".to_string(),
                    IdStrategy::Custom(_) => "id TEXT PRIMARY KEY".to_string(), // 自定义策略使用TEXT
                };
                field_definitions.push(id_definition);
            } else {
                // 添加NULL或NOT NULL约束
                let null_constraint = if field_definition.required {
                    "NOT NULL"
                } else {
                    "NULL"
                };
                debug!("🔍 字段 {} 定义: {} {}", name, sql_type, null_constraint);
                field_definitions.push(format!("{} {} {}", name, sql_type, null_constraint));
            }
        }

        let sql = format!(
            "CREATE TABLE IF NOT EXISTS {} ({})",
            table,
            field_definitions.join(", ")
        );

        debug!("🔍 执行PostgreSQL建表SQL: {}", sql);
        debug!("🔍 字段定义详情: {:?}", field_definitions);

        super::utils::execute_update(adapter, pool, &sql, &[]).await?;

        Ok(())
    } else {
        Err(QuickDbError::ConnectionError {
            message: "连接类型不匹配，期望PostgreSQL连接".to_string(),
        })
    }
}

/// PostgreSQL创建索引操作
pub(crate) async fn create_index(
    adapter: &PostgresAdapter,
    connection: &DatabaseConnection,
    table: &str,
    index_name: &str,
    fields: &[String],
    unique: bool,
) -> QuickDbResult<()> {
    if let DatabaseConnection::PostgreSQL(pool) = connection {
        let unique_clause = if unique { "UNIQUE " } else { "" };
        let sql = format!(
            "CREATE {}INDEX IF NOT EXISTS {} ON {} ({})",
            unique_clause,
            index_name,
            table,
            fields.join(", ")
        );

        debug!("执行PostgreSQL索引创建: {}", sql);

        super::utils::execute_update(adapter, pool, &sql, &[]).await?;

        Ok(())
    } else {
        Err(QuickDbError::ConnectionError {
            message: "连接类型不匹配，期望PostgreSQL连接".to_string(),
        })
    }
}

/// PostgreSQL表存在检查操作
pub(crate) async fn table_exists(
    adapter: &PostgresAdapter,
    connection: &DatabaseConnection,
    table: &str,
) -> QuickDbResult<bool> {
    if let DatabaseConnection::PostgreSQL(pool) = connection {
        let sql = "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public' AND table_name = $1";

        let rows = sqlx::query(sql)
            .bind(table)
            .fetch_all(pool)
            .await
            .map_err(|e| QuickDbError::QueryError {
                message: format!("检查PostgreSQL表是否存在失败: {}", e),
            })?;

        let exists = !rows.is_empty();
        debug!("检查表 {} 是否存在: {}", table, exists);
        Ok(exists)
    } else {
        Err(QuickDbError::ConnectionError {
            message: "连接类型不匹配，期望PostgreSQL连接".to_string(),
        })
    }
}

/// PostgreSQL删除表操作
pub(crate) async fn drop_table(
    adapter: &PostgresAdapter,
    connection: &DatabaseConnection,
    table: &str,
) -> QuickDbResult<()> {
    if let DatabaseConnection::PostgreSQL(pool) = connection {
        let sql = format!("DROP TABLE IF EXISTS {} CASCADE", table);

        debug!("执行PostgreSQL删除表SQL: {}", sql);

        sqlx::query(&sql)
            .execute(pool)
            .await
            .map_err(|e| QuickDbError::QueryError {
                message: format!("删除PostgreSQL表失败: {}", e),
            })?;

        // 验证表是否真的被删除了
        let check_sql = "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public' AND table_name = $1";
        let check_rows = sqlx::query(check_sql)
            .bind(table)
            .fetch_all(pool)
            .await
            .map_err(|e| QuickDbError::QueryError {
                message: format!("验证表删除失败: {}", e),
            })?;

        let still_exists = !check_rows.is_empty();
        debug!("🔍 删除后验证表 {} 是否存在: {}", table, still_exists);

        debug!("成功删除PostgreSQL表: {}", table);
        Ok(())
    } else {
        Err(QuickDbError::ConnectionError {
            message: "连接类型不匹配，期望PostgreSQL连接".to_string(),
        })
    }
}

/// PostgreSQL获取服务器版本操作
pub(crate) async fn get_server_version(
    adapter: &PostgresAdapter,
    connection: &DatabaseConnection,
) -> QuickDbResult<String> {
    if let DatabaseConnection::PostgreSQL(pool) = connection {
        let sql = "SELECT version()";

        debug!("执行PostgreSQL版本查询SQL: {}", sql);

        let row = sqlx::query(sql)
            .fetch_one(pool)
            .await
            .map_err(|e| QuickDbError::QueryError {
                message: format!("查询PostgreSQL版本失败: {}", e),
            })?;

        let version: String = row.try_get(0)
            .map_err(|e| QuickDbError::QueryError {
                message: format!("解析PostgreSQL版本结果失败: {}", e),
            })?;

        debug!("成功获取PostgreSQL版本: {}", version);
        Ok(version)
    } else {
        Err(QuickDbError::ConnectionError {
            message: "连接类型不匹配，期望PostgreSQL连接".to_string(),
        })
    }
}