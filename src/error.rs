//! 错误处理模块
//!
//! 提供统一的错误类型定义和多语言错误信息

/// QuickDB 统一错误类型
#[derive(Debug)]
pub enum QuickDbError {
    /// 数据库连接错误
    ConnectionError { message: String },

    /// 连接池错误
    PoolError { message: String },

    /// 查询执行错误
    QueryError { message: String },

    /// 序列化/反序列化错误
    SerializationError { message: String },

    /// 模型验证错误
    ValidationError { field: String, message: String },

    /// 配置错误
    ConfigError { message: String },

    /// 数据库别名未找到
    AliasNotFound { alias: String },

    /// 不支持的数据库类型
    UnsupportedDatabase { db_type: String },

    /// 事务操作错误（虽然不支持事务，但保留用于未来扩展）
    TransactionError { message: String },

    /// 任务执行错误
    TaskExecutionError(String),

    /// 缓存操作错误
    CacheError { message: String },

    /// IO 错误
    IoError(std::io::Error),

    /// JSON 序列化错误
    JsonError(serde_json::Error),

    /// 通用错误
    Other(anyhow::Error),

    /// 表或集合不存在错误
    TableNotExistError { table: String, message: String },

    /// 版本管理错误
    VersionError { message: String },

    /// 数据未找到错误
    NotFound { message: String },
}

impl std::fmt::Display for QuickDbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConnectionError { message } => write!(
                f,
                "{}",
                crate::i18n::tf("error.connection", &[("message", message)])
            ),
            Self::PoolError { message } => write!(
                f,
                "{}",
                crate::i18n::tf("error.pool", &[("message", message)])
            ),
            Self::QueryError { message } => write!(
                f,
                "{}",
                crate::i18n::tf("error.query", &[("message", message)])
            ),
            Self::SerializationError { message } => write!(
                f,
                "{}",
                crate::i18n::tf("error.serialization", &[("message", message)])
            ),
            Self::ValidationError { field, message } => write!(
                f,
                "{}",
                crate::i18n::tf("error.validation", &[("field", field), ("message", message)])
            ),
            Self::ConfigError { message } => write!(
                f,
                "{}",
                crate::i18n::tf("error.config", &[("message", message)])
            ),
            Self::AliasNotFound { alias } => write!(
                f,
                "{}",
                crate::i18n::tf("error.alias_not_found", &[("alias", alias)])
            ),
            Self::UnsupportedDatabase { db_type } => write!(
                f,
                "{}",
                crate::i18n::tf("error.unsupported_database", &[("db_type", db_type)])
            ),
            Self::TransactionError { message } => write!(
                f,
                "{}",
                crate::i18n::tf("error.transaction", &[("message", message)])
            ),
            Self::TaskExecutionError(msg) => write!(
                f,
                "{}",
                crate::i18n::tf("error.task_execution", &[("message", msg)])
            ),
            Self::CacheError { message } => write!(
                f,
                "{}",
                crate::i18n::tf("error.cache", &[("message", message)])
            ),
            Self::IoError(e) => write!(
                f,
                "{}",
                crate::i18n::tf("error.io", &[("message", &e.to_string())])
            ),
            Self::JsonError(e) => write!(
                f,
                "{}",
                crate::i18n::tf("error.json", &[("message", &e.to_string())])
            ),
            Self::Other(e) => write!(
                f,
                "{}",
                crate::i18n::tf("error.other", &[("message", &e.to_string())])
            ),
            Self::TableNotExistError { table, message } => write!(
                f,
                "{}",
                crate::i18n::tf("error.table_not_exist", &[("table", table), ("message", message)])
            ),
            Self::VersionError { message } => write!(
                f,
                "{}",
                crate::i18n::tf("error.version", &[("message", message)])
            ),
            Self::NotFound { message } => write!(
                f,
                "{}",
                crate::i18n::tf("error.not_found", &[("message", message)])
            ),
        }
    }
}

