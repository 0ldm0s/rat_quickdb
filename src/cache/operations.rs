    //! 缓存操作模块
//!
//! 提供缓存的清理、失效、批量操作等维护功能

use crate::types::{IdType, DataValue, CacheConfig};
use anyhow::{anyhow, Result};
use rat_memcache::{RatMemCache, CacheOptions};
use std::collections::HashMap;
use rat_logger::{warn, info, debug};
use std::sync::atomic::Ordering;

// 从 cache_manager.rs 中引入 CACHE_KEY_PREFIX 和 CacheManager
use super::cache_manager::{CACHE_KEY_PREFIX, CacheManager};
// 从 stats.rs 中引入统计类型
use super::stats::{CachePerformanceStats, CacheStats};

impl CacheManager {
    /// 删除单个记录的缓存
    pub async fn invalidate_record(&self, table: &str, id: &IdType) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let key = self.generate_cache_key(table, id, "record");
        
        if let Err(e) = self.cache.delete(&key).await {
            warn!("删除缓存记录失败: {}", e);
        } else {
            // 更新删除统计
            self.deletes_counter.fetch_add(1, Ordering::Relaxed);
            {
                let mut stats = self.stats.write().await;
                stats.deletes += 1;
            }
        }

        debug!("已删除缓存记录: table={}, id={:?}", table, id);
        Ok(())
    }

    /// 清理表的所有缓存
    pub async fn invalidate_table(&self, table: &str) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let pattern = format!("{}:{}:*", CACHE_KEY_PREFIX, table);
        self.clear_by_pattern(&pattern).await.map(|_| ())
    }



    /// 清理所有缓存
    pub async fn clear_all(&self) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        if let Err(e) = self.cache.clear().await {
            return Err(anyhow!("Failed to clear all cache: {}", e));
        }

        // 清理键跟踪
        let mut table_keys = self.table_keys.write().await;
        table_keys.clear();

        info!("已清理所有缓存");
        Ok(())
    }

    /// 按模式清理缓存（支持通配符匹配）
    /// 
    /// # 参数
    /// * `pattern` - 缓存键模式，支持通配符 * 和 ?
    /// 
    /// # 示例
    /// ```no_run
    /// # use rat_quickdb::cache::CacheManager;
    /// # async fn example(cache_manager: &CacheManager) -> rat_quickdb::QuickDbResult<()> {
    /// // 清理所有用户表相关的缓存
    /// cache_manager.clear_by_pattern("rat_quickdb:users:*").await?;
    /// // 清理所有查询缓存
    /// cache_manager.clear_by_pattern("*:query:*").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn clear_by_pattern(&self, pattern: &str) -> Result<usize> {
        if !self.config.enabled {
            info!("缓存已禁用，跳过模式清理: pattern={}", pattern);
            return Ok(0);
        }
        debug!("开始清理匹配模式的缓存: pattern={}", pattern);

        let mut cleared_count = 0;
        let table_keys = self.table_keys.read().await;
        
        // 遍历所有跟踪的缓存键
        for (table_name, keys) in table_keys.iter() {
            let mut keys_to_remove = Vec::new();
            
            for key in keys {
                if self.matches_pattern(key, pattern) {
                    if let Err(e) = self.cache.delete(key).await {
                        warn!("删除匹配模式的缓存键失败: key={}, pattern={}, error={}", key, pattern, e);
                    } else {
                        keys_to_remove.push(key.clone());
                        cleared_count += 1;
                        info!("已删除匹配模式的缓存键: table={}, key={}, pattern={}", table_name, key, pattern);
                    }
                }
            }
        }

        debug!("按模式清理缓存完成: pattern={}, cleared_count={}", pattern, cleared_count);
        Ok(cleared_count)
    }

    /// 批量清理记录缓存
    /// 
    /// # 参数
    /// * `table` - 表名
    /// * `ids` - 要清理的记录ID列表
    pub async fn clear_records_batch(&self, table: &str, ids: &[IdType]) -> Result<usize> {
        if !self.config.enabled {
            return Ok(0);
        }

        let mut cleared_count = 0;
        
        for id in ids {
            let key = self.generate_cache_key(table, id, "record");
            
            if let Err(e) = self.cache.delete(&key).await {
                warn!("批量删除缓存记录失败: table={}, id={:?}, error={}", table, id, e);
            } else {
                cleared_count += 1;
                debug!("已删除缓存记录: table={}, id={:?}", table, id);
            }
        }

        info!("批量清理记录缓存完成: table={}, total={}, cleared={}", table, ids.len(), cleared_count);
        Ok(cleared_count)
    }

    /// 强制清理过期缓存
    /// 
    /// 手动触发过期缓存的清理，通常用于内存紧张或需要立即释放空间的场景
    pub async fn force_cleanup_expired(&self) -> Result<usize> {
        if !self.config.enabled {
            return Ok(0);
        }

        // 由于 rat_memcache 内部处理过期清理，这里我们通过重新验证所有跟踪的键来实现
        let mut expired_count = 0;
        let mut table_keys = self.table_keys.write().await;
        
        for (table_name, keys) in table_keys.iter_mut() {
            let mut valid_keys = Vec::new();
            
            for key in keys.iter() {
                // 尝试获取缓存，如果不存在则认为已过期
                match self.cache.get(key).await {
                    Ok(Some(_)) => {
                        valid_keys.push(key.clone());
                    }
                    Ok(None) => {
                        expired_count += 1;
                        debug!("发现过期缓存键: key={}", key);
                    }
                    Err(e) => {
                        warn!("检查缓存键时出错: key={}, error={}", key, e);
                        // 出错的键也从跟踪中移除
                        expired_count += 1;
                    }
                }
            }
            
            *keys = valid_keys;
        }

        info!("强制清理过期缓存完成: expired_count={}", expired_count);
        Ok(expired_count)
    }

    /// 缓存预热 - 预加载热点数据
    /// 
    /// # 参数
    /// * `table` - 表名
    /// * `hot_ids` - 热点数据ID列表
    pub async fn warmup_cache(&self, table: &str, hot_ids: &[IdType]) -> Result<usize> {
        if !self.config.enabled || hot_ids.is_empty() {
            return Ok(0);
        }

        let mut warmed_count = 0;
        for id in hot_ids {
            let cache_key = self.generate_cache_key(table, id, "record");
            
            // 检查是否已缓存
            if self.cache.get(&cache_key).await.is_ok() {
                continue; // 已缓存，跳过
            }
            
            // 这里应该从数据库加载数据并缓存
            // 由于需要数据库连接，这个方法应该在适配器层实现
            // 这里只是标记需要预热的键
            debug!("标记预热缓存键: {}", cache_key);
            warmed_count += 1;
        }
        
        info!("缓存预热完成: table={}, warmed_count={}", table, warmed_count);
        Ok(warmed_count)
    }

    /// 批量缓存记录 - 优化批量操作
    /// 
    /// # 参数
    /// * `table` - 表名
    /// * `records` - 记录列表
    pub async fn cache_records_batch_optimized(&self, table: &str, records: &[(IdType, DataValue)]) -> Result<usize> {
        if !self.config.enabled || records.is_empty() {
            return Ok(0);
        }

        let mut cached_count = 0;
        for (id, data) in records {
            if let Err(e) = self.cache_record(table, id, data).await {
                warn!("批量缓存记录失败: table={}, id={:?}, error={}", table, id, e);
                continue;
            }
            cached_count += 1;
        }
        
        info!("批量缓存记录完成: table={}, cached_count={}", table, cached_count);
        Ok(cached_count)
    }

    /// 获取所有缓存键列表（按表分组）
    /// 
    /// 用于调试和监控，可以查看当前缓存中有哪些键
    pub async fn list_cache_keys(&self) -> Result<HashMap<String, Vec<String>>> {
        if !self.config.enabled {
            return Ok(HashMap::new());
        }

        let table_keys = self.table_keys.read().await;
        let result = table_keys.clone();
        
        info!("获取缓存键列表: 表数量={}, 总键数量={}", 
              result.len(), 
              result.values().map(|v| v.len()).sum::<usize>());
        
        Ok(result)
    }

    /// 获取指定表的缓存键列表
    pub async fn list_table_cache_keys(&self, table: &str) -> Result<Vec<String>> {
        if !self.config.enabled {
            return Ok(Vec::new());
        }

        let table_keys = self.table_keys.read().await;
        let keys = table_keys.get(table).cloned().unwrap_or_default();
        
        debug!("获取表缓存键列表: table={}, 键数量={}", table, keys.len());
        Ok(keys)
    }

    /// 清理指定表的查询缓存
    /// 
    /// 只清理查询缓存，保留记录缓存
    pub async fn clear_table_query_cache(&self, table: &str) -> Result<usize> {
        if !self.config.enabled {
            return Ok(0);
        }

        let pattern = format!("{}:{}:query:*", CACHE_KEY_PREFIX, table);
        let cleared_count = self.clear_by_pattern(&pattern).await?;
        
        debug!("清理表查询缓存完成: table={}, cleared_count={}", table, cleared_count);
        Ok(cleared_count)
    }

    /// 清理指定表的记录缓存
    /// 
    /// 只清理记录缓存，保留查询缓存
    pub async fn clear_table_record_cache(&self, table: &str) -> Result<usize> {
        if !self.config.enabled {
            return Ok(0);
        }

        let pattern = format!("{}:{}:record:*", CACHE_KEY_PREFIX, table);
        let cleared_count = self.clear_by_pattern(&pattern).await?;
        
        info!("清理表记录缓存完成: table={}, cleared_count={}", table, cleared_count);
        Ok(cleared_count)
    }

    /// 检查缓存键是否匹配模式
    /// 
    /// 支持简单的通配符匹配：* 匹配任意字符序列，? 匹配单个字符
    fn matches_pattern(&self, key: &str, pattern: &str) -> bool {
        // 简单的通配符匹配实现
        let pattern_chars: Vec<char> = pattern.chars().collect();
        let key_chars: Vec<char> = key.chars().collect();
        
        self.match_recursive(&key_chars, 0, &pattern_chars, 0)
    }

    /// 递归匹配算法
    fn match_recursive(&self, key: &[char], key_idx: usize, pattern: &[char], pattern_idx: usize) -> bool {
        // 如果模式已经匹配完
        if pattern_idx >= pattern.len() {
            return key_idx >= key.len();
        }
        
        // 如果键已经匹配完但模式还有非*字符
        if key_idx >= key.len() {
            return pattern[pattern_idx..].iter().all(|&c| c == '*');
        }
        
        match pattern[pattern_idx] {
            '*' => {
                // * 可以匹配0个或多个字符
                // 尝试匹配0个字符（跳过*）
                if self.match_recursive(key, key_idx, pattern, pattern_idx + 1) {
                    return true;
                }
                // 尝试匹配1个或多个字符
                self.match_recursive(key, key_idx + 1, pattern, pattern_idx)
            }
            '?' => {
                // ? 匹配任意单个字符
                self.match_recursive(key, key_idx + 1, pattern, pattern_idx + 1)
            }
            c => {
                // 普通字符必须完全匹配
                if key[key_idx] == c {
                    self.match_recursive(key, key_idx + 1, pattern, pattern_idx + 1)
                } else {
                    false
                }
            }
        }
    }

    /// 获取缓存统计信息
    pub async fn get_stats(&self) -> Result<CacheStats> {
        if !self.config.enabled {
            return Ok(CacheStats::default());
        }

        // 从原子计数器和详细统计中获取数据
        let hits = self.hits_counter.load(Ordering::Relaxed);
        let misses = self.misses_counter.load(Ordering::Relaxed);
        let writes = self.writes_counter.load(Ordering::Relaxed);
        let deletes = self.deletes_counter.load(Ordering::Relaxed);
        
        let stats = self.stats.read().await;
        let hit_rate = if hits + misses > 0 {
            hits as f64 / (hits + misses) as f64
        } else {
            0.0
        };
        
        // 估算内存使用量（基于跟踪的键数量）
        let table_keys = self.table_keys.read().await;
        let entries = table_keys.values().map(|v| v.len()).sum::<usize>();
        
        Ok(CacheStats {
            hits,
            misses,
            hit_rate,
            entries,
            memory_usage_bytes: entries * 1024, // 粗略估算每个条目1KB
            disk_usage_bytes: 0, // rat_memcache 主要是内存缓存
        })
    }

    /// 获取详细的性能统计信息
    pub async fn get_performance_stats(&self) -> Result<CachePerformanceStats> {
        if !self.config.enabled {
            return Ok(CachePerformanceStats::new());
        }

        let stats = self.stats.read().await;
        Ok(stats.clone())
    }

    /// 重置统计信息
    pub async fn reset_stats(&self) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        self.hits_counter.store(0, Ordering::Relaxed);
        self.misses_counter.store(0, Ordering::Relaxed);
        self.writes_counter.store(0, Ordering::Relaxed);
        self.deletes_counter.store(0, Ordering::Relaxed);
        
        {
            let mut stats = self.stats.write().await;
            *stats = CachePerformanceStats::new();
        }
        
        info!("缓存统计信息已重置");
        Ok(())
    }

    /// 记录缓存键（用于表级别的缓存清理）
    pub(crate) async fn track_cache_key(&self, table: &str, key: String) {
        let mut table_keys = self.table_keys.write().await;
        table_keys.entry(table.to_string())
            .or_insert_with(Vec::new)
            .push(key);
    }

    /// 检查缓存是否启用
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }



    /// 批量缓存记录 - 优化批量操作
    /// 
    /// # 参数
    /// * `table` - 表名
    /// * `records` - 记录列表
    pub async fn cache_records_batch(&self, table: &str, records: Vec<(IdType, DataValue)>) -> Result<usize> {
        if !self.config.enabled {
            return Ok(0);
        }

        let mut cached_count = 0;
        
        // 批量处理，减少锁竞争
        for (id, data) in records {
            if let Err(e) = self.cache_record(table, &id, &data).await {
                warn!("批量缓存记录失败: table={}, id={:?}, error={}", table, id, e);
            } else {
                cached_count += 1;
            }
        }

        info!("批量缓存记录完成: table={}, cached_count={}", table, cached_count);
        Ok(cached_count)
    }
}
