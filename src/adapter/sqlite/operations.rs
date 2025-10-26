
use crate::adapter::{DatabaseAdapter, SqlQueryBuilder};
use crate::error::{QuickDbError, QuickDbResult};
use crate::types::*;
use crate::model::{FieldDefinition, FieldType};
use crate::pool::DatabaseConnection;
use std::collections::HashMap;
use async_trait::async_trait;
use rat_logger::{debug, info};
use sqlx::{sqlite::SqliteRow, Row, Column};

use super::adapter::SqliteAdapter;
use super::query as sqlite_query;
use super::schema as sqlite_schema;

#[async_trait]
impl DatabaseAdapter for SqliteAdapter {
    async fn create(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        data: &HashMap<String, DataValue>,
        id_strategy: &IdStrategy,
    ) -> QuickDbResult<DataValue> {
        let pool = match connection {
            DatabaseConnection::SQLite(pool) => pool,
            _ => return Err(QuickDbError::ConnectionError {
                message: "Invalid connection type for SQLite".to_string(),
            }),
        };
        
        // 自动建表逻辑：检查表是否存在，如果不存在则创建
            if !self.table_exists(connection, table).await? {
                // 获取表创建锁，防止重复创建
                let _lock = self.acquire_table_lock(table).await;
                // 再次检查表是否存在（双重检查锁定模式）
                if !self.table_exists(connection, table).await? {
                    // 尝试从模型管理器获取预定义的元数据
                    if let Some(model_meta) = crate::manager::get_model(table) {
                        debug!("表 {} 不存在，使用预定义模型元数据创建", table);

                        // 使用模型元数据创建表
                        self.create_table(connection, table, &model_meta.fields, id_strategy).await?;
                        debug!("✅ 使用模型元数据创建SQLite表 '{}' 成功", table);
                        // 等待100ms确保数据库事务完全提交
                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                        debug!("⏱️ 等待100ms确保表 '{}' 创建完成", table);
                    } else {
                        return Err(QuickDbError::ValidationError {
                            field: "table_creation".to_string(),
                            message: format!("表 '{}' 不存在，且没有预定义的模型元数据。请先定义模型并使用 define_model! 宏明确指定字段类型。", table),
                        });
                    }
                } else {
                    debug!("表 {} 已存在，跳过创建", table);
                }
                // 锁会在这里自动释放（当 _lock 超出作用域时）
            }
            
            let (sql, params) = SqlQueryBuilder::new()
                .insert(data.clone())
                .from(table)
                .build()?;
            
            // 构建参数化查询，使用正确的参数顺序
            let mut query = sqlx::query(&sql);
            for param in &params {
                match param {
                    DataValue::String(s) => { query = query.bind(s); },
                    DataValue::Int(i) => { query = query.bind(i); },
                    DataValue::Float(f) => { query = query.bind(f); },
                    DataValue::Bool(b) => { query = query.bind(b); },
                    DataValue::Bytes(bytes) => { query = query.bind(bytes); },
                    DataValue::DateTime(dt) => { query = query.bind(dt.to_rfc3339()); },
                    DataValue::Uuid(uuid) => { query = query.bind(uuid.to_string()); },
                    DataValue::Json(json) => { query = query.bind(json.to_string()); },
                    DataValue::Array(_) => {
                        let json = param.to_json_value().to_string();
                        query = query.bind(json);
                    },
                    DataValue::Object(_) => {
                        let json = param.to_json_value().to_string();
                        query = query.bind(json);
                    },
                    DataValue::Null => { query = query.bind(Option::<String>::None); },
                }
            }
            
            let result = query.execute(pool).await
                .map_err(|e| QuickDbError::QueryError {
                    message: format!("执行SQLite插入失败: {}", e),
                })?;
            
            // 根据插入的数据返回相应的ID
            // 优先返回数据中的ID字段，如果没有则使用SQLite的rowid
            if let Some(id_value) = data.get("id") {
                Ok(id_value.clone())
            } else if let Some(id_value) = data.get("_id") {
                Ok(id_value.clone())
            } else {
                // 如果数据中没有ID字段，返回SQLite的自增ID
                let id = result.last_insert_rowid();
                if id > 0 {
                    Ok(DataValue::Int(id))
                } else {
                    // 如果没有自增ID，返回包含详细信息的对象
                    let mut result_map = HashMap::new();
                    result_map.insert("id".to_string(), DataValue::Int(id));
                    result_map.insert("affected_rows".to_string(), DataValue::Int(result.rows_affected() as i64));
                    Ok(DataValue::Object(result_map))
                }
            }
    }

