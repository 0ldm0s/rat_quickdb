//! PostgreSQL数据库适配器
//! 
//! 使用tokio-postgres库实现真实的PostgreSQL数据库操作

use super::{DatabaseAdapter, SqlQueryBuilder};
use crate::error::{QuickDbError, QuickDbResult};
use crate::types::{*, IdStrategy};
use crate::{FieldType, FieldDefinition};
use crate::pool::DatabaseConnection;
use crate::table::{TableManager, TableSchema, ColumnType};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use rat_logger::{info, error, warn, debug};
use sqlx::{Row, Column, TypeInfo};
// 移除不存在的rat_logger::prelude导入

/// PostgreSQL适配器
pub struct PostgresAdapter {
    /// 表创建锁，防止重复创建表
    creation_locks: Arc<Mutex<HashMap<String, ()>>>,
}

impl PostgresAdapter {
    /// 创建新的PostgreSQL适配器实例
    pub fn new() -> Self {
        Self {
            creation_locks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 获取表创建锁
    async fn acquire_table_lock(&self, table: &str) -> tokio::sync::MutexGuard<'_, HashMap<String, ()>> {
        let mut locks = self.creation_locks.lock().await;
        if !locks.contains_key(table) {
            locks.insert(table.to_string(), ());
            debug!("🔒 获取表 {} 的创建锁", table);
        }
        locks
    }

    /// 释放表创建锁
    async fn release_table_lock(&self, table: &str, mut locks: tokio::sync::MutexGuard<'_, HashMap<String, ()>>) {
        locks.remove(table);
        debug!("🔓 释放表 {} 的创建锁", table);
    }
}

impl PostgresAdapter {


    /// 将PostgreSQL行转换为DataValue映射
    fn row_to_data_map(&self, row: &sqlx::postgres::PgRow) -> QuickDbResult<HashMap<String, DataValue>> {
        let mut map = HashMap::new();
        
        for column in row.columns() {
            let column_name = column.name();
            
            // 根据PostgreSQL类型转换值
            let data_value = match column.type_info().name() {
                "INT4" | "INT8" => {
                    if let Ok(val) = row.try_get::<Option<i32>, _>(column_name) {
                        match val {
                            Some(i) => DataValue::Int(i as i64),
                            None => DataValue::Null,
                        }
                    } else if let Ok(val) = row.try_get::<Option<i64>, _>(column_name) {
                        match val {
                            Some(i) => {
                                // 如果是id字段且值很大，可能是雪花ID，转换为字符串保持跨数据库兼容性
                                if column_name == "id" && i > 1000000000000000000 {
                                    DataValue::String(i.to_string())
                                } else {
                                    DataValue::Int(i)
                                }
                            },
                            None => DataValue::Null,
                        }
                    } else {
                        DataValue::Null
                    }
                },
                "FLOAT4" | "FLOAT8" => {
                    if let Ok(val) = row.try_get::<Option<f32>, _>(column_name) {
                        match val {
                            Some(f) => DataValue::Float(f as f64),
                            None => DataValue::Null,
                        }
                    } else if let Ok(val) = row.try_get::<Option<f64>, _>(column_name) {
                        match val {
                            Some(f) => DataValue::Float(f),
                            None => DataValue::Null,
                        }
                    } else {
                        DataValue::Null
                    }
                },
                "BOOL" => {
                    if let Ok(val) = row.try_get::<Option<bool>, _>(column_name) {
                        match val {
                            Some(b) => DataValue::Bool(b),
                            None => DataValue::Null,
                        }
                    } else {
                        DataValue::Null
                    }
                },
                "TEXT" | "VARCHAR" | "CHAR" => {
                    if let Ok(val) = row.try_get::<Option<String>, _>(column_name) {
                        match val {
                            Some(s) => DataValue::String(s),
                            None => DataValue::Null,
                        }
                    } else {
                        DataValue::Null
                    }
                },
                "UUID" => {
                    if let Ok(val) = row.try_get::<Option<uuid::Uuid>, _>(column_name) {
                        match val {
                            Some(u) => {
                                // 将UUID转换为字符串以保持跨数据库兼容性
                                DataValue::String(u.to_string())
                            },
                            None => DataValue::Null,
                        }
                    } else {
                        DataValue::Null
                    }
                },
                "JSON" | "JSONB" => {
                    // PostgreSQL原生支持JSONB，直接获取serde_json::Value
                    // 无需像MySQL/SQLite那样解析JSON字符串
                    if let Ok(val) = row.try_get::<Option<serde_json::Value>, _>(column_name) {
                        match val {
                            Some(json_val) => {
                                // 使用现有的转换函数，确保类型正确
                                crate::types::json_value_to_data_value(json_val)
                            },
                            None => DataValue::Null,
                        }
                    } else {
                        DataValue::Null
                    }
                },
                "TIMESTAMP" | "TIMESTAMPTZ" => {
                    if let Ok(val) = row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>(column_name) {
                        match val {
                            Some(dt) => DataValue::DateTime(dt),
                            None => DataValue::Null,
                        }
                    } else {
                        DataValue::Null
                    }
                },
                _ => {
                    // 对于未知类型，尝试作为字符串获取
                    if let Ok(val) = row.try_get::<Option<String>, _>(column_name) {
                        match val {
                            Some(s) => DataValue::String(s),
                            None => DataValue::Null,
                        }
                    } else {
                        DataValue::Null
                    }
                }
            };
            
            map.insert(column_name.to_string(), data_value);
        }
        
        Ok(map)
    }

