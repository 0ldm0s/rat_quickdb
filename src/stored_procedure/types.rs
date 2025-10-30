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
    /// MongoDB聚合管道操作（可选）
    pub mongo_pipeline: Option<Vec<MongoAggregationOperation>>,
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

/// MongoDB聚合管道操作类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MongoAggregationOperation {
    /// 字段投影
    Project { fields: HashMap<String, MongoFieldExpression> },
    /// 匹配阶段（类似WHERE）
    Match { conditions: Vec<MongoCondition> },
    /// Lookup连接（类似JOIN）
    Lookup {
        from: String,
        local_field: String,
        foreign_field: String,
        as_field: String,
    },
    /// 展开数组
    Unwind { field: String },
    /// 分组
    Group {
        id: MongoGroupKey,
        accumulators: HashMap<String, MongoAccumulator>,
    },
    /// 排序
    Sort { fields: Vec<(String, SortDirection)> },
    /// 限制数量
    Limit { count: i64 },
    /// 跳过数量
    Skip { count: i64 },
    /// 添加字段
    AddFields { fields: HashMap<String, MongoFieldExpression> },
    /// 计数
    Count { field: String },
    /// 添加占位符（用于动态替换）
    Placeholder { placeholder_type: String },
}

/// MongoDB字段表达式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MongoFieldExpression {
    /// 直接字段引用
    Field(String),
    /// 常量值
    Constant(DataValue),
    /// 聚合表达式
    Aggregate(MongoAggregateExpression),
}

/// MongoDB聚合表达式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MongoAggregateExpression {
    /// 数组大小
    Size { field: String },
    /// 求和
    Sum { field: String },
    /// 平均值
    Avg { field: String },
    /// 最大值
    Max { field: String },
    /// 最小值
    Min { field: String },
    /// 如果为空则使用默认值
    IfNull { field: String, default: Box<MongoFieldExpression> },
    /// 条件表达式
    Condition {
        if_condition: Box<MongoCondition>,
        then_expr: Box<MongoFieldExpression>,
        else_expr: Box<MongoFieldExpression>,
    },
}

/// MongoDB分组键
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MongoGroupKey {
    /// 按字段分组
    Field(String),
    /// 按常量分组（将所有文档分为一组）
    Null,
    /// 按多个字段分组
    Multiple(Vec<String>),
}

/// MongoDB累加器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MongoAccumulator {
    /// 计数
    Count,
    /// 求和
    Sum { field: String },
    /// 平均值
    Avg { field: String },
    /// 最大值
    Max { field: String },
    /// 最小值
    Min { field: String },
    /// 推入数组
    Push { field: String },
    /// 添加到集合
    AddToSet { field: String },
}

/// MongoDB条件表达式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MongoCondition {
    /// 等于
    Eq { field: String, value: DataValue },
    /// 不等于
    Ne { field: String, value: DataValue },
    /// 大于
    Gt { field: String, value: DataValue },
    /// 大于等于
    Gte { field: String, value: DataValue },
    /// 小于
    Lt { field: String, value: DataValue },
    /// 小于等于
    Lte { field: String, value: DataValue },
    /// 包含（在数组中）
    In { field: String, values: Vec<DataValue> },
    /// 不包含
    Nin { field: String, values: Vec<DataValue> },
    /// AND条件
    And { conditions: Vec<MongoCondition> },
    /// OR条件
    Or { conditions: Vec<MongoCondition> },
    /// 存在字段
    Exists { field: String, exists: bool },
    /// 正则表达式匹配
    Regex { field: String, pattern: String },
}