//! 数据库安全验证工具
//!
//! 提供跨数据库类型的字段名、表名等标识符的安全验证，
//! 防止SQL注入和NoSQL注入攻击

use crate::error::{QuickDbError, QuickDbResult};
use crate::types::DatabaseType;

/// 数据库安全验证器
pub struct DatabaseSecurityValidator {
    db_type: DatabaseType,
}

impl DatabaseSecurityValidator {
    /// 创建新的安全验证器
    pub fn new(db_type: DatabaseType) -> Self {
        Self { db_type }
    }

    /// 验证字段名的安全性
    ///
    /// 根据数据库类型确保字段名不包含恶意内容，防止SQL/NoSQL注入攻击
    ///
    /// # 参数
    /// * `field_name` - 字段名
    ///
    /// # 返回值
    /// * `Ok(())` - 字段名安全
    /// * `Err(QuickDbError)` - 字段名包含非法字符
    pub fn validate_field_name(&self, field_name: &str) -> QuickDbResult<()> {
        // 字段名不能为空
        if field_name.is_empty() {
            return Err(QuickDbError::ValidationError {
                field: "field_name".to_string(),
                message: "字段名不能为空".to_string(),
            });
        }

        // 检查字段名长度
        if field_name.len() > 64 {
            return Err(QuickDbError::ValidationError {
                field: field_name.to_string(),
                message: "字段名长度不能超过64个字符".to_string(),
            });
        }

        // 根据数据库类型进行不同的验证
        match self.db_type {
            DatabaseType::PostgreSQL | DatabaseType::MySQL | DatabaseType::SQLite => {
                self.validate_sql_field_name(field_name)
            }
            DatabaseType::MongoDB => {
                self.validate_nosql_field_name(field_name)
            }
        }
    }

    /// 验证表名的安全性
    ///
    /// 验证表名不包含恶意内容
    ///
    /// # 参数
    /// * `table_name` - 表名
    ///
    /// # 返回值
    /// * `Ok(())` - 表名安全
    /// * `Err(QuickDbError)` - 表名包含非法字符
    pub fn validate_table_name(&self, table_name: &str) -> QuickDbResult<()> {
        // 表名不能为空
        if table_name.is_empty() {
            return Err(QuickDbError::ValidationError {
                field: "table_name".to_string(),
                message: "表名不能为空".to_string(),
            });
        }

        // 检查表名长度
        if table_name.len() > 64 {
            return Err(QuickDbError::ValidationError {
                field: table_name.to_string(),
                message: "表名长度不能超过64个字符".to_string(),
            });
        }

        // 根据数据库类型进行不同的验证
        match self.db_type {
            DatabaseType::PostgreSQL | DatabaseType::MySQL | DatabaseType::SQLite => {
                self.validate_sql_table_name(table_name)
            }
            DatabaseType::MongoDB => {
                self.validate_nosql_collection_name(table_name)
            }
        }
    }

    /// 获取安全的字段标识符
    ///
    /// 验证字段名并返回可用于查询的安全字段标识符
    ///
    /// # 参数
    /// * `field_name` - 字段名
    ///
    /// # 返回值
    /// * `Ok(String)` - 安全的字段标识符（已添加适当的引号保护）
    /// * `Err(QuickDbError)` - 字段名验证失败
    pub fn get_safe_field_identifier(&self, field_name: &str) -> QuickDbResult<String> {
        // 验证字段名安全性
        self.validate_field_name(field_name)?;

        // 返回带引号的字段标识符
        match self.db_type {
            DatabaseType::PostgreSQL => Ok(format!("\"{}\"", field_name)),
            DatabaseType::MySQL => Ok(format!("`{}`", field_name)),
            DatabaseType::SQLite => Ok(format!("\"{}\"", field_name)),
            DatabaseType::MongoDB => Ok(field_name.to_string()), // MongoDB不需要引号
        }
    }