    /// 将PostgreSQL行转换为JSON值（保留用于向后兼容）
    fn row_to_json(&self, row: &sqlx::postgres::PgRow) -> QuickDbResult<Value> {
        let data_map = self.row_to_data_map(row)?;
        let mut json_map = serde_json::Map::new();
        
        for (key, value) in data_map {
            json_map.insert(key, value.to_json_value());
        }
        
        Ok(Value::Object(json_map))
    }

    /// 执行查询并返回结果
    async fn execute_query(
        &self,
        pool: &sqlx::Pool<sqlx::Postgres>,
        sql: &str,
        params: &[DataValue],
    ) -> QuickDbResult<Vec<DataValue>> {
        let mut query = sqlx::query(sql);

        // 绑定参数
        for param in params {
            query = match param {
                DataValue::String(s) => {
                    // 尝试判断是否为UUID格式，如果是则转换为UUID类型
                    match s.parse::<uuid::Uuid>() {
                        Ok(uuid) => query.bind(uuid), // 绑定为UUID类型
                        Err(_) => query.bind(s),       // 不是UUID格式，绑定为字符串
                    }
                },
                DataValue::Int(i) => query.bind(*i),
                DataValue::Float(f) => query.bind(*f),
                DataValue::Bool(b) => query.bind(*b),
                DataValue::DateTime(dt) => query.bind(*dt),
                DataValue::Uuid(uuid) => query.bind(*uuid),
                DataValue::Json(json) => query.bind(json),
                DataValue::Bytes(bytes) => query.bind(bytes.as_slice()),
                DataValue::Null => query.bind(Option::<String>::None),
                DataValue::Array(arr) => {
                    // 使用 to_json_value() 避免序列化时包含类型标签
                    let json_array = DataValue::Array(arr.clone()).to_json_value();
                    query.bind(json_array)
                },
                DataValue::Object(obj) => {
                    // 使用 to_json_value() 避免序列化时包含类型标签
                    let json_object = DataValue::Object(obj.clone()).to_json_value();
                    query.bind(json_object)
                },
            };
        }
        
        let rows = query.fetch_all(pool)
            .await
            .map_err(|e| QuickDbError::QueryError {
                message: format!("执行PostgreSQL查询失败: {}", e),
            })?;
        
        let mut results = Vec::new();
        for row in rows {
            let data_map = self.row_to_data_map(&row)?;
            results.push(DataValue::Object(data_map));
        }
        
        Ok(results)
    }

