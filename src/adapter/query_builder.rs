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
    table: String,
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
    db_type: DatabaseType,
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
        let db_type = DatabaseType::SQLite; // 默认为 SQLite
        Self {
            query_type: QueryType::Select,
            table: String::new(),
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
            db_type,
            security_validator: DatabaseSecurityValidator::new(db_type),
        }
    }

    /// 设置数据库类型
    pub fn database_type(mut self, db_type: DatabaseType) -> Self {
        self.db_type = db_type;
        self.security_validator = DatabaseSecurityValidator::new(db_type);
        self
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
    pub fn from(mut self, table: &str) -> Self {
        self.table = table.to_string();
        self
    }

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
    pub fn build(&self) -> QuickDbResult<(String, Vec<DataValue>)> {
        let result = match self.query_type {
            QueryType::Select => self.build_select(),
            QueryType::Insert => self.build_insert(),
            QueryType::Update => self.build_update(),
            QueryType::Delete => self.build_delete(),
        };

        result
    }

    /// 构建SELECT语句
    fn build_select(&self) -> QuickDbResult<(String, Vec<DataValue>)> {
        if self.table.is_empty() {
            return Err(QuickDbError::QueryError {
                message: "表名不能为空".to_string(),
            });
        }

        let fields = if self.fields.is_empty() {
            "*".to_string()
        } else {
            self.fields.join(", ")
        };

        let mut sql = format!("SELECT {} FROM {}", fields, self.table);
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
            let (where_clause, where_params) = self.build_where_clause_from_groups(&self.condition_groups)?;
            sql.push_str(&format!(" WHERE {}", where_clause));
            params.extend(where_params);
        } else if !self.conditions.is_empty() {
            let (where_clause, where_params) = self.build_where_clause(&self.conditions)?;
            sql.push_str(&format!(" WHERE {}", where_clause));
            params.extend(where_params);
        }

        // 添加GROUP BY
        if !self.group_by.is_empty() {
            sql.push_str(&format!(" GROUP BY {}", self.group_by.join(", ")));
        }

        // 添加HAVING
        if !self.having.is_empty() {
            let (having_clause, having_params) = self.build_where_clause(&self.having)?;
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
    fn build_insert(&self) -> QuickDbResult<(String, Vec<DataValue>)> {
        if self.table.is_empty() {
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
            self.table,
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
    fn build_update(&self) -> QuickDbResult<(String, Vec<DataValue>)> {
        if self.table.is_empty() {
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

        let mut sql = format!("UPDATE {} SET {}", self.table, set_clauses.join(", "));

        // 添加WHERE条件
        if !self.conditions.is_empty() {
            let (where_clause, where_params) = self.build_where_clause_with_offset(&self.conditions, param_index)?;
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
    fn build_delete(&self) -> QuickDbResult<(String, Vec<DataValue>)> {
        if self.table.is_empty() {
            return Err(QuickDbError::QueryError {
                message: "表名不能为空".to_string(),
            });
        }

        let mut sql = format!("DELETE FROM {}", self.table);
        let mut params = Vec::new();

        // 添加WHERE条件
        if !self.conditions.is_empty() {
            let (where_clause, where_params) = self.build_where_clause(&self.conditions)?;
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
    pub(crate) fn build_where_clause(&self, conditions: &[QueryCondition]) -> QuickDbResult<(String, Vec<DataValue>)> {
        self.build_where_clause_with_offset(conditions, 1)
    }

    /// 构建WHERE子句（支持条件组合）
    pub fn build_where_clause_from_groups(&self, groups: &[QueryConditionGroup]) -> QuickDbResult<(String, Vec<DataValue>)> {
        self.build_where_clause_from_groups_with_offset(groups, 1)
    }

    /// 构建WHERE子句（支持条件组合），从指定的参数索引开始
    fn build_where_clause_from_groups_with_offset(&self, groups: &[QueryConditionGroup], start_index: usize) -> QuickDbResult<(String, Vec<DataValue>)> {
        if groups.is_empty() {
            return Ok((String::new(), Vec::new()));
        }

        let mut clauses = Vec::new();
        let mut params = Vec::new();
        let mut param_index = start_index;

        for group in groups {
            let (clause, group_params, new_index) = self.build_condition_group_clause(group, param_index)?;
            clauses.push(clause);
            params.extend(group_params);
            param_index = new_index;
        }

        Ok((clauses.join(" AND "), params))
    }

    /// 构建单个条件组合的子句
    fn build_condition_group_clause(&self, group: &QueryConditionGroup, start_index: usize) -> QuickDbResult<(String, Vec<DataValue>, usize)> {
        match group {
            QueryConditionGroup::Single(condition) => {
                let (clause, mut params, new_index) = self.build_single_condition_clause(condition, start_index)?;
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
                    let (clause, condition_params, new_index) = self.build_condition_group_clause(condition, param_index)?;
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
    fn build_single_condition_clause(&self, condition: &QueryCondition, param_index: usize) -> QuickDbResult<(String, Vec<DataValue>, usize)> {
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

                // 添加调试日志：打印实际输入的数据
                #[cfg(debug_assertions)]
                {
                    rat_logger::debug!("PostgreSQL JSON查询调试信息:");
                    rat_logger::debug!("  字段名: {}", condition.field);
                    rat_logger::debug!("  原始值: {:?}", condition.value);
                    rat_logger::debug!("  值类型: {:?}", std::mem::discriminant(&condition.value));

                    if let Some(field_type) = self.get_field_type(&self.table, &condition.field) {
                        rat_logger::debug!("  字段类型: {:?}", field_type);
                    } else {
                        rat_logger::debug!("  字段类型: 无法确定");
                    }
                }

                // 对于PostgreSQL数据库，必须根据字段类型确定操作符
                if matches!(self.db_type, DatabaseType::PostgreSQL) {
                    // 检查字段类型
                    if let Some(field_type) = self.get_field_type(&self.table, &condition.field) {
                        if matches!(field_type, crate::model::FieldType::Json) {
                            // 使用PostgreSQL专用工具处理JSON查询
                            match crate::adapter::build_json_query_condition(&safe_field, &condition.value, &placeholder) {
                                Ok((sql_clause, param_value)) => {
                                    #[cfg(debug_assertions)]
                                    rat_logger::debug!("  PostgreSQL JSON查询条件生成成功: {} | {:?}", sql_clause, param_value);
                                    (sql_clause, vec![param_value])
                                }
                                Err(e) => {
                                    #[cfg(debug_assertions)]
                                    rat_logger::debug!("  PostgreSQL JSON查询条件生成失败: {:?}", e);
                                    return Err(e);
                                }
                            }
                        } else {
                            // 非JSON字段使用LIKE查询
                            let value = if let DataValue::String(s) = &condition.value {
                                DataValue::String(format!("%{}%", s))
                            } else {
                                condition.value.clone()
                            };
                            (format!("{} LIKE {}", safe_field, placeholder), vec![value])
                        }
                    } else {
                        // 无法确定字段类型，直接报错
                        return Err(QuickDbError::ValidationError {
                            field: condition.field.clone(),
                            message: format!("无法确定字段 '{}' 的类型，请确保已正确注册模型元数据", condition.field),
                        });
                    }
                } else {
                    // 其他数据库继续使用LIKE操作符
                    let value = if let DataValue::String(s) = &condition.value {
                        DataValue::String(format!("%{}%", s))
                    } else {
                        condition.value.clone()
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
                    let mut placeholders = Vec::new();
                    for _ in 0..values.len() {
                        placeholders.push(self.get_placeholder(new_index));
                        new_index += 1;
                    }
                    (format!("{} IN ({})", safe_field, placeholders.join(", ")), values.clone())
                } else {
                    return Err(QuickDbError::QueryError {
                        message: "IN 操作符需要数组类型的值".to_string(),
                    });
                }
            }
            QueryOperator::NotIn => {
                if let DataValue::Array(values) = &condition.value {
                    let mut placeholders = Vec::new();
                    for _ in 0..values.len() {
                        placeholders.push(self.get_placeholder(new_index));
                        new_index += 1;
                    }
                    (format!("{} NOT IN ({})", safe_field, placeholders.join(", ")), values.clone())
                } else {
                    return Err(QuickDbError::QueryError {
                        message: "NOT IN 操作符需要数组类型的值".to_string(),
                    });
                }
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
        };

        Ok((clause, params, new_index))
    }

    /// 构建WHERE子句，从指定的参数索引开始
    pub(crate) fn build_where_clause_with_offset(&self, conditions: &[QueryCondition], start_index: usize) -> QuickDbResult<(String, Vec<DataValue>)> {
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
                    clauses.push(format!("{} = {}", safe_field, placeholder));
                    params.push(condition.value.clone());
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
                    // 添加调试日志：打印实际输入的数据
                    #[cfg(debug_assertions)]
                    {
                        rat_logger::debug!("PostgreSQL JSON查询调试信息 (build_where_clause_with_offset):");
                        rat_logger::debug!("  字段名: {}", condition.field);
                        rat_logger::debug!("  原始值: {:?}", condition.value);
                        rat_logger::debug!("  值类型: {:?}", std::mem::discriminant(&condition.value));

                        if let Some(field_type) = self.get_field_type(&self.table, &condition.field) {
                            rat_logger::debug!("  字段类型: {:?}", field_type);
                        } else {
                            rat_logger::debug!("  字段类型: 无法确定");
                        }
                    }

                    // 对于PostgreSQL数据库，必须根据字段类型确定操作符
                    if matches!(self.db_type, DatabaseType::PostgreSQL) {
                        // 检查字段类型
                        if let Some(field_type) = self.get_field_type(&self.table, &condition.field) {
                            if matches!(field_type, crate::model::FieldType::Json) {
                                // 使用PostgreSQL专用工具处理JSON查询
                                match crate::adapter::build_json_query_condition(&condition.field, &condition.value, &placeholder) {
                                    Ok((sql_clause, param_value)) => {
                                        #[cfg(debug_assertions)]
                                        rat_logger::debug!("  PostgreSQL JSON查询条件生成成功 (build_where_clause_with_offset): {} | {:?}", sql_clause, param_value);
                                        clauses.push(sql_clause);
                                        params.push(param_value);
                                    }
                                    Err(e) => {
                                        #[cfg(debug_assertions)]
                                        rat_logger::debug!("  PostgreSQL JSON查询条件生成失败 (build_where_clause_with_offset): {:?}", e);
                                        return Err(e);
                                    }
                                }
                            } else {
                                // 非JSON字段使用LIKE查询
                                if let DataValue::String(s) = &condition.value {
                                    params.push(DataValue::String(format!("%{}%", s)));
                                } else {
                                    params.push(condition.value.clone());
                                }
                                clauses.push(format!("{} LIKE {}", condition.field, placeholder));
                            }
                        } else {
                            // 无法确定字段类型，直接报错
                            return Err(QuickDbError::ValidationError {
                                field: condition.field.clone(),
                                message: format!("无法确定字段 '{}' 的类型，请确保已正确注册模型元数据", condition.field),
                            });
                        }
                    } else {
                        // 其他数据库继续使用LIKE操作符
                        if let DataValue::String(s) = &condition.value {
                            params.push(DataValue::String(format!("%{}%", s)));
                        } else {
                            params.push(condition.value.clone());
                        }
                        clauses.push(format!("{} LIKE {}", condition.field, placeholder));
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
                        let mut placeholders = Vec::new();
                        for _ in 0..values.len() {
                            placeholders.push(self.get_placeholder(param_index));
                            param_index += 1;
                        }
                        clauses.push(format!("{} IN ({})", condition.field, placeholders.join(", ")));
                        params.extend(values.clone());
                    } else {
                        return Err(QuickDbError::QueryError {
                            message: "IN 操作符需要数组类型的值".to_string(),
                        });
                    }
                }
                QueryOperator::NotIn => {
                    if let DataValue::Array(values) = &condition.value {
                        let mut placeholders = Vec::new();
                        for _ in 0..values.len() {
                            placeholders.push(self.get_placeholder(param_index));
                            param_index += 1;
                        }
                        clauses.push(format!("{} NOT IN ({})", condition.field, placeholders.join(", ")));
                        params.extend(values.clone());
                    } else {
                        return Err(QuickDbError::QueryError {
                            message: "NOT IN 操作符需要数组类型的值".to_string(),
                        });
                    }
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
            }
        }

        Ok((clauses.join(" AND "), params))
    }

    /// 生成占位符
    fn generate_placeholders(&self, count: usize) -> Vec<String> {
        match self.db_type {
            DatabaseType::PostgreSQL => {
                (1..=count).map(|i| format!("${}", i)).collect()
            }
            DatabaseType::MySQL | DatabaseType::SQLite => {
                (0..count).map(|_| "?".to_string()).collect()
            }
            DatabaseType::MongoDB => {
                // MongoDB不使用SQL占位符，但为了兼容性返回一个列表
                (0..count).map(|_| "?".to_string()).collect()
            }
        }
    }

    /// 获取单个占位符
    fn get_placeholder(&self, index: usize) -> String {
        match self.db_type {
            DatabaseType::PostgreSQL => format!("${}", index),
            DatabaseType::MySQL | DatabaseType::SQLite => "?".to_string(),
            DatabaseType::MongoDB => "?".to_string(), // MongoDB不使用SQL占位符
        }
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