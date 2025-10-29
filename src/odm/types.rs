//! # ODM请求类型定义

use crate::error::QuickDbResult;
use crate::types::*;
use tokio::sync::oneshot;
use std::collections::HashMap;

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
        conditions: Vec<QueryCondition>,
        options: Option<QueryOptions>,
        alias: Option<String>,
        response: oneshot::Sender<QuickDbResult<Vec<DataValue>>>,
    },
    FindWithGroups {
        collection: String,
        condition_groups: Vec<QueryConditionGroup>,
        options: Option<QueryOptions>,
        alias: Option<String>,
        response: oneshot::Sender<QuickDbResult<Vec<DataValue>>>,
    },
    Update {
        collection: String,
        conditions: Vec<QueryCondition>,
        updates: HashMap<String, DataValue>,
        alias: Option<String>,
        response: oneshot::Sender<QuickDbResult<u64>>,
    },
    UpdateWithOperations {
        collection: String,
        conditions: Vec<QueryCondition>,
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
        conditions: Vec<QueryCondition>,
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
        conditions: Vec<QueryCondition>,
        alias: Option<String>,
        response: oneshot::Sender<QuickDbResult<u64>>,
    },
    Exists {
        collection: String,
        conditions: Vec<QueryCondition>,
        alias: Option<String>,
        response: oneshot::Sender<QuickDbResult<bool>>,
    },
    GetServerVersion {
        alias: Option<String>,
        response: oneshot::Sender<QuickDbResult<String>>,
    },
    CreateStoredProcedure {
        config: crate::stored_procedure::StoredProcedureConfig,
        alias: Option<String>,
        response: oneshot::Sender<QuickDbResult<crate::stored_procedure::StoredProcedureCreateResult>>,
    },
}