    async fn find_by_id(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        id: &DataValue,
    ) -> QuickDbResult<Option<DataValue>> {
        let pool = match connection {
            DatabaseConnection::SQLite(pool) => pool,
            _ => return Err(QuickDbError::ConnectionError {
                message: "Invalid connection type for SQLite".to_string(),
            }),
        };
        {
            let sql = format!("SELECT * FROM {} WHERE id = ? LIMIT 1", table);
            
            let mut query = sqlx::query(&sql);
            match id {
                DataValue::String(s) => { query = query.bind(s); },
                DataValue::Int(i) => { query = query.bind(i); },
                _ => { query = query.bind(id.to_string()); },
            }
            
            let row = query.fetch_optional(pool).await
                .map_err(|e| QuickDbError::QueryError {
                    message: format!("执行SQLite根据ID查询失败: {}", e),
                })?;
            
            match row {
                Some(r) => {
                    let data_map = self.row_to_data_map(&r)?;
                    Ok(Some(DataValue::Object(data_map)))
                },
                None => Ok(None),
            }
        }
    }

    async fn find(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
        options: &QueryOptions,
    ) -> QuickDbResult<Vec<DataValue>> {
        // 将简单条件转换为条件组合（AND逻辑）
        let condition_groups = if conditions.is_empty() {
            vec![]
        } else {
            let group_conditions = conditions.iter()
                .map(|c| QueryConditionGroup::Single(c.clone()))
                .collect();
            vec![QueryConditionGroup::Group {
                operator: LogicalOperator::And,
                conditions: group_conditions,
            }]
        };
        
        // 统一使用 find_with_groups 实现
        self.find_with_groups(connection, table, &condition_groups, options).await
    }

    async fn find_with_groups(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        condition_groups: &[QueryConditionGroup],
        options: &QueryOptions,
    ) -> QuickDbResult<Vec<DataValue>> {
        let pool = match connection {
            DatabaseConnection::SQLite(pool) => pool,
            _ => return Err(QuickDbError::ConnectionError {
                message: "Invalid connection type for SQLite".to_string(),
            }),
        };
        {
            let (sql, params) = SqlQueryBuilder::new()
                .database_type(DatabaseType::SQLite)
                .select(&["*"])
                .from(table)
                .where_condition_groups(condition_groups)
                .limit(options.pagination.as_ref().map(|p| p.limit).unwrap_or(1000))
                .offset(options.pagination.as_ref().map(|p| p.skip).unwrap_or(0))
                .build()?;

            debug!("执行SQLite条件组合查询: {}", sql);

            let mut query = sqlx::query(&sql);
            for param in &params {
                match param {
                    DataValue::String(s) => { query = query.bind(s); },
                    DataValue::Int(i) => { query = query.bind(i); },
                    DataValue::Float(f) => { query = query.bind(f); },
                    DataValue::Bool(b) => { query = query.bind(b); },
                    DataValue::Null => { query = query.bind(Option::<String>::None); },
                    _ => { query = query.bind(param.to_string()); },
                }
            }

            let rows = query.fetch_all(pool).await
                .map_err(|e| QuickDbError::QueryError {
                    message: format!("执行SQLite条件组合查询失败: {}", e),
                })?;

            let mut results = Vec::new();
            for row in rows {
                let data_map = self.row_to_data_map(&row)?;
                results.push(DataValue::Object(data_map));
            }

            Ok(results)
        }
    }

    async fn update(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
        data: &HashMap<String, DataValue>,
    ) -> QuickDbResult<u64> {
        let pool = match connection {
            DatabaseConnection::SQLite(pool) => pool,
            _ => return Err(QuickDbError::ConnectionError {
                message: "Invalid connection type for SQLite".to_string(),
            }),
        };
        {
            let (sql, params) = SqlQueryBuilder::new()
                .update(data.clone())
                .from(table)
                .where_conditions(conditions)
                .build()?;
            
            let mut query = sqlx::query(&sql);
            for param in &params {
                match param {
                    DataValue::String(s) => { query = query.bind(s); },
                    DataValue::Int(i) => { query = query.bind(i); },
                    DataValue::Float(f) => { query = query.bind(f); },
                    DataValue::Bool(b) => { query = query.bind(b); },
                    _ => { query = query.bind(param.to_string()); },
                }
            }
            
            let result = query.execute(pool).await
                .map_err(|e| QuickDbError::QueryError {
                    message: format!("执行SQLite更新失败: {}", e),
                })?;
            
            Ok(result.rows_affected())
        }
    }

