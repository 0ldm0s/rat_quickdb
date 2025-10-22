//! 数据库特定的JSON处理器模块
//!
//! 为不同数据库类型提供专门的JSON到DataValue转换逻辑

pub mod postgres;
pub mod mysql;
pub mod sqlite;
pub mod mongodb;

use crate::types::DatabaseType;
use std::collections::HashMap;
use serde_json::Value;

/// 数据库JSON处理器的公共接口
pub trait DatabaseJsonProcessor {
    /// 将JSON对象转换为DataValue映射
    fn convert_json_to_data_map(
        &self,
        json_obj: &serde_json::Map<String, Value>,
        table_name: &str,
        db_alias: &str,
    ) -> Result<HashMap<String, crate::types::DataValue>, String>;

    /// 获取支持的数据库类型
    fn get_database_type(&self) -> DatabaseType;
}

/// 工厂函数：根据数据库类型创建对应的处理器
pub fn create_database_json_processor(db_type: &DatabaseType) -> Box<dyn DatabaseJsonProcessor> {
    match db_type {
        DatabaseType::PostgreSQL => Box::new(postgres::PostgresJsonProcessor::new()),
        DatabaseType::MySQL => Box::new(mysql::MysqlJsonProcessor::new()),
        DatabaseType::SQLite => Box::new(sqlite::SqliteJsonProcessor::new()),
        DatabaseType::MongoDB => Box::new(mongodb::MongoJsonProcessor::new()),
    }
}