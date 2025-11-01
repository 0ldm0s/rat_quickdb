//! 多语言错误消息模块
//!
//! 使用rat_embed_lang框架提供统一的错误消息多语言支持

use std::collections::HashMap;
use rat_embed_lang::register_translations;

/// 错误消息翻译注册器
pub struct ErrorMessageI18n;

impl ErrorMessageI18n {
    /// 注册所有错误消息翻译
    pub fn register_all_translations() {
        let mut translations = HashMap::new();

        // 数据库连接错误
        let mut connection_errors = HashMap::new();
        connection_errors.insert("zh-CN".to_string(), "数据库连接失败: {message}".to_string());
        connection_errors.insert("en-US".to_string(), "Database connection failed: {message}".to_string());
        connection_errors.insert("ja-JP".to_string(), "データベース接続に失敗しました: {message}".to_string());
        translations.insert("error.connection".to_string(), connection_errors);

        // 连接池错误
        let mut pool_errors = HashMap::new();
        pool_errors.insert("zh-CN".to_string(), "连接池操作失败: {message}".to_string());
        pool_errors.insert("en-US".to_string(), "Connection pool operation failed: {message}".to_string());
        pool_errors.insert("ja-JP".to_string(), "接続プール操作が失敗しました: {message}".to_string());
        translations.insert("error.pool".to_string(), pool_errors);

        // 查询错误
        let mut query_errors = HashMap::new();
        query_errors.insert("zh-CN".to_string(), "查询执行失败: {message}".to_string());
        query_errors.insert("en-US".to_string(), "Query execution failed: {message}".to_string());
        query_errors.insert("ja-JP".to_string(), "クエリ実行が失敗しました: {message}".to_string());
        translations.insert("error.query".to_string(), query_errors);

        // 序列化错误
        let mut serialization_errors = HashMap::new();
        serialization_errors.insert("zh-CN".to_string(), "数据序列化失败: {message}".to_string());
        serialization_errors.insert("en-US".to_string(), "Data serialization failed: {message}".to_string());
        serialization_errors.insert("ja-JP".to_string(), "データシリアライズが失敗しました: {message}".to_string());
        translations.insert("error.serialization".to_string(), serialization_errors);

        // 模型验证错误
        let mut validation_errors = HashMap::new();
        validation_errors.insert("zh-CN".to_string(), "模型验证失败: {field} - {message}".to_string());
        validation_errors.insert("en-US".to_string(), "Model validation failed: {field} - {message}".to_string());
        validation_errors.insert("ja-JP".to_string(), "モデル検証が失敗しました: {field} - {message}".to_string());
        translations.insert("error.validation".to_string(), validation_errors);

        // 配置错误
        let mut config_errors = HashMap::new();
        config_errors.insert("zh-CN".to_string(), "配置错误: {message}".to_string());
        config_errors.insert("en-US".to_string(), "Configuration error: {message}".to_string());
        config_errors.insert("ja-JP".to_string(), "設定エラー: {message}".to_string());
        translations.insert("error.config".to_string(), config_errors);

        // 数据库别名未找到
        let mut alias_not_found_errors = HashMap::new();
        alias_not_found_errors.insert("zh-CN".to_string(), "数据库别名 '{alias}' 未找到".to_string());
        alias_not_found_errors.insert("en-US".to_string(), "Database alias '{alias}' not found".to_string());
        alias_not_found_errors.insert("ja-JP".to_string(), "データベースエイリアス '{alias}' が見つかりません".to_string());
        translations.insert("error.alias_not_found".to_string(), alias_not_found_errors);

        // 不支持的数据库类型
        let mut unsupported_db_errors = HashMap::new();
        unsupported_db_errors.insert("zh-CN".to_string(), "不支持的数据库类型: {db_type}".to_string());
        unsupported_db_errors.insert("en-US".to_string(), "Unsupported database type: {db_type}".to_string());
        unsupported_db_errors.insert("ja-JP".to_string(), "サポートされていないデータベースタイプ: {db_type}".to_string());
        translations.insert("error.unsupported_database".to_string(), unsupported_db_errors);

        // 缓存错误
        let mut cache_errors = HashMap::new();
        cache_errors.insert("zh-CN".to_string(), "缓存操作失败: {message}".to_string());
        cache_errors.insert("en-US".to_string(), "Cache operation failed: {message}".to_string());
        cache_errors.insert("ja-JP".to_string(), "キャッシュ操作が失敗しました: {message}".to_string());
        translations.insert("error.cache".to_string(), cache_errors);

        // SQLite工作器启动失败
        let mut sqlite_worker_errors = HashMap::new();
        sqlite_worker_errors.insert("zh-CN".to_string(), "SQLite工作器启动失败: 别名={alias}".to_string());
        sqlite_worker_errors.insert("en-US".to_string(), "SQLite worker startup failed: alias={alias}".to_string());
        sqlite_worker_errors.insert("ja-JP".to_string(), "SQLiteワーカー起動失敗: エイリアス={alias}".to_string());
        translations.insert("error.sqlite_worker_startup".to_string(), sqlite_worker_errors);

        // SQLite内存数据库连接失败
        let mut sqlite_memory_errors = HashMap::new();
        sqlite_memory_errors.insert("zh-CN".to_string(), "SQLite内存数据库连接失败: {message}".to_string());
        sqlite_memory_errors.insert("en-US".to_string(), "SQLite in-memory database connection failed: {message}".to_string());
        sqlite_memory_errors.insert("ja-JP".to_string(), "SQLiteインメモリデータベース接続失敗: {message}".to_string());
        translations.insert("error.sqlite_memory".to_string(), sqlite_memory_errors);

        // SQLite数据库文件不存在
        let mut sqlite_file_not_found = HashMap::new();
        sqlite_file_not_found.insert("zh-CN".to_string(), "SQLite数据库文件不存在且未启用自动创建: {path}".to_string());
        sqlite_file_not_found.insert("en-US".to_string(), "SQLite database file does not exist and auto-create is not enabled: {path}".to_string());
        sqlite_file_not_found.insert("ja-JP".to_string(), "SQLiteデータベースファイルが存在せず、自動作成が有効ではありません: {path}".to_string());
        translations.insert("error.sqlite_file_not_found".to_string(), sqlite_file_not_found);

        // 创建SQLite数据库目录失败
        let mut sqlite_dir_create_failed = HashMap::new();
        sqlite_dir_create_failed.insert("zh-CN".to_string(), "创建SQLite数据库目录失败: {message}".to_string());
        sqlite_dir_create_failed.insert("en-US".to_string(), "Failed to create SQLite database directory: {message}".to_string());
        sqlite_dir_create_failed.insert("ja-JP".to_string(), "SQLiteデータベースディレクトリ作成失敗: {message}".to_string());
        translations.insert("error.sqlite_dir_create".to_string(), sqlite_dir_create_failed);

        // 创建SQLite数据库文件失败
        let mut sqlite_file_create_failed = HashMap::new();
        sqlite_file_create_failed.insert("zh-CN".to_string(), "创建SQLite数据库文件失败: {message}".to_string());
        sqlite_file_create_failed.insert("en-US".to_string(), "Failed to create SQLite database file: {message}".to_string());
        sqlite_file_create_failed.insert("ja-JP".to_string(), "SQLiteデータベースファイル作成失敗: {message}".to_string());
        translations.insert("error.sqlite_file_create".to_string(), sqlite_file_create_failed);

        // SQLite连接失败
        let mut sqlite_connection_failed = HashMap::new();
        sqlite_connection_failed.insert("zh-CN".to_string(), "SQLite连接失败: {message}".to_string());
        sqlite_connection_failed.insert("en-US".to_string(), "SQLite connection failed: {message}".to_string());
        sqlite_connection_failed.insert("ja-JP".to_string(), "SQLite接続失敗: {message}".to_string());
        translations.insert("error.sqlite_connection".to_string(), sqlite_connection_failed);

        // PostgreSQL连接配置类型不匹配
        let mut postgres_config_mismatch = HashMap::new();
        postgres_config_mismatch.insert("zh-CN".to_string(), "PostgreSQL连接配置类型不匹配".to_string());
        postgres_config_mismatch.insert("en-US".to_string(), "PostgreSQL connection configuration type mismatch".to_string());
        postgres_config_mismatch.insert("ja-JP".to_string(), "PostgreSQL接続設定タイプが一致しません".to_string());
        translations.insert("error.postgres_config_mismatch".to_string(), postgres_config_mismatch);

        // PostgreSQL连接池创建失败
        let mut postgres_pool_create_failed = HashMap::new();
        postgres_pool_create_failed.insert("zh-CN".to_string(), "PostgreSQL连接池创建失败: {message}".to_string());
        postgres_pool_create_failed.insert("en-US".to_string(), "PostgreSQL connection pool creation failed: {message}".to_string());
        postgres_pool_create_failed.insert("ja-JP".to_string(), "PostgreSQL接続プール作成失敗: {message}".to_string());
        translations.insert("error.postgres_pool_create".to_string(), postgres_pool_create_failed);

        // MySQL连接配置类型不匹配
        let mut mysql_config_mismatch = HashMap::new();
        mysql_config_mismatch.insert("zh-CN".to_string(), "MySQL连接配置类型不匹配".to_string());
        mysql_config_mismatch.insert("en-US".to_string(), "MySQL connection configuration type mismatch".to_string());
        mysql_config_mismatch.insert("ja-JP".to_string(), "MySQL接続設定タイプが一致しません".to_string());
        translations.insert("error.mysql_config_mismatch".to_string(), mysql_config_mismatch);

        // MySQL连接池创建失败
        let mut mysql_pool_create_failed = HashMap::new();
        mysql_pool_create_failed.insert("zh-CN".to_string(), "MySQL连接池创建失败: {message}".to_string());
        mysql_pool_create_failed.insert("en-US".to_string(), "MySQL connection pool creation failed: {message}".to_string());
        mysql_pool_create_failed.insert("ja-JP".to_string(), "MySQL接続プール作成失敗: {message}".to_string());
        translations.insert("error.mysql_pool_create".to_string(), mysql_pool_create_failed);

        // MongoDB连接配置类型不匹配
        let mut mongodb_config_mismatch = HashMap::new();
        mongodb_config_mismatch.insert("zh-CN".to_string(), "MongoDB连接配置类型不匹配".to_string());
        mongodb_config_mismatch.insert("en-US".to_string(), "MongoDB connection configuration type mismatch".to_string());
        mongodb_config_mismatch.insert("ja-JP".to_string(), "MongoDB接続設定タイプが一致しません".to_string());
        translations.insert("error.mongodb_config_mismatch".to_string(), mongodb_config_mismatch);

        // MongoDB连接失败
        let mut mongodb_connection_failed = HashMap::new();
        mongodb_connection_failed.insert("zh-CN".to_string(), "MongoDB连接失败: {message}".to_string());
        mongodb_connection_failed.insert("en-US".to_string(), "MongoDB connection failed: {message}".to_string());
        mongodb_connection_failed.insert("ja-JP".to_string(), "MongoDB接続失敗: {message}".to_string());
        translations.insert("error.mongodb_connection".to_string(), mongodb_connection_failed);

        // SQLite连接配置类型不匹配
        let mut sqlite_config_mismatch = HashMap::new();
        sqlite_config_mismatch.insert("zh-CN".to_string(), "SQLite连接配置类型不匹配".to_string());
        sqlite_config_mismatch.insert("en-US".to_string(), "SQLite connection configuration type mismatch".to_string());
        sqlite_config_mismatch.insert("ja-JP".to_string(), "SQLite接続設定タイプが一致しません".to_string());
        translations.insert("error.sqlite_config_mismatch".to_string(), sqlite_config_mismatch);

        // 任务队列相关错误
        let mut task_queue_not_initialized = HashMap::new();
        task_queue_not_initialized.insert("zh-CN".to_string(), "任务队列未初始化，请先调用 initialize_global_task_queue".to_string());
        task_queue_not_initialized.insert("en-US".to_string(), "Task queue not initialized, please call initialize_global_task_queue first".to_string());
        task_queue_not_initialized.insert("ja-JP".to_string(), "タスクキューが初期化されていません。まず initialize_global_task_queue を呼び出してください".to_string());
        translations.insert("error.task_queue_not_initialized".to_string(), task_queue_not_initialized);

        let mut task_queue_already_initialized = HashMap::new();
        task_queue_already_initialized.insert("zh-CN".to_string(), "任务队列已经初始化".to_string());
        task_queue_already_initialized.insert("en-US".to_string(), "Task queue already initialized".to_string());
        task_queue_already_initialized.insert("ja-JP".to_string(), "タスクキューは既に初期化されています".to_string());
        translations.insert("error.task_queue_already_initialized".to_string(), task_queue_already_initialized);

        // 表结构相关错误
        let mut table_no_columns = HashMap::new();
        table_no_columns.insert("zh-CN".to_string(), "表必须至少有一个列".to_string());
        table_no_columns.insert("en-US".to_string(), "Table must have at least one column".to_string());
        table_no_columns.insert("ja-JP".to_string(), "テーブルには少なくとも1つの列が必要です".to_string());
        translations.insert("error.table_no_columns".to_string(), table_no_columns);

        let mut column_name_duplicate = HashMap::new();
        column_name_duplicate.insert("zh-CN".to_string(), "列名 '{name}' 重复".to_string());
        column_name_duplicate.insert("en-US".to_string(), "Column name '{name}' is duplicated".to_string());
        column_name_duplicate.insert("ja-JP".to_string(), "列名 '{name}' が重複しています".to_string());
        translations.insert("error.column_name_duplicate".to_string(), column_name_duplicate);

        let mut index_name_duplicate = HashMap::new();
        index_name_duplicate.insert("zh-CN".to_string(), "索引名 '{name}' 重复".to_string());
        index_name_duplicate.insert("en-US".to_string(), "Index name '{name}' is duplicated".to_string());
        index_name_duplicate.insert("ja-JP".to_string(), "インデックス名 '{name}' が重複しています".to_string());
        translations.insert("error.index_name_duplicate".to_string(), index_name_duplicate);

        let mut index_column_not_found = HashMap::new();
        index_column_not_found.insert("zh-CN".to_string(), "索引 '{index}' 引用的列 '{column}' 不存在".to_string());
        index_column_not_found.insert("en-US".to_string(), "Column '{column}' referenced by index '{index}' does not exist".to_string());
        index_column_not_found.insert("ja-JP".to_string(), "インデックス '{index}' が参照する列 '{column}' が存在しません".to_string());
        translations.insert("error.index_column_not_found".to_string(), index_column_not_found);

        let mut constraint_name_duplicate = HashMap::new();
        constraint_name_duplicate.insert("zh-CN".to_string(), "约束名 '{name}' 重复".to_string());
        constraint_name_duplicate.insert("en-US".to_string(), "Constraint name '{name}' is duplicated".to_string());
        constraint_name_duplicate.insert("ja-JP".to_string(), "制約名 '{name}' が重複しています".to_string());
        translations.insert("error.constraint_name_duplicate".to_string(), constraint_name_duplicate);

        let mut constraint_column_not_found = HashMap::new();
        constraint_column_not_found.insert("zh-CN".to_string(), "约束 '{constraint}' 引用的列 '{column}' 不存在".to_string());
        constraint_column_not_found.insert("en-US".to_string(), "Column '{column}' referenced by constraint '{constraint}' does not exist".to_string());
        constraint_column_not_found.insert("ja-JP".to_string(), "制約 '{constraint}' が参照する列 '{column}' が存在しません".to_string());
        translations.insert("error.constraint_column_not_found".to_string(), constraint_column_not_found);

        // JSON序列化相关错误
        let mut json_serialize_failed = HashMap::new();
        json_serialize_failed.insert("zh-CN".to_string(), "序列化为JSON字符串失败: {message}".to_string());
        json_serialize_failed.insert("en-US".to_string(), "Failed to serialize to JSON string: {message}".to_string());
        json_serialize_failed.insert("ja-JP".to_string(), "JSON文字列へのシリアライズ失敗: {message}".to_string());
        translations.insert("error.json_serialize".to_string(), json_serialize_failed);

        let mut json_parse_failed = HashMap::new();
        json_parse_failed.insert("zh-CN".to_string(), "解析JSON字符串失败: {message}".to_string());
        json_parse_failed.insert("en-US".to_string(), "Failed to parse JSON string: {message}".to_string());
        json_parse_failed.insert("ja-JP".to_string(), "JSON文字列の解析失敗: {message}".to_string());
        translations.insert("error.json_parse".to_string(), json_parse_failed);

        let mut serialize_failed = HashMap::new();
        serialize_failed.insert("zh-CN".to_string(), "序列化失败: {message}".to_string());
        serialize_failed.insert("en-US".to_string(), "Serialization failed: {message}".to_string());
        serialize_failed.insert("ja-JP".to_string(), "シリアライズ失敗: {message}".to_string());
        translations.insert("error.serialize".to_string(), serialize_failed);

        // 注册所有翻译
        register_translations(translations);
    }

    /// 初始化错误消息多语言支持
    pub fn init() {
        Self::register_all_translations();

        // 从环境变量获取语言设置，默认为zh-CN
        let lang = std::env::var("RAT_LANG")
            .or_else(|_| std::env::var("LANG"))
            .unwrap_or_else(|_| "zh-CN".to_string());

        // 标准化语言代码
        use rat_embed_lang::normalize_language_code;
        let normalized_lang = normalize_language_code(&lang);
        set_language(&normalized_lang);
    }
}


/// 重新导出rat_embed_lang的核心函数
pub use rat_embed_lang::{t, tf, set_language, current_language};