//! 缓存管理模块
//!
//! 提供基于 rat_memcache 的自动缓存功能，支持 LRU 策略、自动更新/清理缓存
//! 以及可选的手动清理接口。

// 导出所有子模块
pub mod stats;
pub mod key_generator;
pub mod record_cache;
pub mod query_cache;
pub mod operations;
pub mod cache_manager;

// 重新导出主要的公共类型和结构体
pub use stats::{CachePerformanceStats, CacheStats};
pub use cache_manager::CacheManager;