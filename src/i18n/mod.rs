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

        // 注册所有翻译
        register_translations(translations);
    }
}

/// 重新导出rat_embed_lang的核心函数
///
/// 这些函数会自动触发i18n初始化（懒加载）
pub use rat_embed_lang::{current_language, set_language, t, tf};
