//! 存储过程配置和管理

use crate::stored_procedure::types::*;
use crate::error::QuickDbResult;
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
            },
        }
    }

    /// 添加依赖表（通过模型类型）
    pub fn with_dependency<T: crate::model::Model>(mut self) -> Self {
        // 调用 T::meta() 会自动触发模型注册
        let model_meta = T::meta();
        println!("📝 [DEBUG] with_dependency 存储模型元数据，模型: {}, 字段数: {}",
                 model_meta.collection_name, model_meta.fields.len());
        self.config.dependencies.push(model_meta);
        self
    }

    /// 添加JOIN关系
    pub fn with_join<T: crate::model::Model>(mut self, local_field: &str, foreign_field: &str, join_type: JoinType) -> Self {
        let model_meta = T::meta();
        println!("📝 [DEBUG] with_join 调用 T::meta()，模型: {}", model_meta.collection_name);
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
        self.config.fields.insert(field_name.to_string(), expression.to_string());
        self
    }

    /// 构建配置
    pub fn build(self) -> StoredProcedureConfig {
        self.config
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

        if self.fields.is_empty() {
            return Err(crate::error::QuickDbError::ValidationError {
                field: "fields".to_string(),
                message: "至少需要一个字段".to_string(),
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
}