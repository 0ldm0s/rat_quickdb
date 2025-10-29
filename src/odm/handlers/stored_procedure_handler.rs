//! 存储过程处理器
//!
//! 处理存储过程相关的ODM请求

use crate::error::{QuickDbError, QuickDbResult};
use crate::odm::types::OdmRequest;
use crate::odm::manager_core::AsyncOdmManager;
use rat_logger::{debug, error};

/// 处理存储过程相关请求
pub async fn handle_stored_procedure_request(
    request: OdmRequest,
    _manager: &AsyncOdmManager,
) -> Result<Option<crate::odm::types::OdmRequest>, QuickDbError> {
    match request {
        OdmRequest::CreateStoredProcedure { config, alias, response } => {
            debug!("处理存储过程创建请求: {}", config.procedure_name);

            // 直接调用静态方法处理请求
            let result = AsyncOdmManager::handle_create_stored_procedure(config, alias).await;

            // 发送响应
            if let Err(e) = response.send(result) {
                error!("发送存储过程创建响应失败: {:?}", e);
            }

            Ok(None) // 请求已处理，不需要重新入队
        }
        _ => Ok(Some(request)), // 不是存储过程请求，返回重新处理
    }
}