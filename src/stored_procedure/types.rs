//! 存储过程类型定义

use std::collections::HashMap;
use crate::types::*;
use serde::{Deserialize, Serialize};

/// JOIN类型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Full,
}

/// JOIN关系定义（结构化，避免SQL注入）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinRelation {
    /// 要连接的表名
    pub table: String,
    /// 本表的连接字段
    pub local_field: String,
    /// 外表的连接字段
    pub foreign_field: String,
    /// JOIN类型
    pub join_type: JoinType,
}

/// 存储过程配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredProcedureConfig {
    /// 数据库别名
    pub database: String,
    /// 依赖的模型元数据
    pub dependencies: Vec<crate::model::ModelMeta>,
    /// JOIN关系定义
    pub joins: Vec<JoinRelation>,
    /// 字段映射：字段名 -> 表达式
    pub fields: HashMap<String, String>,
    /// 存储过程名称
    pub procedure_name: String,
}

/// 存储过程信息（存储在适配器中）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredProcedureInfo {
    /// 存储过程配置
    pub config: StoredProcedureConfig,
    /// 生成的SQL/NoSQL模板
    pub template: String,
    /// 数据库类型
    pub db_type: String,
    /// 创建时间
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// 存储过程创建结果
#[derive(Debug, Clone)]
pub struct StoredProcedureCreateResult {
    /// 是否成功创建
    pub success: bool,
    /// 存储过程名称
    pub procedure_name: String,
    /// 错误信息（如果有）
    pub error: Option<String>,
}

/// 存储过程执行结果
pub type StoredProcedureQueryResult = Vec<std::collections::HashMap<String, crate::types::DataValue>>;