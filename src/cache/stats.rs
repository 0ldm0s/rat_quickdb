//! 缓存统计模块
//!
//! 提供缓存性能统计和信息收集功能

use serde::{Deserialize, Serialize};

// 从 cache_manager.rs 中引入 CacheManager
use super::cache_manager::CacheManager;

/// 缓存性能统计
#[derive(Debug, Clone)]
pub struct CachePerformanceStats {
    /// 缓存命中次数
    pub hits: u64,
    /// 缓存未命中次数
    pub misses: u64,
    /// 缓存写入次数
    pub writes: u64,
    /// 缓存删除次数
    pub deletes: u64,
    /// 总查询延迟（纳秒）
    pub total_query_latency_ns: u64,
    /// 总写入延迟（纳秒）
    pub total_write_latency_ns: u64,
    /// 查询次数
    pub query_count: u64,
    /// 写入次数
    pub write_count: u64,
}

impl CachePerformanceStats {
    pub fn new() -> Self {
        Self {
            hits: 0,
            misses: 0,
            writes: 0,
            deletes: 0,
            total_query_latency_ns: 0,
            total_write_latency_ns: 0,
            query_count: 0,
            write_count: 0,
        }
    }

    /// 计算命中率
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }

    /// 计算平均查询延迟（毫秒）
    pub fn avg_query_latency_ms(&self) -> f64 {
        if self.query_count == 0 {
            0.0
        } else {
            (self.total_query_latency_ns as f64 / self.query_count as f64) / 1_000_000.0
        }
    }

    /// 计算平均写入延迟（毫秒）
    pub fn avg_write_latency_ms(&self) -> f64 {
        if self.write_count == 0 {
            0.0
        } else {
            (self.total_write_latency_ns as f64 / self.write_count as f64) / 1_000_000.0
        }
    }
}
/// 缓存统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    /// 缓存命中次数
    pub hits: u64,
    /// 缓存未命中次数
    pub misses: u64,
    /// 缓存命中率
    pub hit_rate: f64,
    /// 当前缓存条目数
    pub entries: usize,
    /// 内存使用量（字节）
    pub memory_usage_bytes: usize,
    /// 磁盘使用量（字节）
    pub disk_usage_bytes: usize,
}

impl Default for CacheStats {
    fn default() -> Self {
        Self {
            hits: 0,
            misses: 0,
            hit_rate: 0.0,
            entries: 0,
            memory_usage_bytes: 0,
            disk_usage_bytes: 0,
        }
    }
}
