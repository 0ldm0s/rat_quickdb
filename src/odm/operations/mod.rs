//! # ODM操作实现模块
//!
//! 按操作类型分离的ODM操作实现

pub mod odm_operations_impl;

// 重新导出所有操作实现以保持API兼容性
pub use odm_operations_impl::*;