    /// 执行更新操作
    async fn execute_update(
        &self,
        pool: &sqlx::Pool<sqlx::Postgres>,
        sql: &str,
        params: &[DataValue],
    ) -> QuickDbResult<u64> {
        let mut query = sqlx::query(sql);
        
        // 绑定参数
        for param in params {
            query = match param {
                DataValue::String(s) => {
                    // 尝试判断是否为UUID格式，如果是则转换为UUID类型
                    match s.parse::<uuid::Uuid>() {
                        Ok(uuid) => query.bind(uuid), // 绑定为UUID类型
                        Err(_) => query.bind(s),       // 不是UUID格式，绑定为字符串
                    }
                },
                DataValue::Int(i) => query.bind(*i),
                DataValue::Float(f) => query.bind(*f),
                DataValue::Bool(b) => query.bind(*b),
                DataValue::DateTime(dt) => query.bind(*dt),
                DataValue::Uuid(uuid) => query.bind(*uuid),
                DataValue::Json(json) => query.bind(json),
                DataValue::Bytes(bytes) => query.bind(bytes.as_slice()),
                DataValue::Null => query.bind(Option::<String>::None),
                DataValue::Array(arr) => query.bind(serde_json::to_value(arr).unwrap_or_default()),
                DataValue::Object(obj) => query.bind(serde_json::to_value(obj).unwrap_or_default()),
            };
        }
        
        let result = query.execute(pool)
            .await
            .map_err(|e| QuickDbError::QueryError {
                message: format!("执行PostgreSQL更新失败: {}", e),
            })?;
        
        Ok(result.rows_affected())
    }
}

#[async_trait]
impl DatabaseAdapter for PostgresAdapter {
    async fn create(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        data: &HashMap<String, DataValue>,
        id_strategy: &IdStrategy,
    ) -> QuickDbResult<DataValue> {
        if let DatabaseConnection::PostgreSQL(pool) = connection {
            // 自动建表逻辑：检查表是否存在，如果不存在则创建
            if !self.table_exists(connection, table).await? {
                // 获取表创建锁，防止重复创建
                let _lock = self.acquire_table_lock(table).await;

                // 再次检查表是否存在（双重检查锁定模式）
                if !self.table_exists(connection, table).await? {
                    // 尝试从模型管理器获取预定义的元数据
                    if let Some(model_meta) = crate::manager::get_model(table) {
                        info!("表 {} 不存在，使用预定义模型元数据创建", table);

                        // 使用模型元数据创建表
                        self.create_table(connection, table, &model_meta.fields, id_strategy).await?;
                        info!("✅ 使用模型元数据创建PostgreSQL表 '{}' 成功", table);

                        // 等待100ms确保数据库事务完全提交
                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                        info!("⏱️ 等待100ms确保表 '{}' 创建完成", table);
                    } else {
                        return Err(QuickDbError::ValidationError {
                            field: "table_creation".to_string(),
                            message: format!("表 '{}' 不存在，且没有预定义的模型元数据。请先定义模型并使用 define_model! 宏明确指定字段类型。", table),
                        });
                    }
                } else {
                    info!("表 {} 已存在，跳过创建", table);
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
                if let Ok(Some(default_value)) = row.try_get::<Option<String>, _>("column_default") {
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
                    },
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
                    },
                    _ => {} // 其他策略不需要转换
                }
            }
            
            let (sql, params) = SqlQueryBuilder::new()
                .database_type(super::query_builder::DatabaseType::PostgreSQL)
                .insert(insert_data)
                .from(table)
                .returning(&["id"])
                .build()?;
            
            debug!("执行PostgreSQL插入: {}", sql);
            
            let results = self.execute_query(pool, &sql, &params).await?;
            
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
    ) -> QuickDbResult<Option<DataValue>> {
        if let DatabaseConnection::PostgreSQL(pool) = connection {
            let condition = QueryCondition {
                field: "id".to_string(),
                operator: QueryOperator::Eq,
                value: id.clone(),
            };
            
            let (sql, params) = SqlQueryBuilder::new()
                .database_type(super::query_builder::DatabaseType::PostgreSQL)
                .select(&["*"])
                .from(table)
                .where_condition(condition)
                .limit(1)
                .build()?;
            
            debug!("执行PostgreSQL根据ID查询: {}", sql);
            
            let results = self.execute_query(pool, &sql, &params).await?;
            Ok(results.into_iter().next())
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望PostgreSQL连接".to_string(),
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
        if let DatabaseConnection::PostgreSQL(pool) = connection {
            let mut builder = SqlQueryBuilder::new()
                .database_type(super::query_builder::DatabaseType::PostgreSQL)
                .select(&["*"])
                .from(table)
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
            
            let (sql, params) = builder.build()?;
            
            debug!("执行PostgreSQL条件组查询: {}", sql);
            
            self.execute_query(pool, &sql, &params).await
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望PostgreSQL连接".to_string(),
            })
        }
    }

