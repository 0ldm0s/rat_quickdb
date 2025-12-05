//! MongoDB查询构建器模块
//!
//! 提供MongoDB查询文档的构建功能，支持基于字段元数据的Contains操作符

use crate::adapter::utils::get_field_type;
use crate::error::{QuickDbError, QuickDbResult};
use crate::types::*;
use mongodb::bson::{Bson, Document, Regex, doc};
use rat_logger::debug;

/// MongoDB查询构建器
pub struct MongoQueryBuilder {
    conditions: Vec<QueryCondition>,
    condition_groups: Vec<QueryConditionGroup>,
}

impl MongoQueryBuilder {
    /// 创建新的MongoDB查询构建器
    pub fn new() -> Self {
        Self {
            conditions: Vec::new(),
            condition_groups: Vec::new(),
        }
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

    /// 添加条件组合
    pub fn where_condition_groups(mut self, groups: &[QueryConditionGroup]) -> Self {
        self.condition_groups.extend_from_slice(groups);
        self
    }

    /// 构建MongoDB查询文档
    pub fn build(self, table: &str, alias: &str) -> QuickDbResult<Document> {
        debug!(
            "[MongoDB] 开始构建查询文档，条件数量: {}，表: {}，别名: {}",
            self.conditions.len(),
            table,
            alias
        );
        let mut query_doc = Document::new();

        // 优先使用条件组合
        if !self.condition_groups.is_empty() {
            let groups_doc = self.build_condition_groups_document(table, alias)?;
            if !groups_doc.is_empty() {
                query_doc.extend(groups_doc);
            }
        } else if !self.conditions.is_empty() {
            let conditions_doc = self.build_conditions_document(table, alias)?;
            if !conditions_doc.is_empty() {
                query_doc.extend(conditions_doc);
            }
        }

        debug!("[MongoDB] 完成查询文档构建: {:?}", query_doc);
        Ok(query_doc)
    }

    /// 构建条件文档
    fn build_conditions_document(&self, table: &str, alias: &str) -> QuickDbResult<Document> {
        let mut query_doc = Document::new();

        for condition in &self.conditions {
            let field_name = &condition.field;
            let condition_doc = self.build_single_condition_document(table, alias, condition)?;

            if !condition_doc.is_empty() {
                query_doc.extend(condition_doc);
            }
        }

        Ok(query_doc)
    }

    /// 构建条件组合文档
    fn build_condition_groups_document(&self, table: &str, alias: &str) -> QuickDbResult<Document> {
        let mut group_docs = Vec::new();

        for group in &self.condition_groups {
            let group_doc = self.build_single_condition_group_document(table, alias, group)?;
            if !group_doc.is_empty() {
                group_docs.push(group_doc);
            }
        }

        if group_docs.len() == 1 {
            Ok(group_docs.into_iter().next().unwrap())
        } else {
            Ok(doc! { "$and": group_docs })
        }
    }

    /// 构建单个条件组合的文档
    fn build_single_condition_group_document(
        &self,
        table: &str,
        alias: &str,
        group: &QueryConditionGroup,
    ) -> QuickDbResult<Document> {
        match group {
            QueryConditionGroup::Single(condition) => {
                self.build_single_condition_document(table, alias, condition)
            }
            QueryConditionGroup::Group {
                operator,
                conditions,
            } => {
                if conditions.is_empty() {
                    return Ok(Document::new());
                }

                let mut condition_docs = Vec::new();
                for condition in conditions {
                    let doc =
                        self.build_single_condition_group_document(table, alias, condition)?;
                    if !doc.is_empty() {
                        condition_docs.push(doc);
                    }
                }

                if condition_docs.len() == 1 {
                    Ok(condition_docs.into_iter().next().unwrap())
                } else {
                    let operator_key = match operator {
                        LogicalOperator::And => "$and",
                        LogicalOperator::Or => "$or",
                    };
                    Ok(doc! { operator_key: condition_docs })
                }
            }
        }
    }

    /// 构建单个条件的文档
    fn build_single_condition_document(
        &self,
        table: &str,
        alias: &str,
        condition: &QueryCondition,
    ) -> QuickDbResult<Document> {
        let field_name = &condition.field;
        let bson_value = self.data_value_to_bson(&condition.value);

        debug!(
            "[MongoDB] 处理条件: {} {:?} {:?}",
            field_name, condition.operator, bson_value
        );

        let condition_doc = match condition.operator {
            QueryOperator::Eq => doc! { field_name: bson_value },
            QueryOperator::Ne => doc! { field_name: doc! { "$ne": bson_value } },
            QueryOperator::Gt => doc! { field_name: doc! { "$gt": bson_value } },
            QueryOperator::Gte => doc! { field_name: doc! { "$gte": bson_value } },
            QueryOperator::Lt => doc! { field_name: doc! { "$lt": bson_value } },
            QueryOperator::Lte => doc! { field_name: doc! { "$lte": bson_value } },
            QueryOperator::Contains => {
                self.build_contains_condition(field_name, table, alias, bson_value)?
            }
            QueryOperator::JsonContains => {
                // MongoDB JSON字段包含查询 - 简单平铺实现
                match bson_value {
                    Bson::String(s) => {
                        // 如果输入是JSON字符串，解析它并直接平铺为嵌套查询
                        let json_value: serde_json::Value =
                            serde_json::from_str(&s).map_err(|e| {
                                QuickDbError::ValidationError {
                                    field: condition.field.clone(),
                                    message: format!("无效的JSON格式: {}", e),
                                }
                            })?;

                        // 直接平铺JSON对象为MongoDB点标记法
                        self.flatten_json_to_query(field_name, &json_value)
                    }
                    _ => {
                        // 对于其他BSON类型，直接进行查询
                        doc! { field_name: bson_value }
                    }
                }
            }
            QueryOperator::StartsWith => {
                if let Bson::String(s) = bson_value {
                    doc! { field_name: doc! { "$regex": format!("^{}", &s), "$options": "i" } }
                } else {
                    return Err(QuickDbError::ValidationError {
                        field: condition.field.clone(),
                        message: "StartsWith操作符只支持字符串类型".to_string(),
                    });
                }
            }
            QueryOperator::EndsWith => {
                if let Bson::String(s) = bson_value {
                    doc! { field_name: doc! { "$regex": format!("{}$", &s), "$options": "i" } }
                } else {
                    return Err(QuickDbError::ValidationError {
                        field: condition.field.clone(),
                        message: "EndsWith操作符只支持字符串类型".to_string(),
                    });
                }
            }
            QueryOperator::In => {
                // 验证Array字段IN操作的数据类型
                if let Bson::Array(arr) = &bson_value {
                    // 检查字段类型，如果是Array字段，验证数组中元素的数据类型
                    if let Some(field_type) = get_field_type(table, alias, field_name) {
                        if matches!(field_type, crate::model::FieldType::Array { .. }) {
                            // Array字段：验证数组中元素的数据类型
                            for bson_elem in arr {
                                match bson_elem {
                                    Bson::String(_)
                                    | Bson::Int32(_)
                                    | Bson::Int64(_)
                                    | Bson::Double(_) => {
                                        // 支持的类型：String, Int, Float
                                    }
                                    Bson::ObjectId(_) => {
                                        // Uuid类型映射到ObjectId，支持
                                    }
                                    _ => {
                                        return Err(QuickDbError::ValidationError {
                                            field: field_name.to_string(),
                                            message: format!(
                                                "Array字段的IN操作只支持String、Int、Float、Uuid类型，不支持: {:?}",
                                                bson_elem
                                            ),
                                        });
                                    }
                                }
                            }
                        }
                    }
                    doc! { field_name: doc! { "$in": arr.clone() } }
                } else {
                    doc! { field_name: doc! { "$in": [bson_value] } }
                }
            }
            QueryOperator::NotIn => {
                // 验证Array字段NOT IN操作的数据类型
                if let Bson::Array(arr) = &bson_value {
                    // 检查字段类型，如果是Array字段，验证数组中元素的数据类型
                    if let Some(field_type) = get_field_type(table, alias, field_name) {
                        if matches!(field_type, crate::model::FieldType::Array { .. }) {
                            // Array字段：验证数组中元素的数据类型
                            for bson_elem in arr {
                                match bson_elem {
                                    Bson::String(_)
                                    | Bson::Int32(_)
                                    | Bson::Int64(_)
                                    | Bson::Double(_) => {
                                        // 支持的类型：String, Int, Float
                                    }
                                    Bson::ObjectId(_) => {
                                        // Uuid类型映射到ObjectId，支持
                                    }
                                    _ => {
                                        return Err(QuickDbError::ValidationError {
                                            field: field_name.to_string(),
                                            message: format!(
                                                "Array字段的NOT IN操作只支持String、Int、Float、Uuid类型，不支持: {:?}",
                                                bson_elem
                                            ),
                                        });
                                    }
                                }
                            }
                        }
                    }
                    doc! { field_name: doc! { "$nin": arr.clone() } }
                } else {
                    doc! { field_name: doc! { "$nin": [bson_value] } }
                }
            }
            QueryOperator::Regex => {
                if let Bson::String(s) = bson_value {
                    doc! { field_name: doc! { "$regex": s, "$options": "i" } }
                } else {
                    return Err(QuickDbError::ValidationError {
                        field: condition.field.clone(),
                        message: "Regex操作符只支持字符串类型".to_string(),
                    });
                }
            }
            QueryOperator::Exists => {
                doc! { field_name: doc! { "$exists": true } }
            }
            QueryOperator::IsNull => {
                doc! { field_name: doc! { "$eq": null } }
            }
            QueryOperator::IsNotNull => {
                doc! { field_name: doc! { "$ne": null } }
            }
        };

        Ok(condition_doc)
    }

    /// 构建Contains条件，基于字段元数据
    fn build_contains_condition(
        &self,
        field_name: &str,
        table: &str,
        alias: &str,
        bson_value: Bson,
    ) -> QuickDbResult<Document> {
        // 获取字段类型
        let field_type = get_field_type(table, alias, field_name).ok_or_else(|| {
            QuickDbError::ValidationError {
                field: field_name.to_string(),
                message: format!(
                    "无法确定字段 '{}' 的类型，请确保已正确注册模型元数据 (alias={})",
                    field_name, alias
                ),
            }
        })?;

        debug!(
            "[MongoDB] Contains操作 - 字段类型: {:?}, 值: {:?}",
            field_type, bson_value
        );

        match field_type {
            crate::model::FieldType::String { .. } => {
                // 字符串字段使用正则表达式匹配
                if let Bson::String(s) = bson_value {
                    let regex_doc = doc! { "$regex": format!(".*{}.*", &s), "$options": "i" };
                    debug!(
                        "[MongoDB] Contains操作(字符串): {} = {:?}",
                        field_name, regex_doc
                    );
                    Ok(doc! { field_name: regex_doc })
                } else {
                    return Err(QuickDbError::ValidationError {
                        field: field_name.to_string(),
                        message: "字符串字段的Contains操作符只支持字符串值".to_string(),
                    });
                }
            }
            crate::model::FieldType::Array { .. } => {
                // Array字段使用$in操作符
                debug!(
                    "[MongoDB] Contains操作(Array): {} = {:?}",
                    field_name, bson_value
                );
                Ok(doc! { field_name: doc! { "$in": [bson_value] } })
            }
            crate::model::FieldType::Json => {
                // JSON字段根据类型处理
                match bson_value {
                    Bson::String(s) => {
                        let regex_doc = doc! { "$regex": format!(".*{}.*", &s), "$options": "i" };
                        Ok(doc! { field_name: regex_doc })
                    }
                    _ => Ok(doc! { field_name: doc! { "$in": [bson_value] } }),
                }
            }
            _ => {
                return Err(QuickDbError::ValidationError {
                    field: field_name.to_string(),
                    message: "Contains操作符只支持字符串、Array和JSON字段".to_string(),
                });
            }
        }
    }

    /// 将DataValue转换为BSON值
    fn data_value_to_bson(&self, value: &DataValue) -> Bson {
        match value {
            DataValue::String(s) => Bson::String(s.clone()),
            DataValue::Int(i) => Bson::Int64(*i),
            DataValue::Float(f) => Bson::Double(*f),
            DataValue::Bool(b) => Bson::Boolean(*b),
            DataValue::DateTime(dt) => {
                // 将DateTime<FixedOffset>转换为DateTime<Utc>，然后转换为MongoDB BSON DateTime
                let utc_dt = chrono::DateTime::<chrono::Utc>::from(*dt);
                Bson::DateTime(mongodb::bson::DateTime::from_system_time(utc_dt.into()))
            }
            DataValue::DateTimeUTC(dt) => {
                // DateTime<Utc>直接转换为MongoDB BSON DateTime
                Bson::DateTime(mongodb::bson::DateTime::from_system_time(dt.clone().into()))
            }
            DataValue::Uuid(uuid) => Bson::String(uuid.to_string()),
            DataValue::Json(json) => {
                // 尝试将JSON转换为BSON文档
                if let Ok(doc) = mongodb::bson::to_document(json) {
                    Bson::Document(doc)
                } else {
                    Bson::String(json.to_string())
                }
            }
            DataValue::Array(arr) => {
                let bson_array: Vec<Bson> =
                    arr.iter().map(|v| self.data_value_to_bson(v)).collect();
                Bson::Array(bson_array)
            }
            DataValue::Object(obj) => {
                let mut bson_doc = Document::new();
                for (key, value) in obj {
                    let bson_value = self.data_value_to_bson(value);
                    bson_doc.insert(key, bson_value);
                }
                Bson::Document(bson_doc)
            }
            DataValue::Null => Bson::Null,
            DataValue::Bytes(bytes) => Bson::Binary(mongodb::bson::Binary {
                bytes: bytes.clone(),
                subtype: mongodb::bson::spec::BinarySubtype::Generic,
            }),
        }
    }

    /// 将JSON对象平铺为MongoDB点标记法查询
    /// 简单实现：只处理键值对，不处理数组等复杂结构
    fn flatten_json_to_query(&self, field_name: &str, json_value: &serde_json::Value) -> Document {
        match json_value {
            serde_json::Value::Object(map) => {
                if map.is_empty() {
                    return Document::new();
                }

                let mut conditions = Vec::new();

                for (key, value) in map {
                    let dot_path = format!("{}.{}", field_name, key);
                    if let serde_json::Value::Object(_) = value {
                        // 嵌套对象，递归平铺
                        let nested_condition = self.flatten_json_to_query(&dot_path, value);
                        // 将嵌套条件合并到当前条件
                        for (k, v) in nested_condition {
                            conditions.push(doc! { k: v });
                        }
                    } else {
                        // 基本值，直接构建查询条件
                        if let Ok(bson_value) = mongodb::bson::to_bson(value) {
                            conditions.push(doc! { dot_path: bson_value });
                        }
                    }
                }

                // 多个条件使用$and组合
                if conditions.len() == 1 {
                    conditions.into_iter().next().unwrap()
                } else {
                    doc! { "$and": conditions }
                }
            }
            _ => {
                // 非对象类型，返回空文档
                Document::new()
            }
        }
    }
}

/// 构建MongoDB查询文档的便捷函数
pub fn build_query_document(
    table: &str,
    alias: &str,
    conditions: &[QueryCondition],
) -> QuickDbResult<Document> {
    MongoQueryBuilder::new()
        .where_conditions(conditions)
        .build(table, alias)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::QueryCondition;

    #[test]
    fn test_mongo_query_builder_basic() {
        // 这里可以添加单元测试
    }
}
