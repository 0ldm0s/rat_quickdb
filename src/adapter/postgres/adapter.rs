//! PostgreSQL适配器核心模块
//!
//! 提供PostgreSQL适配器的核心结构定义和基础功能

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use rat_logger::{debug, info};

/// PostgreSQL适配器
pub struct PostgresAdapter {
    /// 表创建锁，防止重复创建表
    creation_locks: Arc<Mutex<HashMap<String, ()>>>,
    /// 存储过程映射表，存储已创建的存储过程信息
    pub(crate) stored_procedures: Arc<Mutex<HashMap<String, crate::stored_procedure::StoredProcedureInfo>>>,
}

impl PostgresAdapter {
    /// 创建新的PostgreSQL适配器
    pub fn new() -> Self {
        Self {
            creation_locks: Arc::new(Mutex::new(HashMap::new())),
            stored_procedures: Arc::new(Mutex::new(HashMap::new())),
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

    /// 生成存储过程的SQL模板（PostgreSQL使用模板模拟存储过程逻辑）
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
            .map(|model_meta| &model_meta.collection_name)
            .ok_or_else(|| crate::error::QuickDbError::ValidationError {
                field: "dependencies".to_string(),
                message: "至少需要一个依赖表作为主表".to_string(),
            })?;

        // 3. 构建JOIN子句 - 支持多表JOIN（local_field和foreign_field都是"表名.字段名"格式）
        let mut joins = Vec::new();
        for join in config.joins.iter() {
            let join_str = match join.join_type {
                JoinType::Inner => "INNER JOIN",
                JoinType::Left => "LEFT JOIN",
                JoinType::Right => "RIGHT JOIN",
                JoinType::Full => "FULL OUTER JOIN",
            };

            // 直接使用local_field和foreign_field，因为它们已经包含了表名
            joins.push(format!(
                " {} {} ON {} = {}",
                join_str,
                join.table,
                join.local_field,
                join.foreign_field
            ));
        }

        // 4. 构建完整的PostgreSQL存储过程SQL模板（包含占位符供后续动态替换）
        let sql_template = format!(
            "SELECT {SELECT_FIELDS} FROM {BASE_TABLE}{JOINS}{WHERE}{GROUP_BY}{HAVING}{ORDER_BY}{LIMIT}{OFFSET}",
            SELECT_FIELDS = fields.join(", "),
            BASE_TABLE = base_table,
            JOINS = if joins.is_empty() { "".to_string() } else { format!(" {}", joins.join(" ")) },
            WHERE = "{WHERE}", // WHERE条件占位符
            GROUP_BY = "{GROUP_BY}", // GROUP BY占位符
            HAVING = "{HAVING}", // HAVING占位符
            ORDER_BY = "{ORDER_BY}", // ORDER BY占位符
            LIMIT = "{LIMIT}", // LIMIT占位符
            OFFSET = "{OFFSET}" // OFFSET占位符
        );

        info!("生成的PostgreSQL存储过程SQL模板: {}", sql_template);
        Ok(sql_template)
    }
}
