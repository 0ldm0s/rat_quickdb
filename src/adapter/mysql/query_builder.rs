//! SQL查询构建器模块
//!
//! 提供安全的SQL查询构建功能，防止SQL注入攻击

use crate::error::{QuickDbError, QuickDbResult};
use crate::types::*;
use crate::security::DatabaseSecurityValidator;
use std::collections::HashMap;

/// SQL查询构建器
pub struct SqlQueryBuilder {
    query_type: QueryType,
    fields: Vec<String>,
    conditions: Vec<QueryCondition>,
    condition_groups: Vec<QueryConditionGroup>,
    joins: Vec<JoinClause>,
    order_by: Vec<OrderClause>,
    group_by: Vec<String>,
    having: Vec<QueryCondition>,
    limit: Option<u64>,
    offset: Option<u64>,
    values: HashMap<String, DataValue>,
    returning_fields: Vec<String>,
    security_validator: DatabaseSecurityValidator,
}

#[derive(Debug, Clone)]
pub enum QueryType {
    Select,
    Insert,
    Update,
    Delete,
}

#[derive(Debug, Clone)]
pub struct JoinClause {
    pub join_type: JoinType,
    pub table: String,
    pub on_condition: String,
}

#[derive(Debug, Clone)]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Full,
}

#[derive(Debug, Clone)]
pub struct OrderClause {
    pub field: String,
    pub direction: SortDirection,
}

impl SqlQueryBuilder {
    /// 创建新的查询构建器
    pub fn new() -> Self {
        Self {
            query_type: QueryType::Select,
            fields: Vec::new(),
            conditions: Vec::new(),
            condition_groups: Vec::new(),
            joins: Vec::new(),
            order_by: Vec::new(),
            group_by: Vec::new(),
            having: Vec::new(),
            limit: None,
            offset: None,
            values: HashMap::new(),
            returning_fields: Vec::new(),
            security_validator: DatabaseSecurityValidator::new(DatabaseType::MySQL),
        }
    }

    
    /// 设置查询类型为SELECT
    pub fn select(mut self, fields: &[&str]) -> Self {
        self.query_type = QueryType::Select;
        self.fields = fields.iter().map(|s| s.to_string()).collect();
        self
    }

    /// 设置查询类型为INSERT
    pub fn insert(mut self, values: HashMap<String, DataValue>) -> Self {
        self.query_type = QueryType::Insert;
        self.values = values;
        self
    }

    /// 设置查询类型为UPDATE
    pub fn update(mut self, values: HashMap<String, DataValue>) -> Self {
        self.query_type = QueryType::Update;
        self.values = values;
        self
    }

    /// 设置查询类型为DELETE
    pub fn delete(mut self) -> Self {
        self.query_type = QueryType::Delete;
        self
    }

    /// 设置表名
    
    /// 添加WHERE条件
    pub fn where_condition(mut self, condition: QueryCondition) -> Self {
        self.conditions.push(condition);
        self
    }

    /// 添加多个WHERE条件
    pub fn where_conditions(mut self, conditions: &[QueryCondition]) -> Self {
        self.conditions.extend_from_slice(conditions);
        self
    }

    /// 添加条件组合（支持OR逻辑）
    pub fn where_condition_groups(mut self, groups: &[QueryConditionGroup]) -> Self {
        // 存储条件组合
        self.condition_groups.extend_from_slice(groups);
        // 清空简单条件，因为条件组合会覆盖简单条件
        self.conditions.clear();
        self
    }

    /// 添加JOIN子句
    pub fn join(mut self, join_type: JoinType, table: &str, on_condition: &str) -> Self {
        self.joins.push(JoinClause {
            join_type,
            table: table.to_string(),
            on_condition: on_condition.to_string(),
        });
        self
    }

    /// 添加ORDER BY子句
    pub fn order_by(mut self, field: &str, direction: SortDirection) -> Self {
        self.order_by.push(OrderClause {
            field: field.to_string(),
            direction,
        });
        self
    }

    /// 添加GROUP BY子句
    pub fn group_by(mut self, fields: &[&str]) -> Self {
        self.group_by = fields.iter().map(|s| s.to_string()).collect();
        self
    }

    /// 添加HAVING条件
    pub fn having(mut self, condition: QueryCondition) -> Self {
        self.having.push(condition);
        self
    }

    /// 设置LIMIT
    pub fn limit(mut self, limit: u64) -> Self {
        self.limit = Some(limit);
        self
    }

