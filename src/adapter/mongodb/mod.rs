//! MongoDB适配器模块
//!
//! 提供MongoDB数据库的完整适配器实现，采用模块化设计：
//! - adapter.rs: 核心适配器结构
//! - operations.rs: DatabaseAdapter trait实现
//! - utils.rs: BSON数据转换工具函数
//! - query.rs: 查询相关操作
//! - schema.rs: 集合和索引管理

pub mod adapter;
pub mod operations;
pub mod utils;
pub mod query;
pub mod query_builder;
pub mod schema;

// 重新导出核心类型
pub use adapter::MongoAdapter;