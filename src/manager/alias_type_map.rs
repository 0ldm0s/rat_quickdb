//! 数据库别名类型映射模块
//!
//! 提供全局的数据库别名到类型的映射功能

use crate::error::{QuickDbError, QuickDbResult};
use crate::types::DatabaseType;
use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

/// 数据库别名到类型的全局映射
static DATABASE_TYPE_MAPPING: OnceLock<RwLock<HashMap<String, DatabaseType>>> = OnceLock::new();

/// 注册数据库别名到类型的映射
///
/// 这个函数应该由 add_database 调用，用于建立别名与数据库类型的关联
pub fn register_database_alias(alias: String, db_type: DatabaseType) -> QuickDbResult<()> {
    let mapping = DATABASE_TYPE_MAPPING.get_or_init(|| RwLock::new(HashMap::new()));
    let mut map = mapping.write().map_err(|_| QuickDbError::PoolError {
        message: "数据库类型映射表锁被污染".to_string(),
    })?;
    map.insert(alias, db_type);
    Ok(())
}

/// 通过别名获取数据库类型
///
/// 这个函数主要用于模型宏展开时确定数据库特定的处理逻辑
pub fn get_database_type_by_alias(alias: &str) -> Option<DatabaseType> {
    let mapping = DATABASE_TYPE_MAPPING.get()?;
    let map = mapping.read().ok()?;
    map.get(alias).cloned()
}

/// 获取所有已注册的数据库别名
pub fn get_all_registered_aliases() -> Vec<String> {
    if let Some(mapping) = DATABASE_TYPE_MAPPING.get() {
        if let Ok(map) = mapping.read() {
            map.keys().cloned().collect()
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alias_type_mapping() {
        // 测试注册和获取
        register_database_alias("test_sqlite".to_string(), DatabaseType::SQLite).unwrap();
        register_database_alias("test_mysql".to_string(), DatabaseType::MySQL).unwrap();

        assert_eq!(get_database_type_by_alias("test_sqlite"), Some(DatabaseType::SQLite));
        assert_eq!(get_database_type_by_alias("test_mysql"), Some(DatabaseType::MySQL));
        assert_eq!(get_database_type_by_alias("nonexistent"), None);

        let aliases = get_all_registered_aliases();
        assert!(aliases.contains(&"test_sqlite".to_string()));
        assert!(aliases.contains(&"test_mysql".to_string()));
    }
}