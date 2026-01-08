//! 连接池类型定义模块

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::oneshot;

use super::ExtendedPoolConfig;
use crate::error::{QuickDbError, QuickDbResult};
use crate::model::{FieldDefinition, FieldType};
use crate::types::*;

/// 池化连接 - 用于兼容旧接口
#[derive(Debug, Clone)]
pub struct PooledConnection {
    /// 连接ID
    pub id: String,
    /// 数据库类型
    pub db_type: DatabaseType,
    /// 数据库别名（用于兼容manager.rs）
    pub alias: String,
}

/// 数据库操作请求
#[derive(Debug)]
pub enum DatabaseOperation {
    /// 创建记录
    Create {
        table: String,
        data: HashMap<String, DataValue>,
        id_strategy: IdStrategy,
        alias: String,
        response: oneshot::Sender<QuickDbResult<DataValue>>,
    },
    /// 根据ID查找记录
    FindById {
        table: String,
        id: DataValue,
        alias: String,
        response: oneshot::Sender<QuickDbResult<Option<DataValue>>>,
    },
    /// 查找记录（支持缓存控制）
    Find {
        table: String,
        conditions: Vec<QueryConditionWithConfig>,
        options: QueryOptions,
        alias: String,
        response: oneshot::Sender<QuickDbResult<Vec<DataValue>>>,
    },
    /// 查找记录（强制跳过缓存）
    FindWithBypassCache {
        table: String,
        conditions: Vec<QueryConditionWithConfig>,
        options: QueryOptions,
        alias: String,
        bypass_cache: bool,
        response: oneshot::Sender<QuickDbResult<Vec<DataValue>>>,
    },
    /// 使用条件组合查找记录（支持OR逻辑）
    FindWithGroups {
        table: String,
        condition_groups: Vec<QueryConditionGroup>,
        options: QueryOptions,
        alias: String,
        response: oneshot::Sender<QuickDbResult<Vec<DataValue>>>,
    },
    /// 使用条件组合查找记录（强制跳过缓存）
    FindWithGroupsWithBypassCache {
        table: String,
        condition_groups: Vec<QueryConditionGroupWithConfig>,
        options: QueryOptions,
        alias: String,
        bypass_cache: bool,
        response: oneshot::Sender<QuickDbResult<Vec<DataValue>>>,
    },
    /// 更新记录
    Update {
        table: String,
        conditions: Vec<QueryConditionWithConfig>,
        data: HashMap<String, DataValue>,
        alias: String,
        response: oneshot::Sender<QuickDbResult<u64>>,
    },
    /// 使用操作数组更新记录
    UpdateWithOperations {
        table: String,
        conditions: Vec<QueryConditionWithConfig>,
        operations: Vec<crate::types::UpdateOperation>,
        alias: String,
        response: oneshot::Sender<QuickDbResult<u64>>,
    },
    /// 根据ID更新记录
    UpdateById {
        table: String,
        id: DataValue,
        data: HashMap<String, DataValue>,
        alias: String,
        response: oneshot::Sender<QuickDbResult<bool>>,
    },
    /// 删除记录
    Delete {
        table: String,
        conditions: Vec<QueryConditionWithConfig>,
        alias: String,
        response: oneshot::Sender<QuickDbResult<u64>>,
    },
    /// 根据ID删除记录
    DeleteById {
        table: String,
        id: DataValue,
        alias: String,
        response: oneshot::Sender<QuickDbResult<bool>>,
    },
    /// 统计记录
    Count {
        table: String,
        conditions: Vec<QueryConditionWithConfig>,
        alias: String,
        response: oneshot::Sender<QuickDbResult<u64>>,
    },
    /// 创建表
    CreateTable {
        table: String,
        fields: HashMap<String, FieldDefinition>,
        id_strategy: IdStrategy,
        alias: String,
        response: oneshot::Sender<QuickDbResult<()>>,
    },
    /// 创建索引
    CreateIndex {
        table: String,
        index_name: String,
        fields: Vec<String>,
        unique: bool,
        response: oneshot::Sender<QuickDbResult<()>>,
    },
    /// 检查表是否存在
    TableExists {
        table: String,
        response: oneshot::Sender<QuickDbResult<bool>>,
    },
    /// 删除表
    DropTable {
        table: String,
        response: oneshot::Sender<QuickDbResult<()>>,
    },
    /// 获取服务器版本
    GetServerVersion {
        response: oneshot::Sender<QuickDbResult<String>>,
    },
    /// 创建存储过程
    CreateStoredProcedure {
        config: crate::stored_procedure::StoredProcedureConfig,
        response:
            oneshot::Sender<QuickDbResult<crate::stored_procedure::StoredProcedureCreateResult>>,
    },
    /// 执行存储过程
    ExecuteStoredProcedure {
        procedure_name: String,
        database: String,
        params: Option<std::collections::HashMap<String, crate::types::DataValue>>,
        response:
            oneshot::Sender<QuickDbResult<crate::stored_procedure::StoredProcedureQueryResult>>,
    },
}

/// 原生数据库连接枚举 - 直接持有数据库连接，不使用Arc包装
#[derive(Debug)]
pub enum DatabaseConnection {
    #[cfg(feature = "sqlite-support")]
    SQLite(sqlx::SqlitePool),
    #[cfg(feature = "postgres-support")]
    PostgreSQL(sqlx::PgPool),
    #[cfg(feature = "mysql-support")]
    MySQL(sqlx::MySqlPool),
    #[cfg(feature = "mongodb-support")]
    MongoDB(mongodb::Database),
}

/// 连接工作器 - 持有数据库连接池并处理操作
pub struct ConnectionWorker {
    /// 工作器ID
    pub id: String,
    /// 数据库连接池
    pub connection: DatabaseConnection,
    /// 连接池配置
    pub pool_config: ExtendedPoolConfig,
    /// 连接创建时间
    pub created_at: Instant,
    /// 最后使用时间
    pub last_used: Instant,
    /// 重试次数
    pub retry_count: u32,
    /// 数据库类型
    pub db_type: DatabaseType,
    /// 数据库适配器（持久化，避免重复创建）
    pub adapter: Box<dyn crate::adapter::DatabaseAdapter + Send + Sync>,
}

impl std::fmt::Debug for ConnectionWorker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConnectionWorker")
            .field("id", &self.id)
            .field("connection", &self.connection)
            .field("created_at", &self.created_at)
            .field("last_used", &self.last_used)
            .field("retry_count", &self.retry_count)
            .field("db_type", &self.db_type)
            .field("adapter", &"<DatabaseAdapter>")
            .finish()
    }
}
