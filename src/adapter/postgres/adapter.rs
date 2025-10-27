//! PostgreSQL适配器核心模块
//!
//! 提供PostgreSQL适配器的核心结构定义和基础功能

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use rat_logger::debug;

/// PostgreSQL适配器
pub struct PostgresAdapter {
    /// 表创建锁，防止重复创建表
    creation_locks: Arc<Mutex<HashMap<String, ()>>>,
}

impl PostgresAdapter {
    /// 创建新的PostgreSQL适配器
    pub fn new() -> Self {
        Self {
            creation_locks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 获取表创建锁
    pub(crate) async fn acquire_table_lock(&self, table: &str) -> tokio::sync::MutexGuard<'_, HashMap<String, ()>> {
        let mut locks = self.creation_locks.lock().await;
        if !locks.contains_key(table) {
            locks.insert(table.to_string(), ());
            debug!("🔒 获取表 {} 的创建锁", table);
        }
        locks
    }

    /// 释放表创建锁
    pub(crate) async fn release_table_lock(&self, table: &str, mut locks: tokio::sync::MutexGuard<'_, HashMap<String, ()>>) {
        locks.remove(table);
        debug!("🔓 释放表 {} 的创建锁", table);
    }
}
