//! 数据库适配器模块
//! 
//! 提供统一的数据库操作接口，屏蔽不同数据库的实现差异

use crate::error::{QuickDbError, QuickDbResult};
use crate::types::*;
use crate::model::{FieldType, FieldDefinition};
use crate::pool::DatabaseConnection;
use async_trait::async_trait;

use std::collections::HashMap;

// 导入各个数据库适配器 (条件编译)
#[cfg(feature = "sqlite-support")]
mod sqlite;
#[cfg(feature = "postgres-support")]
mod postgres;
#[cfg(feature = "mysql-support")]
mod mysql;
#[cfg(feature = "mongodb-support")]
mod mongodb;
mod query_builder;
mod cached;
mod postgres_utils;

// 条件导出适配器
#[cfg(feature = "sqlite-support")]
pub use sqlite::SqliteAdapter;
#[cfg(feature = "postgres-support")]
pub use postgres::PostgresAdapter;
#[cfg(feature = "mysql-support")]
pub use mysql::MysqlAdapter;
#[cfg(feature = "mongodb-support")]
pub use mongodb::MongoAdapter;
pub use query_builder::*;
pub use cached::CachedDatabaseAdapter;
pub use postgres_utils::{build_json_query_condition, convert_to_jsonb_value};

/// 数据库适配器trait，定义统一的数据库操作接口
#[async_trait]
pub trait DatabaseAdapter: Send + Sync {
    /// 创建记录
    async fn create(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        data: &HashMap<String, DataValue>,
        id_strategy: &IdStrategy,
    ) -> QuickDbResult<DataValue>;

    /// 根据ID查找记录
    async fn find_by_id(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        id: &DataValue,
    ) -> QuickDbResult<Option<DataValue>>;

    /// 查找记录
    async fn find(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
        options: &QueryOptions,
    ) -> QuickDbResult<Vec<DataValue>>;

    /// 使用条件组合查找记录（支持OR逻辑）
    async fn find_with_groups(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        condition_groups: &[QueryConditionGroup],
        options: &QueryOptions,
    ) -> QuickDbResult<Vec<DataValue>>;

    /// 更新记录
    async fn update(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
        data: &HashMap<String, DataValue>,
    ) -> QuickDbResult<u64>;

    /// 使用操作数组更新记录
    async fn update_with_operations(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
        operations: &[crate::types::UpdateOperation],
    ) -> QuickDbResult<u64>;

    /// 根据ID更新记录
    async fn update_by_id(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        id: &DataValue,
        data: &HashMap<String, DataValue>,
    ) -> QuickDbResult<bool>;

    /// 删除记录
    async fn delete(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
    ) -> QuickDbResult<u64>;

    /// 根据ID删除记录
    async fn delete_by_id(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        id: &DataValue,
    ) -> QuickDbResult<bool>;

    /// 统计记录数量
    async fn count(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
    ) -> QuickDbResult<u64>;

    /// 检查记录是否存在
    async fn exists(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
    ) -> QuickDbResult<bool>;

    /// 创建表/集合
    async fn create_table(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        fields: &HashMap<String, FieldDefinition>,
        id_strategy: &IdStrategy,
    ) -> QuickDbResult<()>;

    /// 创建索引
    async fn create_index(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        index_name: &str,
        fields: &[String],
        unique: bool,
    ) -> QuickDbResult<()>;

    /// 检查表是否存在
    async fn table_exists(
        &self,
        connection: &DatabaseConnection,
        table: &str,
    ) -> QuickDbResult<bool>;

    /// 删除表/集合
    async fn drop_table(
        &self,
        connection: &DatabaseConnection,
        table: &str,
    ) -> QuickDbResult<()>;

    /// 获取数据库服务器版本信息
    async fn get_server_version(
        &self,
        connection: &DatabaseConnection,
    ) -> QuickDbResult<String>;

    /// 创建存储过程
    async fn create_stored_procedure(
        &self,
        connection: &DatabaseConnection,
        config: &crate::stored_procedure::StoredProcedureConfig,
    ) -> QuickDbResult<crate::stored_procedure::StoredProcedureCreateResult>;

    /// 执行存储过程查询
    async fn execute_stored_procedure(
        &self,
        connection: &DatabaseConnection,
        procedure_name: &str,
        database: &str,
        params: Option<std::collections::HashMap<String, crate::types::DataValue>>,
    ) -> QuickDbResult<crate::stored_procedure::StoredProcedureQueryResult>;
}

/// 根据数据库类型创建适配器
pub fn create_adapter(db_type: &DatabaseType) -> QuickDbResult<Box<dyn DatabaseAdapter>> {
    match db_type {
        #[cfg(feature = "sqlite-support")]
        DatabaseType::SQLite => Ok(Box::new(SqliteAdapter::new())),
        #[cfg(feature = "mysql-support")]
        DatabaseType::MySQL => Ok(Box::new(MysqlAdapter::new())),
        #[cfg(feature = "postgres-support")]
        DatabaseType::PostgreSQL => Ok(Box::new(PostgresAdapter::new())),
        #[cfg(feature = "mongodb-support")]
        DatabaseType::MongoDB => Ok(Box::new(MongoAdapter::new())),
        _ => Err(QuickDbError::UnsupportedDatabase {
            db_type: format!("{:?} (可能需要启用相应的feature)", db_type),
        }),
    }
}

/// 根据数据库类型和缓存管理器创建带缓存的适配器
pub fn create_adapter_with_cache(
    db_type: &DatabaseType,
    cache_manager: std::sync::Arc<crate::cache::CacheManager>,
) -> QuickDbResult<Box<dyn DatabaseAdapter>> {
    let base_adapter = create_adapter(db_type)?;
    Ok(Box::new(CachedDatabaseAdapter::new(base_adapter, cache_manager)))
}