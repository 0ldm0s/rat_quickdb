    //! MySQL适配器trait实现

use crate::adapter::MysqlAdapter;
use crate::adapter::DatabaseAdapter;
use crate::adapter::query_builder::SqlQueryBuilder;
use crate::pool::DatabaseConnection;
use crate::error::{QuickDbError, QuickDbResult};
use crate::types::*;
use crate::model::{FieldType, FieldDefinition};
use crate::manager;
use async_trait::async_trait;
use rat_logger::debug;
use std::collections::HashMap;
use sqlx::Row;

use super::query as mysql_query;
use super::schema as mysql_schema;

#[async_trait]
impl DatabaseAdapter for MysqlAdapter {
    async fn create(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        data: &HashMap<String, DataValue>,
        id_strategy: &IdStrategy,
    ) -> QuickDbResult<DataValue> {
        if let DatabaseConnection::MySQL(pool) = connection {
            // 自动建表逻辑：检查表是否存在，如果不存在则创建
            if !self.table_exists(connection, table).await? {
                // 获取表创建锁，防止重复创建
                let _lock = self.acquire_table_lock(table).await;
                // 再次检查表是否存在（双重检查锁定模式）
                if !self.table_exists(connection, table).await? {
                    // 尝试从模型管理器获取预定义的元数据
                    if let Some(model_meta) = manager::get_model(table) {
                        debug!("表 {} 不存在，使用预定义模型元数据创建", table);

                        // 使用模型元数据创建表
                        self.create_table(connection, table, &model_meta.fields, id_strategy).await?;
                        debug!("✅ 使用模型元数据创建MySQL表 '{}' 成功", table);
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
                .database_type(crate::types::DatabaseType::MySQL)
                .insert(data.clone())
                .from(table)
                .build()?;

            debug!("生成的INSERT SQL: {}", sql);
            debug!("绑定参数: {:?}", params);

            // 使用事务确保插入和获取ID在同一个连接中
            let mut tx = pool.begin().await
                .map_err(|e| QuickDbError::QueryError {
                    message: format!("开始事务失败: {}", e),
                })?;
            
            let affected_rows = {
                let mut query = sqlx::query(&sql);
                // 绑定参数
                for param in &params {
                    query = match param {
                        DataValue::String(s) => query.bind(s),
                        DataValue::Int(i) => query.bind(*i),
                        DataValue::Float(f) => query.bind(*f),
                        DataValue::Bool(b) => query.bind(*b),
                        DataValue::DateTime(dt) => query.bind(*dt),
                        DataValue::Uuid(uuid) => query.bind(*uuid),
                        DataValue::Json(json) => query.bind(json.to_string()),
                        DataValue::Bytes(bytes) => query.bind(bytes.as_slice()),
                        DataValue::Null => query.bind(Option::<String>::None),
                        DataValue::Array(arr) => {
                            let json_values: Vec<serde_json::Value> = arr.iter()
                                .map(|v| v.to_json_value())
                                .collect();
                            query.bind(serde_json::to_string(&json_values).unwrap_or_default())
                        },
                        DataValue::Object(obj) => {
                            let json_map: serde_json::Map<String, serde_json::Value> = obj.iter()
                                .map(|(k, v)| (k.clone(), v.to_json_value()))
                                .collect();
                            query.bind(serde_json::to_string(&json_map).unwrap_or_default())
                        },
                    };
                }
                
                let execute_result = query.execute(&mut *tx).await;
                match execute_result {
                    Ok(result) => {
                        let rows = result.rows_affected();
                        debug!("✅ SQL执行成功，影响的行数: {}", rows);
                        rows
                    },
                    Err(e) => {
                        debug!("❌ SQL执行失败: {}", e);
                        return Err(QuickDbError::QueryError {
                            message: format!("执行插入失败: {}", e),
                        });
                    }
                }
            };

            debug!("插入操作最终影响的行数: {}", affected_rows);

            // 根据ID策略获取返回的ID
            let id_value = match id_strategy {
                IdStrategy::AutoIncrement => {
                    // AutoIncrement策略：获取MySQL自动生成的ID
                    let last_id_row = sqlx::query("SELECT LAST_INSERT_ID()")
                        .fetch_one(&mut *tx)
                        .await
                        .map_err(|e| QuickDbError::QueryError {
                            message: format!("获取LAST_INSERT_ID失败: {}", e),
                        })?;

                    let last_id: u64 = last_id_row.try_get(0)
                        .map_err(|e| QuickDbError::QueryError {
                            message: format!("解析LAST_INSERT_ID失败: {}", e),
                        })?;

                    debug!("在事务中获取到的LAST_INSERT_ID: {}", last_id);
                    DataValue::Int(last_id as i64)
                },
                _ => {
                    // 其他策略：使用数据中的ID字段
                    if let Some(id_data) = data.get("id") {
                        debug!("使用数据中的ID字段: {:?}", id_data);
                        id_data.clone()
                    } else {
                        debug!("数据中没有ID字段，返回默认值0");
                        DataValue::Int(0)
                    }
                }
            };

            // 提交事务
            let commit_result = tx.commit().await;
            match commit_result {
                Ok(_) => debug!("✅ 事务提交成功"),
                Err(e) => {
                    debug!("❌ 事务提交失败: {}", e);
                    return Err(QuickDbError::QueryError {
                        message: format!("提交事务失败: {}", e),
                    });
                }
            }

            // 构造返回的DataValue
            let mut result_map = std::collections::HashMap::new();

            result_map.insert("id".to_string(), id_value.clone());
            result_map.insert("affected_rows".to_string(), DataValue::Int(affected_rows as i64));

            debug!("最终返回的DataValue: {:?}", DataValue::Object(result_map.clone()));
            Ok(DataValue::Object(result_map))
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望MySQL连接".to_string(),
            })
        }
    }

    async fn find_by_id(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        id: &DataValue,
    ) -> QuickDbResult<Option<DataValue>> {
        if let DatabaseConnection::MySQL(pool) = connection {
            let condition = QueryCondition {
                field: "id".to_string(),
                operator: QueryOperator::Eq,
                value: id.clone(),
            };
            
            let (sql, params) = SqlQueryBuilder::new()
                .database_type(crate::types::DatabaseType::MySQL)
                .select(&["*"])
                .from(table)
                .where_condition(condition)
                .limit(1)
                .build()?;
            
            let results = self.execute_query(pool, &sql, &params).await?;
            Ok(results.into_iter().next())
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望MySQL连接".to_string(),
            })
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
                operator: crate::types::LogicalOperator::And,
                conditions: group_conditions,
            }]
        };
        
        // 统一使用 find_with_groups 实现
        self.find_with_groups(connection, table, &condition_groups, options).await
    }

    /// MySQL条件组合查找操作
    async fn find_with_groups(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        condition_groups: &[QueryConditionGroup],
        options: &QueryOptions,
    ) -> QuickDbResult<Vec<DataValue>> {
        if let DatabaseConnection::MySQL(pool) = connection {
            let mut builder = SqlQueryBuilder::new()
                .database_type(crate::types::DatabaseType::MySQL)
                .select(&["*"])
                .from(table)
                .where_condition_groups(condition_groups);
            
            // 添加排序
            for sort_field in &options.sort {
                builder = builder.order_by(&sort_field.field, sort_field.direction.clone());
            }
            
            // 添加分页
            if let Some(pagination) = &options.pagination {
                builder = builder.limit(pagination.limit).offset(pagination.skip);
            }
            
            let (sql, params) = builder.build()?;
            
            debug!("执行MySQL条件组合查询: {}", sql);

            self.execute_query(pool, &sql, &params).await
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望MySQL连接".to_string(),
            })
        }
    }

    /// MySQL更新操作
    async fn update(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
        data: &HashMap<String, DataValue>,
    ) -> QuickDbResult<u64> {
        if let DatabaseConnection::MySQL(pool) = connection {
            let (sql, params) = SqlQueryBuilder::new()
                .database_type(crate::types::DatabaseType::MySQL)
                .update(data.clone())
                .from(table)
                .where_conditions(conditions)
                .build()?;

            self.execute_update(pool, &sql, &params).await
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望MySQL连接".to_string(),
            })
        }
    }

    /// MySQL根据ID更新操作
    async fn update_by_id(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        id: &DataValue,
        data: &HashMap<String, DataValue>,
    ) -> QuickDbResult<bool> {
        if let DatabaseConnection::MySQL(pool) = connection {
            let condition = QueryCondition {
                field: "id".to_string(),
                operator: QueryOperator::Eq,
                value: id.clone(),
            };
            
            let (sql, params) = SqlQueryBuilder::new()
                .database_type(crate::types::DatabaseType::MySQL)
                .update(data.clone())
                .from(table)
                .where_condition(condition)
                .build()?;

            let affected_rows = self.execute_update(pool, &sql, &params).await?;
            Ok(affected_rows > 0)
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望MySQL连接".to_string(),
            })
        }
    }

    /// MySQL操作更新操作
    async fn update_with_operations(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
        operations: &[crate::types::UpdateOperation],
    ) -> QuickDbResult<u64> {
        if let DatabaseConnection::MySQL(pool) = connection {
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
                    .database_type(crate::types::DatabaseType::MySQL)
                    .build_where_clause_with_offset(conditions, params.len() + 1)?;

                sql.push_str(&format!(" WHERE {}", where_clause));
                params.extend(where_params);
            }

            debug!("执行MySQL操作更新: {}", sql);

            self.execute_update(pool, &sql, &params).await
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望MySQL连接".to_string(),
            })
        }
    }

    async fn delete(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
    ) -> QuickDbResult<u64> {
        mysql_query::delete(self, connection, table, conditions).await
    }

    async fn delete_by_id(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        id: &DataValue,
    ) -> QuickDbResult<bool> {
        mysql_query::delete_by_id(self, connection, table, id).await
    }

    async fn count(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
    ) -> QuickDbResult<u64> {
        mysql_query::count(self, connection, table, conditions).await
    }

    async fn exists(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
    ) -> QuickDbResult<bool> {
        mysql_query::exists(self, connection, table, conditions).await
    }

    async fn create_table(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        fields: &HashMap<String, FieldDefinition>,
        id_strategy: &IdStrategy,
    ) -> QuickDbResult<()> {
        mysql_schema::create_table(self, connection, table, fields, id_strategy).await
    }

    async fn create_index(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        index_name: &str,
        fields: &[String],
        unique: bool,
    ) -> QuickDbResult<()> {
        mysql_schema::create_index(self, connection, table, index_name, fields, unique).await
    }

    async fn table_exists(
        &self,
        connection: &DatabaseConnection,
        table: &str,
    ) -> QuickDbResult<bool> {
        mysql_schema::table_exists(self, connection, table).await
    }

    async fn drop_table(
        &self,
        connection: &DatabaseConnection,
        table: &str,
    ) -> QuickDbResult<()> {
        mysql_schema::drop_table(self, connection, table).await
    }

    async fn get_server_version(
        &self,
        connection: &DatabaseConnection,
    ) -> QuickDbResult<String> {
        mysql_schema::get_server_version(self, connection).await
    }
}
