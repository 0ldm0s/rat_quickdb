//! 数据库类型定义和配置
//!
//! 定义支持的数据库类型、连接配置和通用数据类型

pub mod serde_helpers;
pub mod database_config;
pub mod data_value;
pub mod query;
pub mod cache_config;
pub mod id_types;
pub mod update_operations;
pub mod mongo_builder;

// 重新导出所有公共类型以保持API兼容性
pub use database_config::{DatabaseConfig, DatabaseType, ConnectionConfig, TlsConfig, ZstdConfig, PoolConfig};
pub use data_value::DataValue;
pub use query::{QueryCondition, QueryOperator, LogicalOperator, QueryConditionGroup, SortConfig, SortDirection, PaginationConfig, QueryOptions};
pub use cache_config::{CacheConfig, CacheStrategy, L1CacheConfig, L2CacheConfig, TtlConfig, CompressionConfig, CompressionAlgorithm};
pub use id_types::{IdStrategy, IdType};
pub use update_operations::{UpdateOperator, UpdateOperation};
pub use mongo_builder::MongoDbConnectionBuilder;