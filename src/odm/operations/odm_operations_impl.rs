//! # ODM操作接口实现

use crate::error::{QuickDbError, QuickDbResult};
use crate::odm::manager_core::AsyncOdmManager;
use crate::odm::traits::OdmOperations;
use crate::odm::types::OdmRequest;
use crate::types::*;
use async_trait::async_trait;
use std::collections::HashMap;
use tokio::sync::oneshot;

/// 异步ODM操作接口实现
#[async_trait]
impl OdmOperations for AsyncOdmManager {
    async fn create(
        &self,
        collection: &str,
        data: HashMap<String, DataValue>,
        alias: Option<&str>,
    ) -> QuickDbResult<DataValue> {
        let (sender, receiver) = oneshot::channel();

        let request = OdmRequest::Create {
            collection: collection.to_string(),
            data,
            alias: alias.map(|s| s.to_string()),
            response: sender,
        };

        self.request_sender
            .send(request)
            .map_err(|_| QuickDbError::ConnectionError {
                message: "ODM后台任务已停止".to_string(),
            })?;

        receiver.await.map_err(|_| QuickDbError::ConnectionError {
            message: "ODM请求处理失败".to_string(),
        })?
    }

    async fn find_by_id(
        &self,
        collection: &str,
        id: &str,
        alias: Option<&str>,
    ) -> QuickDbResult<Option<DataValue>> {
        let (sender, receiver) = oneshot::channel();

        let request = OdmRequest::FindById {
            collection: collection.to_string(),
            id: id.to_string(),
            alias: alias.map(|s| s.to_string()),
            response: sender,
        };

        self.request_sender
            .send(request)
            .map_err(|_| QuickDbError::ConnectionError {
                message: "ODM后台任务已停止".to_string(),
            })?;

        receiver.await.map_err(|_| QuickDbError::ConnectionError {
            message: "ODM请求处理失败".to_string(),
        })?
    }

    async fn find(
        &self,
        collection: &str,
        conditions: Vec<QueryCondition>,
        options: Option<QueryOptions>,
        alias: Option<&str>,
    ) -> QuickDbResult<Vec<DataValue>> {
        let (sender, receiver) = oneshot::channel();

        let request = OdmRequest::Find {
            collection: collection.to_string(),
            conditions,
            options,
            alias: alias.map(|s| s.to_string()),
            response: sender,
        };

        self.request_sender
            .send(request)
            .map_err(|_| QuickDbError::ConnectionError {
                message: "ODM后台任务已停止".to_string(),
            })?;

        receiver.await.map_err(|_| QuickDbError::ConnectionError {
            message: "ODM请求处理失败".to_string(),
        })?
    }

    async fn find_with_groups(
        &self,
        collection: &str,
        condition_groups: Vec<QueryConditionGroup>,
        options: Option<QueryOptions>,
        alias: Option<&str>,
    ) -> QuickDbResult<Vec<DataValue>> {
        let (sender, receiver) = oneshot::channel();

        let request = OdmRequest::FindWithGroups {
            collection: collection.to_string(),
            condition_groups,
            options,
            alias: alias.map(|s| s.to_string()),
            response: sender,
        };

        self.request_sender
            .send(request)
            .map_err(|_| QuickDbError::ConnectionError {
                message: "ODM后台任务已停止".to_string(),
            })?;

        receiver.await.map_err(|_| QuickDbError::ConnectionError {
            message: "ODM请求处理失败".to_string(),
        })?
    }

    async fn update(
        &self,
        collection: &str,
        conditions: Vec<QueryCondition>,
        updates: HashMap<String, DataValue>,
        alias: Option<&str>,
    ) -> QuickDbResult<u64> {
        let (sender, receiver) = oneshot::channel();

        let request = OdmRequest::Update {
            collection: collection.to_string(),
            conditions,
            updates,
            alias: alias.map(|s| s.to_string()),
            response: sender,
        };

        self.request_sender
            .send(request)
            .map_err(|_| QuickDbError::ConnectionError {
                message: "ODM后台任务已停止".to_string(),
            })?;

        receiver.await.map_err(|_| QuickDbError::ConnectionError {
            message: "ODM请求处理失败".to_string(),
        })?
    }

    async fn update_with_operations(
        &self,
        collection: &str,
        conditions: Vec<QueryCondition>,
        operations: Vec<crate::types::UpdateOperation>,
        alias: Option<&str>,
    ) -> QuickDbResult<u64> {
        let (sender, receiver) = oneshot::channel();

        let request = OdmRequest::UpdateWithOperations {
            collection: collection.to_string(),
            conditions,
            operations,
            alias: alias.map(|s| s.to_string()),
            response: sender,
        };

        self.request_sender
            .send(request)
            .map_err(|_| QuickDbError::ConnectionError {
                message: "ODM后台任务已停止".to_string(),
            })?;

        receiver.await.map_err(|_| QuickDbError::ConnectionError {
            message: "ODM请求处理失败".to_string(),
        })?
    }

