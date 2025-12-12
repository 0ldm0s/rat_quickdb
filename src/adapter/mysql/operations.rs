
//! MySQL适配器trait实现

use crate::adapter::DatabaseAdapter;
use crate::adapter::MysqlAdapter;
use crate::adapter::mysql::query_builder::SqlQueryBuilder;
use crate::error::{QuickDbError, QuickDbResult};
use crate::manager;
use crate::model::{FieldDefinition, FieldType};
use crate::pool::DatabaseConnection;
use crate::types::*;
use async_trait::async_trait;
use rat_logger::debug;
use sqlx::Row;
use std::collections::HashMap;

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
        alias: &str,
    ) -> QuickDbResult<DataValue> {
        if let DatabaseConnection::MySQL(pool) = connection {
            // 自动建表逻辑：检查表是否存在，如果不存在则创建
            if !self.table_exists(connection, table).await? {
                // 获取表创建锁，防止重复创建
                let _lock = self.acquire_table_lock(table).await;
                // 再次检查表是否存在（双重检查锁定模式）
                if !self.table_exists(connection, table).await? {
                    // 尝试从模型管理器获取预定义的元数据
                    if let Some(model_meta) = manager::get_model_with_alias(table, alias) {
                        debug!("表 {} 不存在，使用预定义模型元数据创建", table);

                        // 使用模型元数据创建表
                        self.create_table(
                            connection,
                            table,
                            &model_meta.fields,
                            id_strategy,
                            alias,
                        )
                        .await?;
                        // 等待100ms确保数据库事务完全提交
                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                        debug!("⏱️ 等待100ms确保表 '{}' 创建完成", table);
                    } else {
                        return Err(QuickDbError::ValidationError {
                            field: "table_creation".to_string(),
                            message: format!(
                                "表 '{}' 不存在，且没有预定义的模型元数据。请先定义模型并使用 define_model! 宏明确指定字段类型。",
                                table
                            ),
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

            debug!("生成的INSERT SQL: {}", sql);
            debug!("绑定参数: {:?}", params);

            // 使用事务确保插入和获取ID在同一个连接中
            let mut tx = pool.begin().await.map_err(|e| QuickDbError::QueryError {
                message: format!("开始事务失败: {}", e),
            })?;

            let affected_rows = {
                let mut query = sqlx::query::<sqlx::MySql>(&sql);
                // 绑定参数
                for param in &params {
                    query = match param {
                        DataValue::String(s) => query.bind(s),
                        DataValue::Int(i) => query.bind(i),
                        DataValue::UInt(u) => {
                            // MySQL 支持无符号整数，但 sqlx 可能没有直接支持
                            if *u <= i64::MAX as u64 {
                                query.bind(*u as i64)
                            } else {
                                query.bind(u.to_string())
                            }
                        }
                        DataValue::Float(f) => query.bind(f),
                        DataValue::Bool(b) => query.bind(b),
                        DataValue::DateTime(dt) => query.bind(dt.naive_utc().and_utc()),
                        DataValue::DateTimeUTC(dt) => query.bind(dt.naive_utc()),
                        DataValue::Uuid(uuid) => query.bind(uuid),
                        DataValue::Json(json) => query.bind(json.to_string()),
                        DataValue::Bytes(bytes) => query.bind(bytes.as_slice()),
                        DataValue::Null => query.bind(Option::<String>::None),
                        DataValue::Array(arr) => {
                            let json_values: Vec<serde_json::Value> =
                                arr.iter().map(|v| v.to_json_value()).collect();
                            query.bind(serde_json::to_string(&json_values).unwrap_or_default())
                        }
                        DataValue::Object(obj) => {
                            let json_map: serde_json::Map<String, serde_json::Value> = obj
                                .iter()
                                .map(|(k, v)| (k.clone(), v.to_json_value()))
                                .collect();
                            query.bind(serde_json::to_string(&json_map).unwrap_or_default())
                        }
                    };
                }

                let execute_result = query.execute(&mut *tx).await;
                match execute_result {
                    Ok(result) => {
                        let rows = result.rows_affected();
                        debug!("✅ SQL执行成功，影响的行数: {}", rows);
                        rows
                    }
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
                    let last_id_row = sqlx::query::<sqlx::MySql>("SELECT LAST_INSERT_ID()")
                        .fetch_one(&mut *tx)
                        .await
                        .map_err(|e| QuickDbError::QueryError {
                            message: format!("获取LAST_INSERT_ID失败: {}", e),
                        })?;

                    let last_id: u64 =
                        last_id_row
                            .try_get(0)
                            .map_err(|e| QuickDbError::QueryError {
                                message: format!("解析LAST_INSERT_ID失败: {}", e),
                            })?;

                    debug!("在事务中获取到的LAST_INSERT_ID: {}", last_id);
                    DataValue::Int(last_id as i64)
                }
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
            result_map.insert(
                "affected_rows".to_string(),
                DataValue::Int(affected_rows as i64),
            );

            debug!(
                "最终返回的DataValue: {:?}",
                DataValue::Object(result_map.clone())
            );
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
        alias: &str,
    ) -> QuickDbResult<Option<DataValue>> {
        if let DatabaseConnection::MySQL(pool) = connection {
            let condition = QueryCondition {
                field: "id".to_string(),
                operator: QueryOperator::Eq,
                value: id.clone(),
            };

            let (sql, params) = SqlQueryBuilder::new()
                .select(&["*"])
                .where_condition(condition)
                .limit(1)
                .build(table, alias)?;

            let results = self.execute_query(pool, &sql, &params, table).await?;
            Ok(results.into_iter().next())
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望MySQL连接".to_string(),
            })
        }
    }

    async fn find_with_cache_control(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
        options: &QueryOptions,
        alias: &str,
        bypass_cache: bool,
    ) -> QuickDbResult<Vec<DataValue>> {
        // 将简单条件转换为条件组合（AND逻辑）
        let condition_groups = if conditions.is_empty() {
            vec![]
        } else {
            let group_conditions = conditions
                .iter()
                .map(|c| QueryConditionGroup::Single(c.clone()))
                .collect();
            vec![QueryConditionGroup::Group {
                operator: crate::types::LogicalOperator::And,
                conditions: group_conditions,
            }]
        };

        // 统一使用 find_with_groups_with_cache_control 实现
        self.find_with_groups_with_cache_control(connection, table, &condition_groups, options, alias, bypass_cache)
            .await
    }

    /// MySQL条件组合查找操作
    async fn find_with_groups_with_cache_control(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        condition_groups: &[QueryConditionGroup],
        options: &QueryOptions,
        alias: &str,
        bypass_cache: bool,
    ) -> QuickDbResult<Vec<DataValue>> {
        if let DatabaseConnection::MySQL(pool) = connection {
            let mut builder = SqlQueryBuilder::new()
                .select(&["*"])
                .where_condition_groups(condition_groups);

            // 添加排序
            for sort_field in &options.sort {
                builder = builder.order_by(&sort_field.field, sort_field.direction.clone());
            }

            // 添加分页
            if let Some(pagination) = &options.pagination {
                builder = builder.limit(pagination.limit).offset(pagination.skip);
            }

            let (sql, params) = builder.build(table, alias)?;

            debug!("执行MySQL条件组合查询: {}", sql);

            self.execute_query(pool, &sql, &params, table).await
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
        alias: &str,
    ) -> QuickDbResult<Vec<DataValue>> {
        self.find_with_cache_control(connection, table, conditions, options, alias, false).await
    }

    async fn find_with_groups(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        condition_groups: &[QueryConditionGroup],
        options: &QueryOptions,
        alias: &str,
    ) -> QuickDbResult<Vec<DataValue>> {
        self.find_with_groups_with_cache_control(connection, table, condition_groups, options, alias, false).await
    }

    /// MySQL更新操作
    async fn update(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
        data: &HashMap<String, DataValue>,
        alias: &str,
    ) -> QuickDbResult<u64> {
        if let DatabaseConnection::MySQL(pool) = connection {
            // 获取字段元数据进行验证和转换
            let model_meta =
                crate::manager::get_model_with_alias(table, alias).ok_or_else(|| {
                    QuickDbError::ValidationError {
                        field: "model".to_string(),
                        message: format!("模型 '{}' 不存在", table),
                    }
                })?;

            // 验证字段存在性，并处理DateTimeWithTz字段转换
            let field_map: std::collections::HashMap<String, crate::model::FieldDefinition> =
                model_meta
                    .fields
                    .iter()
                    .map(|(name, f)| (name.clone(), f.clone()))
                    .collect();

            let mut validated_data = HashMap::new();
            for (field_name, data_value) in data {
                if let Some(field_def) = field_map.get(field_name) {
                    if matches!(
                        field_def.field_type,
                        crate::model::FieldType::DateTimeWithTz { .. }
                    ) {
                        // DateTimeWithTz字段：将String转换为DateTime
                        let converted = match data_value {
                            DataValue::String(s) => chrono::DateTime::parse_from_rfc3339(s)
                                .map(|dt| {
                                    DataValue::DateTime(
                                        dt.with_timezone(&chrono::FixedOffset::east(0)),
                                    )
                                })
                                .unwrap_or(data_value.clone()),
                            DataValue::DateTimeUTC(dt) => {
                                DataValue::DateTime(dt.with_timezone(&chrono::FixedOffset::east(0)))
                            }
                            _ => data_value.clone(),
                        };
                        validated_data.insert(field_name.clone(), converted);
                    } else {
                        validated_data.insert(field_name.clone(), data_value.clone());
                    }
                } else {
                    return Err(QuickDbError::ValidationError {
                        field: field_name.clone(),
                        message: format!("字段 '{}' 在模型中不存在", field_name),
                    });
                }
            }

            let (sql, params) = SqlQueryBuilder::new()
                .update(validated_data)
                .where_conditions(conditions)
                .build(table, alias)?;

            self.execute_update(pool, &sql, &params, table).await
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
        alias: &str,
    ) -> QuickDbResult<bool> {
        if let DatabaseConnection::MySQL(pool) = connection {
            let condition = QueryCondition {
                field: "id".to_string(),
                operator: QueryOperator::Eq,
                value: id.clone(),
            };

            let (sql, params) = SqlQueryBuilder::new()
                .update(data.clone())
                .where_condition(condition)
                .build(table, alias)?;

            let affected_rows = self.execute_update(pool, &sql, &params, table).await?;
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
        alias: &str,
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
                        set_clauses.push(format!(
                            "{} = {} * (1.0 + ?/100.0)",
                            operation.field, operation.field
                        ));
                        params.push(operation.value.clone());
                    }
                    crate::types::UpdateOperator::PercentDecrease => {
                        set_clauses.push(format!(
                            "{} = {} * (1.0 - ?/100.0)",
                            operation.field, operation.field
                        ));
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

            debug!("执行MySQL操作更新: {}", sql);

            self.execute_update(pool, &sql, &params, table).await
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
        alias: &str,
    ) -> QuickDbResult<u64> {
        mysql_query::delete(self, connection, table, conditions, alias).await
    }

    async fn delete_by_id(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        id: &DataValue,
        alias: &str,
    ) -> QuickDbResult<bool> {
        mysql_query::delete_by_id(self, connection, table, id, alias).await
    }

    async fn count(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
        alias: &str,
    ) -> QuickDbResult<u64> {
        mysql_query::count(self, connection, table, conditions, alias).await
    }

    async fn create_table(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        fields: &HashMap<String, FieldDefinition>,
        id_strategy: &IdStrategy,
        alias: &str,
    ) -> QuickDbResult<()> {
        mysql_schema::create_table(self, connection, table, fields, id_strategy, alias).await
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

    async fn drop_table(&self, connection: &DatabaseConnection, table: &str) -> QuickDbResult<()> {
        mysql_schema::drop_table(self, connection, table).await
    }

    async fn get_server_version(&self, connection: &DatabaseConnection) -> QuickDbResult<String> {
        mysql_schema::get_server_version(self, connection).await
    }

    async fn create_stored_procedure(
        &self,
        connection: &DatabaseConnection,
        config: &crate::stored_procedure::StoredProcedureConfig,
    ) -> QuickDbResult<crate::stored_procedure::StoredProcedureCreateResult> {
        use crate::stored_procedure::StoredProcedureCreateResult;
        use crate::types::id_types::IdStrategy;

        debug!("开始创建MySQL存储过程: {}", config.procedure_name);

        // 验证配置
        config
            .validate()
            .map_err(|e| crate::error::QuickDbError::ValidationError {
                field: "config".to_string(),
                message: format!("存储过程配置验证失败: {}", e),
            })?;

        // 1. 确保依赖表存在
        for model_meta in &config.dependencies {
            let table_name = &model_meta.collection_name;
            if !self.table_exists(connection, table_name).await? {
                debug!("依赖表 {} 不存在，尝试创建", table_name);
                // 使用存储的模型元数据和数据库的ID策略创建表
                let id_strategy = crate::manager::get_id_strategy(&config.database)
                    .unwrap_or(IdStrategy::AutoIncrement);

                self.create_table(
                    connection,
                    table_name,
                    &model_meta.fields,
                    &id_strategy,
                    &config.database,
                )
                .await?;
            }
        }

        // 2. 生成MySQL存储过程模板（带占位符）
        let sql_template = self.generate_stored_procedure_sql(&config).await?;
        debug!("生成MySQL存储过程SQL模板: {}", sql_template);

        // 3. 将存储过程信息存储到适配器映射表中（MySQL不需要执行创建SQL）
        let procedure_info = crate::stored_procedure::StoredProcedureInfo {
            config: config.clone(),
            template: sql_template.clone(),
            db_type: "MySQL".to_string(),
            created_at: chrono::Utc::now(),
        };

        let mut procedures = self.stored_procedures.lock().await;
        procedures.insert(config.procedure_name.clone(), procedure_info);
        debug!(
            "✅ MySQL存储过程 {} 模板已存储到适配器映射表",
            config.procedure_name
        );

        Ok(StoredProcedureCreateResult {
            success: true,
            procedure_name: config.procedure_name.clone(),
            error: None,
        })
    }

    /// 执行存储过程查询（MySQL使用视图实现）
    async fn execute_stored_procedure(
        &self,
        connection: &DatabaseConnection,
        procedure_name: &str,
        database: &str,
        params: Option<std::collections::HashMap<String, crate::types::DataValue>>,
    ) -> QuickDbResult<crate::stored_procedure::StoredProcedureQueryResult> {
        use crate::adapter::mysql::adapter::MysqlAdapter;

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

        debug!(
            "执行存储过程查询: {}, 模板: {}",
            procedure_name, sql_template
        );

        // 构建最终的SQL查询（复用SQLite的逻辑）
        let final_sql = self
            .build_final_query_from_template(&sql_template, params)
            .await?;

        // 执行查询
        // 直接执行SQL查询（复用find_with_groups的模式）
        let pool = match connection {
            DatabaseConnection::MySQL(pool) => pool,
            _ => {
                return Err(QuickDbError::ConnectionError {
                    message: "Invalid connection type for MySQL".to_string(),
                });
            }
        };

        debug!("执行存储过程查询SQL: {}", final_sql);

        let rows = sqlx::query::<sqlx::MySql>(&final_sql)
            .fetch_all(pool)
            .await
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

        debug!(
            "存储过程 {} 执行完成，返回 {} 条记录",
            procedure_name,
            result.len()
        );
        Ok(result)
    }
}

impl MysqlAdapter {
    /// 根据模板和参数构建最终查询SQL（复用SQLite的逻辑）
    async fn build_final_query_from_template(
        &self,
        template: &str,
        params: Option<std::collections::HashMap<String, crate::types::DataValue>>,
    ) -> QuickDbResult<String> {
        let mut final_sql = template.to_string();

        // 替换占位符（与SQLite逻辑相同）
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

        // 清理多余的空格和逗号
        final_sql = final_sql
            .replace("  ", " ")
            .replace(" ,", ",")
            .replace(", ", ", ")
            .replace(" WHERE ", "")
            // MySQL特殊处理：不清理GROUP BY子句，因为它是自动生成的
            .replace(" HAVING ", "")
            .replace(" ORDER BY ", "")
            .replace(" LIMIT ", "")
            .replace(" OFFSET ", "");

        debug!("构建的最终SQL: {}", final_sql);
        Ok(final_sql)
    }
}