    /// 设置OFFSET
    pub fn offset(mut self, offset: u64) -> Self {
        self.offset = Some(offset);
        self
    }

    /// 设置RETURNING子句（用于INSERT/UPDATE/DELETE）
    pub fn returning(mut self, fields: &[&str]) -> Self {
        self.returning_fields = fields.iter().map(|s| s.to_string()).collect();
        self
    }

    /// 构建SQL查询语句
    pub fn build(self, table: &str, alias: &str) -> QuickDbResult<(String, Vec<DataValue>)> {
        let result = match self.query_type {
            QueryType::Select => self.build_select(table, alias),
            QueryType::Insert => self.build_insert(table, alias),
            QueryType::Update => self.build_update(table, alias),
            QueryType::Delete => self.build_delete(table, alias),
        };

        result
    }

    /// 构建SELECT语句
    fn build_select(&self, table: &str, alias: &str) -> QuickDbResult<(String, Vec<DataValue>)> {
        if table.is_empty() {
            return Err(QuickDbError::QueryError {
                message: "表名不能为空".to_string(),
            });
        }

        let fields = if self.fields.is_empty() {
            "*".to_string()
        } else {
            self.fields.join(", ")
        };

        let mut sql = format!("SELECT {} FROM {}", fields, table);
        let mut params = Vec::new();

        // 添加JOIN子句
        for join in &self.joins {
            let join_type = match join.join_type {
                JoinType::Inner => "INNER JOIN",
                JoinType::Left => "LEFT JOIN",
                JoinType::Right => "RIGHT JOIN",
                JoinType::Full => "FULL OUTER JOIN",
            };
            sql.push_str(&format!(" {} {} ON {}", join_type, join.table, join.on_condition));
        }

        // 添加WHERE条件（优先使用条件组合）
        if !self.condition_groups.is_empty() {
            let (where_clause, where_params) = self.build_where_clause_from_groups(&self.condition_groups, table, alias)?;
            sql.push_str(&format!(" WHERE {}", where_clause));
            params.extend(where_params);
        } else if !self.conditions.is_empty() {
            let (where_clause, where_params) = self.build_where_clause(&self.conditions, table, alias)?;
            sql.push_str(&format!(" WHERE {}", where_clause));
            params.extend(where_params);
        }

        // 添加GROUP BY
        if !self.group_by.is_empty() {
            sql.push_str(&format!(" GROUP BY {}", self.group_by.join(", ")));
        }

        // 添加HAVING
        if !self.having.is_empty() {
            let (having_clause, having_params) = self.build_where_clause(&self.having, table, alias)?;
            sql.push_str(&format!(" HAVING {}", having_clause));
            params.extend(having_params);
        }

        // 添加ORDER BY
        if !self.order_by.is_empty() {
            let order_clauses: Vec<String> = self.order_by
                .iter()
                .map(|o| {
                    let direction = match o.direction {
                        SortDirection::Asc => "ASC",
                        SortDirection::Desc => "DESC",
                    };
                    format!("{} {}", o.field, direction)
                })
                .collect();
            sql.push_str(&format!(" ORDER BY {}", order_clauses.join(", ")));
        }

        // 添加LIMIT和OFFSET
        if let Some(limit) = self.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }
        if let Some(offset) = self.offset {
            sql.push_str(&format!(" OFFSET {}", offset));
        }

