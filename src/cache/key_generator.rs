  //! 缓存键生成模块
//!
//! 提供各种缓存键的生成策略和实现

use crate::types::{IdType, QueryCondition, QueryConditionGroup, QueryOptions, SortDirection, CacheConfig, DataValue};
use rat_logger::debug;
use std::vec::Vec;

// 从 cache_manager.rs 中引入 CACHE_KEY_PREFIX 和 CacheManager
use super::cache_manager::{CACHE_KEY_PREFIX, CacheManager};

impl CacheManager {
    /// 生成缓存键
    pub(crate) fn generate_cache_key(&self, table: &str, id: &IdType, operation: &str) -> String {
        let id_str = match id {
            IdType::Number(n) => n.to_string(),
            IdType::String(s) => s.clone(),
        };
        format!("{}:{}:{}:{}", CACHE_KEY_PREFIX, table, operation, id_str)
    }

    /// 生成查询缓存键 - 优化版本，避免复杂序列化
    pub fn generate_query_cache_key(&self, table: &str, conditions: &[QueryCondition], options: &QueryOptions) -> String {
        let query_signature = self.build_query_signature(options);
        let conditions_signature = self.build_conditions_signature(conditions);
        // 添加版本标识避免脏数据问题
        let key = format!("{}:{}:query:{}:{}:{}", CACHE_KEY_PREFIX, table, conditions_signature, query_signature, self.config.version);
        debug!("生成查询缓存键: table={}, key={}", table, key);
        key
    }

    /// 生成条件组合查询缓存键
    pub fn generate_condition_groups_cache_key(&self, table: &str, condition_groups: &[QueryConditionGroup], options: &QueryOptions) -> String {
        let query_signature = self.build_query_signature(options);
        let groups_signature = self.build_condition_groups_signature(condition_groups);
        let key = format!("{}:{}:groups:{}:{}", CACHE_KEY_PREFIX, table, groups_signature, query_signature);
        debug!("生成条件组合查询缓存键: table={}, key={}", table, key);
        key
    }

    /// 构建查询签名 - 高效版本，避免JSON序列化
    fn build_query_signature(&self, options: &QueryOptions) -> String {
        let mut parts = Vec::new();
        
        // 分页信息
        if let Some(pagination) = &options.pagination {
            parts.push(format!("p{}_{}", pagination.skip, pagination.limit));
        }
        
        // 排序信息
        if !options.sort.is_empty() {
            let sort_str = options.sort.iter()
                .map(|s| format!("{}{}", s.field, match s.direction { SortDirection::Asc => "a", SortDirection::Desc => "d" }))
                .collect::<Vec<_>>()
                .join(",");
            parts.push(format!("s{}", sort_str));
        }
        
        // 投影信息
        if !options.fields.is_empty() {
            let proj_str = options.fields.join(",");
            parts.push(format!("f{}", proj_str));
        }
        
        // 连接部分生成最终签名
        if parts.is_empty() {
            "default".to_string()
        } else {
            parts.join("_")
        }
    }

    /// 构建条件签名
    fn build_conditions_signature(&self, conditions: &[QueryCondition]) -> String {
        if conditions.is_empty() {
            return "no_cond".to_string();
        }
        
        let mut signature = String::new();
        for (i, condition) in conditions.iter().enumerate() {
            if i > 0 {
                signature.push('_');
            }
            signature.push_str(&format!("{}{:?}{}", 
                condition.field, 
                condition.operator, 
                match &condition.value {
                     DataValue::String(s) => s.clone(),  // 修复：不截断字符串，使用完整值
                     DataValue::Int(n) => n.to_string(),
                     DataValue::Float(f) => f.to_string(),
                     DataValue::Bool(b) => b.to_string(),
                     _ => "val".to_string(),
                 }
            ));
        }
        signature
    }

    /// 构建条件组合签名
    fn build_condition_groups_signature(&self, condition_groups: &[QueryConditionGroup]) -> String {
        if condition_groups.is_empty() {
            return "no_groups".to_string();
        }
        
        let mut signature = String::new();
        for (i, group) in condition_groups.iter().enumerate() {
            if i > 0 {
                signature.push('_');
            }
            match group {
                QueryConditionGroup::Single(condition) => {
                    signature.push_str(&format!("s{}{:?}{}", 
                        condition.field, 
                        condition.operator, 
                        match &condition.value {
                             DataValue::String(s) => s.clone(),  // 修复：不截断字符串，使用完整值
                             DataValue::Int(n) => n.to_string(),
                             DataValue::Float(f) => f.to_string(),
                             DataValue::Bool(b) => b.to_string(),
                             _ => "val".to_string(),
                         }
                    ));
                },
                QueryConditionGroup::Group { conditions, operator } => {
                    signature.push_str(&format!("g{:?}_", operator));
                    for (j, condition) in conditions.iter().enumerate() {
                        if j > 0 {
                            signature.push('|');
                        }
                        match condition {
                            QueryConditionGroup::Single(cond) => {
                                signature.push_str(&format!("{}{:?}{}", 
                                    cond.field, 
                                    cond.operator, 
                                    match &cond.value {
                                        DataValue::String(s) => s.clone(),
                                        DataValue::Int(n) => n.to_string(),
                                        DataValue::Float(f) => f.to_string(),
                                        DataValue::Bool(b) => b.to_string(),
                                        _ => "val".to_string(),
                                    }
                                ));
                            },
                            QueryConditionGroup::Group { .. } => {
                                // 递归处理嵌套组合（简化处理）
                                signature.push_str("nested");
                            }
                        }
                    }
                }
            }
        }
        signature
    }
}