    async fn update(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
        data: &HashMap<String, DataValue>,
    ) -> QuickDbResult<u64> {
        if let DatabaseConnection::PostgreSQL(pool) = connection {
            let (sql, params) = SqlQueryBuilder::new()
                .database_type(super::query_builder::DatabaseType::PostgreSQL)
                .update(data.clone())
                .from(table)
                .where_conditions(conditions)
                .build()?;
            
            debug!("执行PostgreSQL更新: {}", sql);
            
            self.execute_update(pool, &sql, &params).await
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
    ) -> QuickDbResult<bool> {
        let conditions = vec![QueryCondition {
            field: "id".to_string(),
            operator: QueryOperator::Eq,
            value: id.clone(),
        }];
        
        let affected = self.update(connection, table, &conditions, data).await?;
        Ok(affected > 0)
    }

    async fn delete(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
    ) -> QuickDbResult<u64> {
        if let DatabaseConnection::PostgreSQL(pool) = connection {
            let (sql, params) = SqlQueryBuilder::new()
                .database_type(super::query_builder::DatabaseType::PostgreSQL)
                .delete()
                .from(table)
                .where_conditions(conditions)
                .build()?;
            
            debug!("执行PostgreSQL删除: {}", sql);
            
            self.execute_update(pool, &sql, &params).await
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望PostgreSQL连接".to_string(),
            })
        }
    }

    async fn delete_by_id(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        id: &DataValue,
    ) -> QuickDbResult<bool> {
        let conditions = vec![QueryCondition {
            field: "id".to_string(),
            operator: QueryOperator::Eq,
            value: id.clone(),
        }];
        
        let affected = self.delete(connection, table, &conditions).await?;
        Ok(affected > 0)
    }

    async fn count(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
    ) -> QuickDbResult<u64> {
        if let DatabaseConnection::PostgreSQL(pool) = connection {
            let (sql, params) = SqlQueryBuilder::new()
                .database_type(super::query_builder::DatabaseType::PostgreSQL)
                .select(&["COUNT(*) as count"])
                .from(table)
                .where_conditions(conditions)
                .build()?;
            
            debug!("执行PostgreSQL计数: {}", sql);
            
            let results = self.execute_query(pool, &sql, &params).await?;
            if let Some(result) = results.first() {
                if let DataValue::Object(obj) = result {
                    if let Some(DataValue::Int(count)) = obj.get("count") {
                        return Ok(*count as u64);
                    }
                }
            }
            
            Ok(0)
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望PostgreSQL连接".to_string(),
            })
        }
    }

    async fn exists(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
    ) -> QuickDbResult<bool> {
        let count = self.count(connection, table, conditions).await?;
        Ok(count > 0)
    }

    async fn create_table(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        fields: &HashMap<String, FieldDefinition>,
        id_strategy: &IdStrategy,
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
                        info!("🔍 字段 {} 类型为 DateTime，required: {}", name, field_definition.required);
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
                    info!("🔍 字段 {} 定义: {} {}", name, sql_type, null_constraint);
                    field_definitions.push(format!("{} {} {}", name, sql_type, null_constraint));
                }
            }
            
            let sql = format!(
                "CREATE TABLE IF NOT EXISTS {} ({})",
                table,
                field_definitions.join(", ")
            );

            info!("🔍 执行PostgreSQL建表SQL: {}", sql);
            info!("🔍 字段定义详情: {:?}", field_definitions);

            self.execute_update(pool, &sql, &[]).await?;
            
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
            let sql = format!(
                "CREATE {}INDEX IF NOT EXISTS {} ON {} ({})",
                unique_clause,
                index_name,
                table,
                fields.join(", ")
            );
            
            debug!("执行PostgreSQL索引创建: {}", sql);
            
            self.execute_update(pool, &sql, &[]).await?;
            
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

    async fn drop_table(
        &self,
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
            println!("🔍 删除后验证表 {} 是否存在: {}", table, still_exists);

            info!("成功删除PostgreSQL表: {}", table);
            Ok(())
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望PostgreSQL连接".to_string(),
            })
        }
    }

    async fn get_server_version(
        &self,
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

            info!("成功获取PostgreSQL版本: {}", version);
            Ok(version)
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望PostgreSQL连接".to_string(),
            })
        }
    }
}