//! MySQL适配器模块
//!
//! 基于sqlx库实现的MySQL数据库适配器，拆分为多个专门模块

// 导入所有子模块
pub mod adapter;
pub mod operations;
pub mod query;
pub mod query_builder;
pub mod schema;
pub mod utils;

// 重新导出主要的公共类型和结构体
pub use adapter::MysqlAdapter;
