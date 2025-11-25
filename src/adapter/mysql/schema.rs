  //! MySQL表和索引管理操作

use crate::adapter::MysqlAdapter;
use crate::pool::DatabaseConnection;
use crate::error::{QuickDbError, QuickDbResult};
use crate::types::*;
use crate::model::{FieldType, FieldDefinition};
use rat_logger::debug;
use std::collections::HashMap;

/// MySQL创建表操作
pub(crate) async fn create_table(
    adapter: &MysqlAdapter,
        connection: &DatabaseConnection,
        table: &str,
        fields: &HashMap<String, FieldDefinition>,
        id_strategy: &IdStrategy,
        alias: &str,
    ) -> QuickDbResult<()> {
        if let DatabaseConnection::MySQL(pool) = connection {
            let mut field_definitions = Vec::new();
            
            // 统一处理id字段，根据ID策略决定类型和属性
            let id_definition = match id_strategy {
                IdStrategy::AutoIncrement => "id BIGINT AUTO_INCREMENT PRIMARY KEY".to_string(),
                IdStrategy::ObjectId => "id VARCHAR(255) PRIMARY KEY".to_string(), // ObjectId存储为字符串
                IdStrategy::Uuid => "id VARCHAR(36) PRIMARY KEY".to_string(),
                IdStrategy::Snowflake { .. } => "id BIGINT PRIMARY KEY".to_string(),
                IdStrategy::Custom(_) => "id VARCHAR(255) PRIMARY KEY".to_string(), // 自定义ID使用字符串
            };
            field_definitions.push(id_definition);

            for (name, field_definition) in fields {
                // 跳过id字段，因为已经根据策略处理过了
                if name == "id" {
                    continue;
                }

                // 非id字段的正常处理
                let sql_type = match &field_definition.field_type {
                    FieldType::String { max_length, .. } => {
                        if let Some(max_len) = max_length {
                            format!("VARCHAR({})", max_len)
                        } else {
                            // 对于没有指定长度的字符串字段，使用合理的默认长度
                            "VARCHAR(1000)".to_string()
                        }
                    },
                    FieldType::Integer { .. } => "INT".to_string(),
                    FieldType::BigInteger => "BIGINT".to_string(),
                    FieldType::Float { .. } => "FLOAT".to_string(),
                    FieldType::Double => "DOUBLE".to_string(),
                    FieldType::Text => "TEXT".to_string(),
                    FieldType::Boolean => "BOOLEAN".to_string(),
                    FieldType::DateTime => "DATETIME".to_string(),
                    FieldType::Date => "DATE".to_string(),
                    FieldType::Time => "TIME".to_string(),
                    FieldType::Uuid => "VARCHAR(36)".to_string(),
                    FieldType::Json => "JSON".to_string(),
                    FieldType::Binary => "BLOB".to_string(),
                    FieldType::Decimal { precision, scale } => format!("DECIMAL({},{})", precision, scale),
                    FieldType::Array { .. } => "JSON".to_string(),
                    FieldType::Object { .. } => "JSON".to_string(),
                    FieldType::Reference { .. } => "VARCHAR(255)".to_string(),
                };

                // 添加NULL或NOT NULL约束
                let null_constraint = if field_definition.required {
                    "NOT NULL"
                } else {
                    "NULL"
                };
                field_definitions.push(format!("{} {} {}", name, sql_type, null_constraint));
            }
            
            let sql = format!(
                "CREATE TABLE IF NOT EXISTS {} ({})",
                table,
                field_definitions.join(", ")
            );
            
            adapter.execute_update(pool, &sql, &[]).await?;
            
            Ok(())
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望MySQL连接".to_string(),
            })
        }
    }

    /// MySQL创建索引操作
pub(crate) async fn create_index(
    adapter: &MysqlAdapter,
        connection: &DatabaseConnection,
        table: &str,
        index_name: &str,
        fields: &[String],
        unique: bool,
    ) -> QuickDbResult<()> {
        if let DatabaseConnection::MySQL(pool) = connection {
            let unique_clause = if unique { "UNIQUE " } else { "" };
            let sql = format!(
                "CREATE {}INDEX {} ON {} ({})",
                unique_clause,
                index_name,
                table,
                fields.join(", ")
            );
            
            adapter.execute_update(pool, &sql, &[]).await?;
            
            Ok(())
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望MySQL连接".to_string(),
            })
        }
    }

    /// MySQL表存在检查操作
pub(crate) async fn table_exists(
    adapter: &MysqlAdapter,
        connection: &DatabaseConnection,
        table: &str,
    ) -> QuickDbResult<bool> {
        if let DatabaseConnection::MySQL(pool) = connection {
            let sql = "SELECT TABLE_NAME FROM INFORMATION_SCHEMA.TABLES WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = ?";
            let params = vec![DataValue::String(table.to_string())];
            let results = adapter.execute_query(pool, sql, &params).await?;
            
            Ok(!results.is_empty())
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望MySQL连接".to_string(),
            })
        }
    }

    /// MySQL删除表操作
pub(crate) async fn drop_table(
    adapter: &MysqlAdapter,
        connection: &DatabaseConnection,
        table: &str,
    ) -> QuickDbResult<()> {
        if let DatabaseConnection::MySQL(pool) = connection {
            let sql = format!("DROP TABLE IF EXISTS {}", table);

            debug!("执行MySQL删除表SQL: {}", sql);

            adapter.execute_update(pool, &sql, &[]).await?;

            debug!("成功删除MySQL表: {}", table);
            Ok(())
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望MySQL连接".to_string(),
            })
        }
    }

    /// MySQL获取服务器版本操作
pub(crate) async fn get_server_version(
    adapter: &MysqlAdapter,
        connection: &DatabaseConnection,
    ) -> QuickDbResult<String> {
        if let DatabaseConnection::MySQL(pool) = connection {
            let sql = "SELECT VERSION()";

            debug!("执行MySQL版本查询SQL: {}", sql);

            let results = adapter.execute_query(pool, sql, &[]).await?;

            if let Some(result) = results.first() {
                match result {
                    DataValue::Object(obj) => {
                        // MySQL适配器返回的是Object包装的结果，需要提取版本信息
                        if let Some((_, DataValue::String(version))) = obj.iter().next() {
                            debug!("成功获取MySQL版本: {}", version);
                            Ok(version.clone())
                        } else {
                            Err(QuickDbError::QueryError {
                                message: "MySQL版本查询返回的对象中没有找到字符串版本信息".to_string(),
                            })
                        }
                    },
                    DataValue::String(version) => {
                        // 兼容直接返回字符串的情况
                        debug!("成功获取MySQL版本: {}", version);
                        Ok(version.clone())
                    },
                    _ => {
                        debug!("MySQL版本查询返回了意外的数据类型: {:?}", result);
                        Err(QuickDbError::QueryError {
                            message: "MySQL版本查询返回了非字符串结果".to_string(),
                        })
                    },
                }
            } else {
                Err(QuickDbError::QueryError {
                    message: "MySQL版本查询返回了空结果".to_string(),
                })
            }
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望MySQL连接".to_string(),
            })
        }
    }
