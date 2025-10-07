//! rat_quickdb - 跨数据库ODM库
//! 
//! 提供统一的数据库操作接口，支持SQLite、PostgreSQL、MySQL和MongoDB
//! 通过连接池和无锁队列实现高性能的数据后端无关性

// 导出所有公共模块
pub mod error;
pub mod types;
pub mod pool;
pub mod manager;
pub mod odm;
pub mod model;
pub mod serializer;
pub mod adapter;
pub mod config;
pub mod task_queue;
pub mod table;

// 条件编译的模块
pub mod cache;
pub mod id_generator;

// Python API 模块（仅在启用 python-bindings 特性时编译）
#[cfg(feature = "python-bindings")]
pub mod python_api;

// 重新导出常用类型和函数
pub use error::{QuickDbError, QuickDbResult};
pub use types::*;
pub use pool::DatabaseConnection;
pub use manager::{
    add_database, remove_database, get_aliases, set_default_alias, health_check, shutdown,
    get_id_generator, get_mongo_auto_increment_generator, drop_table
};

pub use manager::{
    get_cache_manager, get_cache_stats, clear_cache, clear_all_caches
};
pub use odm::{AsyncOdmManager, get_odm_manager, get_odm_manager_mut, OdmOperations};
pub use model::{
    Model, ModelOperations, ModelManager, FieldType, FieldDefinition, ModelMeta, IndexDefinition,
    array_field, list_field, string_field, integer_field, float_field, boolean_field,
    datetime_field, uuid_field, json_field, dict_field, reference_field
};
pub use serializer::{DataSerializer, SerializerConfig, OutputFormat, SerializationResult};
pub use adapter::{DatabaseAdapter, create_adapter};
pub use config::{
    GlobalConfig, GlobalConfigBuilder, DatabaseConfigBuilder, PoolConfigBuilder,
    AppConfig, AppConfigBuilder, LoggingConfig, LoggingConfigBuilder,
    Environment, LogLevel, sqlite_config, postgres_config, mysql_config,
    mongodb_config, mongodb_config_with_builder
};
pub use task_queue::{
    TaskQueueManager, get_global_task_queue, initialize_global_task_queue, 
    shutdown_global_task_queue
};
pub use table::{TableManager, TableSchema, ColumnDefinition, ColumnType, IndexType};

// 条件导出缓存相关类型
pub use cache::{CacheManager, CacheStats};

// 导出ID生成器相关类型
pub use id_generator::{IdGenerator, MongoAutoIncrementGenerator};

// 重新导出 ODM 操作函数（底层 DataValue API）
// 注意：这些函数返回 DataValue，主要用于内部使用
// 对于应用开发，推荐使用 ModelManager<T> 获取结构化数据
pub use odm::{create, find_by_id, find, update, update_by_id, delete, delete_by_id, count, exists, get_server_version};

// Python API 导出（仅在启用 python-bindings 特性时）
// 注意：Python绑定相关的导出已移至专门的Python绑定库中

// 日志系统导入
use rat_logger::{info, debug};


/// 初始化rat_quickdb库
///
/// 这个函数会初始化rat_quickdb库
///
/// 注意：日志系统由调用者自行初始化，本库不再自动初始化日志
pub fn init() {
    // 库的基本初始化逻辑
    // 日志系统由调用者负责初始化
}

/// 生成ObjectId字符串
///
/// 生成类似MongoDB ObjectId的24位十六进制字符串
/// 格式：时间戳(4字节) + 机器ID(3字节) + 进程ID(2字节) + 计数器(3字节)
///
/// # 返回值
/// 返回24位十六进制字符串
pub fn generate_object_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    // 获取当前时间戳（秒）
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // 获取计数器值
    let counter = COUNTER.fetch_add(1, Ordering::SeqCst);

    // 简单的机器ID（基于进程ID）
    let machine_id = std::process::id() % 0xFFFFFF;

    // 格式化为24位十六进制字符串
    format!(
        "{:08x}{:06x}{:04x}{:06x}",
        timestamp,
        machine_id,
        (machine_id >> 8) & 0xFFFF,
        counter % 0xFFFFFF
    )
}

/// 根据模型元数据处理DataValue中的字段类型转换
///
/// 这是一个通用的数据后处理工具，用于处理数据库适配器返回的原始数据
/// 根据模型元数据将字符串形式的复杂数据（JSON、数组等）转换为正确的DataValue类型
///
/// # 参数
/// * `data_map` - 从数据库读取的原始数据映射
/// * `fields` - 模型的字段定义
///
/// # 返回值
/// 返回处理后的数据映射，其中复杂字段被正确转换
pub fn process_data_fields_from_metadata(
    mut data_map: std::collections::HashMap<String, DataValue>,
    fields: &std::collections::HashMap<String, crate::model::FieldDefinition>,
) -> std::collections::HashMap<String, DataValue> {
    for (field_name, field_def) in fields {
        if let Some(current_value) = data_map.get(field_name) {
            let converted_value = match current_value {
                // 处理字符串类型的JSON数据
                DataValue::String(json_str) if json_str.starts_with('[') || json_str.starts_with('{') => {
                    // 尝试解析JSON
                    match serde_json::from_str::<serde_json::Value>(json_str.as_str()) {
                        Ok(json_value) => {
                            let converted = crate::types::json_value_to_data_value(json_value);
                            debug!("字段 {} JSON转换成功: {:?} -> {:?}", field_name, json_str, converted);
                            Some(converted)
                        }
                        Err(e) => {
                            debug!("字段 {} JSON解析失败，保持原字符串: {} (错误: {})", field_name, json_str, e);
                            None // 解析失败，保持原字符串值
                        }
                    }
                },
                // 处理布尔字段的整数转换（SQLite等数据库的兼容性）
                DataValue::Int(int_val) if matches!(field_def.field_type, crate::model::FieldType::Boolean) => {
                    if *int_val == 0 || *int_val == 1 {
                        debug!("字段 {} 整数转布尔: {} -> {}", field_name, int_val, *int_val == 1);
                        Some(DataValue::Bool(*int_val == 1))
                    } else {
                        debug!("字段 {} 整数值超出布尔范围: {}，保持原值", field_name, int_val);
                        None
                    }
                },
                _ => None, // 其他类型保持不变
            };

            // 如果有转换结果，更新数据映射
            if let Some(converted) = converted_value {
                data_map.insert(field_name.clone(), converted);
            }
        }
    }
    data_map
}

/// 初始化rat_quickdb库
///
/// 注意：此函数已弃用，请使用init()
/// 日志系统由调用者自行初始化
#[deprecated(since = "0.2.0", note = "请使用init()，日志系统由调用者自行初始化")]
pub fn init_with_log_level(_level: rat_logger::LevelFilter) {
    init();
}

/// 库版本信息
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// 库名称
pub const NAME: &str = env!("CARGO_PKG_NAME");

/// 获取库信息
pub fn get_info() -> String {
    format!("{} v{}", NAME, VERSION)
}
