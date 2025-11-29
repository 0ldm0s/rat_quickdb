
use crate::adapter::DatabaseAdapter;
use super::SqlQueryBuilder;
use crate::error::{QuickDbError, QuickDbResult};
use crate::types::*;
use crate::model::{FieldDefinition, FieldType};
use crate::pool::DatabaseConnection;
use std::collections::HashMap;
use async_trait::async_trait;
use rat_logger::{debug, info, warn};
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
        alias: &str,
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
                    if let Some(model_meta) = crate::manager::get_model_with_alias(table, alias) {
                        debug!("表 {} 不存在，使用预定义模型元数据创建", table);

                        // 使用模型元数据创建表
                        self.create_table(connection, table, &model_meta.fields, id_strategy, alias).await?;
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
                .build(table, alias)?;
            
            // 构建参数化查询，使用正确的参数顺序
            let mut query = sqlx::query(&sql);
            for param in &params {
                match param {
                    DataValue::String(s) => { query = query.bind(s); },
                    DataValue::Int(i) => { query = query.bind(i); },
                    DataValue::Float(f) => { query = query.bind(f); },
                    DataValue::Bool(b) => { query = query.bind(b); },
                    DataValue::Bytes(bytes) => { query = query.bind(bytes); },
                    DataValue::DateTime(dt) => { query = query.bind(dt.timestamp()); },
                    DataValue::DateTimeUTC(dt) => { query = query.bind(dt.timestamp()); },
                    DataValue::Uuid(uuid) => { query = query.bind(uuid.to_string()); },
                    DataValue::Json(json) => { query = query.bind(json.to_string()); },
                    DataValue::Array(arr) => {
                        // Array字段统一转为字符串数组存储
                        let string_array: Result<Vec<String>, QuickDbError> = arr.iter().map(|item| {
                            Ok(match item {
                                DataValue::String(s) => s.clone(),
                                DataValue::Int(i) => i.to_string(),
                                DataValue::Float(f) => f.to_string(),
                                DataValue::Uuid(uuid) => uuid.to_string(),
                                _ => {
                                    return Err(QuickDbError::ValidationError {
                                        field: "array_field".to_string(),
                                        message: format!("Array字段不支持该类型: {:?}，只支持String、Int、Float、Uuid类型", item),
                                    });
                                }
                            })
                        }).collect();
                        let string_array = string_array?;
                        let json = serde_json::to_string(&string_array).unwrap_or_default();
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
        alias: &str,
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
                    // 获取字段元数据
                    let model_meta = crate::manager::get_model_with_alias(table, alias)
                        .ok_or_else(|| QuickDbError::ValidationError {
                            field: "model".to_string(),
                            message: format!("模型 '{}' 不存在", table),
                        })?;

                    // 使用新的元数据转换函数
                    let data_map = super::data_conversion::row_to_data_map_with_metadata(&r, &model_meta.fields)?;
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
        alias: &str,
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
        self.find_with_groups(connection, table, &condition_groups, options, alias).await
    }

    async fn find_with_groups(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        condition_groups: &[QueryConditionGroup],
        options: &QueryOptions,
        alias: &str,
    ) -> QuickDbResult<Vec<DataValue>> {
        let pool = match connection {
            DatabaseConnection::SQLite(pool) => pool,
            _ => return Err(QuickDbError::ConnectionError {
                message: "Invalid connection type for SQLite".to_string(),
            }),
        };
        {
            let (sql, params) = SqlQueryBuilder::new()
                .select(&["*"])
                .where_condition_groups(condition_groups)
                .limit(options.pagination.as_ref().map(|p| p.limit).unwrap_or(1000))
                .offset(options.pagination.as_ref().map(|p| p.skip).unwrap_or(0))
                .build(table, alias)?;

            debug!("执行SQLite条件组合查询: {}", sql);

            let mut query = sqlx::query(&sql);
            for param in &params {
                match param {
                    DataValue::String(s) => { query = query.bind(s); },
                    DataValue::Int(i) => { query = query.bind(i); },
                    DataValue::Float(f) => { query = query.bind(f); },
                    DataValue::Bool(b) => { query = query.bind(b); },
                    DataValue::DateTime(dt) => { query = query.bind(dt.timestamp()); },
                    DataValue::DateTimeUTC(dt) => { query = query.bind(dt.timestamp()); }, // DateTime转换为时间戳
                    DataValue::Null => { query = query.bind(Option::<String>::None); },
                    _ => { query = query.bind(param.to_string()); },
                }
            }

            let rows = query.fetch_all(pool).await
                .map_err(|e| QuickDbError::QueryError {
                    message: format!("执行SQLite条件组合查询失败: {}", e),
                })?;

            // 获取字段元数据
            let model_meta = crate::manager::get_model_with_alias(table, alias)
                .ok_or_else(|| QuickDbError::ValidationError {
                    field: "model".to_string(),
                    message: format!("模型 '{}' 不存在", table),
                })?;

            let mut results = Vec::new();
            for row in rows {
                // 使用新的元数据转换函数
                let data_map = super::data_conversion::row_to_data_map_with_metadata(&row, &model_meta.fields)?;
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
        alias: &str,
    ) -> QuickDbResult<u64> {
        let pool = match connection {
            DatabaseConnection::SQLite(pool) => pool,
            _ => return Err(QuickDbError::ConnectionError {
                message: "Invalid connection type for SQLite".to_string(),
            }),
        };
        {
            // 获取字段元数据进行验证和转换
            let model_meta = crate::manager::get_model_with_alias(table, alias)
                .ok_or_else(|| QuickDbError::ValidationError {
                    field: "model".to_string(),
                    message: format!("模型 '{}' 不存在", table),
                })?;

            // 使用timezone模块中的字段元数据处理函数进行验证和转换
            let field_map: std::collections::HashMap<String, crate::model::FieldDefinition> = model_meta.fields.iter()
                .map(|(name, f)| (name.clone(), f.clone()))
                .collect();
            let validated_data = crate::utils::timezone::process_data_fields_from_metadata(data.clone(), &field_map);

            let (sql, params) = SqlQueryBuilder::new()
                .update(validated_data)
                .where_conditions(conditions)
                .build(table, alias)?;

            // 构建包含实际参数的完整SQL用于调试
            let mut complete_sql = sql.clone();
            for param in &params {
                let param_value = match param {
                    DataValue::String(s) => format!("'{}'", s.replace('\'', "''")),
                    DataValue::Int(i) => i.to_string(),
                    DataValue::Float(f) => f.to_string(),
                    DataValue::Bool(b) => if *b { "1".to_string() } else { "0".to_string() },
                    DataValue::DateTime(dt) => dt.timestamp().to_string(),
                    DataValue::DateTimeUTC(dt) => dt.timestamp().to_string(),
                    DataValue::Null => "NULL".to_string(),
                    _ => format!("'{}'", param.to_string()),
                };
                complete_sql = complete_sql.replacen('?', &param_value, 1);
            }

            println!("🔍 SQLite Complete SQL: {}", complete_sql);

            let mut query = sqlx::query(&sql);
            for param in &params {
                match param {
                    DataValue::String(s) => { query = query.bind(s); },
                    DataValue::Int(i) => { query = query.bind(i); },
                    DataValue::Float(f) => { query = query.bind(f); },
                    DataValue::Bool(b) => { query = query.bind(b); },
                    DataValue::DateTime(dt) => { query = query.bind(dt.timestamp()); },
                    DataValue::DateTimeUTC(dt) => { query = query.bind(dt.timestamp()); },
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
        alias: &str,
    ) -> QuickDbResult<bool> {
        let condition = QueryCondition {
            field: "id".to_string(),
            operator: QueryOperator::Eq,
            value: id.clone(),
        };
        
        let affected_rows = self.update(connection, table, &[condition], data, alias).await?;
        Ok(affected_rows > 0)
    }

    async fn update_with_operations(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
        operations: &[crate::types::UpdateOperation],
        alias: &str,
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
                .build_where_clause_with_offset(conditions, params.len() + 1, table, alias)?;

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
        alias: &str,
    ) -> QuickDbResult<u64> {
        sqlite_query::delete(self, connection, table, conditions, alias).await
    }

    async fn delete_by_id(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        id: &DataValue,
        alias: &str,
    ) -> QuickDbResult<bool> {
        sqlite_query::delete_by_id(self, connection, table, id, alias).await
    }

    async fn count(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
        alias: &str,
    ) -> QuickDbResult<u64> {
        sqlite_query::count(self, connection, table, conditions, alias).await
    }

    async fn create_table(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        fields: &HashMap<String, FieldDefinition>,
        id_strategy: &IdStrategy,
        alias: &str,
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

    async fn create_stored_procedure(
        &self,
        connection: &DatabaseConnection,
        config: &crate::stored_procedure::StoredProcedureConfig,
    ) -> QuickDbResult<crate::stored_procedure::StoredProcedureCreateResult> {
        use crate::stored_procedure::{StoredProcedureCreateResult, JoinType};

        debug!("开始创建SQLite存储过程: {}", config.procedure_name);

        // 验证配置
        config.validate()
            .map_err(|e| crate::error::QuickDbError::ValidationError {
                field: "config".to_string(),
                message: format!("存储过程配置验证失败: {}", e),
            })?;

        let pool = match connection {
            DatabaseConnection::SQLite(pool) => pool,
            _ => return Err(crate::error::QuickDbError::ConnectionError {
                message: "Invalid connection type for SQLite".to_string(),
            }),
        };

        // 1. 确保依赖表存在
        for model_meta in &config.dependencies {
            let table_name = &model_meta.collection_name;
            if !self.table_exists(connection, table_name).await? {
                debug!("依赖表 {} 不存在，尝试创建", table_name);
                // 使用存储的模型元数据和数据库的ID策略创建表
                let id_strategy = crate::manager::get_id_strategy(&config.database)
                    .unwrap_or(IdStrategy::AutoIncrement);

                self.create_table(connection, table_name, &model_meta.fields, &id_strategy, &config.database).await?;
            }
        }

        // 2. 生成SQL存储过程模板（带占位符）
        let sql_template = self.generate_stored_procedure_sql(&config).await?;
        debug!("生成存储过程SQL模板: {}", sql_template);

        // 3. 将存储过程信息存储到适配器映射表中（SQLite不需要执行创建SQL）
        let procedure_info = crate::stored_procedure::StoredProcedureInfo {
            config: config.clone(),
            template: sql_template.clone(),
            db_type: "SQLite".to_string(),
            created_at: chrono::Utc::now(),
        };

        let mut procedures = self.stored_procedures.lock().await;
        procedures.insert(config.procedure_name.clone(), procedure_info);
        debug!("✅ 存储过程 {} 模板已存储到适配器映射表", config.procedure_name);

        Ok(StoredProcedureCreateResult {
            success: true,
            procedure_name: config.procedure_name.clone(),
            error: None,
        })
    }

    /// 执行存储过程查询（SQLite使用视图实现）
    async fn execute_stored_procedure(
        &self,
        connection: &DatabaseConnection,
        procedure_name: &str,
        database: &str,
        params: Option<std::collections::HashMap<String, crate::types::DataValue>>,
    ) -> QuickDbResult<crate::stored_procedure::StoredProcedureQueryResult> {
        use crate::adapter::sqlite::adapter::SqliteAdapter;

        // 获取存储过程信息
        let procedures = self.stored_procedures.lock().await;
        let procedure_info = procedures.get(procedure_name).ok_or_else(|| {
            crate::error::QuickDbError::ValidationError {
                field: "procedure_name".to_string(),
                message: format!("存储过程 '{}' 不存在", procedure_name),
            }
        })?;
        let sql_template = procedure_info.template.clone();
        drop(procedures);

        debug!("执行存储过程查询: {}, 模板: {}", procedure_name, sql_template);

        // 构建最终的SQL查询
        let final_sql = self.build_final_query_from_template(&sql_template, params).await?;

        // 执行查询
        // 直接执行SQL查询（复用find_with_groups的模式）
        let pool = match connection {
            DatabaseConnection::SQLite(pool) => pool,
            _ => return Err(QuickDbError::ConnectionError {
                message: "Invalid connection type for SQLite".to_string(),
            }),
        };

        debug!("执行存储过程查询SQL: {}", final_sql);

        let rows = sqlx::query(&final_sql).fetch_all(pool).await
            .map_err(|e| QuickDbError::QueryError {
                message: format!("执行存储过程查询失败: {}", e),
            })?;

        let mut query_result = Vec::new();
        for row in rows {
            let data_map = self.row_to_data_map(&row)?;
            query_result.push(data_map);
        }

        // 转换结果格式
        let mut result = Vec::new();
        for row_data in query_result {
            let mut row_map = std::collections::HashMap::new();
            for (key, value) in row_data {
                row_map.insert(key, value);
            }
            result.push(row_map);
        }

        debug!("存储过程 {} 执行完成，返回 {} 条记录", procedure_name, result.len());
        Ok(result)
    }
}

impl SqliteAdapter {
    /// 根据模板和参数构建最终查询SQL
    async fn build_final_query_from_template(
        &self,
        template: &str,
        params: Option<std::collections::HashMap<String, crate::types::DataValue>>,
    ) -> QuickDbResult<String> {
        let mut final_sql = template.to_string();

        // 替换占位符
        if let Some(param_map) = params {
            // WHERE条件替换
            if let Some(where_clause) = param_map.get("WHERE") {
                let where_str = match where_clause {
                    crate::types::DataValue::String(s) => s.clone(),
                    _ => where_clause.to_string(),
                };
                final_sql = final_sql.replace("{WHERE}", &format!(" WHERE {}", where_str));
            } else {
                final_sql = final_sql.replace("{WHERE}", "");
            }

            // GROUP BY替换
            if let Some(group_by) = param_map.get("GROUP_BY") {
                let group_by_str = match group_by {
                    crate::types::DataValue::String(s) => s.clone(),
                    _ => group_by.to_string(),
                };
                final_sql = final_sql.replace("{GROUP_BY}", &format!(" GROUP BY {}", group_by_str));
            } else {
                final_sql = final_sql.replace("{GROUP_BY}", "");
            }

            // HAVING替换
            if let Some(having) = param_map.get("HAVING") {
                let having_str = match having {
                    crate::types::DataValue::String(s) => s.clone(),
                    _ => having.to_string(),
                };
                final_sql = final_sql.replace("{HAVING}", &format!(" HAVING {}", having_str));
            } else {
                final_sql = final_sql.replace("{HAVING}", "");
            }

            // ORDER BY替换
            if let Some(order_by) = param_map.get("ORDER_BY") {
                let order_by_str = match order_by {
                    crate::types::DataValue::String(s) => s.clone(),
                    _ => order_by.to_string(),
                };
                final_sql = final_sql.replace("{ORDER_BY}", &format!(" ORDER BY {}", order_by_str));
            } else {
                final_sql = final_sql.replace("{ORDER_BY}", "");
            }

            // LIMIT替换
            if let Some(limit) = param_map.get("LIMIT") {
                let limit_str = match limit {
                    crate::types::DataValue::Int(i) => i.to_string(),
                    _ => limit.to_string(),
                };
                final_sql = final_sql.replace("{LIMIT}", &format!(" LIMIT {}", limit_str));
            } else {
                final_sql = final_sql.replace("{LIMIT}", "");
            }

            // OFFSET替换
            if let Some(offset) = param_map.get("OFFSET") {
                let offset_str = match offset {
                    crate::types::DataValue::Int(i) => i.to_string(),
                    _ => offset.to_string(),
                };
                final_sql = final_sql.replace("{OFFSET}", &format!(" OFFSET {}", offset_str));
            } else {
                final_sql = final_sql.replace("{OFFSET}", "");
            }
        } else {
            // 没有参数时，移除所有占位符
            final_sql = final_sql
                .replace("{WHERE}", "")
                .replace("{GROUP_BY}", "")
                .replace("{HAVING}", "")
                .replace("{ORDER_BY}", "")
                .replace("{LIMIT}", "")
                .replace("{OFFSET}", "");
        }

        // 只清理没有参数时留下的空占位符
        final_sql = final_sql
            .replace("{WHERE}", "")
            .replace("{GROUP_BY}", "")
            .replace("{HAVING}", "")
            .replace("{ORDER_BY}", "")
            .replace("{LIMIT}", "")
            .replace("{OFFSET}", "")
            .replace("  ", " ")
            .replace(" ,", ",")
            .replace(", ", ", ");

        // 移除连续空格和多余的空格
        final_sql = final_sql.trim().to_string();

        info!("构建的最终SQL: {}", final_sql);
        Ok(final_sql)
    }
}

