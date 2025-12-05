//! 存储过程配置和管理

use crate::error::QuickDbResult;
use crate::stored_procedure::types::*;
use std::collections::HashMap;

/// 存储过程构建器
pub struct StoredProcedureBuilder {
    config: StoredProcedureConfig,
}

impl StoredProcedureBuilder {
    /// 创建新的存储过程构建器
    pub fn new(name: &str, database: &str) -> Self {
        Self {
            config: StoredProcedureConfig {
                database: database.to_string(),
                dependencies: Vec::new(),
                joins: Vec::new(),
                fields: HashMap::new(),
                procedure_name: name.to_string(),
                mongo_pipeline: None,
            },
        }
    }

    /// 添加依赖表（通过模型类型）
    pub fn with_dependency<T: crate::model::Model>(mut self) -> Self {
        // 调用 T::meta() 会自动触发模型注册
        let model_meta = T::meta();
        self.config.dependencies.push(model_meta);
        self
    }

    /// 添加JOIN关系
    pub fn with_join<T: crate::model::Model>(
        mut self,
        local_field: &str,
        foreign_field: &str,
        join_type: JoinType,
    ) -> Self {
        let model_meta = T::meta();
        self.config.joins.push(JoinRelation {
            table: model_meta.collection_name.clone(),
            local_field: local_field.to_string(),
            foreign_field: foreign_field.to_string(),
            join_type,
        });
        self
    }

    /// 添加字段映射
    pub fn with_field(mut self, field_name: &str, expression: &str) -> Self {
        self.config
            .fields
            .insert(field_name.to_string(), expression.to_string());
        self
    }

    /// MongoDB专用：添加聚合管道操作
    pub fn with_mongo_pipeline(
        mut self,
        operations: Vec<crate::stored_procedure::types::MongoAggregationOperation>,
    ) -> Self {
        self.config.mongo_pipeline = Some(operations);
        self
    }

    /// MongoDB专用：开始构建聚合管道
    pub fn with_mongo_aggregation(self) -> MongoPipelineBuilder {
        MongoPipelineBuilder::new(self)
    }

    /// 构建配置
    pub fn build(self) -> StoredProcedureConfig {
        self.config
    }
}

/// MongoDB聚合管道专用构建器
pub struct MongoPipelineBuilder {
    stored_procedure_builder: StoredProcedureBuilder,
    pipeline: Vec<crate::stored_procedure::types::MongoAggregationOperation>,
}

impl MongoPipelineBuilder {
    /// 创建新的聚合管道构建器
    pub fn new(stored_procedure_builder: StoredProcedureBuilder) -> Self {
        Self {
            stored_procedure_builder,
            pipeline: Vec::new(),
        }
    }

    /// 添加字段投影
    pub fn project(
        mut self,
        fields: Vec<(&str, crate::stored_procedure::types::MongoFieldExpression)>,
    ) -> Self {
        let mut field_map = std::collections::HashMap::new();
        for (name, expr) in fields {
            field_map.insert(name.to_string(), expr);
        }
        self.pipeline.push(
            crate::stored_procedure::types::MongoAggregationOperation::Project {
                fields: field_map,
            },
        );
        self
    }

    /// 添加匹配条件
    pub fn match_condition(
        mut self,
        conditions: Vec<crate::stored_procedure::types::MongoCondition>,
    ) -> Self {
        self.pipeline
            .push(crate::stored_procedure::types::MongoAggregationOperation::Match { conditions });
        self
    }

    /// 添加Lookup连接
    pub fn lookup(
        mut self,
        from: &str,
        local_field: &str,
        foreign_field: &str,
        as_field: &str,
    ) -> Self {
        self.pipeline.push(
            crate::stored_procedure::types::MongoAggregationOperation::Lookup {
                from: from.to_string(),
                local_field: local_field.to_string(),
                foreign_field: foreign_field.to_string(),
                as_field: as_field.to_string(),
            },
        );
        self
    }

    /// 展开数组
    pub fn unwind(mut self, field: &str) -> Self {
        self.pipeline.push(
            crate::stored_procedure::types::MongoAggregationOperation::Unwind {
                field: field.to_string(),
            },
        );
        self
    }

    /// 分组操作
    pub fn group(
        mut self,
        id: crate::stored_procedure::types::MongoGroupKey,
        accumulators: Vec<(&str, crate::stored_procedure::types::MongoAccumulator)>,
    ) -> Self {
        let mut acc_map = std::collections::HashMap::new();
        for (name, acc) in accumulators {
            acc_map.insert(name.to_string(), acc);
        }
        self.pipeline.push(
            crate::stored_procedure::types::MongoAggregationOperation::Group {
                id,
                accumulators: acc_map,
            },
        );
        self
    }