        Ok((sql, params))
    }

    /// 构建INSERT语句
    fn build_insert(&self, table: &str, alias: &str) -> QuickDbResult<(String, Vec<DataValue>)> {
        if table.is_empty() {
            return Err(QuickDbError::QueryError {
                message: "表名不能为空".to_string(),
            });
        }

        if self.values.is_empty() {
            return Err(QuickDbError::QueryError {
                message: "插入值不能为空".to_string(),
            });
        }

        // 过滤掉 NULL 值，让数据库使用默认值或 NULL
        let non_null_values: HashMap<String, DataValue> = self.values
            .iter()
            .filter(|(_, value)| !matches!(value, DataValue::Null))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        if non_null_values.is_empty() {
            return Err(QuickDbError::QueryError {
                message: "所有插入值都是 NULL，无法插入".to_string(),
            });
        }

        let columns: Vec<String> = non_null_values.keys().cloned().collect();
        let placeholders: Vec<String> = self.generate_placeholders(columns.len());
        let params: Vec<DataValue> = columns.iter().map(|k| non_null_values[k].clone()).collect();

        let mut sql = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            table,
            columns.join(", "),
            placeholders.join(", ")
        );

        // 添加RETURNING子句
        if !self.returning_fields.is_empty() {
            sql.push_str(&format!(" RETURNING {}", self.returning_fields.join(", ")));
        }

        Ok((sql, params))
    }

    /// 构建UPDATE语句
    fn build_update(&self, table: &str, alias: &str) -> QuickDbResult<(String, Vec<DataValue>)> {
        if table.is_empty() {
            return Err(QuickDbError::QueryError {
                message: "表名不能为空".to_string(),
            });
        }

        if self.values.is_empty() {
            return Err(QuickDbError::QueryError {
                message: "更新值不能为空".to_string(),
            });
        }

        // 过滤掉 NULL 值，让数据库保持原值或设置为 NULL（如果需要显式设置 NULL，应该使用 IS NULL 操作）
        let non_null_values: HashMap<String, DataValue> = self.values
            .iter()
            .filter(|(_, value)| !matches!(value, DataValue::Null))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        if non_null_values.is_empty() {
            return Err(QuickDbError::QueryError {
                message: "所有更新值都是 NULL，无法更新".to_string(),
            });
        }

        let mut param_index = 1;
        let set_clauses: Vec<String> = non_null_values.keys().map(|k| {
            let placeholder = self.get_placeholder(param_index);
            param_index += 1;
            format!("{} = {}", k, placeholder)
        }).collect();
        let mut params: Vec<DataValue> = non_null_values.values().cloned().collect();

        let mut sql = format!("UPDATE {} SET {}", table, set_clauses.join(", "));

        // 添加WHERE条件
        if !self.conditions.is_empty() {
            let (where_clause, where_params) = self.build_where_clause_with_offset(&self.conditions, param_index, table, alias)?;
            sql.push_str(&format!(" WHERE {}", where_clause));
            params.extend(where_params);
        }

        // 添加RETURNING子句
        if !self.returning_fields.is_empty() {
            sql.push_str(&format!(" RETURNING {}", self.returning_fields.join(", ")));
        }

        Ok((sql, params))
    }

    /// 构建DELETE语句
    fn build_delete(&self, table: &str, alias: &str) -> QuickDbResult<(String, Vec<DataValue>)> {
        if table.is_empty() {
            return Err(QuickDbError::QueryError {
                message: "表名不能为空".to_string(),
            });
        }

        let mut sql = format!("DELETE FROM {}", table);
        let mut params = Vec::new();

        // 添加WHERE条件
        if !self.conditions.is_empty() {
            let (where_clause, where_params) = self.build_where_clause(&self.conditions, table, alias)?;
            sql.push_str(&format!(" WHERE {}", where_clause));
            params.extend(where_params);
        }

        // 添加RETURNING子句
        if !self.returning_fields.is_empty() {
            sql.push_str(&format!(" RETURNING {}", self.returning_fields.join(", ")));
        }

        Ok((sql, params))
    }

    /// 构建WHERE子句
    pub(crate) fn build_where_clause(&self, conditions: &[QueryCondition], table: &str, alias: &str) -> QuickDbResult<(String, Vec<DataValue>)> {
        self.build_where_clause_with_offset(conditions, 1, table, alias)
    }

    /// 构建WHERE子句（支持条件组合）
    pub fn build_where_clause_from_groups(&self, groups: &[QueryConditionGroup], table: &str, alias: &str) -> QuickDbResult<(String, Vec<DataValue>)> {
        self.build_where_clause_from_groups_with_offset(groups, 1, table, alias)
    }

    /// 构建WHERE子句（支持条件组合），从指定的参数索引开始
    fn build_where_clause_from_groups_with_offset(&self, groups: &[QueryConditionGroup], start_index: usize, table: &str, alias: &str) -> QuickDbResult<(String, Vec<DataValue>)> {
        if groups.is_empty() {
            return Ok((String::new(), Vec::new()));
        }

        let mut clauses = Vec::new();
        let mut params = Vec::new();
        let mut param_index = start_index;

        for group in groups {
            let (clause, group_params, new_index) = self.build_condition_group_clause(group, param_index, table, alias)?;
            clauses.push(clause);
            params.extend(group_params);
            param_index = new_index;
        }

        Ok((clauses.join(" AND "), params))
    }

    /// 构建单个条件组合的子句
    fn build_condition_group_clause(&self, group: &QueryConditionGroup, start_index: usize, table: &str, alias: &str) -> QuickDbResult<(String, Vec<DataValue>, usize)> {
        match group {
            QueryConditionGroup::Single(condition) => {
                let (clause, mut params, new_index) = self.build_single_condition_clause(condition, start_index, table, alias)?;
                Ok((clause, params, new_index))
            }
            QueryConditionGroup::Group { operator, conditions } => {
                if conditions.is_empty() {
                    return Ok((String::new(), Vec::new(), start_index));
                }

                let mut clauses = Vec::new();
                let mut params = Vec::new();
                let mut param_index = start_index;

                for condition in conditions {
                    let (clause, condition_params, new_index) = self.build_condition_group_clause(condition, param_index, table, alias)?;
                    if !clause.is_empty() {
                        clauses.push(clause);
                        params.extend(condition_params);
                        param_index = new_index;
                    }
                }

                if clauses.is_empty() {
                    return Ok((String::new(), Vec::new(), param_index));
                }

                let logical_op = match operator {
                    LogicalOperator::And => " AND ",
                    LogicalOperator::Or => " OR ",
                };

                let combined_clause = if clauses.len() == 1 {
                    clauses[0].clone()
                } else {
                    format!("({})", clauses.join(logical_op))
                };

                Ok((combined_clause, params, param_index))
            }
        }
    }

    /// 构建单个条件的子句
    fn build_single_condition_clause(&self, condition: &QueryCondition, param_index: usize, table: &str, alias: &str) -> QuickDbResult<(String, Vec<DataValue>, usize)> {
        let placeholder = self.get_placeholder(param_index);
        let mut new_index = param_index;

        let safe_field = self.security_validator.get_safe_field_identifier(&condition.field)?;
        let (clause, params) = match condition.operator {
            QueryOperator::Eq => {
                new_index += 1;
                (format!("{} = {}", safe_field, placeholder), vec![condition.value.clone()])
            }
            QueryOperator::Ne => {
                new_index += 1;
                (format!("{} != {}", safe_field, placeholder), vec![condition.value.clone()])
            }
            QueryOperator::Gt => {
                new_index += 1;
                (format!("{} > {}", safe_field, placeholder), vec![condition.value.clone()])
            }
            QueryOperator::Gte => {
                new_index += 1;
                (format!("{} >= {}", safe_field, placeholder), vec![condition.value.clone()])
            }
            QueryOperator::Lt => {
                new_index += 1;
                (format!("{} < {}", safe_field, placeholder), vec![condition.value.clone()])
            }
            QueryOperator::Lte => {
                new_index += 1;
                (format!("{} <= {}", safe_field, placeholder), vec![condition.value.clone()])
            }
            QueryOperator::Contains => {
                new_index += 1;

                // 检查字段类型
                if let Some(field_type) = self.get_field_type(&table, &condition.field) {
                    match field_type {
                        crate::model::FieldType::String { .. } => {
                            // 字符串字段使用 LIKE 查询
                            let value = if let DataValue::String(s) = &condition.value {
                                DataValue::String(format!("%{}%", s))
                            } else {
                                return Err(QuickDbError::ValidationError {
                                    field: condition.field.clone(),
                                    message: "字符串字段的Contains操作符只支持字符串值".to_string(),
                                });
                            };
                            (format!("{} LIKE {}", safe_field, placeholder), vec![value])
                        }
                        crate::model::FieldType::Json => {
                            // JSON字段使用JSON_CONTAINS函数 (MySQL/MariaDB特有)
                            // 检查整个JSON文档是否包含指定值
                            (format!("JSON_CONTAINS({}, '{}', '$')", safe_field, condition.value.to_json_value().to_string()), vec![])
                        }
                        _ => {
                            return Err(QuickDbError::ValidationError {
                                field: condition.field.clone(),
                                message: "Contains操作符只支持字符串和JSON字段".to_string(),
                            });
                        }
                    }
                } else {
                    // 无法确定字段类型，默认使用字符串LIKE查询
                    let value = if let DataValue::String(s) = &condition.value {
                        DataValue::String(format!("%{}%", s))
                    } else {
                        return Err(QuickDbError::ValidationError {
                            field: condition.field.clone(),
                            message: "无法确定字段类型，Contains操作符需要字符串值".to_string(),
                        });
                    };
                    (format!("{} LIKE {}", safe_field, placeholder), vec![value])
                }
            }
            QueryOperator::StartsWith => {
                new_index += 1;
                let value = if let DataValue::String(s) = &condition.value {
                    DataValue::String(format!("{}%", s))
                } else {
                    condition.value.clone()
                };
                (format!("{} LIKE {}", safe_field, placeholder), vec![value])
            }
            QueryOperator::EndsWith => {
                new_index += 1;
                let value = if let DataValue::String(s) = &condition.value {
                    DataValue::String(format!("%{}", s))
                } else {
                    condition.value.clone()
                };
                (format!("{} LIKE {}", safe_field, placeholder), vec![value])
            }
            QueryOperator::In => {
                if let DataValue::Array(values) = &condition.value {
                    // 检查查询值是否为支持的类型
                    for value in values {
                        match value {
                            DataValue::String(_) | DataValue::Int(_) | DataValue::Float(_) | DataValue::Uuid(_) => {
                                // 支持的类型
                            },
                            _ => {
                                return Err(QuickDbError::ValidationError {
                                    field: condition.field.clone(),
                                    message: format!("Array字段的IN操作只支持String、Int、Float、Uuid类型，不支持: {:?}", value),
                                });
                            }
                        }
                    }

                    if values.len() == 1 {
                        // 单个值，使用JSON_CONTAINS查询
                        // 将值转换为JSON字符串格式
                        let json_param = match &values[0] {
                            DataValue::String(s) => serde_json::to_string(&s).unwrap_or_else(|_| "\"\"".to_string()),
                            DataValue::Int(i) => serde_json::to_string(&i).unwrap_or_else(|_| "0".to_string()),
                            DataValue::Float(f) => serde_json::to_string(&f).unwrap_or_else(|_| "0.0".to_string()),
                            DataValue::Uuid(uuid) => serde_json::to_string(&uuid).unwrap_or_else(|_| "\"\"".to_string()),
                            _ => unreachable!(), // 已经检查过
                        };
                                                new_index += 1;
                        (format!("JSON_CONTAINS({}, ?)", safe_field), vec![DataValue::String(json_param)])
                    } else {
                        // 多个值，使用OR连接的JSON_CONTAINS查询
                        let mut json_conditions = Vec::new();
                        let mut json_params = Vec::new();

                        for value in values {
                            json_conditions.push(format!("JSON_CONTAINS({}, ?)", safe_field));
                            json_params.push(value.clone());
                            new_index += 1;
                        }

                        (format!("({})", json_conditions.join(" OR ")), json_params)
                    }
                } else {
                    return Err(QuickDbError::QueryError {
                        message: "IN 操作符需要数组类型的值".to_string(),
                    });
                }
            }
            QueryOperator::NotIn => {
                // MySQL的Array字段不支持NOT IN操作
                return Err(QuickDbError::QueryError {
                    message: "MySQL的Array字段不支持NOT IN操作，建议使用其他查询条件".to_string(),
                });
            }
            QueryOperator::Regex => {
                new_index += 1;
                (format!("{} REGEXP {}", safe_field, placeholder), vec![condition.value.clone()])
            }
            QueryOperator::Exists => {
                (format!("{} IS NOT NULL", safe_field), vec![])
            }
            QueryOperator::IsNull => {
                (format!("{} IS NULL", safe_field), vec![])
            }
            QueryOperator::IsNotNull => {
                (format!("{} IS NOT NULL", safe_field), vec![])
            }
            QueryOperator::JsonContains => {
                // MySQL暂时不支持JSON字段的JsonContains操作
                return Err(QuickDbError::ValidationError {
                    field: condition.field.clone(),
                    message: "MySQL暂时不支持JSON字段的JsonContains操作，建议使用PostgreSQL或MongoDB".to_string(),
                });
            }
        };

        Ok((clause, params, new_index))
    }

    /// 构建WHERE子句，从指定的参数索引开始
    pub(crate) fn build_where_clause_with_offset(&self, conditions: &[QueryCondition], start_index: usize, table: &str, alias: &str) -> QuickDbResult<(String, Vec<DataValue>)> {
        if conditions.is_empty() {
            return Ok((String::new(), Vec::new()));
        }

        let mut clauses = Vec::new();
        let mut params = Vec::new();
        let mut param_index = start_index;

        for condition in conditions {
            let placeholder = self.get_placeholder(param_index);
            let safe_field = self.security_validator.get_safe_field_identifier(&condition.field)?;

            match condition.operator {
                QueryOperator::Eq => {
                    let value = condition.value.clone();
                    clauses.push(format!("{} = {}", safe_field, placeholder));
                    params.push(value);
                    param_index += 1;
                }
                QueryOperator::Ne => {
                    clauses.push(format!("{} != {}", safe_field, placeholder));
                    params.push(condition.value.clone());
                    param_index += 1;
                }
                QueryOperator::Gt => {
                    clauses.push(format!("{} > {}", safe_field, placeholder));
                    params.push(condition.value.clone());
                    param_index += 1;
                }
                QueryOperator::Gte => {
                    clauses.push(format!("{} >= {}", safe_field, placeholder));
                    params.push(condition.value.clone());
                    param_index += 1;
                }
                QueryOperator::Lt => {
                    clauses.push(format!("{} < {}", safe_field, placeholder));
                    params.push(condition.value.clone());
                    param_index += 1;
                }
                QueryOperator::Lte => {
                    clauses.push(format!("{} <= {}", safe_field, placeholder));
                    params.push(condition.value.clone());
                    param_index += 1;
                }
                QueryOperator::Contains => {
                    // 检查字段类型
                    if let Some(field_type) = self.get_field_type(&table, &condition.field) {
                        match field_type {
                            crate::model::FieldType::String { .. } => {
                                // 字符串字段使用 LIKE 查询
                                if let DataValue::String(s) = &condition.value {
                                    params.push(DataValue::String(format!("%{}%", s)));
                                } else {
                                    return Err(QuickDbError::ValidationError {
                                        field: condition.field.clone(),
                                        message: "字符串字段的Contains操作符只支持字符串值".to_string(),
                                    });
                                }
                                clauses.push(format!("{} LIKE {}", self.security_validator.get_safe_field_identifier(&condition.field)?, placeholder));
                            }
                            crate::model::FieldType::Json => {
                                // JSON字段使用JSON_CONTAINS函数 (MySQL/MariaDB特有)
                                clauses.push(format!("JSON_CONTAINS({}, '{}', '$')", self.security_validator.get_safe_field_identifier(&condition.field)?, condition.value.to_json_value().to_string()));
                            }
                            _ => {
                                return Err(QuickDbError::ValidationError {
                                    field: condition.field.clone(),
                                    message: "Contains操作符只支持字符串和JSON字段".to_string(),
                                });
                            }
                        }
                    } else {
                        // 无法确定字段类型，默认使用字符串LIKE查询
                        if let DataValue::String(s) = &condition.value {
                            params.push(DataValue::String(format!("%{}%", s)));
                        } else {
                            return Err(QuickDbError::ValidationError {
                                field: condition.field.clone(),
                                message: "无法确定字段类型，Contains操作符需要字符串值".to_string(),
                            });
                        }
                        clauses.push(format!("{} LIKE {}", self.security_validator.get_safe_field_identifier(&condition.field)?, placeholder));
                    }
                    param_index += 1;
                }
                QueryOperator::StartsWith => {
                    clauses.push(format!("{} LIKE {}", condition.field, placeholder));
                    if let DataValue::String(s) = &condition.value {
                        params.push(DataValue::String(format!("{}%", s)));
                    } else {
                        params.push(condition.value.clone());
                    }
                    param_index += 1;
                }
                QueryOperator::EndsWith => {
                    clauses.push(format!("{} LIKE {}", condition.field, placeholder));
                    if let DataValue::String(s) = &condition.value {
                        params.push(DataValue::String(format!("%{}", s)));
                    } else {
                        params.push(condition.value.clone());
                    }
                    param_index += 1;
                }
                QueryOperator::In => {
                    if let DataValue::Array(values) = &condition.value {
                        // 检查查询值是否为支持的类型
                        for value in values {
                            match value {
                                DataValue::String(_) | DataValue::Int(_) | DataValue::Float(_) | DataValue::Uuid(_) => {
                                    // 支持的类型
                                },
                                _ => {
                                    return Err(QuickDbError::ValidationError {
                                        field: condition.field.clone(),
                                        message: format!("Array字段的IN操作只支持String、Int、Float、Uuid类型，不支持: {:?}", value),
                                    });
                                }
                            }
                        }

                        if values.len() == 1 {
                            // 单个值，使用JSON_CONTAINS查询
                            // 将值转换为JSON字符串格式
                            let json_param = match &values[0] {
                                DataValue::String(s) => serde_json::to_string(&s).unwrap_or_else(|_| "\"\"".to_string()),
                                DataValue::Int(i) => serde_json::to_string(&i).unwrap_or_else(|_| "0".to_string()),
                                DataValue::Float(f) => serde_json::to_string(&f).unwrap_or_else(|_| "0.0".to_string()),
                                DataValue::Uuid(uuid) => serde_json::to_string(&uuid).unwrap_or_else(|_| "\"\"".to_string()),
                                _ => unreachable!(), // 已经检查过
                            };
                            clauses.push(format!("JSON_CONTAINS({}, ?)", condition.field));
                            params.push(DataValue::String(json_param));
                            param_index += 1;
                        } else {
                            // 多个值，使用OR连接的JSON_CONTAINS查询
                            for value in values {
                                let json_param = match value {
                                    DataValue::String(s) => serde_json::to_string(&s).unwrap_or_else(|_| "\"\"".to_string()),
                                    DataValue::Int(i) => serde_json::to_string(&i).unwrap_or_else(|_| "0".to_string()),
                                    DataValue::Float(f) => serde_json::to_string(&f).unwrap_or_else(|_| "0.0".to_string()),
                                    DataValue::Uuid(uuid) => serde_json::to_string(&uuid).unwrap_or_else(|_| "\"\"".to_string()),
                                    _ => unreachable!(), // 已经检查过
                                };
                                clauses.push(format!("JSON_CONTAINS({}, ?)", condition.field));
                                params.push(DataValue::String(json_param));
                                param_index += 1;
                            }
                        }
                    } else {
                        return Err(QuickDbError::QueryError {
                            message: "IN 操作符需要数组类型的值".to_string(),
                        });
                    }
                }
                QueryOperator::NotIn => {
                    // MySQL的Array字段不支持NOT IN操作
                    return Err(QuickDbError::QueryError {
                        message: "MySQL的Array字段不支持NOT IN操作，建议使用其他查询条件".to_string(),
                    });
                }
                QueryOperator::Regex => {
                    // 不同数据库的正则表达式语法不同，这里使用通用的LIKE
                    clauses.push(format!("{} REGEXP {}", condition.field, placeholder));
                    params.push(condition.value.clone());
                    param_index += 1;
                }
                QueryOperator::Exists => {
                    // 检查字段是否存在（主要用于NoSQL数据库）
                    clauses.push(format!("{} IS NOT NULL", condition.field));
                    // Exists操作符不需要参数值
                }
                QueryOperator::IsNull => {
                    clauses.push(format!("{} IS NULL", condition.field));
                    // IsNull操作符不需要参数值
                }
                QueryOperator::IsNotNull => {
                    clauses.push(format!("{} IS NOT NULL", condition.field));
                    // IsNotNull操作符不需要参数值
                }
                QueryOperator::JsonContains => {
                    // MySQL暂时不支持JSON字段的JsonContains操作
                    return Err(QuickDbError::ValidationError {
                        field: condition.field.clone(),
                        message: "MySQL暂时不支持JSON字段的JsonContains操作，建议使用PostgreSQL或MongoDB".to_string(),
                    });
                }
            }
        }

        Ok((clauses.join(" AND "), params))
    }

    /// 生成占位符
    fn generate_placeholders(&self, count: usize) -> Vec<String> {
        // MySQL使用?占位符
        (0..count).map(|_| "?".to_string()).collect()
    }

    /// 获取单个占位符
    fn get_placeholder(&self, index: usize) -> String {
        // MySQL使用?占位符
        "?".to_string()
    }

    /// 获取字段类型信息
    ///
    /// 通过查询模型元数据来确定指定字段的类型
    ///
    /// # 参数
    /// * `table_name` - 表名
    /// * `field_name` - 字段名
    ///
    /// # 返回值
    /// * `Some(FieldType)` - 字段的类型定义
    /// * `None` - 无法确定字段类型（表不存在或字段不存在）
    fn get_field_type(&self, table_name: &str, field_name: &str) -> Option<crate::model::FieldType> {
        // 通过全局管理器获取模型元数据
        if let Some(model_meta) = crate::manager::get_model(table_name) {
            model_meta.fields.get(field_name).map(|field_def| field_def.field_type.clone())
        } else {
            None
        }
    }
}

impl Default for SqlQueryBuilder {
    fn default() -> Self {
        Self::new()
    }
}