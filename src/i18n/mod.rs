//! 多语言错误消息模块
//!
//! 使用rat_embed_lang框架提供统一的错误消息多语言支持

use rat_embed_lang::register_translations;
use std::collections::HashMap;
use std::sync::OnceLock;

/// i18n 初始化锁（确保只执行一次）
static INIT: OnceLock<()> = OnceLock::new();

/// 错误消息翻译注册器
pub struct ErrorMessageI18n;

impl ErrorMessageI18n {

    /// 初始化错误消息多语言支持（懒加载，只执行一次）
    ///
    /// 此函数会自动从环境变量读取语言设置（RAT_LANG 或 LANG），
    /// 并注册所有错误消息的翻译。
    ///
    /// 注意：此函数是线程安全的，且只在首次调用时执行初始化。
    pub(crate) fn init_i18n() {
        INIT.get_or_init(|| {
            Self::register_all_translations();

            let lang = std::env::var("RAT_LANG")
                .or_else(|_| std::env::var("LANG"))
                .unwrap_or_else(|_| "zh-CN".to_string());

            use rat_embed_lang::normalize_language_code;
            let normalized_lang = normalize_language_code(&lang);
            set_language(&normalized_lang);
        });
    }

    /// 确保i18n已初始化（内部辅助函数）
    #[inline]
    fn ensure_initialized() {
        Self::init_i18n();
    }
    /// 注册所有错误消息翻译
    pub(crate) fn register_all_translations() {
        let mut translations = HashMap::new();

        // 数据库连接错误
        let mut connection_errors = HashMap::new();
        connection_errors.insert("zh-CN".to_string(), "数据库连接失败: {message}".to_string());
        connection_errors.insert(
            "en-US".to_string(),
            "Database connection failed: {message}".to_string(),
        );
        connection_errors.insert(
            "ja-JP".to_string(),
            "データベース接続に失敗しました: {message}".to_string(),
        );
        translations.insert("error.connection".to_string(), connection_errors);

        // 连接池错误
        let mut pool_errors = HashMap::new();
        pool_errors.insert("zh-CN".to_string(), "连接池操作失败: {message}".to_string());
        pool_errors.insert(
            "en-US".to_string(),
            "Connection pool operation failed: {message}".to_string(),
        );
        pool_errors.insert(
            "ja-JP".to_string(),
            "接続プール操作が失敗しました: {message}".to_string(),
        );
        translations.insert("error.pool".to_string(), pool_errors);

        // 查询错误
        let mut query_errors = HashMap::new();
        query_errors.insert("zh-CN".to_string(), "查询执行失败: {message}".to_string());
        query_errors.insert(
            "en-US".to_string(),
            "Query execution failed: {message}".to_string(),
        );
        query_errors.insert(
            "ja-JP".to_string(),
            "クエリ実行が失敗しました: {message}".to_string(),
        );
        translations.insert("error.query".to_string(), query_errors);

        // 序列化错误
        let mut serialization_errors = HashMap::new();
        serialization_errors.insert("zh-CN".to_string(), "数据序列化失败: {message}".to_string());
        serialization_errors.insert(
            "en-US".to_string(),
            "Data serialization failed: {message}".to_string(),
        );
        serialization_errors.insert(
            "ja-JP".to_string(),
            "データシリアライズが失敗しました: {message}".to_string(),
        );
        translations.insert("error.serialization".to_string(), serialization_errors);

        // 模型验证错误
        let mut validation_errors = HashMap::new();
        validation_errors.insert(
            "zh-CN".to_string(),
            "模型验证失败: {field} - {message}".to_string(),
        );
        validation_errors.insert(
            "en-US".to_string(),
            "Model validation failed: {field} - {message}".to_string(),
        );
        validation_errors.insert(
            "ja-JP".to_string(),
            "モデル検証が失敗しました: {field} - {message}".to_string(),
        );
        translations.insert("error.validation".to_string(), validation_errors);

        // 配置错误
        let mut config_errors = HashMap::new();
        config_errors.insert("zh-CN".to_string(), "配置错误: {message}".to_string());
        config_errors.insert(
            "en-US".to_string(),
            "Configuration error: {message}".to_string(),
        );
        config_errors.insert("ja-JP".to_string(), "設定エラー: {message}".to_string());
        translations.insert("error.config".to_string(), config_errors);

        // 数据库别名未找到
        let mut alias_not_found_errors = HashMap::new();
        alias_not_found_errors.insert(
            "zh-CN".to_string(),
            "数据库别名 '{alias}' 未找到".to_string(),
        );
        alias_not_found_errors.insert(
            "en-US".to_string(),
            "Database alias '{alias}' not found".to_string(),
        );
        alias_not_found_errors.insert(
            "ja-JP".to_string(),
            "データベースエイリアス '{alias}' が見つかりません".to_string(),
        );
        translations.insert("error.alias_not_found".to_string(), alias_not_found_errors);

        // 不支持的数据库类型
        let mut unsupported_db_errors = HashMap::new();
        unsupported_db_errors.insert(
            "zh-CN".to_string(),
            "不支持的数据库类型: {db_type}".to_string(),
        );
        unsupported_db_errors.insert(
            "en-US".to_string(),
            "Unsupported database type: {db_type}".to_string(),
        );
        unsupported_db_errors.insert(
            "ja-JP".to_string(),
            "サポートされていないデータベースタイプ: {db_type}".to_string(),
        );
        translations.insert(
            "error.unsupported_database".to_string(),
            unsupported_db_errors,
        );

        // 缓存错误
        let mut cache_errors = HashMap::new();
        cache_errors.insert("zh-CN".to_string(), "缓存操作失败: {message}".to_string());
        cache_errors.insert(
            "en-US".to_string(),
            "Cache operation failed: {message}".to_string(),
        );
        cache_errors.insert(
            "ja-JP".to_string(),
            "キャッシュ操作が失敗しました: {message}".to_string(),
        );
        translations.insert("error.cache".to_string(), cache_errors);

        // SQLite工作器启动失败
        let mut sqlite_worker_errors = HashMap::new();
        sqlite_worker_errors.insert(
            "zh-CN".to_string(),
            "SQLite工作器启动失败: 别名={alias}".to_string(),
        );
        sqlite_worker_errors.insert(
            "en-US".to_string(),
            "SQLite worker startup failed: alias={alias}".to_string(),
        );
        sqlite_worker_errors.insert(
            "ja-JP".to_string(),
            "SQLiteワーカー起動失敗: エイリアス={alias}".to_string(),
        );
        translations.insert(
            "error.sqlite_worker_startup".to_string(),
            sqlite_worker_errors,
        );

        // SQLite内存数据库连接失败
        let mut sqlite_memory_errors = HashMap::new();
        sqlite_memory_errors.insert(
            "zh-CN".to_string(),
            "SQLite内存数据库连接失败: {message}".to_string(),
        );
        sqlite_memory_errors.insert(
            "en-US".to_string(),
            "SQLite in-memory database connection failed: {message}".to_string(),
        );
        sqlite_memory_errors.insert(
            "ja-JP".to_string(),
            "SQLiteインメモリデータベース接続失敗: {message}".to_string(),
        );
        translations.insert("error.sqlite_memory".to_string(), sqlite_memory_errors);

        // SQLite数据库文件不存在
        let mut sqlite_file_not_found = HashMap::new();
        sqlite_file_not_found.insert(
            "zh-CN".to_string(),
            "SQLite数据库文件不存在且未启用自动创建: {path}".to_string(),
        );
        sqlite_file_not_found.insert(
            "en-US".to_string(),
            "SQLite database file does not exist and auto-create is not enabled: {path}"
                .to_string(),
        );
        sqlite_file_not_found.insert(
            "ja-JP".to_string(),
            "SQLiteデータベースファイルが存在せず、自動作成が有効ではありません: {path}"
                .to_string(),
        );
        translations.insert(
            "error.sqlite_file_not_found".to_string(),
            sqlite_file_not_found,
        );

        // 创建SQLite数据库目录失败
        let mut sqlite_dir_create_failed = HashMap::new();
        sqlite_dir_create_failed.insert(
            "zh-CN".to_string(),
            "创建SQLite数据库目录失败: {message}".to_string(),
        );
        sqlite_dir_create_failed.insert(
            "en-US".to_string(),
            "Failed to create SQLite database directory: {message}".to_string(),
        );
        sqlite_dir_create_failed.insert(
            "ja-JP".to_string(),
            "SQLiteデータベースディレクトリ作成失敗: {message}".to_string(),
        );
        translations.insert(
            "error.sqlite_dir_create".to_string(),
            sqlite_dir_create_failed,
        );

        // 创建SQLite数据库文件失败
        let mut sqlite_file_create_failed = HashMap::new();
        sqlite_file_create_failed.insert(
            "zh-CN".to_string(),
            "创建SQLite数据库文件失败: {message}".to_string(),
        );
        sqlite_file_create_failed.insert(
            "en-US".to_string(),
            "Failed to create SQLite database file: {message}".to_string(),
        );
        sqlite_file_create_failed.insert(
            "ja-JP".to_string(),
            "SQLiteデータベースファイル作成失敗: {message}".to_string(),
        );
        translations.insert(
            "error.sqlite_file_create".to_string(),
            sqlite_file_create_failed,
        );

        // SQLite连接失败
        let mut sqlite_connection_failed = HashMap::new();
        sqlite_connection_failed
            .insert("zh-CN".to_string(), "SQLite连接失败: {message}".to_string());
        sqlite_connection_failed.insert(
            "en-US".to_string(),
            "SQLite connection failed: {message}".to_string(),
        );
        sqlite_connection_failed
            .insert("ja-JP".to_string(), "SQLite接続失敗: {message}".to_string());
        translations.insert(
            "error.sqlite_connection".to_string(),
            sqlite_connection_failed,
        );

        // PostgreSQL连接配置类型不匹配
        let mut postgres_config_mismatch = HashMap::new();
        postgres_config_mismatch.insert(
            "zh-CN".to_string(),
            "PostgreSQL连接配置类型不匹配".to_string(),
        );
        postgres_config_mismatch.insert(
            "en-US".to_string(),
            "PostgreSQL connection configuration type mismatch".to_string(),
        );
        postgres_config_mismatch.insert(
            "ja-JP".to_string(),
            "PostgreSQL接続設定タイプが一致しません".to_string(),
        );
        translations.insert(
            "error.postgres_config_mismatch".to_string(),
            postgres_config_mismatch,
        );

        // PostgreSQL连接池创建失败
        let mut postgres_pool_create_failed = HashMap::new();
        postgres_pool_create_failed.insert(
            "zh-CN".to_string(),
            "PostgreSQL连接池创建失败: {message}".to_string(),
        );
        postgres_pool_create_failed.insert(
            "en-US".to_string(),
            "PostgreSQL connection pool creation failed: {message}".to_string(),
        );
        postgres_pool_create_failed.insert(
            "ja-JP".to_string(),
            "PostgreSQL接続プール作成失敗: {message}".to_string(),
        );
        translations.insert(
            "error.postgres_pool_create".to_string(),
            postgres_pool_create_failed,
        );

        // MySQL连接配置类型不匹配
        let mut mysql_config_mismatch = HashMap::new();
        mysql_config_mismatch.insert("zh-CN".to_string(), "MySQL连接配置类型不匹配".to_string());
        mysql_config_mismatch.insert(
            "en-US".to_string(),
            "MySQL connection configuration type mismatch".to_string(),
        );
        mysql_config_mismatch.insert(
            "ja-JP".to_string(),
            "MySQL接続設定タイプが一致しません".to_string(),
        );
        translations.insert(
            "error.mysql_config_mismatch".to_string(),
            mysql_config_mismatch,
        );

        // MySQL连接池创建失败
        let mut mysql_pool_create_failed = HashMap::new();
        mysql_pool_create_failed.insert(
            "zh-CN".to_string(),
            "MySQL连接池创建失败: {message}".to_string(),
        );
        mysql_pool_create_failed.insert(
            "en-US".to_string(),
            "MySQL connection pool creation failed: {message}".to_string(),
        );
        mysql_pool_create_failed.insert(
            "ja-JP".to_string(),
            "MySQL接続プール作成失敗: {message}".to_string(),
        );
        translations.insert(
            "error.mysql_pool_create".to_string(),
            mysql_pool_create_failed,
        );

        // MongoDB连接配置类型不匹配
        let mut mongodb_config_mismatch = HashMap::new();
        mongodb_config_mismatch
            .insert("zh-CN".to_string(), "MongoDB连接配置类型不匹配".to_string());
        mongodb_config_mismatch.insert(
            "en-US".to_string(),
            "MongoDB connection configuration type mismatch".to_string(),
        );
        mongodb_config_mismatch.insert(
            "ja-JP".to_string(),
            "MongoDB接続設定タイプが一致しません".to_string(),
        );
        translations.insert(
            "error.mongodb_config_mismatch".to_string(),
            mongodb_config_mismatch,
        );

        // MongoDB连接失败
        let mut mongodb_connection_failed = HashMap::new();
        mongodb_connection_failed.insert(
            "zh-CN".to_string(),
            "MongoDB连接失败: {message}".to_string(),
        );
        mongodb_connection_failed.insert(
            "en-US".to_string(),
            "MongoDB connection failed: {message}".to_string(),
        );
        mongodb_connection_failed.insert(
            "ja-JP".to_string(),
            "MongoDB接続失敗: {message}".to_string(),
        );
        translations.insert(
            "error.mongodb_connection".to_string(),
            mongodb_connection_failed,
        );

        // SQLite连接配置类型不匹配
        let mut sqlite_config_mismatch = HashMap::new();
        sqlite_config_mismatch.insert("zh-CN".to_string(), "SQLite连接配置类型不匹配".to_string());
        sqlite_config_mismatch.insert(
            "en-US".to_string(),
            "SQLite connection configuration type mismatch".to_string(),
        );
        sqlite_config_mismatch.insert(
            "ja-JP".to_string(),
            "SQLite接続設定タイプが一致しません".to_string(),
        );
        translations.insert(
            "error.sqlite_config_mismatch".to_string(),
            sqlite_config_mismatch,
        );

        // 任务队列相关错误
        let mut task_queue_not_initialized = HashMap::new();
        task_queue_not_initialized.insert(
            "zh-CN".to_string(),
            "任务队列未初始化，请先调用 initialize_global_task_queue".to_string(),
        );
        task_queue_not_initialized.insert(
            "en-US".to_string(),
            "Task queue not initialized, please call initialize_global_task_queue first"
                .to_string(),
        );
        task_queue_not_initialized.insert("ja-JP".to_string(), "タスクキューが初期化されていません。まず initialize_global_task_queue を呼び出してください".to_string());
        translations.insert(
            "error.task_queue_not_initialized".to_string(),
            task_queue_not_initialized,
        );

        let mut task_queue_already_initialized = HashMap::new();
        task_queue_already_initialized
            .insert("zh-CN".to_string(), "任务队列已经初始化".to_string());
        task_queue_already_initialized.insert(
            "en-US".to_string(),
            "Task queue already initialized".to_string(),
        );
        task_queue_already_initialized.insert(
            "ja-JP".to_string(),
            "タスクキューは既に初期化されています".to_string(),
        );
        translations.insert(
            "error.task_queue_already_initialized".to_string(),
            task_queue_already_initialized,
        );

        // 表结构相关错误
        let mut table_no_columns = HashMap::new();
        table_no_columns.insert("zh-CN".to_string(), "表必须至少有一个列".to_string());
        table_no_columns.insert(
            "en-US".to_string(),
            "Table must have at least one column".to_string(),
        );
        table_no_columns.insert(
            "ja-JP".to_string(),
            "テーブルには少なくとも1つの列が必要です".to_string(),
        );
        translations.insert("error.table_no_columns".to_string(), table_no_columns);

        let mut column_name_duplicate = HashMap::new();
        column_name_duplicate.insert("zh-CN".to_string(), "列名 '{name}' 重复".to_string());
        column_name_duplicate.insert(
            "en-US".to_string(),
            "Column name '{name}' is duplicated".to_string(),
        );
        column_name_duplicate.insert(
            "ja-JP".to_string(),
            "列名 '{name}' が重複しています".to_string(),
        );
        translations.insert(
            "error.column_name_duplicate".to_string(),
            column_name_duplicate,
        );

        let mut index_name_duplicate = HashMap::new();
        index_name_duplicate.insert("zh-CN".to_string(), "索引名 '{name}' 重复".to_string());
        index_name_duplicate.insert(
            "en-US".to_string(),
            "Index name '{name}' is duplicated".to_string(),
        );
        index_name_duplicate.insert(
            "ja-JP".to_string(),
            "インデックス名 '{name}' が重複しています".to_string(),
        );
        translations.insert(
            "error.index_name_duplicate".to_string(),
            index_name_duplicate,
        );

        let mut index_column_not_found = HashMap::new();
        index_column_not_found.insert(
            "zh-CN".to_string(),
            "索引 '{index}' 引用的列 '{column}' 不存在".to_string(),
        );
        index_column_not_found.insert(
            "en-US".to_string(),
            "Column '{column}' referenced by index '{index}' does not exist".to_string(),
        );
        index_column_not_found.insert(
            "ja-JP".to_string(),
            "インデックス '{index}' が参照する列 '{column}' が存在しません".to_string(),
        );
        translations.insert(
            "error.index_column_not_found".to_string(),
            index_column_not_found,
        );

        let mut constraint_name_duplicate = HashMap::new();
        constraint_name_duplicate.insert("zh-CN".to_string(), "约束名 '{name}' 重复".to_string());
        constraint_name_duplicate.insert(
            "en-US".to_string(),
            "Constraint name '{name}' is duplicated".to_string(),
        );
        constraint_name_duplicate.insert(
            "ja-JP".to_string(),
            "制約名 '{name}' が重複しています".to_string(),
        );
        translations.insert(
            "error.constraint_name_duplicate".to_string(),
            constraint_name_duplicate,
        );

        let mut constraint_column_not_found = HashMap::new();
        constraint_column_not_found.insert(
            "zh-CN".to_string(),
            "约束 '{constraint}' 引用的列 '{column}' 不存在".to_string(),
        );
        constraint_column_not_found.insert(
            "en-US".to_string(),
            "Column '{column}' referenced by constraint '{constraint}' does not exist".to_string(),
        );
        constraint_column_not_found.insert(
            "ja-JP".to_string(),
            "制約 '{constraint}' が参照する列 '{column}' が存在しません".to_string(),
        );
        translations.insert(
            "error.constraint_column_not_found".to_string(),
            constraint_column_not_found,
        );

        // JSON序列化相关错误
        let mut json_serialize_failed = HashMap::new();
        json_serialize_failed.insert(
            "zh-CN".to_string(),
            "序列化为JSON字符串失败: {message}".to_string(),
        );
        json_serialize_failed.insert(
            "en-US".to_string(),
            "Failed to serialize to JSON string: {message}".to_string(),
        );
        json_serialize_failed.insert(
            "ja-JP".to_string(),
            "JSON文字列へのシリアライズ失敗: {message}".to_string(),
        );
        translations.insert("error.json_serialize".to_string(), json_serialize_failed);

        let mut json_parse_failed = HashMap::new();
        json_parse_failed.insert(
            "zh-CN".to_string(),
            "解析JSON字符串失败: {message}".to_string(),
        );
        json_parse_failed.insert(
            "en-US".to_string(),
            "Failed to parse JSON string: {message}".to_string(),
        );
        json_parse_failed.insert(
            "ja-JP".to_string(),
            "JSON文字列の解析失敗: {message}".to_string(),
        );
        translations.insert("error.json_parse".to_string(), json_parse_failed);

        let mut serialize_failed = HashMap::new();
        serialize_failed.insert("zh-CN".to_string(), "序列化失败: {message}".to_string());
        serialize_failed.insert(
            "en-US".to_string(),
            "Serialization failed: {message}".to_string(),
        );
        serialize_failed.insert(
            "ja-JP".to_string(),
            "シリアライズ失敗: {message}".to_string(),
        );
        translations.insert("error.serialize".to_string(), serialize_failed);

        // 事务操作错误
        let mut transaction_errors = HashMap::new();
        transaction_errors.insert("zh-CN".to_string(), "事务操作失败: {message}".to_string());
        transaction_errors.insert("en-US".to_string(), "Transaction operation failed: {message}".to_string());
        transaction_errors.insert("ja-JP".to_string(), "トランザクション操作が失敗しました: {message}".to_string());
        translations.insert("error.transaction".to_string(), transaction_errors);

        // 任务执行错误
        let mut task_execution_errors = HashMap::new();
        task_execution_errors.insert("zh-CN".to_string(), "任务执行失败: {message}".to_string());
        task_execution_errors.insert("en-US".to_string(), "Task execution failed: {message}".to_string());
        task_execution_errors.insert("ja-JP".to_string(), "タスク実行が失敗しました: {message}".to_string());
        translations.insert("error.task_execution".to_string(), task_execution_errors);

        // IO 操作错误
        let mut io_errors = HashMap::new();
        io_errors.insert("zh-CN".to_string(), "IO 操作失败: {message}".to_string());
        io_errors.insert("en-US".to_string(), "IO operation failed: {message}".to_string());
        io_errors.insert("ja-JP".to_string(), "IO操作が失敗しました: {message}".to_string());
        translations.insert("error.io".to_string(), io_errors);

        // JSON 处理错误
        let mut json_errors = HashMap::new();
        json_errors.insert("zh-CN".to_string(), "JSON 处理失败: {message}".to_string());
        json_errors.insert("en-US".to_string(), "JSON processing failed: {message}".to_string());
        json_errors.insert("ja-JP".to_string(), "JSON処理が失敗しました: {message}".to_string());
        translations.insert("error.json".to_string(), json_errors);

        // 通用操作错误
        let mut other_errors = HashMap::new();
        other_errors.insert("zh-CN".to_string(), "操作失败: {message}".to_string());
        other_errors.insert("en-US".to_string(), "Operation failed: {message}".to_string());
        other_errors.insert("ja-JP".to_string(), "操作が失敗しました: {message}".to_string());
        translations.insert("error.other".to_string(), other_errors);

        // 表或集合不存在错误
        let mut table_not_exist_errors = HashMap::new();
        table_not_exist_errors.insert("zh-CN".to_string(), "表或集合 '{table}' 不存在: {message}".to_string());
        table_not_exist_errors.insert("en-US".to_string(), "Table or collection '{table}' does not exist: {message}".to_string());
        table_not_exist_errors.insert("ja-JP".to_string(), "テーブルまたはコレクション '{table}' が存在しません: {message}".to_string());
        translations.insert("error.table_not_exist".to_string(), table_not_exist_errors);

        // 版本管理错误
        let mut version_errors = HashMap::new();
        version_errors.insert("zh-CN".to_string(), "版本管理操作失败: {message}".to_string());
        version_errors.insert("en-US".to_string(), "Version management operation failed: {message}".to_string());
        version_errors.insert("ja-JP".to_string(), "バージョン管理操作が失敗しました: {message}".to_string());
        translations.insert("error.version".to_string(), version_errors);

        // 数据未找到错误
        let mut not_found_errors = HashMap::new();
        not_found_errors.insert("zh-CN".to_string(), "数据未找到: {message}".to_string());
        not_found_errors.insert("en-US".to_string(), "Data not found: {message}".to_string());
        not_found_errors.insert("ja-JP".to_string(), "データが見つかりません: {message}".to_string());
        translations.insert("error.not_found".to_string(), not_found_errors);

        // ===== 字段验证消息 =====

        let mut v = |map: &mut HashMap<String, HashMap<String, String>>, key: &str, zh: &str, en: &str, ja: &str| {
            let mut m = HashMap::new();
            m.insert("zh-CN".to_string(), zh.to_string());
            m.insert("en-US".to_string(), en.to_string());
            m.insert("ja-JP".to_string(), ja.to_string());
            map.insert(key.to_string(), m);
        };

        v(&mut translations, "validation.required_empty",
            "必填字段不能为空", "Required field cannot be empty", "必須フィールドは空にできません");
        v(&mut translations, "validation.string_max_length",
            "字符串长度不能超过{max}", "String length cannot exceed {max}", "文字列の長さは{max}を超えることはできません");
        v(&mut translations, "validation.string_min_length",
            "字符串长度不能少于{min}", "String length cannot be less than {min}", "文字列の長さは{min}以上である必要があります");
        v(&mut translations, "validation.regex_invalid",
            "正则表达式无效: {error}", "Invalid regex: {error}", "正規表現が無効です: {error}");
        v(&mut translations, "validation.regex_not_match",
            "字符串不匹配正则表达式", "String does not match regex pattern", "文字列が正規表現パターンに一致しません");
        v(&mut translations, "validation.type_string",
            "字段类型不匹配，期望字符串类型", "Type mismatch, expected string type", "フィールドタイプが一致しません、文字列型を期待");
        v(&mut translations, "validation.integer_min",
            "整数值不能小于{min}", "Integer value cannot be less than {min}", "整数値は{min}より小さくすることはできません");
        v(&mut translations, "validation.integer_max",
            "整数值不能大于{max}", "Integer value cannot be greater than {max}", "整数値は{max}より大きくすることはできません");
        v(&mut translations, "validation.type_integer",
            "字段类型不匹配，期望整数类型", "Type mismatch, expected integer type", "フィールドタイプが一致しません、整数型を期待");
        v(&mut translations, "validation.float_min",
            "浮点数值不能小于{min}", "Float value cannot be less than {min}", "浮動小数点値は{min}より小さくすることはできません");
        v(&mut translations, "validation.float_max",
            "浮点数值不能大于{max}", "Float value cannot be greater than {max}", "浮動小数点値は{max}より大きくすることはできません");
        v(&mut translations, "validation.type_float",
            "字段类型不匹配，期望浮点数类型", "Type mismatch, expected float type", "フィールドタイプが一致しません、浮動小数点型を期待");
        v(&mut translations, "validation.type_boolean",
            "字段类型不匹配，期望布尔类型", "Type mismatch, expected boolean type", "フィールドタイプが一致しません、ブール型を期待");
        v(&mut translations, "validation.type_datetime",
            "字段类型不匹配，期望日期时间类型", "Type mismatch, expected datetime type", "フィールドタイプが一致しません、日時型を期待");
        v(&mut translations, "validation.rfc3339_invalid",
            "无效的RFC3339日期时间格式: '{value}' (字段: {field})",
            "Invalid RFC3339 datetime format: '{value}' (field: {field})",
            "無効なRFC3339日時形式: '{value}' (フィールド: {field})");
        v(&mut translations, "validation.datetime_invalid",
            "无效的日期时间格式，期望RFC3339或YYYY-MM-DD HH:MM:SS格式: '{value}' (字段: {field})",
            "Invalid datetime format, expected RFC3339 or YYYY-MM-DD HH:MM:SS: '{value}' (field: {field})",
            "無効な日時形式、RFC3339またはYYYY-MM-DD HH:MM:SSを期待: '{value}' (フィールド: {field})");
        v(&mut translations, "validation.type_datetime_or_string",
            "字段类型不匹配，期望日期时间类型或字符串或整数 (字段: {field})",
            "Type mismatch, expected datetime, string or integer type (field: {field})",
            "フィールドタイプが一致しません、日時・文字列・整数型を期待 (フィールド: {field})");
        v(&mut translations, "validation.timezone_offset_invalid",
            "无效的时区偏移格式: '{offset}', 期望格式: +00:00, +08:00, -05:00",
            "Invalid timezone offset format: '{offset}', expected: +00:00, +08:00, -05:00",
            "無効なタイムゾーンオフセット形式: '{offset}'、期待: +00:00, +08:00, -05:00");
        v(&mut translations, "validation.uuid_invalid",
            "无效的UUID格式: '{value}' (字段: {field})",
            "Invalid UUID format: '{value}' (field: {field})",
            "無効なUUID形式: '{value}' (フィールド: {field})");
        v(&mut translations, "validation.type_uuid",
            "字段类型不匹配，期望UUID字符串或UUID类型，实际收到: {actual} (字段: {field})",
            "Type mismatch, expected UUID string or UUID type, got: {actual} (field: {field})",
            "フィールドタイプが一致しません、UUID文字列またはUUID型を期待、実際: {actual} (フィールド: {field})");
        v(&mut translations, "validation.array_max_items",
            "数组元素数量不能超过{max}", "Array item count cannot exceed {max}", "配列要素数は{max}を超えることはできません");
        v(&mut translations, "validation.array_min_items",
            "数组元素数量不能少于{min}", "Array item count cannot be less than {min}", "配列要素数は{min}以上である必要があります");
        v(&mut translations, "validation.type_array_json_invalid",
            "JSON字符串不是有效的数组格式", "JSON string is not a valid array format", "JSON文字列は有効な配列形式ではありません");
        v(&mut translations, "validation.json_parse_failed",
            "无法解析JSON字符串", "Failed to parse JSON string", "JSON文字列の解析に失敗しました");
        v(&mut translations, "validation.type_array",
            "字段类型不匹配，期望数组类型或JSON字符串", "Type mismatch, expected array type or JSON string", "フィールドタイプが一致しません、配列型またはJSON文字列を期待");
        v(&mut translations, "validation.type_object",
            "字段类型不匹配，期望对象类型", "Type mismatch, expected object type", "フィールドタイプが一致しません、オブジェクト型を期待");
        v(&mut translations, "validation.type_reference",
            "引用字段必须是字符串ID", "Reference field must be a string ID", "参照フィールドは文字列IDである必要があります");
        v(&mut translations, "validation.type_biginteger",
            "字段类型不匹配，期望大整数类型", "Type mismatch, expected big integer type", "フィールドタイプが一致しません、BigInteger型を期待");
        v(&mut translations, "validation.type_double",
            "字段类型不匹配，期望双精度浮点数类型", "Type mismatch, expected double type", "フィールドタイプが一致しません、Double型を期待");
        v(&mut translations, "validation.type_text",
            "字段类型不匹配，期望文本类型", "Type mismatch, expected text type", "フィールドタイプが一致しません、Text型を期待");
        v(&mut translations, "validation.type_date",
            "字段类型不匹配，期望日期类型", "Type mismatch, expected date type", "フィールドタイプが一致しません、Date型を期待");
        v(&mut translations, "validation.type_time",
            "字段类型不匹配，期望时间类型", "Type mismatch, expected time type", "フィールドタイプが一致しません、Time型を期待");
        v(&mut translations, "validation.type_binary",
            "字段类型不匹配，期望二进制数据（Base64字符串）", "Type mismatch, expected binary data (Base64 string)", "フィールドタイプが一致しません、バイナリデータ（Base64文字列）を期待");
        v(&mut translations, "validation.type_decimal",
            "字段类型不匹配，期望十进制数类型", "Type mismatch, expected decimal type", "フィールドタイプが一致しません、Decimal型を期待");

        // ===== 安全验证消息 =====

        v(&mut translations, "security.field_empty",
            "字段名不能为空", "Field name cannot be empty", "フィールド名は空にできません");
        v(&mut translations, "security.field_too_long",
            "字段名长度不能超过64个字符", "Field name length cannot exceed 64 characters", "フィールド名の長さは64文字を超えることはできません");
        v(&mut translations, "security.table_empty",
            "表名不能为空", "Table name cannot be empty", "テーブル名は空にできません");
        v(&mut translations, "security.table_too_long",
            "表名长度不能超过64个字符", "Table name length cannot exceed 64 characters", "テーブル名の長さは64文字を超えることはできません");
        v(&mut translations, "security.sql_field_start_digit",
            "SQL字段名不能以数字开头", "SQL field name cannot start with a digit", "SQLフィールド名は数字で始めることはできません");
        v(&mut translations, "security.sql_field_invalid_char",
            "SQL字段名包含非法字符 '{char}' 在位置 {pos}",
            "SQL field name contains invalid character '{char}' at position {pos}",
            "SQLフィールド名に無効な文字 '{char}' が位置 {pos} に含まれています");
        v(&mut translations, "security.field_sql_keyword",
            "字段名不能使用SQL关键字: {name}",
            "Field name cannot use SQL keyword: {name}",
            "フィールド名にSQLキーワードは使用できません: {name}");
        v(&mut translations, "security.nosql_field_start_dollar",
            "NoSQL字段名不能以$开头", "NoSQL field name cannot start with $", "NoSQLフィールド名は$で始めることはできません");
        v(&mut translations, "security.nosql_field_contains_dot",
            "NoSQL字段名不能包含点号", "NoSQL field name cannot contain dots", "NoSQLフィールド名にドットを含めることはできません");
        v(&mut translations, "security.field_mongo_reserved",
            "字段名不能使用MongoDB保留字: {name}",
            "Field name cannot use MongoDB reserved word: {name}",
            "フィールド名にMongoDB予約語は使用できません: {name}");
        v(&mut translations, "security.sql_table_start_digit",
            "SQL表名不能以数字开头", "SQL table name cannot start with a digit", "SQLテーブル名は数字で始めることはできません");
        v(&mut translations, "security.sql_table_invalid_char",
            "SQL表名包含非法字符 '{char}' 在位置 {pos}",
            "SQL table name contains invalid character '{char}' at position {pos}",
            "SQLテーブル名に無効な文字 '{char}' が位置 {pos} に含まれています");
        v(&mut translations, "security.table_sql_keyword",
            "表名不能使用SQL关键字: {name}",
            "Table name cannot use SQL keyword: {name}",
            "テーブル名にSQLキーワードは使用できません: {name}");
        v(&mut translations, "security.collection_start_dollar",
            "集合名不能以$开头", "Collection name cannot start with $", "コレクション名は$で始めることはできません");
        v(&mut translations, "security.collection_contains_null",
            "集合名不能包含空字符", "Collection name cannot contain null characters", "コレクション名にnull文字を含めることはできません");
        v(&mut translations, "security.collection_start_system",
            "集合名不能以system.开头", "Collection name cannot start with system.", "コレクション名はsystem.で始めることはできません");

        // ===== 配置验证消息 =====

        v(&mut translations, "config.parse_toml_failed",
            "解析TOML配置文件失败: {message}", "Failed to parse TOML config file: {message}", "TOML設定ファイルの解析に失敗しました: {message}");
        v(&mut translations, "config.parse_json_failed",
            "解析JSON配置文件失败: {message}", "Failed to parse JSON config file: {message}", "JSON設定ファイルの解析に失敗しました: {message}");
        v(&mut translations, "config.serialize_toml_failed",
            "序列化TOML配置失败: {message}", "Failed to serialize TOML config: {message}", "TOML設定のシリアライズに失敗しました: {message}");
        v(&mut translations, "config.serialize_json_failed",
            "序列化JSON配置失败: {message}", "Failed to serialize JSON config: {message}", "JSON設定のシリアライズに失敗しました: {message}");
        v(&mut translations, "config.default_database_not_set",
            "未设置默认数据库", "Default database not set", "デフォルトデータベースが設定されていません");
        v(&mut translations, "config.default_database_not_found",
            "找不到默认数据库配置: {alias}", "Default database config not found: {alias}", "デフォルトデータベース設定が見つかりません: {alias}");
        v(&mut translations, "config.database_not_found",
            "找不到数据库配置: {alias}", "Database config not found: {alias}", "データベース設定が見つかりません: {alias}");
        v(&mut translations, "config.database_type_required",
            "数据库类型必须设置", "Database type is required", "データベースタイプは必須です");
        v(&mut translations, "config.connection_required",
            "连接配置必须设置", "Connection config is required", "接続設定は必須です");
        v(&mut translations, "config.pool_required",
            "连接池配置必须设置", "Pool config is required", "接続プール設定は必須です");
        v(&mut translations, "config.database_alias_required",
            "数据库别名必须设置", "Database alias is required", "データベースエイリアスは必須です");
        v(&mut translations, "config.id_strategy_required",
            "ID生成策略必须设置", "ID strategy is required", "ID生成戦略は必須です");
        v(&mut translations, "config.database_type_mismatch",
            "数据库类型 {db_type} 与连接配置不匹配", "Database type {db_type} does not match connection config", "データベースタイプ {db_type} が接続設定と一致しません");
        v(&mut translations, "config.at_least_one_database_required",
            "至少需要配置一个数据库", "At least one database is required", "少なくとも1つのデータベースが必要です");
        v(&mut translations, "config.app_config_required",
            "应用配置必须设置", "App config is required", "アプリ設定は必須です");
        v(&mut translations, "config.logging_config_required",
            "日志配置必须设置", "Logging config is required", "ロギング設定は必須です");
        v(&mut translations, "config.default_database_not_exist",
            "默认数据库 '{alias}' 不存在于数据库配置中", "Default database '{alias}' does not exist in database configs", "デフォルトデータベース '{alias}' がデータベース設定に存在しません");
        v(&mut translations, "config.min_connections_required",
            "最小连接数必须设置", "Min connections is required", "最小接続数は必須です");
        v(&mut translations, "config.max_connections_required",
            "最大连接数必须设置", "Max connections is required", "最大接続数は必須です");
        v(&mut translations, "config.connection_timeout_required",
            "连接超时时间必须设置", "Connection timeout is required", "接続タイムアウトは必須です");
        v(&mut translations, "config.idle_timeout_required",
            "空闲连接超时时间必须设置", "Idle timeout is required", "アイドルタイムアウトは必須です");
        v(&mut translations, "config.max_lifetime_required",
            "连接最大生存时间必须设置", "Max lifetime is required", "最大寿命は必須です");
        v(&mut translations, "config.max_retries_required",
            "最大重试次数必须设置", "Max retries is required", "最大リトライ回数は必須です");
        v(&mut translations, "config.retry_interval_required",
            "重试间隔必须设置", "Retry interval is required", "リトライ間隔は必須です");
        v(&mut translations, "config.keepalive_interval_required",
            "保活检测间隔必须设置", "Keepalive interval is required", "キープアライブ間隔は必須です");
        v(&mut translations, "config.health_check_timeout_required",
            "健康检查超时时间必须设置", "Health check timeout is required", "ヘルスチェックタイムアウトは必須です");
        v(&mut translations, "config.min_exceeds_max_connections",
            "最小连接数不能大于最大连接数", "Min connections cannot exceed max connections", "最小接続数は最大接続数を超えることはできません");
        v(&mut translations, "config.connection_timeout_zero",
            "连接超时时间不能为零", "Connection timeout cannot be zero", "接続タイムアウトはゼロにできません");
        v(&mut translations, "config.idle_timeout_zero",
            "空闲连接超时时间不能为零", "Idle timeout cannot be zero", "アイドルタイムアウトはゼロにできません");
        v(&mut translations, "config.max_lifetime_zero",
            "连接最大生存时间不能为零", "Max lifetime cannot be zero", "最大寿命はゼロにできません");
        v(&mut translations, "config.app_name_required",
            "应用名称必须设置", "App name is required", "アプリ名は必須です");
        v(&mut translations, "config.app_version_required",
            "应用版本必须设置", "App version is required", "アプリバージョンは必須です");
        v(&mut translations, "config.environment_required",
            "环境类型必须设置", "Environment is required", "環境タイプは必須です");
        v(&mut translations, "config.debug_mode_required",
            "调试模式必须设置", "Debug mode is required", "デバッグモードは必須です");
        v(&mut translations, "config.work_dir_required",
            "工作目录必须设置", "Working directory is required", "作業ディレクトリは必須です");
        v(&mut translations, "config.log_level_required",
            "日志级别必须设置", "Log level is required", "ログレベルは必須です");
        v(&mut translations, "config.console_output_required",
            "控制台输出选项必须设置", "Console output option is required", "コンソール出力オプションは必須です");
        v(&mut translations, "config.max_file_size_required",
            "日志文件最大大小必须设置", "Max log file size is required", "ログファイルの最大サイズは必須です");
        v(&mut translations, "config.max_files_required",
            "保留日志文件数量必须设置", "Max log files is required", "保持ログファイル数は必須です");
        v(&mut translations, "config.structured_logging_required",
            "结构化日志选项必须设置", "Structured logging option is required", "構造化ロギングオプションは必須です");
        v(&mut translations, "config.max_file_size_zero",
            "日志文件最大大小不能为零", "Max log file size cannot be zero", "ログファイルの最大サイズはゼロにできません");
        v(&mut translations, "config.max_files_zero",
            "保留日志文件数量不能为零", "Max log files cannot be zero", "保持ログファイル数はゼロにできません");

        // ===== ODM 层消息 =====

        v(&mut translations, "odm.task_stopped",
            "ODM后台任务已停止", "ODM background task has stopped", "ODMバックグラウンドタスクが停止しました");
        v(&mut translations, "odm.request_failed",
            "ODM请求处理失败", "ODM request processing failed", "ODMリクエスト処理が失敗しました");
        v(&mut translations, "odm.channel_closed",
            "连接池操作通道已关闭", "Connection pool operation channel closed", "接続プール操作チャンネルが閉じました");
        v(&mut translations, "odm.response_timeout",
            "等待连接池响应超时", "Waiting for connection pool response timed out", "接続プール応答タイムアウト");
        v(&mut translations, "odm.operation_timeout",
            "等待数据库操作结果超时", "Waiting for database operation result timed out", "データベース操作結果のタイムアウト");
        v(&mut translations, "odm.create_missing_id",
            "创建操作返回的数据中缺少id字段", "Created data is missing the id field", "作成結果のデータにidフィールドがありません");

        // ===== 连接池层消息 =====

        v(&mut translations, "pool.unsupported_db_type",
            "不支持的数据库类型（可能需要启用相应的feature）",
            "Unsupported database type (may need to enable the corresponding feature)",
            "サポートされていないデータベースタイプ（対応するフィーチャを有効にしてください）");
        v(&mut translations, "pool.operation_not_implemented",
            "操作发送未实现", "Operation send not implemented", "操作送信が実装されていません");
        v(&mut translations, "pool.send_operation_failed",
            "发送操作失败", "Failed to send operation", "操作の送信に失敗しました");
        v(&mut translations, "pool.receive_response_failed",
            "接收响应失败", "Failed to receive response", "応答の受信に失敗しました");

        // ===== MongoDB 适配器层消息 =====

        v(&mut translations, "adapter.mongo.connection_mismatch",
            "连接类型不匹配，期望MongoDB连接", "Connection type mismatch, expected MongoDB connection", "接続タイプが一致しません、MongoDB接続を期待");
        v(&mut translations, "adapter.mongo.convert_id_bson_failed",
            "转换ID为BSON失败: {error}", "Failed to convert ID to BSON: {error}", "IDのBSON変換に失敗しました: {error}");
        v(&mut translations, "adapter.mongo.collection_not_exist",
            "MongoDB集合 '{collection}' 不存在", "MongoDB collection '{collection}' does not exist", "MongoDBコレクション '{collection}' が存在しません");
        v(&mut translations, "adapter.mongo.collection_empty",
            "MongoDB集合 '{collection}' 不存在或为空", "MongoDB collection '{collection}' does not exist or is empty", "MongoDBコレクション '{collection}' が存在しないか空です");
        v(&mut translations, "adapter.mongo.query_failed",
            "MongoDB查询失败: {error}", "MongoDB query failed: {error}", "MongoDBクエリー: {error}");
        v(&mut translations, "adapter.mongo.combined_query_failed",
            "MongoDB条件组合查询失败: {error}", "MongoDB combined query failed: {error}", "MongoDB組み合わせクエリー: {error}");
        v(&mut translations, "adapter.mongo.cursor_failed",
            "MongoDB游标遍历失败: {error}", "MongoDB cursor traversal failed: {error}", "MongoDBカーソルトラバーサル失敗: {error}");
        v(&mut translations, "adapter.mongo.deserialize_failed",
            "MongoDB文档反序列化失败: {error}", "MongoDB document deserialization failed: {error}", "MongoDBドキュメントの逆シリアライズ失敗: {error}");
        v(&mut translations, "adapter.mongo.count_failed",
            "MongoDB计数失败: {error}", "MongoDB count failed: {error}", "MongoDBカウント失敗: {error}");
        v(&mut translations, "adapter.mongo.combined_count_failed",
            "MongoDB条件组合计数失败: {error}", "MongoDB combined count failed: {error}", "MongoDB組み合わせカウント失敗: {error}");
        v(&mut translations, "adapter.mongo.convert_dv_bson_failed",
            "转换DataValue为BSON失败: {error}", "Failed to convert DataValue to BSON: {error}", "DataValueのBSON変換に失敗しました: {error}");
        v(&mut translations, "adapter.mongo.insert_failed",
            "MongoDB插入失败: {error}", "MongoDB insert failed: {error}", "MongoDB挿入失敗: {error}");
        v(&mut translations, "adapter.mongo.update_failed",
            "MongoDB更新失败: {error}", "MongoDB update failed: {error}", "MongoDB更新失敗: {error}");
        v(&mut translations, "adapter.mongo.delete_failed",
            "MongoDB删除失败: {error}", "MongoDB delete failed: {error}", "MongoDB削除失敗: {error}");
        v(&mut translations, "adapter.mongo.update_empty",
            "更新操作不能为空", "Update operation cannot be empty", "更新操作は空にできません");
        v(&mut translations, "adapter.mongo.stored_proc_validate_failed",
            "存储过程配置验证失败: {error}", "Stored procedure config validation failed: {error}", "ストアドプロシージャ設定検証失敗: {error}");
        v(&mut translations, "adapter.mongo.stored_proc_not_exist",
            "存储过程 '{name}' 不存在", "Stored procedure '{name}' does not exist", "ストアドプロシージャ '{name}' が存在しません");
        v(&mut translations, "adapter.mongo.parse_pipeline_failed",
            "解析聚合管道模板失败: {error}", "Failed to parse aggregation pipeline template: {error}", "集約パイプラインテンプレートの解析に失敗しました: {error}");
        v(&mut translations, "adapter.mongo.pipeline_missing_collection",
            "聚合管道模板缺少collection字段", "Aggregation pipeline template missing collection field", "集約パイプラインテンプレートにcollectionフィールドがありません");
        v(&mut translations, "adapter.mongo.pipeline_missing_pipeline",
            "聚合管道模板缺少pipeline字段", "Aggregation pipeline template missing pipeline field", "集約パイプラインテンプレートにpipelineフィールドがありません");
        v(&mut translations, "adapter.mongo.need_primary_collection",
            "至少需要一个依赖集合作为主集合", "At least one dependent collection is required as primary", "少なくとも1つの依存コレクションをプライマリとして必要です");
        v(&mut translations, "adapter.mongo.serialize_pipeline_failed",
            "序列化MongoDB聚合管道失败: {error}", "Failed to serialize MongoDB aggregation pipeline: {error}", "MongoDB集約パイプラインのシリアライズに失敗しました: {error}");
        v(&mut translations, "adapter.mongo.pipeline_serialize_failed",
            "聚合管道序列化失败: {error}", "Pipeline serialization failed: {error}", "パイプラインシリアライズ失敗: {error}");
        v(&mut translations, "adapter.mongo.aggregate_query_failed",
            "MongoDB聚合查询失败: {error}", "MongoDB aggregation query failed: {error}", "MongoDB集約クエリー: {error}");
        v(&mut translations, "adapter.mongo.aggregate_cursor_failed",
            "MongoDB聚合游标遍历失败: {error}", "MongoDB aggregation cursor traversal failed: {error}", "MongoDB集約カーソルトラバースル失敗: {error}");
        v(&mut translations, "adapter.mongo.aggregate_deserialize_failed",
            "MongoDB聚合文档反序列化失败: {error}", "MongoDB aggregation document deserialization failed: {error}", "MongoDB集約ドキュメントの逆シリアライズ失敗: {error}");
        v(&mut translations, "adapter.mongo.invalid_json",
            "无效的JSON格式: {error}", "Invalid JSON format: {error}", "無効なJSON形式: {error}");
        v(&mut translations, "adapter.mongo.startswith_string_only",
            "StartsWith操作符只支持字符串类型", "StartsWith operator only supports string type", "StartsWith演算子は文字列型のみサポート");
        v(&mut translations, "adapter.mongo.endswith_string_only",
            "EndsWith操作符只支持字符串类型", "EndsWith operator only supports string type", "EndsWith演算子は文字列型のみサポート");
        v(&mut translations, "adapter.mongo.array_in_unsupported_type",
            "Array字段的IN操作只支持String、Int、Float、Uuid类型，不支持: {type}",
            "Array field IN operation only supports String, Int, Float, Uuid types, unsupported: {type}",
            "ArrayフィールドのIN操作はString、Int、Float、Uuid型のみサポート、非対応: {type}");
        v(&mut translations, "adapter.mongo.array_notin_unsupported_type",
            "Array字段的NOT IN操作只支持String、Int、Float、Uuid类型，不支持: {type}",
            "Array field NOT IN operation only supports String, Int, Float, Uuid types, unsupported: {type}",
            "ArrayフィールドのNOT IN操作はString、Int、Float、Uuid型のみサポート、非対応: {type}");
        v(&mut translations, "adapter.mongo.regex_string_only",
            "Regex操作符只支持字符串类型", "Regex operator only supports string type", "Regex演算子は文字列型のみサポート");
        v(&mut translations, "adapter.mongo.field_type_unknown",
            "无法确定字段 '{field}' 的类型，请确保已正确注册模型元数据 (alias={alias})",
            "Cannot determine type of field '{field}', ensure model metadata is registered correctly (alias={alias})",
            "フィールド '{field}' の型を特定できません、モデルメタデータが正しく登録されていることを確認してください (alias={alias})");
        v(&mut translations, "adapter.mongo.contains_string_only",
            "字符串字段的Contains操作符只支持字符串值", "String field Contains operator only supports string values", "文字列フィールドのContains演算子は文字列値のみサポート");
        v(&mut translations, "adapter.mongo.contains_supported_types",
            "Contains操作符只支持字符串、Array和JSON字段", "Contains operator only supports string, Array and JSON fields", "Contains演算子は文字列、Array、JSONフィールドのみサポート");
        v(&mut translations, "adapter.mongo.convert_update_bson_failed",
            "转换更新数据为BSON失败: {error}", "Failed to convert update data to BSON: {error}", "更新データのBSON変換に失敗しました: {error}");
        v(&mut translations, "adapter.mongo.json_invalid_data",
            "Json字段类型接收到非对象/数组数据: {data}, 这是内部错误，应该在验证阶段被拒绝",
            "Json field received non-object/array data: {data}, this is an internal error that should have been rejected during validation",
            "Jsonフィールドにオブジェクト/配列以外のデータ: {data}、これは検証段階で拒否されるべき内部エラーです");
        v(&mut translations, "adapter.mongo.decrement_numeric_only",
            "Decrement操作只支持数值类型", "Decrement operation only supports numeric types", "デクリメント操作は数値型のみサポート");
        v(&mut translations, "adapter.mongo.increment_numeric_only",
            "Increment操作只支持数值类型", "Increment operation only supports numeric types", "インクリメント操作は数値型のみサポート");
        v(&mut translations, "adapter.mongo.multiply_numeric_only",
            "Multiply操作只支持数值类型", "Multiply operation only supports numeric types", "乗算操作は数値型のみサポート");
        v(&mut translations, "adapter.mongo.divide_numeric_only",
            "Divide操作只支持数值类型", "Divide operation only supports numeric types", "除算操作は数値型のみサポート");
        v(&mut translations, "adapter.mongo.percent_increase_numeric_only",
            "PercentIncrease操作只支持数值类型", "PercentIncrease operation only supports numeric types", "パーセント増加操作は数値型のみサポート");
        v(&mut translations, "adapter.mongo.percent_decrease_numeric_only",
            "PercentDecrease操作只支持数值类型", "PercentDecrease operation only supports numeric types", "パーセント減少操作は数値型のみサポート");
        v(&mut translations, "adapter.mongo.id_strategy_requires_id",
            "使用{strategy:?}策略时必须提供ID字段", "ID field is required when using {strategy:?} strategy", "{strategy:?}戦略使用時はIDフィールドが必須です");
        v(&mut translations, "adapter.mongo.collection_no_metadata",
            "集合 '{collection}' 不存在，且没有预定义的模型元数据。MongoDB使用无模式设计，但建议先定义模型。",
            "Collection '{collection}' does not exist and has no predefined model metadata. MongoDB uses schemaless design, but defining a model is recommended.",
            "コレクション '{collection}' が存在せず、事前定義されたモデルメタデータもありません。MongoDBはスキーマレス設計ですが、モデルの定義を推奨します。");
        v(&mut translations, "adapter.mongo.convert_to_bson_failed",
            "转换{operation}为BSON失败: {error}", "Failed to convert {operation} to BSON: {error}", "{operation}のBSON変換に失敗しました: {error}");
        v(&mut translations, "adapter.mongo.create_collection_failed",
            "创建MongoDB集合失败: {error}", "Failed to create MongoDB collection: {error}", "MongoDBコレクションの作成に失敗しました: {error}");
        v(&mut translations, "adapter.mongo.create_index_failed",
            "创建MongoDB索引失败: {error}", "Failed to create MongoDB index: {error}", "MongoDBインデックスの作成に失敗しました: {error}");
        v(&mut translations, "adapter.mongo.check_collection_failed",
            "检查MongoDB集合是否存在失败: {error}", "Failed to check MongoDB collection existence: {error}", "MongoDBコレクション存在確認に失敗しました: {error}");
        v(&mut translations, "adapter.mongo.drop_collection_failed",
            "删除MongoDB集合失败: {error}", "Failed to drop MongoDB collection: {error}", "MongoDBコレクションの削除に失敗しました: {error}");
        v(&mut translations, "adapter.mongo.query_version_failed",
            "查询MongoDB版本失败: {error}", "Failed to query MongoDB version: {error}", "MongoDBバージョンクエリー: {error}");
        v(&mut translations, "adapter.mongo.version_format_invalid",
            "MongoDB版本信息格式错误", "MongoDB version info format error", "MongoDBバージョン情報の形式エラー");
        v(&mut translations, "adapter.mongo.version_no_info",
            "MongoDB版本查询结果中没有版本信息", "No version info in MongoDB version query result", "MongoDBバージョンクエリーの結果にバージョン情報がありません");

        // 注册所有翻译
        register_translations(translations);
    }
}

/// 重新导出rat_embed_lang的核心函数
///
/// 这些函数会自动触发i18n初始化（懒加载）
pub use rat_embed_lang::{current_language, set_language, t, tf};
