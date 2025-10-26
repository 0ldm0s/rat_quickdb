//! # ODM请求处理器模块
//!
//! 按操作类型分离的请求处理器实现

pub mod create_handler;
pub mod read_handler;
pub mod update_handler;
pub mod delete_handler;

// 重新导出所有处理器以保持API兼容性
pub use create_handler::*;
pub use read_handler::*;
pub use update_handler::*;
pub use delete_handler::*;