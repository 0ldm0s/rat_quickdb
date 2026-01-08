//! 数据库类型定义和配置
//!
//! 定义支持的数据库类型、连接配置和通用数据类型

pub mod cache_config;
pub mod data_value;
pub mod database_config;
pub mod id_types;
pub mod mongo_builder;
pub mod query;
pub mod serde_helpers;
pub mod update_operations;

// 重新导出所有公共类型以保持API兼容性
pub use cache_config::{
    CacheConfig, CacheStrategy, CompressionAlgorithm, CompressionConfig, L1CacheConfig,
    L2CacheConfig, TtlConfig,
};
pub use data_value::DataValue;
pub use database_config::{
    ConnectionConfig, DatabaseConfig, DatabaseType, PoolConfig, TlsConfig, ZstdConfig,
};
pub use id_types::{IdStrategy, IdType};
pub use mongo_builder::MongoDbConnectionBuilder;
pub use query::{
    LogicalOperator, PaginationConfig, QueryCondition, QueryConditionGroup, QueryConditionGroupWithConfig,
    QueryConditionWithConfig, QueryOperator,
    QueryOptions, SortConfig, SortDirection,
};
pub use update_operations::{UpdateOperation, UpdateOperator};