    async fn update_by_id(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        id: &DataValue,
        data: &HashMap<String, DataValue>,
    ) -> QuickDbResult<bool> {
        let condition = QueryCondition {
            field: "id".to_string(),
            operator: QueryOperator::Eq,
            value: id.clone(),
        };
        
        let affected_rows = self.update(connection, table, &[condition], data).await?;
        Ok(affected_rows > 0)
    }

    async fn update_with_operations(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
        operations: &[crate::types::UpdateOperation],
    ) -> QuickDbResult<u64> {
        let pool = match connection {
            DatabaseConnection::SQLite(pool) => pool,
            _ => return Err(QuickDbError::ConnectionError {
                message: "Invalid connection type for SQLite".to_string(),
            }),
        };

        let mut set_clauses = Vec::new();
        let mut params = Vec::new();

        for operation in operations {
            match &operation.operation {
                crate::types::UpdateOperator::Set => {
                    set_clauses.push(format!("{} = ?", operation.field));
                    params.push(operation.value.clone());
                }
                crate::types::UpdateOperator::Increment => {
                    set_clauses.push(format!("{} = {} + ?", operation.field, operation.field));
                    params.push(operation.value.clone());
                }
                crate::types::UpdateOperator::Decrement => {
                    set_clauses.push(format!("{} = {} - ?", operation.field, operation.field));
                    params.push(operation.value.clone());
                }
                crate::types::UpdateOperator::Multiply => {
                    set_clauses.push(format!("{} = {} * ?", operation.field, operation.field));
                    params.push(operation.value.clone());
                }
                crate::types::UpdateOperator::Divide => {
                    set_clauses.push(format!("{} = {} / ?", operation.field, operation.field));
                    params.push(operation.value.clone());
                }
                crate::types::UpdateOperator::PercentIncrease => {
                    set_clauses.push(format!("{} = {} * (1.0 + ?/100.0)", operation.field, operation.field));
                    params.push(operation.value.clone());
                }
                crate::types::UpdateOperator::PercentDecrease => {
                    set_clauses.push(format!("{} = {} * (1.0 - ?/100.0)", operation.field, operation.field));
                    params.push(operation.value.clone());
                }
            }
        }

        if set_clauses.is_empty() {
            return Err(QuickDbError::ValidationError {
                field: "operations".to_string(),
                message: "更新操作不能为空".to_string(),
            });
        }

        let mut sql = format!("UPDATE {} SET {}", table, set_clauses.join(", "));

        // 添加WHERE条件
        if !conditions.is_empty() {
            let (where_clause, mut where_params) = SqlQueryBuilder::new()
                .database_type(crate::types::DatabaseType::SQLite)
                .build_where_clause_with_offset(conditions, params.len() + 1)?;

            sql.push_str(&format!(" WHERE {}", where_clause));
            params.extend(where_params);
        }

        debug!("执行SQLite操作更新: {}", sql);

        self.execute_update(pool, &sql, &params).await
    }

    async fn delete(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
    ) -> QuickDbResult<u64> {
        sqlite_query::delete(self, connection, table, conditions).await
    }

    async fn delete_by_id(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        id: &DataValue,
    ) -> QuickDbResult<bool> {
        sqlite_query::delete_by_id(self, connection, table, id).await
    }

    async fn count(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
    ) -> QuickDbResult<u64> {
        sqlite_query::count(self, connection, table, conditions).await
    }

    async fn exists(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
    ) -> QuickDbResult<bool> {
        sqlite_query::exists(self, connection, table, conditions).await
    }

    async fn create_table(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        fields: &HashMap<String, FieldDefinition>,
        id_strategy: &IdStrategy,
    ) -> QuickDbResult<()> {
        sqlite_schema::create_table(self, connection, table, fields, id_strategy).await
    }

    async fn create_index(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        index_name: &str,
        fields: &[String],
        unique: bool,
    ) -> QuickDbResult<()> {
        sqlite_schema::create_index(self, connection, table, index_name, fields, unique).await
    }

    async fn table_exists(
        &self,
        connection: &DatabaseConnection,
        table: &str,
    ) -> QuickDbResult<bool> {
        sqlite_schema::table_exists(self, connection, table).await
    }

    async fn drop_table(
        &self,
        connection: &DatabaseConnection,
        table: &str,
    ) -> QuickDbResult<()> {
        sqlite_schema::drop_table(self, connection, table).await
    }

    async fn get_server_version(
        &self,
        connection: &DatabaseConnection,
    ) -> QuickDbResult<String> {
        sqlite_schema::get_server_version(self, connection).await
    }
}

