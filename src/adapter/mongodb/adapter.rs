//! MongoDB适配器核心模块
//!
//! 提供MongoDB适配器的核心结构定义和基础功能

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use rat_logger::{debug, info};

/// MongoDB适配器
pub struct MongoAdapter {
    /// 表创建锁，防止重复创建表
    creation_locks: Arc<Mutex<HashMap<String, ()>>>,
    /// 存储过程映射表，存储已创建的存储过程信息
    pub(crate) stored_procedures: Arc<Mutex<HashMap<String, crate::stored_procedure::StoredProcedureInfo>>>,
}

impl MongoAdapter {
    /// 创建新的MongoDB适配器
    pub fn new() -> Self {
        Self {
            creation_locks: Arc::new(Mutex::new(HashMap::new())),
            stored_procedures: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 获取表创建锁
    pub(crate) async fn acquire_table_lock(&self, table: &str) -> tokio::sync::MutexGuard<'_, HashMap<String, ()>> {
        let mut locks = self.creation_locks.lock().await;
        if !locks.contains_key(table) {
            locks.insert(table.to_string(), ());
            debug!("🔒 获取表 {} 的创建锁", table);
        }
        locks
    }

    /// 释放表创建锁
    pub(crate) async fn release_table_lock(&self, table: &str, mut locks: tokio::sync::MutexGuard<'_, HashMap<String, ()>>) {
        locks.remove(table);
        debug!("🔓 释放表 {} 的创建锁", table);
    }

    /// 生成存储过程的MongoDB聚合管道（MongoDB使用聚合管道模拟存储过程逻辑）
    pub async fn generate_stored_procedure_pipeline(
        &self,
        config: &crate::stored_procedure::StoredProcedureConfig,
    ) -> crate::error::QuickDbResult<String> {
        use serde_json::json;

        // 优先使用新的聚合管道API
        if let Some(pipeline) = &config.mongo_pipeline {
            return self.convert_pipeline_to_json(pipeline, config).await;
        }

        // 如果没有新的聚合管道，使用旧的基于fields和joins的方法
        self.generate_legacy_pipeline(config).await
    }

    /// 将新的聚合管道API转换为JSON
    async fn convert_pipeline_to_json(
        &self,
        pipeline: &[crate::stored_procedure::types::MongoAggregationOperation],
        config: &crate::stored_procedure::StoredProcedureConfig,
    ) -> crate::error::QuickDbResult<String> {
        use serde_json::json;
        let mut pipeline_stages = Vec::new();

        for operation in pipeline {
            let stage = match operation {
                crate::stored_procedure::types::MongoAggregationOperation::Project { fields } => {
                    let mut field_map = serde_json::Map::new();
                    for (name, expr) in fields {
                        field_map.insert(name.clone(), self.convert_field_expression_to_json(expr));
                    }
                    json!({ "$project": field_map })
                },
                crate::stored_procedure::types::MongoAggregationOperation::Match { conditions } => {
                    let mut cond_array = Vec::new();
                    for condition in conditions {
                        cond_array.push(self.convert_condition_to_json(condition));
                    }
                    let match_condition = if cond_array.len() == 1 {
                        cond_array.into_iter().next().unwrap()
                    } else {
                        json!({ "$and": cond_array })
                    };
                    json!({ "$match": match_condition })
                },
                crate::stored_procedure::types::MongoAggregationOperation::Lookup { from, local_field, foreign_field, as_field } => {
                    json!({
                        "$lookup": {
                            "from": from,
                            "localField": local_field,
                            "foreignField": foreign_field,
                            "as": as_field
                        }
                    })
                },
                crate::stored_procedure::types::MongoAggregationOperation::Unwind { field } => {
                    json!({ "$unwind": format!("${}", field) })
                },
                crate::stored_procedure::types::MongoAggregationOperation::Group { id, accumulators } => {
                    let mut acc_map = serde_json::Map::new();
                    for (name, acc) in accumulators {
                        acc_map.insert(name.clone(), self.convert_accumulator_to_json(acc));
                    }
                    // 构建正确的$group语法，不使用accumulators包装
                    let mut group_obj = serde_json::Map::new();
                    group_obj.insert("_id".to_string(), self.convert_group_key_to_json(id));

                    // 将累加器字段直接添加到group对象中
                    for (key, value) in acc_map {
                        group_obj.insert(key, value);
                    }

                    json!({ "$group": group_obj })
                },
                crate::stored_procedure::types::MongoAggregationOperation::Sort { fields } => {
                    let sort_fields: Vec<serde_json::Value> = fields.iter()
                        .map(|(name, direction)| {
                            match direction {
                                crate::types::SortDirection::Asc => json!({ name: 1 }),
                                crate::types::SortDirection::Desc => json!({ name: -1 }),
                            }
                        })
                        .collect();
                    json!({ "$sort": sort_fields })
                },
                crate::stored_procedure::types::MongoAggregationOperation::Limit { count } => {
                    json!({ "$limit": count })
                },
                crate::stored_procedure::types::MongoAggregationOperation::Skip { count } => {
                    json!({ "$skip": count })
                },
                crate::stored_procedure::types::MongoAggregationOperation::AddFields { fields } => {
                    let mut field_map = serde_json::Map::new();
                    for (name, expr) in fields {
                        field_map.insert(name.clone(), self.convert_field_expression_to_json(expr));
                    }
                    json!({ "$addFields": field_map })
                },
                crate::stored_procedure::types::MongoAggregationOperation::Count { field } => {
                    json!({ "$count": field })
                },
                crate::stored_procedure::types::MongoAggregationOperation::Placeholder { placeholder_type } => {
                    json!({
                        "$addFields": {
                            format!("_{}_PLACEHOLDER", placeholder_type.to_uppercase()): format!("{{{}}}", placeholder_type.to_uppercase())
                        }
                    })
                },
            };
            pipeline_stages.push(stage);
        }

        // 确定主集合
        let base_collection = config.dependencies.first()
            .map(|model_meta| &model_meta.collection_name)
            .ok_or_else(|| crate::error::QuickDbError::ValidationError {
                field: "dependencies".to_string(),
                message: "至少需要一个依赖集合作为主集合".to_string(),
            })?;

        // 生成最终的聚合管道JSON
        let pipeline_json = serde_json::to_string_pretty(&json!({
            "collection": base_collection,
            "pipeline": pipeline_stages
        })).map_err(|e| crate::error::QuickDbError::SerializationError {
            message: format!("序列化MongoDB聚合管道失败: {}", e),
        })?;

        rat_logger::info!("生成的MongoDB存储过程聚合管道: {}", pipeline_json);
        Ok(pipeline_json)
    }

    /// 转换字段表达式为JSON
    fn convert_field_expression_to_json(
        &self,
        expr: &crate::stored_procedure::types::MongoFieldExpression,
    ) -> serde_json::Value {
        use serde_json::json;
        match expr {
            crate::stored_procedure::types::MongoFieldExpression::Field(field) => {
                json!(format!("${}", field))
            },
            crate::stored_procedure::types::MongoFieldExpression::Constant(value) => {
                match value {
                    crate::types::DataValue::String(s) => json!(s),
                    crate::types::DataValue::Int(i) => json!(i),
                    crate::types::DataValue::Float(f) => json!(f),
                    crate::types::DataValue::Bool(b) => json!(b),
                    crate::types::DataValue::Null => json!(null),
                    _ => json!(value.to_string()),
                }
            },
            crate::stored_procedure::types::MongoFieldExpression::Aggregate(agg_expr) => {
                match agg_expr {
                    crate::stored_procedure::types::MongoAggregateExpression::Size { field } => {
                        json!({ "$size": format!("${}", field) })
                    },
                    crate::stored_procedure::types::MongoAggregateExpression::Sum { field } => {
                        json!({ "$sum": format!("${}", field) })
                    },
                    crate::stored_procedure::types::MongoAggregateExpression::Avg { field } => {
                        json!({ "$avg": format!("${}", field) })
                    },
                    crate::stored_procedure::types::MongoAggregateExpression::Max { field } => {
                        json!({ "$max": format!("${}", field) })
                    },
                    crate::stored_procedure::types::MongoAggregateExpression::Min { field } => {
                        json!({ "$min": format!("${}", field) })
                    },
                    crate::stored_procedure::types::MongoAggregateExpression::IfNull { field, default } => {
                        json!({
                            "$ifNull": [
                                format!("${}", field),
                                self.convert_field_expression_to_json(default)
                            ]
                        })
                    },
                    crate::stored_procedure::types::MongoAggregateExpression::Condition { if_condition, then_expr, else_expr } => {
                        json!({
                            "$cond": {
                                "if": self.convert_condition_to_json(if_condition),
                                "then": self.convert_field_expression_to_json(then_expr),
                                "else": self.convert_field_expression_to_json(else_expr)
                            }
                        })
                    },
                }
            },
        }
    }

    /// 转换条件为JSON
    fn convert_condition_to_json(
        &self,
        condition: &crate::stored_procedure::types::MongoCondition,
    ) -> serde_json::Value {
        use serde_json::json;
        match condition {
            crate::stored_procedure::types::MongoCondition::Eq { field, value } => {
                json!({ field: self.data_value_to_json(value) })
            },
            crate::stored_procedure::types::MongoCondition::Ne { field, value } => {
                json!({ field: { "$ne": self.data_value_to_json(value) } })
            },
            crate::stored_procedure::types::MongoCondition::Gt { field, value } => {
                json!({ field: { "$gt": self.data_value_to_json(value) } })
            },
            crate::stored_procedure::types::MongoCondition::Gte { field, value } => {
                json!({ field: { "$gte": self.data_value_to_json(value) } })
            },
            crate::stored_procedure::types::MongoCondition::Lt { field, value } => {
                json!({ field: { "$lt": self.data_value_to_json(value) } })
            },
            crate::stored_procedure::types::MongoCondition::Lte { field, value } => {
                json!({ field: { "$lte": self.data_value_to_json(value) } })
            },
            crate::stored_procedure::types::MongoCondition::In { field, values } => {
                let json_values: Vec<serde_json::Value> = values.iter()
                    .map(|v| self.data_value_to_json(v))
                    .collect();
                json!({ field: { "$in": json_values } })
            },
            crate::stored_procedure::types::MongoCondition::And { conditions } => {
                let json_conditions: Vec<serde_json::Value> = conditions.iter()
                    .map(|c| self.convert_condition_to_json(c))
                    .collect();
                json!({ "$and": json_conditions })
            },
            crate::stored_procedure::types::MongoCondition::Or { conditions } => {
                let json_conditions: Vec<serde_json::Value> = conditions.iter()
                    .map(|c| self.convert_condition_to_json(c))
                    .collect();
                json!({ "$or": json_conditions })
            },
            crate::stored_procedure::types::MongoCondition::Exists { field, exists } => {
                json!({ field: { "$exists": exists } })
            },
            crate::stored_procedure::types::MongoCondition::Regex { field, pattern } => {
                json!({ field: { "$regex": pattern } })
            },
            _ => json!(null),
        }
    }

    /// 转换分组键为JSON
    fn convert_group_key_to_json(
        &self,
        key: &crate::stored_procedure::types::MongoGroupKey,
    ) -> serde_json::Value {
        use serde_json::json;
        match key {
            crate::stored_procedure::types::MongoGroupKey::Field(field) => {
                json!(format!("${}", field))
            },
            crate::stored_procedure::types::MongoGroupKey::Null => {
                json!(null)
            },
            crate::stored_procedure::types::MongoGroupKey::Multiple(fields) => {
                let mut field_map = serde_json::Map::new();
                for field in fields {
                    field_map.insert(field.clone(), serde_json::Value::String(format!("${}", field)));
                }
                json!(field_map)
            },
        }
    }

    /// 转换累加器为JSON
    fn convert_accumulator_to_json(
        &self,
        acc: &crate::stored_procedure::types::MongoAccumulator,
    ) -> serde_json::Value {
        use serde_json::json;
        match acc {
            crate::stored_procedure::types::MongoAccumulator::Count => {
                json!({ "$sum": 1 })
            },
            crate::stored_procedure::types::MongoAccumulator::Sum { field } => {
                json!({ "$sum": format!("${}", field) })
            },
            crate::stored_procedure::types::MongoAccumulator::Avg { field } => {
                json!({ "$avg": format!("${}", field) })
            },
            crate::stored_procedure::types::MongoAccumulator::Max { field } => {
                json!({ "$max": format!("${}", field) })
            },
            crate::stored_procedure::types::MongoAccumulator::Min { field } => {
                json!({ "$min": format!("${}", field) })
            },
            crate::stored_procedure::types::MongoAccumulator::Push { field } => {
                json!({ "$push": format!("${}", field) })
            },
            crate::stored_procedure::types::MongoAccumulator::AddToSet { field } => {
                json!({ "$addToSet": format!("${}", field) })
            },
        }
    }

    /// 转换DataValue为JSON
    fn data_value_to_json(&self, value: &crate::types::DataValue) -> serde_json::Value {
        use serde_json::json;
        match value {
            crate::types::DataValue::String(s) => json!(s),
            crate::types::DataValue::Int(i) => json!(i),
            crate::types::DataValue::Float(f) => json!(f),
            crate::types::DataValue::Bool(b) => json!(b),
            crate::types::DataValue::Null => json!(null),
            crate::types::DataValue::Array(arr) => {
                let json_array: Vec<serde_json::Value> = arr.iter()
                    .map(|v| self.data_value_to_json(v))
                    .collect();
                json!(json_array)
            },
            crate::types::DataValue::Object(obj) => {
                let json_obj: serde_json::Map<String, serde_json::Value> = obj.iter()
                    .map(|(k, v)| (k.clone(), self.data_value_to_json(v)))
                    .collect();
                json!(json_obj)
            },
            _ => json!(value.to_string()),
        }
    }

    /// 生成旧版本基于fields和joins的聚合管道（向后兼容）
    async fn generate_legacy_pipeline(
        &self,
        config: &crate::stored_procedure::StoredProcedureConfig,
    ) -> crate::error::QuickDbResult<String> {
        use crate::stored_procedure::JoinType;
        use serde_json::json;

        // 1. 构建投影字段
        let mut projection = serde_json::Map::new();
        for (alias, expr) in &config.fields {
            // 简单处理表达式，直接作为字段映射
            if alias == expr {
                // 如果别名和表达式相同，可能是一个字段名
                projection.insert(alias.clone(), json!(1));
            } else {
                // 否则作为表达式处理
                projection.insert(alias.clone(), json!(expr));
            }
        }

        // 2. 确定主集合
        let base_collection = config.dependencies.first()
            .map(|model_meta| &model_meta.collection_name)
            .ok_or_else(|| crate::error::QuickDbError::ValidationError {
                field: "dependencies".to_string(),
                message: "至少需要一个依赖集合作为主集合".to_string(),
            })?;

        // 3. 构建Lookup阶段（对应SQL的JOIN）
        let mut pipeline_stages = Vec::new();

        // 首先添加投影阶段
        pipeline_stages.push(json!({
            "$project": projection
        }));

        // 4. 处理JOIN关系，转换为MongoDB的$lookup
        for join in &config.joins {
            let lookup_stage = match join.join_type {
                JoinType::Inner => json!({
                    "$lookup": {
                        "from": join.table,
                        "localField": join.local_field,
                        "foreignField": join.foreign_field,
                        "as": format!("{}_joined", join.table)
                    }
                }),
                JoinType::Left => json!({
                    "$lookup": {
                        "from": join.table,
                        "localField": join.local_field,
                        "foreignField": join.foreign_field,
                        "as": format!("{}_joined", join.table)
                    }
                }),
                JoinType::Right => {
                    // MongoDB的右连接需要特殊处理，这里简化为左连接
                    rat_logger::info!("警告：MongoDB不支持RIGHT JOIN，使用LEFT JOIN作为替代");
                    json!({
                        "$lookup": {
                            "from": join.table,
                            "localField": join.local_field,
                            "foreignField": join.foreign_field,
                            "as": format!("{}_joined", join.table)
                        }
                    })
                },
                JoinType::Full => {
                    // MongoDB的全外连接需要特殊处理，这里简化为左连接
                    rat_logger::info!("警告：MongoDB不支持FULL OUTER JOIN，使用LEFT JOIN作为替代");
                    json!({
                        "$lookup": {
                            "from": join.table,
                            "localField": join.local_field,
                            "foreignField": join.foreign_field,
                            "as": format!("{}_joined", join.table)
                        }
                    })
                },
            };
            pipeline_stages.push(lookup_stage);

            // 添加$unwind阶段来展开数组
            pipeline_stages.push(json!({
                "$unwind": format!("${}_joined", join.table)
            }));
        }

        // 5. 添加占位符标记阶段
        pipeline_stages.push(json!({
            "$addFields": {
                "_WHERE_PLACEHOLDER": "{WHERE}",
                "_GROUP_BY_PLACEHOLDER": "{GROUP_BY}",
                "_HAVING_PLACEHOLDER": "{HAVING}",
                "_ORDER_BY_PLACEHOLDER": "{ORDER_BY}",
                "_LIMIT_PLACEHOLDER": "{LIMIT}",
                "_OFFSET_PLACEHOLDER": "{OFFSET}"
            }
        }));

        // 6. 生成最终的聚合管道JSON
        let pipeline_json = serde_json::to_string_pretty(&json!({
            "collection": base_collection,
            "pipeline": pipeline_stages
        })).map_err(|e| crate::error::QuickDbError::SerializationError {
            message: format!("序列化MongoDB聚合管道失败: {}", e),
        })?;

        rat_logger::info!("生成的MongoDB存储过程聚合管道: {}", pipeline_json);
        Ok(pipeline_json)
    }

    /// 执行MongoDB聚合管道查询
    pub async fn aggregate_query(
        &self,
        connection: &crate::pool::DatabaseConnection,
        collection_name: &str,
        pipeline_stages: Vec<serde_json::Value>,
    ) -> crate::error::QuickDbResult<Vec<std::collections::HashMap<String, crate::types::DataValue>>> {
        use mongodb::bson::Document;
        use std::collections::HashMap;

        if let crate::pool::DatabaseConnection::MongoDB(db) = connection {
            let collection = crate::adapter::mongodb::utils::get_collection(self, db, collection_name);

            // 将JSON阶段转换为MongoDB Document
            let pipeline_docs: Result<Vec<Document>, _> = pipeline_stages.iter()
                .map(|stage| mongodb::bson::to_document(stage))
                .collect();

            let pipeline_docs = pipeline_docs.map_err(|e| crate::error::QuickDbError::SerializationError {
                message: format!("聚合管道序列化失败: {}", e),
            })?;

            rat_logger::debug!("执行MongoDB聚合管道: 集合={}, 阶段数={}", collection_name, pipeline_docs.len());

            // 执行聚合查询
            let mut cursor = collection.aggregate(pipeline_docs, None)
                .await
                .map_err(|e| crate::error::QuickDbError::QueryError {
                    message: format!("MongoDB聚合查询失败: {}", e),
                })?;

            let mut results = Vec::new();
            while cursor.advance().await.map_err(|e| crate::error::QuickDbError::QueryError {
                message: format!("MongoDB聚合游标遍历失败: {}", e),
            })? {
                let doc = cursor.deserialize_current().map_err(|e| crate::error::QuickDbError::QueryError {
                    message: format!("MongoDB聚合文档反序列化失败: {}", e),
                })?;

                // 将BSON文档转换为DataValue映射
                let data_map = crate::adapter::mongodb::utils::document_to_data_map(self, &doc)?;
                results.push(data_map);
            }

            rat_logger::debug!("MongoDB聚合查询完成，返回 {} 条记录", results.len());
            Ok(results)
        } else {
            Err(crate::error::QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望MongoDB连接".to_string(),
            })
        }
    }
}
