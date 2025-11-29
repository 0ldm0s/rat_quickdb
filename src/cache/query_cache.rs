    //! 查询结果缓存模块
//!
//! 提供查询结果和条件组合查询结果的缓存功能

use crate::types::{QueryOptions, DataValue, QueryCondition, QueryConditionGroup};
use anyhow::{anyhow, Result};
use rat_memcache::{RatMemCache, CacheOptions};
use bytes::Bytes;
use std::time::Instant;
use serde_json;
use rat_logger::{warn, debug};
use std::sync::atomic::Ordering;

// 从 cache_manager.rs 中引入 CacheManager
use super::cache_manager::CacheManager;

impl CacheManager {
    pub async fn cache_query_result(
        &self,
        table: &str,
        options: &QueryOptions,
        results: &[DataValue],
    ) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let start_time = Instant::now();
        let key = self.generate_query_cache_key(table, &options.conditions, options);
        
        debug!("尝试缓存查询结果: table={}, key={}, options={:?}, 结果数量={}", table, key, options, results.len());

        // 修复：允许缓存空结果，使用特殊标记区分"无缓存"和"空结果"
        // 空结果也需要缓存，避免TTL过期后的死循环问题
        
        // 限制缓存结果大小，避免内存浪费
        if results.len() > 1000 {
            debug!("跳过缓存过大查询结果: table={}, count={}", table, results.len());
            return Ok(());
        }
        
        // 修复：正确序列化所有类型的DataValue
        let json_results: Vec<serde_json::Value> = results.iter()
            .map(|dv| {
                match dv {
                    DataValue::Json(json_val) => json_val.clone(),
                    DataValue::String(s) => serde_json::Value::String(s.clone()),
                    DataValue::Int(i) => serde_json::Value::Number(serde_json::Number::from(*i)),
                    DataValue::Float(f) => serde_json::Value::Number(serde_json::Number::from_f64(*f).unwrap_or(serde_json::Number::from(0))),
                    DataValue::Bool(b) => serde_json::Value::Bool(*b),
                    DataValue::DateTime(dt) => serde_json::Value::String(dt.to_rfc3339()),
                    DataValue::Null => serde_json::Value::Null,
                    DataValue::Bytes(bytes) => serde_json::Value::String(base64::encode(bytes)),
                    DataValue::Uuid(uuid) => serde_json::Value::String(uuid.to_string()),
                    DataValue::Array(arr) => {
                        let json_array: Vec<serde_json::Value> = arr.iter().map(|item| {
                            // 递归处理数组元素
                            match item {
                                DataValue::Json(json_val) => json_val.clone(),
                                DataValue::String(s) => serde_json::Value::String(s.clone()),
                                DataValue::Int(i) => serde_json::Value::Number(serde_json::Number::from(*i)),
                                DataValue::Float(f) => serde_json::Value::Number(serde_json::Number::from_f64(*f).unwrap_or(serde_json::Number::from(0))),
                                DataValue::Bool(b) => serde_json::Value::Bool(*b),
                                DataValue::DateTime(dt) => serde_json::Value::String(dt.to_rfc3339()),
                                DataValue::Null => serde_json::Value::Null,
                                DataValue::Bytes(bytes) => serde_json::Value::String(base64::encode(bytes)),
                                DataValue::Uuid(uuid) => serde_json::Value::String(uuid.to_string()),
                                _ => serde_json::Value::String(format!("{:?}", item)), // 其他复杂类型转为字符串
                            }
                        }).collect();
                        serde_json::Value::Array(json_array)
                    },
                    DataValue::Object(obj) => {
                        let mut json_obj = serde_json::Map::new();
                        for (key, value) in obj {
                            let json_value = match value {
                                DataValue::Json(json_val) => json_val.clone(),
                                DataValue::String(s) => serde_json::Value::String(s.clone()),
                                DataValue::Int(i) => serde_json::Value::Number(serde_json::Number::from(*i)),
                                DataValue::Float(f) => serde_json::Value::Number(serde_json::Number::from_f64(*f).unwrap_or(serde_json::Number::from(0))),
                                DataValue::Bool(b) => serde_json::Value::Bool(*b),
                                DataValue::DateTime(dt) => serde_json::Value::String(dt.to_rfc3339()),
                                DataValue::Null => serde_json::Value::Null,
                                DataValue::Bytes(bytes) => serde_json::Value::String(base64::encode(bytes)),
                                DataValue::Uuid(uuid) => serde_json::Value::String(uuid.to_string()),
                                _ => serde_json::Value::String(format!("{:?}", value)), // 其他复杂类型转为字符串
                            };
                            json_obj.insert(key.clone(), json_value);
                        }
                        serde_json::Value::Object(json_obj)
                    },
                }
            })
            .collect();
            