    /// 获取安全的表标识符
    ///
    /// 验证表名并返回可用于查询的安全表标识符
    ///
    /// # 参数
    /// * `table_name` - 表名
    ///
    /// # 返回值
    /// * `Ok(String)` - 安全的表标识符（已添加适当的引号保护）
    /// * `Err(QuickDbError)` - 表名验证失败
    pub fn get_safe_table_identifier(&self, table_name: &str) -> QuickDbResult<String> {
        // 验证表名安全性
        self.validate_table_name(table_name)?;

        // 返回带引号的表标识符
        match self.db_type {
            DatabaseType::PostgreSQL => Ok(format!("\"{}\"", table_name)),
            DatabaseType::MySQL => Ok(format!("`{}`", table_name)),
            DatabaseType::SQLite => Ok(format!("\"{}\"", table_name)),
            DatabaseType::MongoDB => Ok(table_name.to_string()), // MongoDB不需要引号
        }
    }

    /// 验证SQL数据库字段名的安全性
    ///
    /// SQL数据库字段名验证规则
    fn validate_sql_field_name(&self, field_name: &str) -> QuickDbResult<()> {
        // 检查第一个字符不能是数字
        if field_name.chars().next().unwrap().is_ascii_digit() {
            return Err(QuickDbError::ValidationError {
                field: field_name.to_string(),
                message: "SQL字段名不能以数字开头".to_string(),
            });
        }

        // 检查字段名只包含安全字符（SQL数据库通常更严格）
        for (i, ch) in field_name.chars().enumerate() {
            if !ch.is_ascii_alphanumeric() && ch != '_' {
                return Err(QuickDbError::ValidationError {
                    field: field_name.to_string(),
                    message: format!("SQL字段名包含非法字符 '{}' 在位置 {}", ch, i),
                });
            }
        }

        // 检查是否为SQL关键字
        let upper_name = field_name.to_uppercase();
        let sql_keywords = [
            "SELECT", "FROM", "WHERE", "INSERT", "UPDATE", "DELETE", "CREATE", "DROP",
            "ALTER", "TABLE", "INDEX", "AND", "OR", "NOT", "NULL", "IS", "IN", "EXISTS",
            "BETWEEN", "LIKE", "REGEXP", "UNION", "JOIN", "INNER", "LEFT", "RIGHT", "OUTER",
            "GROUP", "BY", "HAVING", "ORDER", "LIMIT", "OFFSET", "DISTINCT", "COUNT", "SUM",
            "AVG", "MIN", "MAX", "AS", "ON", "PRIMARY", "KEY", "FOREIGN", "REFERENCES",
            "CASE", "WHEN", "THEN", "ELSE", "END", "IF", "COALESCE", "CAST", "CONVERT",
        ];

        if sql_keywords.contains(&upper_name.as_str()) {
            return Err(QuickDbError::ValidationError {
                field: field_name.to_string(),
                message: format!("字段名不能使用SQL关键字: {}", field_name),
            });
        }

        Ok(())
    }

    /// 验证NoSQL数据库字段名的安全性
    ///
    /// MongoDB等NoSQL数据库字段名验证规则（相对宽松）
    fn validate_nosql_field_name(&self, field_name: &str) -> QuickDbResult<()> {
        // NoSQL数据库通常允许更灵活的字段名，但仍有安全限制

        // 不能以$开头（MongoDB系统字段）
        if field_name.starts_with('$') {
            return Err(QuickDbError::ValidationError {
                field: field_name.to_string(),
                message: "NoSQL字段名不能以$开头".to_string(),
            });
        }

        // 不能包含点号（MongoDB嵌套字段路径分隔符）
        if field_name.contains('.') {
            return Err(QuickDbError::ValidationError {
                field: field_name.to_string(),
                message: "NoSQL字段名不能包含点号".to_string(),
            });
        }

        // 检查MongoDB的特殊字段名
        let mongo_reserved_names = [
            "_id", "id", "ns", "system", "op", "query", "update", "fields", "new",
            "upsert", "multi", "writeConcern", "collation", "arrayFilters", "hint",
        ];

        if mongo_reserved_names.contains(&field_name) {
            return Err(QuickDbError::ValidationError {
                field: field_name.to_string(),
                message: format!("字段名不能使用MongoDB保留字: {}", field_name),
            });
        }

        Ok(())
    }