    /// 排序
    pub fn sort(mut self, fields: Vec<(&str, crate::types::SortDirection)>) -> Self {
        let sort_fields: Vec<(String, crate::types::SortDirection)> = fields
            .into_iter()
            .map(|(name, dir)| (name.to_string(), dir))
            .collect();
        self.pipeline.push(
            crate::stored_procedure::types::MongoAggregationOperation::Sort {
                fields: sort_fields,
            },
        );
        self
    }

    /// 限制数量
    pub fn limit(mut self, count: i64) -> Self {
        self.pipeline
            .push(crate::stored_procedure::types::MongoAggregationOperation::Limit { count });
        self
    }

    /// 跳过数量
    pub fn skip(mut self, count: i64) -> Self {
        self.pipeline
            .push(crate::stored_procedure::types::MongoAggregationOperation::Skip { count });
        self
    }

    /// 添加字段
    pub fn add_fields(
        mut self,
        fields: Vec<(&str, crate::stored_procedure::types::MongoFieldExpression)>,
    ) -> Self {
        let mut field_map = std::collections::HashMap::new();
        for (name, expr) in fields {
            field_map.insert(name.to_string(), expr);
        }
        self.pipeline.push(
            crate::stored_procedure::types::MongoAggregationOperation::AddFields {
                fields: field_map,
            },
        );
        self
    }

    /// 完成管道构建并返回存储过程构建器
    pub fn done(self) -> StoredProcedureBuilder {
        self.stored_procedure_builder
            .with_mongo_pipeline(self.pipeline)
    }

    /// 添加占位符（用于动态参数替换）
    pub fn add_placeholder(mut self, placeholder_type: &str) -> Self {
        self.pipeline.push(
            crate::stored_procedure::types::MongoAggregationOperation::Placeholder {
                placeholder_type: placeholder_type.to_string(),
            },
        );
        self
    }

    /// 添加多个常用占位符
    pub fn with_common_placeholders(self) -> Self {
        self.add_placeholder("where")
            .add_placeholder("group_by")
            .add_placeholder("having")
            .add_placeholder("order_by")
            .add_placeholder("limit")
            .add_placeholder("offset")
    }

    /// 直接构建最终的存储过程配置
    pub fn build(self) -> StoredProcedureConfig {
        self.done().build()
    }
}

/// MongoDB聚合表达式的便捷构建函数
impl crate::stored_procedure::types::MongoFieldExpression {
    /// 创建字段引用
    pub fn field(field: &str) -> Self {
        Self::Field(field.to_string())
    }

    /// 创建常量值
    pub fn constant(value: crate::types::DataValue) -> Self {
        Self::Constant(value)
    }

    /// 创建数组大小表达式
    pub fn size(field: &str) -> Self {
        Self::Aggregate(
            crate::stored_procedure::types::MongoAggregateExpression::Size {
                field: field.to_string(),
            },
        )
    }

    /// 创建求和表达式
    pub fn sum(field: &str) -> Self {
        Self::Aggregate(
            crate::stored_procedure::types::MongoAggregateExpression::Sum {
                field: field.to_string(),
            },
        )
    }

    /// 创建平均值表达式
    pub fn avg(field: &str) -> Self {
        Self::Aggregate(
            crate::stored_procedure::types::MongoAggregateExpression::Avg {
                field: field.to_string(),
            },
        )
    }

    /// 创建最大值表达式
    pub fn max(field: &str) -> Self {
        Self::Aggregate(
            crate::stored_procedure::types::MongoAggregateExpression::Max {
                field: field.to_string(),
            },
        )
    }

    /// 创建最小值表达式
    pub fn min(field: &str) -> Self {
        Self::Aggregate(
            crate::stored_procedure::types::MongoAggregateExpression::Min {
                field: field.to_string(),
            },
        )
    }

    /// 创建IfNull表达式
    pub fn if_null(
        field: &str,
        default: crate::stored_procedure::types::MongoFieldExpression,
    ) -> Self {
        Self::Aggregate(
            crate::stored_procedure::types::MongoAggregateExpression::IfNull {
                field: field.to_string(),
                default: Box::new(default),
            },
        )
    }
}

/// MongoDB条件的便捷构建函数
impl crate::stored_procedure::types::MongoCondition {
    /// 等于条件
    pub fn eq(field: &str, value: crate::types::DataValue) -> Self {
        Self::Eq {
            field: field.to_string(),
            value,
        }
    }

    /// 不等于条件
    pub fn ne(field: &str, value: crate::types::DataValue) -> Self {
        Self::Ne {
            field: field.to_string(),
            value,
        }
    }

    /// 大于条件
    pub fn gt(field: &str, value: crate::types::DataValue) -> Self {
        Self::Gt {
            field: field.to_string(),
            value,
        }
    }

    /// 大于等于条件
    pub fn gte(field: &str, value: crate::types::DataValue) -> Self {
        Self::Gte {
            field: field.to_string(),
            value,
        }
    }

    /// 小于条件
    pub fn lt(field: &str, value: crate::types::DataValue) -> Self {
        Self::Lt {
            field: field.to_string(),
            value,
        }
    }

