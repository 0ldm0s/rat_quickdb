
//! # ODM管理器核心实现

use crate::error::{QuickDbError, QuickDbResult};
use crate::types::*;
use crate::manager::get_global_pool_manager;
use crate::odm::types::OdmRequest;
use tokio::sync::{mpsc, oneshot};
use rat_logger::{debug, error, info, warn};

/// 异步ODM管理器 - 使用消息传递避免生命周期问题
pub struct AsyncOdmManager {
    /// 请求发送器
    pub(crate) request_sender: mpsc::UnboundedSender<OdmRequest>,
    /// 默认别名
    default_alias: String,
    /// 后台任务句柄（用于优雅关闭）
    _task_handle: Option<tokio::task::JoinHandle<()>>,
}

impl AsyncOdmManager {
    /// 创建新的异步ODM管理器
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        
        // 启动后台处理任务
        let task_handle = tokio::spawn(Self::process_requests(receiver));
        
        info!("创建异步ODM管理器");
        
        Self {
            request_sender: sender,
            default_alias: "default".to_string(),
            _task_handle: Some(task_handle),
        }
    }
    
    /// 设置默认别名
    pub fn set_default_alias(&mut self, alias: &str) {
        info!("设置默认别名: {}", alias);
        self.default_alias = alias.to_string();
    }
    
    /// 获取实际使用的别名
    fn get_actual_alias(&self, alias: Option<&str>) -> String {
        alias.unwrap_or(&self.default_alias).to_string()
    }
    
    /// 后台请求处理任务
    async fn process_requests(mut receiver: mpsc::UnboundedReceiver<OdmRequest>) {
        info!("启动ODM后台处理任务");
        
        while let Some(request) = receiver.recv().await {
            match request {
                OdmRequest::Create { collection, data, alias, response } => {
                    let result = Self::handle_create(&collection, data, alias).await;
                    let _ = response.send(result);
                },
                OdmRequest::FindById { collection, id, alias, response } => {
                    let result = Self::handle_find_by_id(&collection, &id, alias).await;
                    let _ = response.send(result);
                },
                OdmRequest::Find { collection, conditions, options, alias, response } => {
                    let result = Self::handle_find(&collection, conditions, options, alias).await;
                    let _ = response.send(result);
                },
                OdmRequest::FindWithGroups { collection, condition_groups, options, alias, response } => {
                    let result = Self::handle_find_with_groups(&collection, condition_groups, options, alias).await;
                    let _ = response.send(result);
                },
                OdmRequest::Update { collection, conditions, updates, alias, response } => {
                    let result = Self::handle_update(&collection, conditions, updates, alias).await;
                    let _ = response.send(result);
                },
                OdmRequest::UpdateWithOperations { collection, conditions, operations, alias, response } => {
                    let result = Self::handle_update_with_operations(&collection, conditions, operations, alias).await;
                    let _ = response.send(result);
                },
                OdmRequest::UpdateById { collection, id, updates, alias, response } => {
                    let result = Self::handle_update_by_id(&collection, &id, updates, alias).await;
                    let _ = response.send(result);
                },
                OdmRequest::Delete { collection, conditions, alias, response } => {
                    let result = Self::handle_delete(&collection, conditions, alias).await;
                    let _ = response.send(result);
                },
                OdmRequest::DeleteById { collection, id, alias, response } => {
                    let result = Self::handle_delete_by_id(&collection, &id, alias).await;
                    let _ = response.send(result);
                },
                OdmRequest::Count { collection, conditions, alias, response } => {
                    let result = Self::handle_count(&collection, conditions, alias).await;
                    let _ = response.send(result);
                },
                OdmRequest::Exists { collection, conditions, alias, response } => {
                    let result = Self::handle_exists(&collection, conditions, alias).await;
                    let _ = response.send(result);
                },
                OdmRequest::GetServerVersion { alias, response } => {
                    let result = Self::handle_get_server_version(alias).await;
                    let _ = response.send(result);
                },
            }
        }

        warn!("ODM后台处理任务结束");
    }
}
impl Drop for AsyncOdmManager {
    fn drop(&mut self) {
        info!("开始清理AsyncOdmManager资源");
        
        // 关闭请求发送器，这会导致后台任务自然退出
        // 注意：这里不需要显式关闭sender，因为当所有sender被drop时，
        // receiver会自动关闭，导致process_requests循环退出
        
        // 如果有任务句柄，尝试取消任务
        if let Some(handle) = self._task_handle.take() {
            if !handle.is_finished() {
                warn!("ODM后台任务仍在运行，将被取消");
                handle.abort();
            } else {
                info!("ODM后台任务已正常结束");
            }
        }
        
        info!("AsyncOdmManager资源清理完成");
    }
}
