    //! 记录缓存操作模块
//!
//! 提供单个记录的缓存存储和读取功能

use crate::types::{IdType, DataValue, CacheConfig};
use anyhow::{anyhow, Result};
use rat_memcache::{RatMemCache, CacheOptions};
use bytes::Bytes;
use std::time::Instant;
use serde_json;
use rat_logger::{warn, debug};
use std::sync::atomic::Ordering;
use std::collections::HashMap;

// 从 cache_manager.rs 中引入 CacheManager
use super::cache_manager::CacheManager;

impl CacheManager {
    pub async fn cache_record(
        &self,
        table: &str,
        id: &IdType,
        data: &DataValue,
    ) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let start_time = Instant::now();
        let key = self.generate_cache_key(table, id, "record");
        
        // 统一序列化方式：始终序列化DataValue包装，确保存储和读取格式一致
        let serialized = serde_json::to_vec(data)
            .map_err(|e| anyhow!("Failed to serialize data: {}", e))?;

        let options = CacheOptions {
            ttl_seconds: Some(self.config.ttl_config.default_ttl_secs),
            ..Default::default()
        };

        self.cache.set_with_options(key.clone(), Bytes::from(serialized), &options).await
            .map_err(|e| anyhow!("Failed to cache record: {}", e))?;

        // 记录缓存键
        self.track_cache_key(table, key).await;

        // 更新统计信息
        let elapsed = start_time.elapsed();
        self.writes_counter.fetch_add(1, Ordering::Relaxed);
        {
            let mut stats = self.stats.write().await;
            stats.writes += 1;
            stats.write_count += 1;
            stats.total_write_latency_ns += elapsed.as_nanos() as u64;
        }

        debug!("已缓存记录: table={}, id={:?}", table, id);
        Ok(())
    }

    /// 获取缓存的记录
    pub async fn get_cached_record(
        &self,
        table: &str,
        id: &IdType,
    ) -> Result<Option<DataValue>> {
        if !self.config.enabled {
            return Ok(None);
        }

        let start_time = Instant::now();
        let key = self.generate_cache_key(table, id, "record");
        match self.cache.get(&key).await {
            Ok(Some(data)) => {
                let deserialized: DataValue = serde_json::from_slice(&data)
                    .map_err(|e| anyhow!("Failed to deserialize cached data: {}", e))?;
                
                // 更新命中统计
                let elapsed = start_time.elapsed();
                self.hits_counter.fetch_add(1, Ordering::Relaxed);
                {
                    let mut stats = self.stats.write().await;
                    stats.hits += 1;
                    stats.query_count += 1;
                    stats.total_query_latency_ns += elapsed.as_nanos() as u64;
                }
                
                debug!("缓存命中: table={}, id={:?}", table, id);
                Ok(Some(deserialized))
            }
            Ok(None) => {
                // 更新未命中统计
                let elapsed = start_time.elapsed();
                self.misses_counter.fetch_add(1, Ordering::Relaxed);
                {
                    let mut stats = self.stats.write().await;
                    stats.misses += 1;
                    stats.query_count += 1;
                    stats.total_query_latency_ns += elapsed.as_nanos() as u64;
                }
                
                debug!("缓存未命中: table={}, id={:?}", table, id);
                Ok(None)
            }
            Err(e) => {
                // 错误也算作未命中
                let elapsed = start_time.elapsed();
                self.misses_counter.fetch_add(1, Ordering::Relaxed);
                {
                    let mut stats = self.stats.write().await;
                    stats.misses += 1;
                    stats.query_count += 1;
                    stats.total_query_latency_ns += elapsed.as_nanos() as u64;
                }
                
                warn!("缓存读取失败: {}", e);
                Ok(None)
            }
        }
    }
}
