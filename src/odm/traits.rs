//! # ODM操作接口定义

use crate::error::{QuickDbResult};
use crate::types::*;
use async_trait::async_trait;
use std::collections::HashMap;
use tokio::sync::oneshot;

/// ODM操作接口
#[async_trait]
pub trait OdmOperations {
    /// 创建记录
    async fn create(
        &self,
        collection: &str,
        data: HashMap<String, DataValue>,
        alias: Option<&str>,
    ) -> QuickDbResult<DataValue>;

    /// 根据ID查找记录
    async fn find_by_id(
        &self,
        collection: &str,
        id: &str,
        alias: Option<&str>,
    ) -> QuickDbResult<Option<DataValue>>;

    /// 查找记录
    async fn find(
        &self,
        collection: &str,
        conditions: Vec<QueryCondition>,
        options: Option<QueryOptions>,
        alias: Option<&str>,
    ) -> QuickDbResult<Vec<DataValue>>;
    
    /// 使用条件组合查找记录（支持复杂OR/AND逻辑）
    async fn find_with_groups(
        &self,
        collection: &str,
        condition_groups: Vec<QueryConditionGroup>,
        options: Option<QueryOptions>,
        alias: Option<&str>,
    ) -> QuickDbResult<Vec<DataValue>>;
    
    /// 更新记录
    async fn update(
        &self,
        collection: &str,
        conditions: Vec<QueryCondition>,
        updates: HashMap<String, DataValue>,
        alias: Option<&str>,
    ) -> QuickDbResult<u64>;

    /// 使用操作数组更新记录
    async fn update_with_operations(
        &self,
        collection: &str,
        conditions: Vec<QueryCondition>,
        operations: Vec<crate::types::UpdateOperation>,
        alias: Option<&str>,
    ) -> QuickDbResult<u64>;

    /// 根据ID更新记录
    async fn update_by_id(
        &self,
        collection: &str,
        id: &str,
        updates: HashMap<String, DataValue>,
        alias: Option<&str>,
    ) -> QuickDbResult<bool>;
    
    /// 删除记录
    async fn delete(
        &self,
        collection: &str,
        conditions: Vec<QueryCondition>,
        alias: Option<&str>,
    ) -> QuickDbResult<u64>;
    
    /// 根据ID删除记录
    async fn delete_by_id(
        &self,
        collection: &str,
        id: &str,
        alias: Option<&str>,
    ) -> QuickDbResult<bool>;
    
    /// 统计记录数量
    async fn count(
        &self,
        collection: &str,
        conditions: Vec<QueryCondition>,
        alias: Option<&str>,
    ) -> QuickDbResult<u64>;
    
    /// 检查记录是否存在
    async fn exists(
        &self,
        collection: &str,
        conditions: Vec<QueryCondition>,
        alias: Option<&str>,
    ) -> QuickDbResult<bool>;

    /// 获取数据库服务器版本信息
    async fn get_server_version(
        &self,
        alias: Option<&str>,
    ) -> QuickDbResult<String>;

    /// 创建存储过程
    async fn create_stored_procedure(
        &self,
        config: crate::stored_procedure::StoredProcedureConfig,
    ) -> QuickDbResult<crate::stored_procedure::StoredProcedureCreateResult>;

    /// 执行存储过程查询
    async fn execute_stored_procedure(
        &self,
        procedure_name: &str,
        database_alias: Option<&str>,
        params: Option<std::collections::HashMap<String, crate::types::DataValue>>,
    ) -> QuickDbResult<crate::stored_procedure::StoredProcedureQueryResult>;
}

// 重新导出OdmRequest类型
pub use super::types::OdmRequest;