impl std::error::Error for QuickDbError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::IoError(e) => Some(e),
            Self::JsonError(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for QuickDbError {
    fn from(err: std::io::Error) -> Self {
        QuickDbError::IoError(err)
    }
}

impl From<serde_json::Error> for QuickDbError {
    fn from(err: serde_json::Error) -> Self {
        QuickDbError::JsonError(err)
    }
}

impl From<anyhow::Error> for QuickDbError {
    fn from(err: anyhow::Error) -> Self {
        QuickDbError::Other(err)
    }
}

// 实现 From<sled::Error> 以支持 ? 操作符
impl From<sled::Error> for QuickDbError {
    fn from(err: sled::Error) -> Self {
        QuickDbError::VersionError {
            message: err.to_string(),
        }
    }
}

/// QuickDB 结果类型别名
pub type QuickDbResult<T> = Result<T, QuickDbError>;

/// 错误构建器 - 提供便捷的错误创建方法
pub struct ErrorBuilder;

impl ErrorBuilder {
    /// 创建连接错误
    pub fn connection_error(message: impl Into<String>) -> QuickDbError {
        QuickDbError::ConnectionError {
            message: message.into(),
        }
    }

    /// 创建连接池错误
    pub fn pool_error(message: impl Into<String>) -> QuickDbError {
        QuickDbError::PoolError {
            message: message.into(),
        }
    }

    /// 创建查询错误
    pub fn query_error(message: impl Into<String>) -> QuickDbError {
        QuickDbError::QueryError {
            message: message.into(),
        }
    }

    /// 创建序列化错误
    pub fn serialization_error(message: impl Into<String>) -> QuickDbError {
        QuickDbError::SerializationError {
            message: message.into(),
        }
    }

    /// 创建验证错误
    pub fn validation_error(field: impl Into<String>, message: impl Into<String>) -> QuickDbError {
        QuickDbError::ValidationError {
            field: field.into(),
            message: message.into(),
        }
    }

    /// 创建配置错误
    pub fn config_error(message: impl Into<String>) -> QuickDbError {
        QuickDbError::ConfigError {
            message: message.into(),
        }
    }

    /// 创建别名未找到错误
    pub fn alias_not_found(alias: impl Into<String>) -> QuickDbError {
        QuickDbError::AliasNotFound {
            alias: alias.into(),
        }
    }

    /// 创建不支持的数据库类型错误
    pub fn unsupported_database(db_type: impl Into<String>) -> QuickDbError {
        QuickDbError::UnsupportedDatabase {
            db_type: db_type.into(),
        }
    }

    /// 创建缓存错误
    pub fn cache_error(message: impl Into<String>) -> QuickDbError {
        QuickDbError::CacheError {
            message: message.into(),
        }
    }

    /// 创建表不存在错误
    pub fn table_not_exist_error(table: impl Into<String>, message: impl Into<String>) -> QuickDbError {
        QuickDbError::TableNotExistError {
            table: table.into(),
            message: message.into(),
        }
    }
}

/// 便捷宏 - 快速创建错误
#[macro_export]
macro_rules! quick_error {
    (connection, $msg:expr) => {
        $crate::error::ErrorBuilder::connection_error($msg)
    };
    (pool, $msg:expr) => {
        $crate::error::ErrorBuilder::pool_error($msg)
    };
    (query, $msg:expr) => {
        $crate::error::ErrorBuilder::query_error($msg)
    };
    (serialization, $msg:expr) => {
        $crate::error::ErrorBuilder::serialization_error($msg)
    };
    (validation, $field:expr, $msg:expr) => {
        $crate::error::ErrorBuilder::validation_error($field, $msg)
    };
    (config, $msg:expr) => {
        $crate::error::ErrorBuilder::config_error($msg)
    };
    (alias_not_found, $alias:expr) => {
        $crate::error::ErrorBuilder::alias_not_found($alias)
    };
    (unsupported_db, $db_type:expr) => {
        $crate::error::ErrorBuilder::unsupported_database($db_type)
    };
    (cache, $msg:expr) => {
        $crate::error::ErrorBuilder::cache_error($msg)
    };
    (table_not_exist, $table:expr, $msg:expr) => {
        $crate::error::ErrorBuilder::table_not_exist_error($table, $msg)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 辅助函数：初始化 i18n 并设置指定语言
    fn setup_i18n(lang: &str) {
        crate::i18n::ErrorMessageI18n::init_i18n();
        crate::i18n::set_language(lang);
    }

    // =========================================================================
    // 原有测试（保留，补充 i18n 初始化调用）
    // =========================================================================

    #[test]
    fn test_error_creation() {
        setup_i18n("zh-CN");
        let err = ErrorBuilder::connection_error("测试连接失败");
        assert!(matches!(err, QuickDbError::ConnectionError { .. }));
        assert_eq!(err.to_string(), "数据库连接失败: 测试连接失败");
    }

    #[test]
    fn test_error_macro() {
        setup_i18n("zh-CN");
        let err = quick_error!(validation, "用户名", "不能为空");
        assert!(matches!(err, QuickDbError::ValidationError { .. }));
        assert_eq!(err.to_string(), "模型验证失败: 用户名 - 不能为空");
    }

    #[test]
    fn test_table_not_exist_error() {
        setup_i18n("zh-CN");
        let err = ErrorBuilder::table_not_exist_error("users", "表不存在");
        assert!(matches!(err, QuickDbError::TableNotExistError { .. }));
        assert_eq!(err.to_string(), "表或集合 'users' 不存在: 表不存在");
    }

    #[test]
    fn test_table_not_exist_macro() {
        setup_i18n("zh-CN");
        let err = quick_error!(table_not_exist, "products", "产品表不存在");
        assert!(matches!(err, QuickDbError::TableNotExistError { .. }));
        assert_eq!(err.to_string(), "表或集合 'products' 不存在: 产品表不存在");
    }

    // =========================================================================
    // i18n Display 测试 - zh-CN（全部 17 个变体）
    // =========================================================================

    #[test]
    fn test_i18n_zh_cn_connection_error() {
        setup_i18n("zh-CN");
        let err = QuickDbError::ConnectionError {
            message: "连接超时".to_string(),
        };
        assert_eq!(err.to_string(), "数据库连接失败: 连接超时");
    }

    #[test]
    fn test_i18n_zh_cn_pool_error() {
        setup_i18n("zh-CN");
        let err = QuickDbError::PoolError {
            message: "连接池已耗尽".to_string(),
        };
        assert_eq!(err.to_string(), "连接池操作失败: 连接池已耗尽");
    }

    #[test]
    fn test_i18n_zh_cn_query_error() {
        setup_i18n("zh-CN");
        let err = QuickDbError::QueryError {
            message: "SQL语法错误".to_string(),
        };
        assert_eq!(err.to_string(), "查询执行失败: SQL语法错误");
    }

    #[test]
    fn test_i18n_zh_cn_serialization_error() {
        setup_i18n("zh-CN");
        let err = QuickDbError::SerializationError {
            message: "无法序列化".to_string(),
        };
        assert_eq!(err.to_string(), "数据序列化失败: 无法序列化");
    }

    #[test]
    fn test_i18n_zh_cn_validation_error() {
        setup_i18n("zh-CN");
        let err = QuickDbError::ValidationError {
            field: "email".to_string(),
            message: "格式不正确".to_string(),
        };
        assert_eq!(err.to_string(), "模型验证失败: email - 格式不正确");
    }

    #[test]
    fn test_i18n_zh_cn_config_error() {
        setup_i18n("zh-CN");
        let err = QuickDbError::ConfigError {
            message: "缺少必要参数".to_string(),
        };
        assert_eq!(err.to_string(), "配置错误: 缺少必要参数");
    }

    #[test]
    fn test_i18n_zh_cn_alias_not_found() {
        setup_i18n("zh-CN");
        let err = QuickDbError::AliasNotFound {
            alias: "primary_db".to_string(),
        };
        assert_eq!(err.to_string(), "数据库别名 'primary_db' 未找到");
    }

    #[test]
    fn test_i18n_zh_cn_unsupported_database() {
        setup_i18n("zh-CN");
        let err = QuickDbError::UnsupportedDatabase {
            db_type: "oracle".to_string(),
        };
        assert_eq!(err.to_string(), "不支持的数据库类型: oracle");
    }

    #[test]
    fn test_i18n_zh_cn_transaction_error() {
        setup_i18n("zh-CN");
        let err = QuickDbError::TransactionError {
            message: "事务回滚".to_string(),
        };
        assert_eq!(err.to_string(), "事务操作失败: 事务回滚");
    }

    #[test]
    fn test_i18n_zh_cn_task_execution_error() {
        setup_i18n("zh-CN");
        let err = QuickDbError::TaskExecutionError("后台任务超时".to_string());
        assert_eq!(err.to_string(), "任务执行失败: 后台任务超时");
    }

    #[test]
    fn test_i18n_zh_cn_cache_error() {
        setup_i18n("zh-CN");
        let err = QuickDbError::CacheError {
            message: "缓存写入失败".to_string(),
        };
        assert_eq!(err.to_string(), "缓存操作失败: 缓存写入失败");
    }

    #[test]
    fn test_i18n_zh_cn_io_error() {
        setup_i18n("zh-CN");
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "文件不存在");
        let err = QuickDbError::IoError(io_err);
        assert_eq!(err.to_string(), "IO 操作失败: 文件不存在");
    }

    #[test]
    fn test_i18n_zh_cn_json_error() {
        setup_i18n("zh-CN");
        let json_err: serde_json::Error = serde_json::from_str::<serde_json::Value>("{invalid}")
            .unwrap_err();
        let err = QuickDbError::JsonError(json_err);
        assert!(err.to_string().starts_with("JSON 处理失败:"));
    }

    #[test]
    fn test_i18n_zh_cn_other_error() {
        setup_i18n("zh-CN");
        let anyhow_err = anyhow::anyhow!("自定义错误信息");
        let err = QuickDbError::Other(anyhow_err);
        assert_eq!(err.to_string(), "操作失败: 自定义错误信息");
    }

    #[test]
    fn test_i18n_zh_cn_table_not_exist_error() {
        setup_i18n("zh-CN");
        let err = QuickDbError::TableNotExistError {
            table: "orders".to_string(),
            message: "表不存在".to_string(),
        };
        assert_eq!(err.to_string(), "表或集合 'orders' 不存在: 表不存在");
    }

    #[test]
    fn test_i18n_zh_cn_version_error() {
        setup_i18n("zh-CN");
        let err = QuickDbError::VersionError {
            message: "版本不兼容".to_string(),
        };
        assert_eq!(err.to_string(), "版本管理操作失败: 版本不兼容");
    }

    #[test]
    fn test_i18n_zh_cn_not_found_error() {
        setup_i18n("zh-CN");
        let err = QuickDbError::NotFound {
            message: "记录不存在".to_string(),
        };
        assert_eq!(err.to_string(), "数据未找到: 记录不存在");
    }

    // =========================================================================
    // i18n Display 测试 - en-US（全部 17 个变体）
    // =========================================================================

    #[test]
    fn test_i18n_en_us_connection_error() {
        setup_i18n("en-US");
        let err = QuickDbError::ConnectionError {
            message: "connection timeout".to_string(),
        };
        assert_eq!(err.to_string(), "Database connection failed: connection timeout");
    }

    #[test]
    fn test_i18n_en_us_pool_error() {
        setup_i18n("en-US");
        let err = QuickDbError::PoolError {
            message: "pool exhausted".to_string(),
        };
        assert_eq!(err.to_string(), "Connection pool operation failed: pool exhausted");
    }

    #[test]
    fn test_i18n_en_us_query_error() {
        setup_i18n("en-US");
        let err = QuickDbError::QueryError {
            message: "SQL syntax error".to_string(),
        };
        assert_eq!(err.to_string(), "Query execution failed: SQL syntax error");
    }

    #[test]
    fn test_i18n_en_us_serialization_error() {
        setup_i18n("en-US");
        let err = QuickDbError::SerializationError {
            message: "cannot serialize".to_string(),
        };
        assert_eq!(err.to_string(), "Data serialization failed: cannot serialize");
    }

    #[test]
    fn test_i18n_en_us_validation_error() {
        setup_i18n("en-US");
        let err = QuickDbError::ValidationError {
            field: "email".to_string(),
            message: "invalid format".to_string(),
        };
        assert_eq!(err.to_string(), "Model validation failed: email - invalid format");
    }

    #[test]
    fn test_i18n_en_us_config_error() {
        setup_i18n("en-US");
        let err = QuickDbError::ConfigError {
            message: "missing required parameter".to_string(),
        };
        assert_eq!(err.to_string(), "Configuration error: missing required parameter");
    }

    #[test]
    fn test_i18n_en_us_alias_not_found() {
        setup_i18n("en-US");
        let err = QuickDbError::AliasNotFound {
            alias: "primary_db".to_string(),
        };
        assert_eq!(err.to_string(), "Database alias 'primary_db' not found");
    }

    #[test]
    fn test_i18n_en_us_unsupported_database() {
        setup_i18n("en-US");
        let err = QuickDbError::UnsupportedDatabase {
            db_type: "oracle".to_string(),
        };
        assert_eq!(err.to_string(), "Unsupported database type: oracle");
    }

    #[test]
    fn test_i18n_en_us_transaction_error() {
        setup_i18n("en-US");
        let err = QuickDbError::TransactionError {
            message: "transaction rolled back".to_string(),
        };
        assert_eq!(err.to_string(), "Transaction operation failed: transaction rolled back");
    }

    #[test]
    fn test_i18n_en_us_task_execution_error() {
        setup_i18n("en-US");
        let err = QuickDbError::TaskExecutionError("background task timeout".to_string());
        assert_eq!(err.to_string(), "Task execution failed: background task timeout");
    }

    #[test]
    fn test_i18n_en_us_cache_error() {
        setup_i18n("en-US");
        let err = QuickDbError::CacheError {
            message: "cache write failed".to_string(),
        };
        assert_eq!(err.to_string(), "Cache operation failed: cache write failed");
    }

    #[test]
    fn test_i18n_en_us_io_error() {
        setup_i18n("en-US");
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err = QuickDbError::IoError(io_err);
        assert_eq!(err.to_string(), "IO operation failed: file not found");
    }

    #[test]
    fn test_i18n_en_us_json_error() {
        setup_i18n("en-US");
        let json_err: serde_json::Error = serde_json::from_str::<serde_json::Value>("{invalid}")
            .unwrap_err();
        let err = QuickDbError::JsonError(json_err);
        assert!(err.to_string().starts_with("JSON processing failed:"));
    }

    #[test]
    fn test_i18n_en_us_other_error() {
        setup_i18n("en-US");
        let anyhow_err = anyhow::anyhow!("custom error message");
        let err = QuickDbError::Other(anyhow_err);
        assert_eq!(err.to_string(), "Operation failed: custom error message");
    }

    #[test]
    fn test_i18n_en_us_table_not_exist_error() {
        setup_i18n("en-US");
        let err = QuickDbError::TableNotExistError {
            table: "orders".to_string(),
            message: "table does not exist".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "Table or collection 'orders' does not exist: table does not exist"
        );
    }

    #[test]
    fn test_i18n_en_us_version_error() {
        setup_i18n("en-US");
        let err = QuickDbError::VersionError {
            message: "version incompatible".to_string(),
        };
        assert_eq!(err.to_string(), "Version management operation failed: version incompatible");
    }

    #[test]
    fn test_i18n_en_us_not_found_error() {
        setup_i18n("en-US");
        let err = QuickDbError::NotFound {
            message: "record not found".to_string(),
        };
        assert_eq!(err.to_string(), "Data not found: record not found");
    }

    // =========================================================================
    // i18n Display 测试 - ja-JP（全部 17 个变体）
    // =========================================================================

    #[test]
    fn test_i18n_ja_jp_connection_error() {
        setup_i18n("ja-JP");
        let err = QuickDbError::ConnectionError {
            message: "接続タイムアウト".to_string(),
        };
        assert_eq!(err.to_string(), "データベース接続に失敗しました: 接続タイムアウト");
    }

    #[test]
    fn test_i18n_ja_jp_pool_error() {
        setup_i18n("ja-JP");
        let err = QuickDbError::PoolError {
            message: "プール枯渇".to_string(),
        };
        assert_eq!(err.to_string(), "接続プール操作が失敗しました: プール枯渇");
    }

    #[test]
    fn test_i18n_ja_jp_query_error() {
        setup_i18n("ja-JP");
        let err = QuickDbError::QueryError {
            message: "SQL構文エラー".to_string(),
        };
        assert_eq!(err.to_string(), "クエリ実行が失敗しました: SQL構文エラー");
    }

    #[test]
    fn test_i18n_ja_jp_serialization_error() {
        setup_i18n("ja-JP");
        let err = QuickDbError::SerializationError {
            message: "シリアライズ不可".to_string(),
        };
        assert_eq!(err.to_string(), "データシリアライズが失敗しました: シリアライズ不可");
    }

    #[test]
    fn test_i18n_ja_jp_validation_error() {
        setup_i18n("ja-JP");
        let err = QuickDbError::ValidationError {
            field: "email".to_string(),
            message: "形式が正しくありません".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "モデル検証が失敗しました: email - 形式が正しくありません"
        );
    }

    #[test]
    fn test_i18n_ja_jp_config_error() {
        setup_i18n("ja-JP");
        let err = QuickDbError::ConfigError {
            message: "必須パラメータが不足".to_string(),
        };
        assert_eq!(err.to_string(), "設定エラー: 必須パラメータが不足");
    }

    #[test]
    fn test_i18n_ja_jp_alias_not_found() {
        setup_i18n("ja-JP");
        let err = QuickDbError::AliasNotFound {
            alias: "primary_db".to_string(),
        };
        assert_eq!(err.to_string(), "データベースエイリアス 'primary_db' が見つかりません");
    }

    #[test]
    fn test_i18n_ja_jp_unsupported_database() {
        setup_i18n("ja-JP");
        let err = QuickDbError::UnsupportedDatabase {
            db_type: "oracle".to_string(),
        };
        assert_eq!(err.to_string(), "サポートされていないデータベースタイプ: oracle");
    }

    #[test]
    fn test_i18n_ja_jp_transaction_error() {
        setup_i18n("ja-JP");
        let err = QuickDbError::TransactionError {
            message: "トランザクションロールバック".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "トランザクション操作が失敗しました: トランザクションロールバック"
        );
    }

    #[test]
    fn test_i18n_ja_jp_task_execution_error() {
        setup_i18n("ja-JP");
        let err = QuickDbError::TaskExecutionError("バックグラウンドタスクタイムアウト".to_string());
        assert_eq!(err.to_string(), "タスク実行が失敗しました: バックグラウンドタスクタイムアウト");
    }

    #[test]
    fn test_i18n_ja_jp_cache_error() {
        setup_i18n("ja-JP");
        let err = QuickDbError::CacheError {
            message: "キャッシュ書き込み失敗".to_string(),
        };
        assert_eq!(err.to_string(), "キャッシュ操作が失敗しました: キャッシュ書き込み失敗");
    }

    #[test]
    fn test_i18n_ja_jp_io_error() {
        setup_i18n("ja-JP");
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "ファイルが見つかりません");
        let err = QuickDbError::IoError(io_err);
        assert_eq!(err.to_string(), "IO操作が失敗しました: ファイルが見つかりません");
    }

    #[test]
    fn test_i18n_ja_jp_json_error() {
        setup_i18n("ja-JP");
        let json_err: serde_json::Error = serde_json::from_str::<serde_json::Value>("{invalid}")
            .unwrap_err();
        let err = QuickDbError::JsonError(json_err);
        assert!(err.to_string().starts_with("JSON処理が失敗しました:"));
    }

    #[test]
    fn test_i18n_ja_jp_other_error() {
        setup_i18n("ja-JP");
        let anyhow_err = anyhow::anyhow!("カスタムエラーメッセージ");
        let err = QuickDbError::Other(anyhow_err);
        assert_eq!(err.to_string(), "操作が失敗しました: カスタムエラーメッセージ");
    }

    #[test]
    fn test_i18n_ja_jp_table_not_exist_error() {
        setup_i18n("ja-JP");
        let err = QuickDbError::TableNotExistError {
            table: "orders".to_string(),
            message: "テーブルが存在しません".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "テーブルまたはコレクション 'orders' が存在しません: テーブルが存在しません"
        );
    }

    #[test]
    fn test_i18n_ja_jp_version_error() {
        setup_i18n("ja-JP");
        let err = QuickDbError::VersionError {
            message: "バージョン非互換".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "バージョン管理操作が失敗しました: バージョン非互換"
        );
    }

    #[test]
    fn test_i18n_ja_jp_not_found_error() {
        setup_i18n("ja-JP");
        let err = QuickDbError::NotFound {
            message: "レコードが見つかりません".to_string(),
        };
        assert_eq!(err.to_string(), "データが見つかりません: レコードが見つかりません");
    }

    // =========================================================================
    // Error trait source() 测试 - IoError, JsonError, Other
    // =========================================================================

    #[test]
    fn test_source_for_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        let err = QuickDbError::IoError(io_err);
        let source = std::error::Error::source(&err);
        assert!(source.is_some());
        let source = source.unwrap();
        assert_eq!(source.to_string(), "access denied");
    }

    #[test]
    fn test_source_for_json_error() {
        let json_err: serde_json::Error =
            serde_json::from_str::<serde_json::Value>("{invalid}").unwrap_err();
        let err = QuickDbError::JsonError(json_err);
        let source = std::error::Error::source(&err);
        assert!(source.is_some());
        // serde_json::Error's Display output contains descriptive text
        let source_str = source.unwrap().to_string();
        assert!(!source_str.is_empty());
    }

    #[test]
    fn test_source_for_other_error() {
        let anyhow_err = anyhow::anyhow!("some error");
        let err = QuickDbError::Other(anyhow_err);
        // anyhow::Error 的 source() 不暴露内部错误链，返回 None
        assert!(std::error::Error::source(&err).is_none());
    }

    #[test]
    fn test_source_for_non_wrapped_variants_is_none() {
        let err = QuickDbError::ConnectionError {
            message: "test".to_string(),
        };
        assert!(std::error::Error::source(&err).is_none());

        let err = QuickDbError::ValidationError {
            field: "f".to_string(),
            message: "m".to_string(),
        };
        assert!(std::error::Error::source(&err).is_none());

        let err = QuickDbError::TaskExecutionError("task".to_string());
        assert!(std::error::Error::source(&err).is_none());
    }
}
