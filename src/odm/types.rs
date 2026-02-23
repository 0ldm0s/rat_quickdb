//! # ODM请求类型定义

use crate::error::QuickDbResult;
use crate::types::*;
use std::collections::HashMap;
use tokio::sync::oneshot;

/// ODM操作请求类型
#[derive(Debug)]
pub enum OdmRequest {
    Create {
        collection: String,
        data: HashMap<String, DataValue>,
        alias: Option<String>,
        response: oneshot::Sender<QuickDbResult<DataValue>>,
    },
    FindById {
        collection: String,
        id: String,
        alias: Option<String>,
        response: oneshot::Sender<QuickDbResult<Option<DataValue>>>,
    },
    Find {
        collection: String,
        conditions: Vec<QueryConditionWithConfig>,
        options: Option<QueryOptions>,
        alias: Option<String>,
        response: oneshot::Sender<QuickDbResult<Vec<DataValue>>>,
    },
    FindWithCacheControl {
        collection: String,
        conditions: Vec<QueryConditionWithConfig>,
        options: Option<QueryOptions>,
        alias: Option<String>,
        bypass_cache: bool,
        response: oneshot::Sender<QuickDbResult<Vec<DataValue>>>,
    },
    FindWithGroups {
        collection: String,
        condition_groups: Vec<QueryConditionGroupWithConfig>,
        options: Option<QueryOptions>,
        alias: Option<String>,
        response: oneshot::Sender<QuickDbResult<Vec<DataValue>>>,
    },
    FindWithGroupsWithCacheControl {
        collection: String,
        condition_groups: Vec<QueryConditionGroupWithConfig>,
        options: Option<QueryOptions>,
        alias: Option<String>,
        bypass_cache: bool,
        response: oneshot::Sender<QuickDbResult<Vec<DataValue>>>,
    },
    Update {
        collection: String,
        conditions: Vec<QueryConditionWithConfig>,
        updates: HashMap<String, DataValue>,
        alias: Option<String>,
        response: oneshot::Sender<QuickDbResult<u64>>,
    },
    UpdateWithOperations {
        collection: String,
        conditions: Vec<QueryConditionWithConfig>,
        operations: Vec<crate::types::UpdateOperation>,
        alias: Option<String>,
        response: oneshot::Sender<QuickDbResult<u64>>,
    },
    UpdateById {
        collection: String,
        id: String,
        updates: HashMap<String, DataValue>,
        alias: Option<String>,
        response: oneshot::Sender<QuickDbResult<bool>>,
    },
    Delete {
        collection: String,
        conditions: Vec<QueryConditionWithConfig>,
        alias: Option<String>,
        response: oneshot::Sender<QuickDbResult<u64>>,
    },
    DeleteById {
        collection: String,
        id: String,
        alias: Option<String>,
        response: oneshot::Sender<QuickDbResult<bool>>,
    },
    Count {
        collection: String,
        conditions: Vec<QueryConditionWithConfig>,
        alias: Option<String>,
        response: oneshot::Sender<QuickDbResult<u64>>,
    },
    CountWithGroups {
        collection: String,
        condition_groups: Vec<QueryConditionGroupWithConfig>,
        alias: Option<String>,
        response: oneshot::Sender<QuickDbResult<u64>>,
    },
    GetServerVersion {
        alias: Option<String>,
        response: oneshot::Sender<QuickDbResult<String>>,
    },
    CreateStoredProcedure {
        config: crate::stored_procedure::StoredProcedureConfig,
        response:
            oneshot::Sender<QuickDbResult<crate::stored_procedure::StoredProcedureCreateResult>>,
    },
    ExecuteStoredProcedure {
        procedure_name: String,
        database_alias: Option<String>,
        params: Option<std::collections::HashMap<String, crate::types::DataValue>>,
        response:
            oneshot::Sender<QuickDbResult<crate::stored_procedure::StoredProcedureQueryResult>>,
    },
}
