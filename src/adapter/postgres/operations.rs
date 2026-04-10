//! PostgreSQL适配器trait实现

use crate::adapter::DatabaseAdapter;
use crate::adapter::PostgresAdapter;
use crate::adapter::postgres::query_builder::SqlQueryBuilder;
use crate::adapter::postgres::utils::row_to_data_map;
use crate::error::{QuickDbError, QuickDbResult};
use crate::manager;
use crate::model::{FieldDefinition, FieldType};
use crate::pool::DatabaseConnection;
use crate::security::quote_identifier;
use crate::types::*;
use async_trait::async_trait;
use rat_logger::debug;
use sqlx::Row;
use std::collections::HashMap;

use super::query as postgres_query;
use super::schema as postgres_schema;


#[async_trait]
impl DatabaseAdapter for PostgresAdapter {
    async fn create(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        data: &HashMap<String, DataValue>,
        id_strategy: &IdStrategy,
        alias: &str,
    ) -> QuickDbResult<DataValue> {
        if let DatabaseConnection::PostgreSQL(pool) = connection {
            // 自动建表逻辑：检查表是否存在，如果不存在则创建
            if !postgres_schema::table_exists(self, connection, table).await? {
                // 获取表创建锁，防止重复创建
                let _lock = self.acquire_table_lock(table).await;

                // 再次检查表是否存在（双重检查锁定模式）
                if !postgres_schema::table_exists(self, connection, table).await? {
                    // 尝试从模型管理器获取预定义的元数据
                    if let Some(model_meta) = crate::manager::get_model_with_alias(table, alias) {
                        debug!("表 {} 不存在，使用预定义模型元数据创建", table);

                        // 使用模型元数据创建表
                        postgres_schema::create_table(
                            self,
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

            // 表已存在，检查是否有SERIAL类型的id字段
            let mut has_auto_increment_id = false;
            let check_serial_sql = "SELECT column_default FROM information_schema.columns WHERE table_name = $1 AND column_name = 'id'";
            let rows = sqlx::query(check_serial_sql)
                .bind(table)
                .fetch_all(pool)
                .await
                .map_err(|e| QuickDbError::QueryError {
                    message: format!("检查表结构失败: {}", e),
                })?;

            if let Some(row) = rows.first() {
                if let Ok(Some(default_value)) = row.try_get::<Option<String>, _>("column_default")
                {
                    has_auto_increment_id = default_value.starts_with("nextval");
                }
            }

            // 准备插入数据
            // 如果数据中没有id字段，说明期望使用自增ID，不需要在INSERT中包含id字段
            // 如果数据中有id字段但表使用SERIAL自增，也要移除id字段让PostgreSQL自动生成
            let mut insert_data = data.clone();
            let data_has_id = insert_data.contains_key("id");

            if !data_has_id || (data_has_id && has_auto_increment_id) {
                insert_data.remove("id");
                debug!("使用PostgreSQL SERIAL自增，不在INSERT中包含id字段");
            } else if data_has_id {
                // 如果有ID字段且指定了ID策略，可能需要转换数据类型
                match id_strategy {
                    IdStrategy::Snowflake { .. } => {
                        // 雪花ID需要转换为整数
                        if let Some(id_value) = insert_data.get("id").cloned() {
                            if let DataValue::String(s) = id_value {
                                if let Ok(num) = s.parse::<i64>() {
                                    insert_data.insert("id".to_string(), DataValue::Int(num));
                                    debug!("将雪花ID从字符串转换为整数: {} -> {}", s, num);
                                }
                            }
                        }
                    }
                    IdStrategy::Uuid => {
                        // UUID需要转换为UUID类型
                        if let Some(id_value) = insert_data.get("id").cloned() {
                            if let DataValue::String(s) = id_value {
                                if let Ok(uuid) = s.parse::<uuid::Uuid>() {
                                    insert_data.insert("id".to_string(), DataValue::Uuid(uuid));
                                    debug!("将UUID从字符串转换为UUID类型: {}", s);
                                }
                            }
                        }
                    }
                    _ => {} // 其他策略不需要转换
                }
            }

            let (sql, params) = SqlQueryBuilder::new()
                .insert(insert_data)
                .returning(&["id"])
                .build(table, alias)?;

            debug!("执行PostgreSQL插入: {}", sql);

            let results = super::utils::execute_query(self, pool, &sql, &params, table).await?;

            if let Some(result) = results.first() {
                Ok(result.clone())
            } else {
                // 创建一个表示成功插入的DataValue
                let mut success_map = HashMap::new();
                success_map.insert("affected_rows".to_string(), DataValue::Int(1));
                Ok(DataValue::Object(success_map))
            }
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望PostgreSQL连接".to_string(),
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
        if let DatabaseConnection::PostgreSQL(pool) = connection {
            let condition = QueryConditionWithConfig {
                field: "id".to_string(),
                operator: QueryOperator::Eq,
                value: id.clone(),
                case_insensitive: false,
            };

            let (sql, params) = SqlQueryBuilder::new()
                .select(&["*"])
                .where_condition(condition)
                .limit(1)
                .build(table, alias)?;

            debug!("执行PostgreSQL根据ID查询: {}", sql);

            let results = super::utils::execute_query(self, pool, &sql, &params, table).await?;
            Ok(results.into_iter().next())
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望PostgreSQL连接".to_string(),
            })
        }
    }

    async fn find_with_cache_control(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryConditionWithConfig],
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
                .map(|c| QueryConditionGroupWithConfig::Single(c.clone()))
                .collect();
            vec![QueryConditionGroupWithConfig::GroupWithConfig {
                operator: LogicalOperator::And,
                conditions: group_conditions,
            }]
        };

        // 统一使用 find_with_groups_with_cache_control_and_config 实现
        self.find_with_groups_with_cache_control_and_config(connection, table, &condition_groups, options, alias, bypass_cache)
            .await
    }

    async fn find_with_groups_with_cache_control_and_config(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        condition_groups: &[QueryConditionGroupWithConfig],
        options: &QueryOptions,
        alias: &str,
        bypass_cache: bool,
    ) -> QuickDbResult<Vec<DataValue>> {
        if let DatabaseConnection::PostgreSQL(pool) = connection {
            let mut builder = SqlQueryBuilder::new()
                .select(&["*"])
                .where_condition_groups(condition_groups);

            // 添加排序
            if !options.sort.is_empty() {
                for sort_field in &options.sort {
                    builder = builder.order_by(&sort_field.field, sort_field.direction.clone());
                }
            }

            // 添加分页
            if let Some(pagination) = &options.pagination {
                builder = builder.limit(pagination.limit).offset(pagination.skip);
            }

            let (sql, params) = builder.build(table, alias)?;

            debug!("执行PostgreSQL条件组查询: {}", sql);

            super::utils::execute_query(self, pool, &sql, &params, table).await
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望PostgreSQL连接".to_string(),
            })
        }
    }

    async fn find_with_groups_with_cache_control(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        condition_groups: &[QueryConditionGroup],
        options: &QueryOptions,
        alias: &str,
        bypass_cache: bool,
    ) -> QuickDbResult<Vec<DataValue>> {
        // 将 QueryConditionGroup 转换为 QueryConditionGroupWithConfig
        fn convert_group(group: &QueryConditionGroup) -> QueryConditionGroupWithConfig {
            match group {
                QueryConditionGroup::Single(c) => {
                    QueryConditionGroupWithConfig::Single(QueryConditionWithConfig {
                        field: c.field.clone(),
                        operator: c.operator.clone(),
                        value: c.value.clone(),
                        case_insensitive: false,
                    })
                }
                QueryConditionGroup::Group { operator, conditions } => {
                    QueryConditionGroupWithConfig::GroupWithConfig {
                        operator: operator.clone(),
                        conditions: conditions.iter().map(convert_group).collect(),
                    }
                }
            }
        }

        let condition_groups_with_config: Vec<QueryConditionGroupWithConfig> =
            condition_groups.iter().map(convert_group).collect();

        self.find_with_groups_with_cache_control_and_config(
            connection,
            table,
            &condition_groups_with_config,
            options,
            alias,
            bypass_cache,
        )
        .await
    }

    async fn find(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryConditionWithConfig],
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

    async fn update(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryConditionWithConfig],
        data: &HashMap<String, DataValue>,
        alias: &str,
    ) -> QuickDbResult<u64> {
        if let DatabaseConnection::PostgreSQL(pool) = connection {
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

            debug!("执行PostgreSQL更新: {}", sql);

            super::utils::execute_update(self, pool, &sql, &params, table).await
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望PostgreSQL连接".to_string(),
            })
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
        let conditions = vec![QueryConditionWithConfig {
            field: "id".to_string(),
            operator: QueryOperator::Eq,
            value: id.clone(),
            case_insensitive: false,
        }];

        let affected = self
            .update(connection, table, &conditions, data, alias)
            .await?;
        Ok(affected > 0)
    }

    async fn update_with_operations(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryConditionWithConfig],
        operations: &[crate::types::UpdateOperation],
        alias: &str,
    ) -> QuickDbResult<u64> {
        if let DatabaseConnection::PostgreSQL(pool) = connection {
            let mut set_clauses = Vec::new();
            let mut params = Vec::new();

            for operation in operations {
                let safe_field = quote_identifier(&operation.field, DatabaseType::PostgreSQL);
                match &operation.operation {
                    crate::types::UpdateOperator::Set => {
                        set_clauses.push(format!("{} = ${}", safe_field, params.len() + 1));
                        params.push(operation.value.clone());
                    }
                    crate::types::UpdateOperator::Increment => {
                        set_clauses.push(format!(
                            "{} = {} + ${}",
                            safe_field,
                            safe_field,
                            params.len() + 1
                        ));
                        params.push(operation.value.clone());
                    }
                    crate::types::UpdateOperator::Decrement => {
                        set_clauses.push(format!(
                            "{} = {} - ${}",
                            safe_field,
                            safe_field,
                            params.len() + 1
                        ));
                        params.push(operation.value.clone());
                    }
                    crate::types::UpdateOperator::Multiply => {
                        set_clauses.push(format!(
                            "{} = {} * ${}",
                            safe_field,
                            safe_field,
                            params.len() + 1
                        ));
                        params.push(operation.value.clone());
                    }
                    crate::types::UpdateOperator::Divide => {
                        set_clauses.push(format!(
                            "{} = {} / ${}",
                            safe_field,
                            safe_field,
                            params.len() + 1
                        ));
                        params.push(operation.value.clone());
                    }
                    crate::types::UpdateOperator::PercentIncrease => {
                        set_clauses.push(format!(
                            "{} = {} * (1.0 + ${}/100.0)",
                            safe_field,
                            safe_field,
                            params.len() + 1
                        ));
                        params.push(operation.value.clone());
                    }
                    crate::types::UpdateOperator::PercentDecrease => {
                        set_clauses.push(format!(
                            "{} = {} * (1.0 - ${}/100.0)",
                            safe_field,
                            safe_field,
                            params.len() + 1
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

            let safe_table = quote_identifier(table, DatabaseType::PostgreSQL);
            let mut sql = format!("UPDATE {} SET {}", safe_table, set_clauses.join(", "));

            // 添加WHERE条件
            if !conditions.is_empty() {
                let (where_clause, mut where_params) = SqlQueryBuilder::new()
                    .build_where_clause_with_offset(conditions, params.len() + 1, table, alias)?;

                sql.push_str(&format!(" WHERE {}", where_clause));
                params.extend(where_params);
            }

            debug!("执行PostgreSQL操作更新: {}", sql);

            super::utils::execute_update(self, pool, &sql, &params, table).await
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望PostgreSQL连接".to_string(),
            })
        }
    }

    async fn delete(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryConditionWithConfig],
        alias: &str,
    ) -> QuickDbResult<u64> {
        postgres_query::delete(self, connection, table, conditions, alias).await
    }

    async fn delete_by_id(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        id: &DataValue,
        alias: &str,
    ) -> QuickDbResult<bool> {
        postgres_query::delete_by_id(self, connection, table, id, alias).await
    }

    async fn count(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryConditionWithConfig],
        alias: &str,
    ) -> QuickDbResult<u64> {
        postgres_query::count(self, connection, table, conditions, alias).await
    }

    async fn count_with_groups(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        condition_groups: &[QueryConditionGroupWithConfig],
        alias: &str,
    ) -> QuickDbResult<u64> {
        postgres_query::count_with_groups(self, connection, table, condition_groups, alias).await
    }

    async fn create_table(
        &self,
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
                    }
                    FieldType::Integer { .. } => "INTEGER".to_string(),
                    FieldType::BigInteger => "BIGINT".to_string(),
                    FieldType::Float { .. } => "REAL".to_string(),
                    FieldType::Double => "DOUBLE PRECISION".to_string(),
                    FieldType::Text => "TEXT".to_string(),
                    FieldType::Boolean => "BOOLEAN".to_string(),
                    FieldType::DateTime => {
                        debug!(
                            "🔍 字段 {} 类型为 DateTime，required: {}",
                            name, field_definition.required
                        );
                        "TIMESTAMPTZ".to_string()
                    }
                    FieldType::DateTimeWithTz { .. } => {
                        debug!(
                            "🔍 字段 {} 类型为 DateTimeWithTz，required: {}",
                            name, field_definition.required
                        );
                        "TIMESTAMPTZ".to_string()
                    }
                    FieldType::Date => "DATE".to_string(),
                    FieldType::Time => "TIME".to_string(),
                    FieldType::Uuid => "UUID".to_string(),
                    FieldType::Json => "JSONB".to_string(),
                    FieldType::Binary => "BYTEA".to_string(),
                    FieldType::Decimal { precision, scale } => {
                        format!("DECIMAL({},{})", precision, scale)
                    }
                    FieldType::Array {
                        item_type: _,
                        max_items: _,
                        min_items: _,
                    } => "JSONB".to_string(),
                    FieldType::Object { .. } => "JSONB".to_string(),
                    FieldType::Reference {
                        target_collection: _,
                    } => "TEXT".to_string(),
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
                    field_definitions.push(format!("{} {} {}", quote_identifier(name, DatabaseType::PostgreSQL), sql_type, null_constraint));
                }
            }

            let sql = format!(
                "CREATE TABLE IF NOT EXISTS {} ({})",
                quote_identifier(table, DatabaseType::PostgreSQL),
                field_definitions.join(", ")
            );

            debug!("🔍 执行PostgreSQL建表SQL: {}", sql);
            debug!("🔍 字段定义详情: {:?}", field_definitions);

            super::utils::execute_update(self, pool, &sql, &[], table).await?;

            Ok(())
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望PostgreSQL连接".to_string(),
            })
        }
    }

    async fn create_index(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        index_name: &str,
        fields: &[String],
        unique: bool,
    ) -> QuickDbResult<()> {
        if let DatabaseConnection::PostgreSQL(pool) = connection {
            let unique_clause = if unique { "UNIQUE " } else { "" };
            let safe_fields: Vec<String> = fields
                .iter()
                .map(|f| quote_identifier(f, DatabaseType::PostgreSQL))
                .collect();
            let sql = format!(
                "CREATE {}INDEX IF NOT EXISTS {} ON {} ({})",
                unique_clause,
                quote_identifier(index_name, DatabaseType::PostgreSQL),
                quote_identifier(table, DatabaseType::PostgreSQL),
                safe_fields.join(", ")
            );

            debug!("执行PostgreSQL索引创建: {}", sql);

            super::utils::execute_update(self, pool, &sql, &[], table).await?;

            Ok(())
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望PostgreSQL连接".to_string(),
            })
        }
    }

    async fn table_exists(
        &self,
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

    async fn drop_table(&self, connection: &DatabaseConnection, table: &str) -> QuickDbResult<()> {
        if let DatabaseConnection::PostgreSQL(pool) = connection {
            let sql = format!("DROP TABLE IF EXISTS {} CASCADE", quote_identifier(table, DatabaseType::PostgreSQL));

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

    async fn get_server_version(&self, connection: &DatabaseConnection) -> QuickDbResult<String> {
        if let DatabaseConnection::PostgreSQL(pool) = connection {
            let sql = "SELECT version()";

            debug!("执行PostgreSQL版本查询SQL: {}", sql);

            let row =
                sqlx::query(sql)
                    .fetch_one(pool)
                    .await
                    .map_err(|e| QuickDbError::QueryError {
                        message: format!("查询PostgreSQL版本失败: {}", e),
                    })?;

            let version: String = row.try_get(0).map_err(|e| QuickDbError::QueryError {
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

    async fn create_stored_procedure(
        &self,
        connection: &DatabaseConnection,
        config: &crate::stored_procedure::StoredProcedureConfig,
    ) -> QuickDbResult<crate::stored_procedure::StoredProcedureCreateResult> {
        use crate::stored_procedure::StoredProcedureCreateResult;
        use crate::types::id_types::IdStrategy;

        debug!("开始创建PostgreSQL存储过程: {}", config.procedure_name);

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

        // 2. 生成PostgreSQL存储过程模板（带占位符）
        let sql_template = self.generate_stored_procedure_sql(&config).await?;
        debug!("生成PostgreSQL存储过程SQL模板: {}", sql_template);

        // 3. 将存储过程信息存储到适配器映射表中（PostgreSQL不需要执行创建SQL）
        let procedure_info = crate::stored_procedure::StoredProcedureInfo {
            config: config.clone(),
            template: sql_template.clone(),
            db_type: "PostgreSQL".to_string(),
            created_at: chrono::Utc::now(),
        };

        let mut procedures = self.stored_procedures.lock().await;
        procedures.insert(config.procedure_name.clone(), procedure_info);
        debug!(
            "✅ PostgreSQL存储过程 {} 模板已存储到适配器映射表",
            config.procedure_name
        );

        Ok(StoredProcedureCreateResult {
            success: true,
            procedure_name: config.procedure_name.clone(),
            error: None,
        })
    }

    /// 执行存储过程查询（PostgreSQL使用视图实现）
    async fn execute_stored_procedure(
        &self,
        connection: &DatabaseConnection,
        procedure_name: &str,
        database: &str,
        params: Option<std::collections::HashMap<String, crate::types::DataValue>>,
    ) -> QuickDbResult<crate::stored_procedure::StoredProcedureQueryResult> {
        use crate::adapter::postgres::adapter::PostgresAdapter;

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
            DatabaseConnection::PostgreSQL(pool) => pool,
            _ => {
                return Err(QuickDbError::ConnectionError {
                    message: "Invalid connection type for PostgreSQL".to_string(),
                });
            }
        };

        debug!("执行存储过程查询SQL: {}", final_sql);

        let rows = sqlx::query(&final_sql).fetch_all(pool).await.map_err(|e| {
            QuickDbError::QueryError {
                message: format!("执行存储过程查询失败: {}", e),
            }
        })?;

        let mut query_result = Vec::new();
        for row in rows {
            let data_map = row_to_data_map(self, &row)?;
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

impl PostgresAdapter {
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

            // GROUP BY替换 - PostgreSQL特殊处理
            if let Some(group_by) = param_map.get("GROUP_BY") {
                let group_by_str = match group_by {
                    crate::types::DataValue::String(s) => s.clone(),
                    _ => group_by.to_string(),
                };
                final_sql = final_sql.replace("{GROUP_BY}", &format!(" GROUP BY {}", group_by_str));
            } else {
                // PostgreSQL特殊处理：如果SQL模板已经包含GROUP BY，则不替换为空字符串
                if final_sql.contains(" GROUP BY") {
                    final_sql = final_sql.replace("{GROUP_BY}", "");
                } else {
                    final_sql = final_sql.replace("{GROUP_BY}", "");
                }
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

        // PostgreSQL特殊处理：不清理GROUP BY子句，因为它是自动生成的
        // 只清理没有内容的占位符
        final_sql = final_sql
            .replace("  ", " ")
            .replace(" ,", ",")
            .replace(", ", ", ")
            .replace(" WHERE ", "")
            // 不删除GROUP BY，因为PostgreSQL需要它
            .replace(" HAVING ", "")
            .replace(" ORDER BY ", "")
            .replace(" LIMIT ", "")
            .replace(" OFFSET ", "");

        debug!("构建的最终SQL: {}", final_sql);
        Ok(final_sql)
    }
}
