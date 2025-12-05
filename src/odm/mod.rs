//! # 异步ODM层模块
//!
//! 通过消息传递和无锁队列机制避免直接持有连接引用，解决生命周期问题
//! 按职责分离的细粒度模块组织

// 核心模块
pub mod manager_core;
pub mod traits;
pub mod types;

// 请求处理器模块
pub mod handlers;

// 操作实现模块
pub mod operations;

// 全局管理器模块
pub mod global;

// 重新导出所有公共类型以保持API兼容性
pub use global::*;
pub use handlers::*;
pub use manager_core::AsyncOdmManager;
pub use operations::*;
pub use traits::OdmOperations;
pub use types::OdmRequest;