    /// 小于等于条件
    pub fn lte(field: &str, value: crate::types::DataValue) -> Self {
        Self::Lte {
            field: field.to_string(),
            value,
        }
    }

    /// AND条件
    pub fn and(conditions: Vec<Self>) -> Self {
        Self::And { conditions }
    }

    /// OR条件
    pub fn or(conditions: Vec<Self>) -> Self {
        Self::Or { conditions }
    }

    /// 字段存在条件
    pub fn exists(field: &str, exists: bool) -> Self {
        Self::Exists {
            field: field.to_string(),
            exists,
        }
    }

    /// 正则表达式条件
    pub fn regex(field: &str, pattern: &str) -> Self {
        Self::Regex {
            field: field.to_string(),
            pattern: pattern.to_string(),
        }
    }
}

impl StoredProcedureConfig {
    /// 创建存储过程配置构建器
    pub fn builder(name: &str, database: &str) -> StoredProcedureBuilder {
        StoredProcedureBuilder::new(name, database)
    }

    /// 验证配置是否有效
    pub fn validate(&self) -> QuickDbResult<()> {
        if self.procedure_name.is_empty() {
            return Err(crate::error::QuickDbError::ValidationError {
                field: "procedure_name".to_string(),
                message: "存储过程名称不能为空".to_string(),
            });
        }

        if self.database.is_empty() {
            return Err(crate::error::QuickDbError::ValidationError {
                field: "database".to_string(),
                message: "数据库别名不能为空".to_string(),
            });
        }

        // 验证数据库类型与配置的匹配性
        self.validate_database_type_compatibility()?;

        // 如果没有使用MongoDB聚合管道，则必须要有传统字段映射
        if self.mongo_pipeline.is_none() && self.fields.is_empty() {
            return Err(crate::error::QuickDbError::ValidationError {
                field: "fields".to_string(),
                message: "至少需要一个字段或聚合管道操作".to_string(),
            });
        }

        // 验证JOIN关系中的字段是否存在
        for join in &self.joins {
            if join.local_field.is_empty() || join.foreign_field.is_empty() {
                return Err(crate::error::QuickDbError::ValidationError {
                    field: "join_fields".to_string(),
                    message: "JOIN字段不能为空".to_string(),
                });
            }
        }

        Ok(())
    }

    /// 验证数据库类型与配置的兼容性
    fn validate_database_type_compatibility(&self) -> QuickDbResult<()> {
        use crate::manager::get_global_pool_manager;

        // 获取数据库类型以验证配置兼容性
        let db_type = get_global_pool_manager()
            .get_database_type(&self.database)
            .map_err(|_| crate::error::QuickDbError::ValidationError {
                field: "database".to_string(),
                message: format!("数据库别名 '{}' 不存在", self.database),
            })?;

        match db_type {
            crate::types::DatabaseType::MongoDB => {
                // MongoDB配置验证
                if self.mongo_pipeline.is_none() && self.fields.is_empty() {
                    return Err(crate::error::QuickDbError::ValidationError {
                        field: "mongo_config".to_string(),
                        message: "MongoDB存储过程必须使用聚合管道或字段映射".to_string(),
                    });
                }

                // 检查是否在MongoDB中误用了SQL特有的复杂JOIN配置
                if self.joins.len() > 1 {
                    rat_logger::warn!(
                        "警告：MongoDB对复杂JOIN支持有限，建议使用聚合管道中的$lookup操作"
                    );
                }
            }
            crate::types::DatabaseType::SQLite
            | crate::types::DatabaseType::MySQL
            | crate::types::DatabaseType::PostgreSQL => {
                // SQL系数据库配置验证
                if self.mongo_pipeline.is_some() {
                    return Err(crate::error::QuickDbError::ValidationError {
                        field: "mongo_pipeline".to_string(),
                        message: format!(
                            "{} 不支持MongoDB聚合管道，请使用传统字段映射和JOIN配置",
                            match db_type {
                                crate::types::DatabaseType::SQLite => "SQLite",
                                crate::types::DatabaseType::MySQL => "MySQL",
                                crate::types::DatabaseType::PostgreSQL => "PostgreSQL",
                                _ => "该数据库",
                            }
                        ),
                    });
                }

                // SQL数据库必须要有字段映射
                if self.fields.is_empty() {
                    return Err(crate::error::QuickDbError::ValidationError {
                        field: "fields".to_string(),
                        message: format!(
                            "{} 存储过程必须定义字段映射",
                            match db_type {
                                crate::types::DatabaseType::SQLite => "SQLite",
                                crate::types::DatabaseType::MySQL => "MySQL",
                                crate::types::DatabaseType::PostgreSQL => "PostgreSQL",
                                _ => "该数据库",
                            }
                        ),
                    });
                }
            }
        }

        Ok(())
    }
}