        let serialized = serde_json::to_vec(&json_results)
            .map_err(|e| anyhow!("Failed to serialize query results: {}", e))?;

        let cache_options = CacheOptions {
            ttl_seconds: Some(self.config.ttl_config.default_ttl_secs),
            ..Default::default()
        };

        self.cache.set_with_options(key.clone(), Bytes::from(serialized), &cache_options).await
            .map_err(|e| anyhow!("Failed to cache query results: {}", e))?;

        // 记录缓存键
        self.track_cache_key(table, key.clone()).await;

        // 更新统计信息
        let elapsed = start_time.elapsed();
        self.writes_counter.fetch_add(1, Ordering::Relaxed);
        {
            let mut stats = self.stats.write().await;
            stats.writes += 1;
            stats.write_count += 1;
            stats.total_write_latency_ns += elapsed.as_nanos() as u64;
        }

        debug!("已缓存查询结果: table={}, key={}, count={}", table, key, results.len());
        Ok(())
    }

    /// 缓存条件组合查询结果
    pub async fn cache_condition_groups_result(
        &self,
        table: &str,
        condition_groups: &[QueryConditionGroup],
        options: &QueryOptions,
        results: &[DataValue],
    ) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let start_time = Instant::now();
        let key = self.generate_condition_groups_cache_key(table, condition_groups, options);
        
        debug!("开始缓存条件组合查询结果: table={}, key={}, count={}", table, key, results.len());
        
        // 检查结果大小限制，避免缓存过大结果
        if results.len() > 1000 {
            debug!("跳过缓存：结果集过大 ({} > 1000)", results.len());
            return Ok(());
        }
        
        // 将所有DataValue转换为JSON值进行序列化
        let json_results: Vec<serde_json::Value> = results.iter()
            .map(|dv| {
                match dv {
                    DataValue::Json(json_val) => json_val.clone(),
                    DataValue::String(s) => serde_json::Value::String(s.clone()),
                    DataValue::Int(i) => serde_json::Value::Number(serde_json::Number::from(*i)),
                    DataValue::Float(f) => serde_json::Value::Number(serde_json::Number::from_f64(*f).unwrap_or(serde_json::Number::from(0))),
                    DataValue::Bool(b) => serde_json::Value::Bool(*b),
                    DataValue::DateTime(dt) => serde_json::Value::String(dt.to_rfc3339()),
                    DataValue::Null => serde_json::Value::Null,
                    DataValue::Bytes(bytes) => serde_json::Value::String(base64::encode(bytes)),
                    DataValue::Uuid(uuid) => serde_json::Value::String(uuid.to_string()),
                    DataValue::Array(arr) => {
                        let json_array: Vec<serde_json::Value> = arr.iter().map(|item| {
                            match item {
                                DataValue::Json(json_val) => json_val.clone(),
                                DataValue::String(s) => serde_json::Value::String(s.clone()),
                                DataValue::Int(i) => serde_json::Value::Number(serde_json::Number::from(*i)),
                                DataValue::Float(f) => serde_json::Value::Number(serde_json::Number::from_f64(*f).unwrap_or(serde_json::Number::from(0))),
                                DataValue::Bool(b) => serde_json::Value::Bool(*b),
                                DataValue::DateTime(dt) => serde_json::Value::String(dt.to_rfc3339()),
                                DataValue::Null => serde_json::Value::Null,
                                DataValue::Bytes(bytes) => serde_json::Value::String(base64::encode(bytes)),
                                DataValue::Uuid(uuid) => serde_json::Value::String(uuid.to_string()),
                                _ => serde_json::Value::String(format!("{:?}", item)), // 其他复杂类型转为字符串
                            }
                        }).collect();
                        serde_json::Value::Array(json_array)
                    },
                    DataValue::Object(obj) => {
                        let mut json_obj = serde_json::Map::new();
                        for (key, value) in obj {
                            let json_value = match value {
                                DataValue::Json(json_val) => json_val.clone(),
                                DataValue::String(s) => serde_json::Value::String(s.clone()),
                                DataValue::Int(i) => serde_json::Value::Number(serde_json::Number::from(*i)),
                                DataValue::Float(f) => serde_json::Value::Number(serde_json::Number::from_f64(*f).unwrap_or(serde_json::Number::from(0))),
                                DataValue::Bool(b) => serde_json::Value::Bool(*b),
                                DataValue::DateTime(dt) => serde_json::Value::String(dt.to_rfc3339()),
                                DataValue::Null => serde_json::Value::Null,
                                DataValue::Bytes(bytes) => serde_json::Value::String(base64::encode(bytes)),
                                DataValue::Uuid(uuid) => serde_json::Value::String(uuid.to_string()),
                                _ => serde_json::Value::String(format!("{:?}", value)), // 其他复杂类型转为字符串
                            };
                            json_obj.insert(key.clone(), json_value);
                        }
                        serde_json::Value::Object(json_obj)
                    },
                }
            })
            .collect();
            
        let serialized = serde_json::to_vec(&json_results)
            .map_err(|e| anyhow!("Failed to serialize condition groups query results: {}", e))?;

        let cache_options = CacheOptions {
            ttl_seconds: Some(self.config.ttl_config.default_ttl_secs),
            ..Default::default()
        };

        self.cache.set_with_options(key.clone(), Bytes::from(serialized), &cache_options).await
            .map_err(|e| anyhow!("Failed to cache condition groups query results: {}", e))?;

        // 记录缓存键
        self.track_cache_key(table, key.clone()).await;

        // 更新统计信息
        let elapsed = start_time.elapsed();
        self.writes_counter.fetch_add(1, Ordering::Relaxed);
        {
            let mut stats = self.stats.write().await;
            stats.writes += 1;
            stats.write_count += 1;
            stats.total_write_latency_ns += elapsed.as_nanos() as u64;
        }

        debug!("已缓存条件组合查询结果: table={}, key={}, count={}", table, key, results.len());
        Ok(())
    }

    /// 获取缓存的查询结果 - 优化版本
    pub async fn get_cached_query_result(
        &self,
        table: &str,
        options: &QueryOptions,
    ) -> Result<Option<Vec<DataValue>>> {
        if !self.config.enabled {
            return Ok(None);
        }

        let start_time = Instant::now();
        let key = self.generate_query_cache_key(table, &options.conditions, options);
        
        debug!("尝试获取查询缓存: table={}, key={}, options={:?}", table, key, options);
        
        self.get_cached_result_by_key(&key, table, start_time).await
    }

    /// 获取缓存的条件组合查询结果
    pub async fn get_cached_condition_groups_result(
        &self,
        table: &str,
        condition_groups: &[QueryConditionGroup],
        options: &QueryOptions,
    ) -> Result<Option<Vec<DataValue>>> {
        if !self.config.enabled {
            return Ok(None);
        }

        let start_time = Instant::now();
        let key = self.generate_condition_groups_cache_key(table, condition_groups, options);
        
        debug!("尝试获取条件组合查询缓存: table={}, key={}, options={:?}", table, key, options);
        
        self.get_cached_result_by_key(&key, table, start_time).await
    }

    /// 通用的缓存结果获取方法
    async fn get_cached_result_by_key(
        &self,
        key: &str,
        table: &str,
        start_time: Instant,
    ) -> Result<Option<Vec<DataValue>>> {
        
        match self.cache.get(&key).await {
            Ok(Some(data)) => {
                // 修复：正确反序列化为对应的DataValue类型
                let json_results: Vec<serde_json::Value> = serde_json::from_slice(&data)
                    .map_err(|e| anyhow!("Failed to deserialize cached query results: {}", e))?;
                    
                let data_values: Vec<DataValue> = json_results.into_iter()
                    .map(|json_val| {
                        match json_val {
                            serde_json::Value::String(s) => {
                                // 尝试解析为UUID
                                if let Ok(uuid) = uuid::Uuid::parse_str(&s) {
                                    DataValue::Uuid(uuid)
                                }
                                // 尝试解析为DateTime
                                else if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&s) {
                                    DataValue::DateTime(dt.with_timezone(&chrono::FixedOffset::east(0)))
                                }
                                // 尝试解析为base64编码的字节数据
                                else if s.starts_with("data:") || (s.len() % 4 == 0 && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=')) {
                                    if let Ok(bytes) = base64::decode(&s) {
                                        DataValue::Bytes(bytes)
                                    } else {
                                        DataValue::String(s)
                                    }
                                } else {
                                    DataValue::String(s)
                                }
                            },
                            serde_json::Value::Number(n) => {
                                if let Some(i) = n.as_i64() {
                                    DataValue::Int(i)
                                } else if let Some(f) = n.as_f64() {
                                    DataValue::Float(f)
                                } else {
                                    DataValue::Int(0) // 默认值
                                }
                            },
                            serde_json::Value::Bool(b) => DataValue::Bool(b),
                            serde_json::Value::Null => DataValue::Null,
                            serde_json::Value::Array(arr) => {
                                let data_array: Vec<DataValue> = arr.into_iter().map(|item| {
                                    // 递归处理数组元素
                                    match item {
                                        serde_json::Value::String(s) => DataValue::String(s),
                                        serde_json::Value::Number(n) => {
                                            if let Some(i) = n.as_i64() {
                                                DataValue::Int(i)
                                            } else if let Some(f) = n.as_f64() {
                                                DataValue::Float(f)
                                            } else {
                                                DataValue::Int(0)
                                            }
                                        },
                                        serde_json::Value::Bool(b) => DataValue::Bool(b),
                                        serde_json::Value::Null => DataValue::Null,
                                        other => DataValue::Json(other),
                                    }
                                }).collect();
                                DataValue::Array(data_array)
                            },
                            serde_json::Value::Object(obj) => {
                                let mut data_obj = std::collections::HashMap::new();
                                for (key, value) in obj {
                                    let data_value = match value {
                                        serde_json::Value::String(s) => DataValue::String(s),
                                        serde_json::Value::Number(n) => {
                                            if let Some(i) = n.as_i64() {
                                                DataValue::Int(i)
                                            } else if let Some(f) = n.as_f64() {
                                                DataValue::Float(f)
                                            } else {
                                                DataValue::Int(0)
                                            }
                                        },
                                        serde_json::Value::Bool(b) => DataValue::Bool(b),
                                        serde_json::Value::Null => DataValue::Null,
                                        other => DataValue::Json(other),
                                    };
                                    data_obj.insert(key, data_value);
                                }
                                DataValue::Object(data_obj)
                            },
                        }
                    })
                    .collect();
                
                // 更新命中统计
                let elapsed = start_time.elapsed();
                self.hits_counter.fetch_add(1, Ordering::Relaxed);
                {
                    let mut stats = self.stats.write().await;
                    stats.hits += 1;
                    stats.query_count += 1;
                    stats.total_query_latency_ns += elapsed.as_nanos() as u64;
                }
                    
                debug!("查询缓存命中: table={}, key={}, count={}", table, key, data_values.len());
                Ok(Some(data_values))
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
                
                debug!("查询缓存未命中: table={}, key={}", table, key);
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
                
                warn!("查询缓存读取失败: table={}, key={}, error={}", table, key, e);
                Ok(None)
            }
        }
    }
}
