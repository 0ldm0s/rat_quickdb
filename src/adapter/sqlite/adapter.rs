//! SQLite适配器核心模块
//!
//! 提供SQLite适配器的核心结构定义和基础功能

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use rat_logger::debug;

/// SQLite适配器
pub struct SqliteAdapter {
    /// 表创建锁，防止重复创建表
    creation_locks: Arc<Mutex<HashMap<String, ()>>>,
}

impl SqliteAdapter {
    /// 创建新的SQLite适配器
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

    /// 生成存储过程的SQL语句
    pub async fn generate_stored_procedure_sql(
        &self,
        config: &crate::stored_procedure::StoredProcedureConfig,
    ) -> crate::error::QuickDbResult<String> {
        use crate::stored_procedure::JoinType;

        // 1. 构建SELECT字段列表
        let fields: Vec<String> = config.fields
            .iter()
            .map(|(alias, expr)| {
                if alias == expr {
                    expr.clone()
                } else {
                    format!("{} AS {}", expr, alias)
                }
            })
            .collect();

        // 2. 构建FROM子句（主表）
        let base_table = config.dependencies.first()
            .ok_or_else(|| crate::error::QuickDbError::ValidationError {
                field: "dependencies".to_string(),
                message: "至少需要一个依赖表作为主表".to_string(),
            })?;

        // 3. 构建JOIN子句
        let mut joins = Vec::new();
        for (i, join) in config.joins.iter().enumerate() {
            let join_str = match join.join_type {
                JoinType::Inner => "INNER JOIN",
                JoinType::Left => "LEFT JOIN",
                JoinType::Right => "RIGHT JOIN",
                JoinType::Full => "FULL OUTER JOIN",
            };

            // 使用第一个表作为主表，后续表通过字段连接
            let local_table = if i == 0 { base_table } else { &config.joins[i-1].table };

            joins.push(format!(
                " {} {} ON {}.{} = {}.{}",
                join_str,
                join.table,
                local_table,
                join.local_field,
                join.table,
                join.foreign_field
            ));
        }

        // 4. 构建完整的存储过程SQL
        let sql = format!(
            r#"CREATE PROCEDURE IF NOT EXISTS {}()
AS BEGIN
    SELECT {}
    FROM {}{}
; END"#,
            config.procedure_name,
            fields.join(", "),
            base_table,
            joins.join(" ")
        );

        debug!("生成的存储过程SQL: {}", sql);
        Ok(sql)
    }
}