    async fn update_by_id(
        &self,
        collection: &str,
        id: &str,
        updates: HashMap<String, DataValue>,
        alias: Option<&str>,
    ) -> QuickDbResult<bool> {
        let (sender, receiver) = oneshot::channel();

        let request = OdmRequest::UpdateById {
            collection: collection.to_string(),
            id: id.to_string(),
            updates,
            alias: alias.map(|s| s.to_string()),
            response: sender,
        };

        self.request_sender
            .send(request)
            .map_err(|_| QuickDbError::ConnectionError {
                message: "ODM后台任务已停止".to_string(),
            })?;

        receiver.await.map_err(|_| QuickDbError::ConnectionError {
            message: "ODM请求处理失败".to_string(),
        })?
    }

    async fn delete(
        &self,
        collection: &str,
        conditions: Vec<QueryCondition>,
        alias: Option<&str>,
    ) -> QuickDbResult<u64> {
        let (sender, receiver) = oneshot::channel();

        let request = OdmRequest::Delete {
            collection: collection.to_string(),
            conditions,
            alias: alias.map(|s| s.to_string()),
            response: sender,
        };

        self.request_sender
            .send(request)
            .map_err(|_| QuickDbError::ConnectionError {
                message: "ODM后台任务已停止".to_string(),
            })?;

        receiver.await.map_err(|_| QuickDbError::ConnectionError {
            message: "ODM请求处理失败".to_string(),
        })?
    }

    async fn delete_by_id(
        &self,
        collection: &str,
        id: &str,
        alias: Option<&str>,
    ) -> QuickDbResult<bool> {
        let (sender, receiver) = oneshot::channel();

        let request = OdmRequest::DeleteById {
            collection: collection.to_string(),
            id: id.to_string(),
            alias: alias.map(|s| s.to_string()),
            response: sender,
        };

        self.request_sender
            .send(request)
            .map_err(|_| QuickDbError::ConnectionError {
                message: "ODM后台任务已停止".to_string(),
            })?;

        receiver.await.map_err(|_| QuickDbError::ConnectionError {
            message: "ODM请求处理失败".to_string(),
        })?
    }

    async fn count(
        &self,
        collection: &str,
        conditions: Vec<QueryCondition>,
        alias: Option<&str>,
    ) -> QuickDbResult<u64> {
        let (sender, receiver) = oneshot::channel();

        let request = OdmRequest::Count {
            collection: collection.to_string(),
            conditions,
            alias: alias.map(|s| s.to_string()),
            response: sender,
        };

        self.request_sender
            .send(request)
            .map_err(|_| QuickDbError::ConnectionError {
                message: "ODM后台任务已停止".to_string(),
            })?;

        receiver.await.map_err(|_| QuickDbError::ConnectionError {
            message: "ODM请求处理失败".to_string(),
        })?
    }

    async fn get_server_version(&self, alias: Option<&str>) -> QuickDbResult<String> {
        let (sender, receiver) = oneshot::channel();

        let request = OdmRequest::GetServerVersion {
            alias: alias.map(|s| s.to_string()),
            response: sender,
        };

        self.request_sender
            .send(request)
            .map_err(|_| QuickDbError::ConnectionError {
                message: "ODM后台任务已停止".to_string(),
            })?;

        receiver.await.map_err(|_| QuickDbError::ConnectionError {
            message: "ODM请求处理失败".to_string(),
        })?
    }

    async fn create_stored_procedure(
        &self,
        config: crate::stored_procedure::StoredProcedureConfig,
    ) -> QuickDbResult<crate::stored_procedure::StoredProcedureCreateResult> {
        let (sender, receiver) = oneshot::channel();

        let request = OdmRequest::CreateStoredProcedure {
            config,
            response: sender,
        };

        self.request_sender
            .send(request)
            .map_err(|_| QuickDbError::ConnectionError {
                message: "ODM后台任务已停止".to_string(),
            })?;

        receiver.await.map_err(|_| QuickDbError::ConnectionError {
            message: "ODM请求处理失败".to_string(),
        })?
    }

    async fn execute_stored_procedure(
        &self,
        procedure_name: &str,
        database_alias: Option<&str>,
        params: Option<std::collections::HashMap<String, crate::types::DataValue>>,
    ) -> QuickDbResult<crate::stored_procedure::StoredProcedureQueryResult> {
        let (sender, receiver) = oneshot::channel();

        let request = OdmRequest::ExecuteStoredProcedure {
            procedure_name: procedure_name.to_string(),
            database_alias: database_alias.map(|s| s.to_string()),
            params,
            response: sender,
        };

        self.request_sender
            .send(request)
            .map_err(|_| QuickDbError::ConnectionError {
                message: "ODM后台任务已停止".to_string(),
            })?;

        receiver.await.map_err(|_| QuickDbError::ConnectionError {
            message: "ODM请求处理失败".to_string(),
        })?
    }
}
