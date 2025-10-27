//! 连接池模块
//!
//! 基于生产者/消费者模式的高性能数据库连接池
//! SQLite: 单线程队列模式，避免锁竞争
//! MySQL/PostgreSQL/MongoDB: 多连接长连接池，支持保活和重试

// 导入所有子模块
pub mod types;
pub mod config;
pub mod pool;
pub mod sqlite_worker;
pub mod multi_connection_manager;

// 重新导出主要的公共类型和结构体
pub use types::{PooledConnection, DatabaseOperation, DatabaseConnection, ConnectionWorker};
pub use config::ExtendedPoolConfig;
pub use pool::ConnectionPool;
#[cfg(feature = "sqlite-support")]
pub use sqlite_worker::SqliteWorker;
pub use multi_connection_manager::MultiConnectionManager;