//! 缓存管理器核心模块
//!
//! 提供CacheManager的结构定义和构造函数

use super::stats::CachePerformanceStats;
use crate::types::{CacheConfig, CacheStrategy, CompressionAlgorithm};
use anyhow::{Result, anyhow};
use rat_logger::{debug, info};
use rat_memcache::config::{L1Config, L2Config, LoggingConfig, PerformanceConfig, TtlConfig};
use rat_memcache::types::EvictionStrategy;
use rat_memcache::{RatMemCache, RatMemCacheBuilder};
use std::{collections::HashMap, path::PathBuf, sync::Arc, sync::atomic::AtomicU64};
use tokio::sync::RwLock;

/// 缓存键前缀
pub const CACHE_KEY_PREFIX: &str = "rat_quickdb";

/// 缓存管理器
#[derive(Debug, Clone)]
pub struct CacheManager {
    /// 内部缓存实例
    pub(crate) cache: Arc<RatMemCache>,
    /// 缓存配置
    pub(crate) config: CacheConfig,
    /// 表名到缓存键的映射
    pub(crate) table_keys: Arc<RwLock<HashMap<String, Vec<String>>>>,
    /// 性能统计
    pub(crate) stats: Arc<RwLock<CachePerformanceStats>>,
    /// 原子计数器用于高频统计
    pub(crate) hits_counter: Arc<AtomicU64>,
    pub(crate) misses_counter: Arc<AtomicU64>,
    pub(crate) writes_counter: Arc<AtomicU64>,
    pub(crate) deletes_counter: Arc<AtomicU64>,
}

impl CacheManager {
    /// 创建新的缓存管理器
    pub async fn new(config: CacheConfig) -> Result<Self> {
        // 直接使用用户传入的配置，不使用预设配置
        debug!("创建缓存管理器，配置: {:?}", config);

        let builder = RatMemCacheBuilder::new()
            .l1_config(rat_memcache::config::L1Config {
                max_memory: config.l1_config.max_memory_mb * 1024 * 1024, // 转换为字节
                max_entries: config.l1_config.max_capacity,
                eviction_strategy: match config.strategy {
                    CacheStrategy::Lru => EvictionStrategy::Lru,
                    CacheStrategy::Lfu => EvictionStrategy::Lfu,
                    CacheStrategy::Fifo => EvictionStrategy::Fifo,
                    CacheStrategy::Custom(_) => EvictionStrategy::Lru, // 默认使用LRU
                },
            })
            .l2_config(rat_memcache::config::L2Config {
                enable_l2_cache: config.l2_config.is_some(),
                data_dir: config
                    .l2_config
                    .as_ref()
                    .map(|c| PathBuf::from(&c.storage_path)),
                max_disk_size: config
                    .l2_config
                    .as_ref()
                    .map(|c| c.max_disk_mb as u64 * 1024 * 1024)
                    .unwrap_or(500 * 1024 * 1024),
                write_buffer_size: 64 * 1024 * 1024,
                max_write_buffer_number: 3,
                block_cache_size: 16 * 1024 * 1024,
                enable_lz4: config.compression_config.enabled,
                compression_threshold: config.compression_config.threshold_bytes,
                compression_max_threshold: config.compression_config.threshold_bytes * 10, // 最大阈值为最小阈值的10倍
                compression_level: config
                    .l2_config
                    .as_ref()
                    .map(|c| c.compression_level)
                    .unwrap_or(6),
                background_threads: 2,
                clear_on_startup: config
                    .l2_config
                    .as_ref()
                    .map(|c| c.clear_on_startup)
                    .unwrap_or(false),
                cache_size_mb: config
                    .l2_config
                    .as_ref()
                    .map(|c| c.max_disk_mb)
                    .unwrap_or(500),
                max_file_size_mb: config
                    .l2_config
                    .as_ref()
                    .map(|c| c.max_disk_mb / 2)
                    .unwrap_or(250),
                smart_flush_enabled: true,
                smart_flush_base_interval_ms: 100,
                smart_flush_min_interval_ms: 20,
                smart_flush_max_interval_ms: 500,
                smart_flush_write_rate_threshold: 10000,
                smart_flush_accumulated_bytes_threshold: 4 * 1024 * 1024,
                cache_warmup_strategy: rat_memcache::config::CacheWarmupStrategy::Recent,
                zstd_compression_level: None,
                l2_write_strategy: "write_through".to_string(),
                l2_write_threshold: 1024,
                l2_write_ttl_threshold: 3600,
            })
            .ttl_config(rat_memcache::config::TtlConfig {
                expire_seconds: Some(config.ttl_config.default_ttl_secs),
                cleanup_interval: config.ttl_config.check_interval_secs,
                max_cleanup_entries: 1000,
                lazy_expiration: true,
                active_expiration: true,
            })
            .performance_config(rat_memcache::config::PerformanceConfig {
                worker_threads: 4,
                enable_concurrency: true,
                read_write_separation: true,
                batch_size: 1000,
                enable_warmup: true,
                large_value_threshold: 10240,
            })
            .logging_config(rat_memcache::config::LoggingConfig {
                level: "INFO".to_string(),
                enable_colors: true,
                show_timestamp: true,
                enable_performance_logs: true,
                enable_audit_logs: true,
                enable_cache_logs: true,
                enable_logging: true,
                enable_async: false,
                batch_size: 2048,
                batch_interval_ms: 25,
                buffer_size: 16384,
            });

        let cache = builder
            .build()
            .await
            .map_err(|e| anyhow!("Failed to create cache: {}", e))?;

        info!(
            "缓存管理器初始化成功 - L1容量: {}, L1内存: {}MB, L2磁盘: {}MB, 策略: {:?}",
            config.l1_config.max_capacity,
            config.l1_config.max_memory_mb,
            config
                .l2_config
                .as_ref()
                .map(|c| c.max_disk_mb)
                .unwrap_or(0),
            config.strategy
        );

        Ok(Self {
            cache: Arc::new(cache),
            config,
            table_keys: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(CachePerformanceStats::new())),
            hits_counter: Arc::new(AtomicU64::new(0)),
            misses_counter: Arc::new(AtomicU64::new(0)),
            writes_counter: Arc::new(AtomicU64::new(0)),
            deletes_counter: Arc::new(AtomicU64::new(0)),
        })
    }
}