    /// 验证SQL数据库表名的安全性
    fn validate_sql_table_name(&self, table_name: &str) -> QuickDbResult<()> {
        // 检查第一个字符不能是数字
        if table_name.chars().next().unwrap().is_ascii_digit() {
            return Err(QuickDbError::ValidationError {
                field: table_name.to_string(),
                message: "SQL表名不能以数字开头".to_string(),
            });
        }

        // 检查表名只包含安全字符
        for (i, ch) in table_name.chars().enumerate() {
            if !ch.is_ascii_alphanumeric() && ch != '_' {
                return Err(QuickDbError::ValidationError {
                    field: table_name.to_string(),
                    message: format!("SQL表名包含非法字符 '{}' 在位置 {}", ch, i),
                });
            }
        }

        // 检查是否为SQL关键字
        let upper_name = table_name.to_uppercase();
        let sql_keywords = [
            "SELECT", "FROM", "WHERE", "INSERT", "UPDATE", "DELETE", "CREATE", "DROP",
            "ALTER", "TABLE", "INDEX", "DATABASE", "SCHEMA", "USER", "ROLE", "GRANT",
            "REVOKE", "COMMIT", "ROLLBACK", "TRANSACTION", "VIEW", "TRIGGER", "PROCEDURE",
            "FUNCTION", "SEQUENCE", "CONSTRAINT", "PRIMARY", "FOREIGN", "REFERENCES",
        ];

        if sql_keywords.contains(&upper_name.as_str()) {
            return Err(QuickDbError::ValidationError {
                field: table_name.to_string(),
                message: format!("表名不能使用SQL关键字: {}", table_name),
            });
        }

        Ok(())
    }

    /// 验证NoSQL数据库集合名的安全性
    fn validate_nosql_collection_name(&self, collection_name: &str) -> QuickDbResult<()> {
        // MongoDB集合名限制

        // 不能以$开头
        if collection_name.starts_with('$') {
            return Err(QuickDbError::ValidationError {
                field: collection_name.to_string(),
                message: "集合名不能以$开头".to_string(),
            });
        }

        // 不能包含空字符串
        if collection_name.contains('\0') {
            return Err(QuickDbError::ValidationError {
                field: collection_name.to_string(),
                message: "集合名不能包含空字符".to_string(),
            });
        }

        // 不能是system集合
        if collection_name.starts_with("system.") {
            return Err(QuickDbError::ValidationError {
                field: collection_name.to_string(),
                message: "集合名不能以system.开头".to_string(),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sql_field_validation() {
        let validator = DatabaseSecurityValidator::new(DatabaseType::PostgreSQL);

        // 有效字段名
        assert!(validator.validate_field_name("name").is_ok());
        assert!(validator.validate_field_name("user_name").is_ok());
        assert!(validator.validate_field_name("createdAt").is_ok());

        // 无效字段名
        assert!(validator.validate_field_name("").is_err());
        assert!(validator.validate_field_name("123name").is_err());
        assert!(validator.validate_field_name("na-me").is_err());
        assert!(validator.validate_field_name("na me").is_err());
        assert!(validator.validate_field_name("select").is_err());
        assert!(validator.validate_field_name("WHERE").is_err());
    }

    #[test]
    fn test_nosql_field_validation() {
        let validator = DatabaseSecurityValidator::new(DatabaseType::MongoDB);

        // 有效字段名
        assert!(validator.validate_field_name("name").is_ok());
        assert!(validator.validate_field_name("user-name").is_ok()); // NoSQL允许连字符
        assert!(validator.validate_field_name("123name").is_ok()); // NoSQL允许数字开头

        // 无效字段名
        assert!(validator.validate_field_name("").is_err());
        assert!(validator.validate_field_name("$name").is_err());
        assert!(validator.validate_field_name("nested.field").is_err());
        assert!(validator.validate_field_name("_id").is_err());
    }

    #[test]
    fn test_safe_identifier_generation() {
        let pg_validator = DatabaseSecurityValidator::new(DatabaseType::PostgreSQL);
        let mysql_validator = DatabaseSecurityValidator::new(DatabaseType::MySQL);

        assert_eq!(pg_validator.get_safe_field_identifier("name").unwrap(), "\"name\"");
        assert_eq!(mysql_validator.get_safe_field_identifier("name").unwrap(), "`name`");

        // 测试非法字段名
        assert!(pg_validator.get_safe_field_identifier("select").is_err());
        assert!(mysql_validator.get_safe_field_identifier("123name").is_err());
    }
}