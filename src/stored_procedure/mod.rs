//! 存储过程模块
//!
//! 提供跨数据库的存储过程功能，支持结构化的JOIN关系定义
//! 未来将扩展支持事务控制和复杂存储过程逻辑

use crate::error::QuickDbResult;
use crate::types::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// 导出子模块
pub mod config;
pub mod types;

// 重新导出主要类型
pub use config::*;
pub use types::*;
