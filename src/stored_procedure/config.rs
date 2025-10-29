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

    /// 添加依赖表
    pub fn with_dependency(mut self, table: &str) -> Self {
        self.config.dependencies.push(table.to_string());
        self
    }

    /// 添加JOIN关系
    pub fn with_join(mut self, table: &str, local_field: &str, foreign_field: &str, join_type: JoinType) -> Self {
        self.config.joins.push(JoinRelation {
            table: table.to_string(),
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