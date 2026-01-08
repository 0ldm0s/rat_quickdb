use crate::types::data_value::DataValue;
use serde::{Deserialize, Serialize};

/// 查询条件（简化版）
///
/// 不包含配置选项的简化查询条件，使用默认配置（大小写敏感）。
/// 可以通过 `.into()` 自动转换为 `QueryConditionWithConfig`。
#[derive(Debug, Clone, PartialEq)]
pub struct QueryCondition {
    /// 字段名
    pub field: String,
    /// 操作符
    pub operator: QueryOperator,
    /// 值
    pub value: DataValue,
}

impl From<QueryCondition> for QueryConditionWithConfig {
    fn from(condition: QueryCondition) -> Self {
        QueryConditionWithConfig {
            field: condition.field,
            operator: condition.operator,
            value: condition.value,
            case_insensitive: false,  // 默认大小写敏感
        }
    }
}

/// 查询条件（带配置）
///
/// 包含所有配置选项的完整查询条件，支持大小写不敏感查询等功能。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryConditionWithConfig {
    /// 字段名
    pub field: String,
    /// 操作符
    pub operator: QueryOperator,
    /// 值
    pub value: DataValue,
    /// 是否大小写不敏感（仅对字符串操作符有效）
    #[serde(default)]
    pub case_insensitive: bool,
}

/// 逻辑操作符
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LogicalOperator {
    /// AND 逻辑
    And,
    /// OR 逻辑
    Or,
}

/// 查询条件组合（简化版）
///
/// 使用简化的 QueryCondition，适用于不需要额外配置的场景
#[derive(Debug, Clone, PartialEq)]
pub enum QueryConditionGroup {
    /// 单个条件
    Single(QueryCondition),
    /// 条件组合
    Group {
        /// 逻辑操作符
        operator: LogicalOperator,
        /// 子条件列表
        conditions: Vec<QueryConditionGroup>,
    },
}

// 支持从 QueryCondition 自动转换
impl From<QueryCondition> for QueryConditionGroup {
    fn from(condition: QueryCondition) -> Self {
        QueryConditionGroup::Single(condition)
    }
}

/// 查询条件组合（完整版）
///
/// 使用 QueryConditionWithConfig，支持大小写不敏感等高级配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum QueryConditionGroupWithConfig {
    /// 单个条件
    Single(QueryConditionWithConfig),
    /// 条件组合
    GroupWithConfig {
        /// 逻辑操作符
        operator: LogicalOperator,
        /// 子条件列表
        conditions: Vec<QueryConditionGroupWithConfig>,
    },
}

// 支持从 QueryCondition 自动转换
impl From<QueryCondition> for QueryConditionGroupWithConfig {
    fn from(condition: QueryCondition) -> Self {
        QueryConditionGroupWithConfig::Single(condition.into())
    }
}

// 支持从 QueryConditionWithConfig 自动转换
impl From<QueryConditionWithConfig> for QueryConditionGroupWithConfig {
    fn from(condition: QueryConditionWithConfig) -> Self {
        QueryConditionGroupWithConfig::Single(condition)
    }
}

// 支持从 QueryConditionGroup 自动转换（递归转换）
impl From<QueryConditionGroup> for QueryConditionGroupWithConfig {
    fn from(group: QueryConditionGroup) -> Self {
        match group {
            QueryConditionGroup::Single(condition) => {
                QueryConditionGroupWithConfig::Single(condition.into())
            }
            QueryConditionGroup::Group { operator, conditions } => {
                QueryConditionGroupWithConfig::GroupWithConfig {
                    operator,
                    conditions: conditions.into_iter().map(|g| g.into()).collect(),
                }
            }
        }
    }
}

/// 查询操作符
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum QueryOperator {
    /// 等于
    Eq,
    /// 不等于
    Ne,
    /// 大于
    Gt,
    /// 大于等于
    Gte,
    /// 小于
    Lt,
    /// 小于等于
    Lte,
    /// 包含（字符串）
    Contains,
    /// JSON包含（JSON字段内容搜索）
    JsonContains,
    /// 开始于（字符串）
    StartsWith,
    /// 结束于（字符串）
    EndsWith,
    /// 在列表中
    In,
    /// 不在列表中
    NotIn,
    /// 正则表达式匹配
    Regex,
    /// 存在（字段存在）
    Exists,
    /// 为空
    IsNull,
    /// 不为空
    IsNotNull,
}

/// 排序配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortConfig {
    /// 字段名
    pub field: String,
    /// 排序方向
    pub direction: SortDirection,
}

/// 排序方向
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SortDirection {
    /// 升序
    Asc,
    /// 降序
    Desc,
}

/// 分页配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationConfig {
    /// 跳过的记录数
    pub skip: u64,
    /// 限制返回的记录数
    pub limit: u64,
}

/// 查询选项
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QueryOptions {
    /// 查询条件
    pub conditions: Vec<QueryConditionWithConfig>,
    /// 排序配置
    pub sort: Vec<SortConfig>,
    /// 分页配置
    pub pagination: Option<PaginationConfig>,
    /// 选择的字段（空表示选择所有字段）
    pub fields: Vec<String>,
}

impl QueryOptions {
    /// 创建新的查询选项
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置条件
    pub fn with_conditions(mut self, conditions: Vec<QueryConditionWithConfig>) -> Self {
        self.conditions = conditions;
        self
    }

    /// 设置排序
    pub fn with_sort(mut self, sort: Vec<SortConfig>) -> Self {
        self.sort = sort;
        self
    }

    /// 设置分页
    pub fn with_pagination(mut self, pagination: PaginationConfig) -> Self {
        self.pagination = Some(pagination);
        self
    }

    /// 设置字段选择
    pub fn with_fields(mut self, fields: Vec<String>) -> Self {
        self.fields = fields;
        self
    }
}
