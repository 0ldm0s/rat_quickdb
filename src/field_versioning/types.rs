//! 字段版本管理类型定义

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 模型版本元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelVersionMeta {
    /// 模型名称
    pub model_name: String,
    /// 当前版本号
    pub current_version: u32,
    /// 上一次升级时间
    pub last_upgrade_time: Option<DateTime<Utc>>,
    /// 上一次降级时间
    pub last_downgrade_time: Option<DateTime<Utc>>,
}

impl ModelVersionMeta {
    /// 创建新的版本元数据
    pub fn new(model_name: String, version: u32) -> Self {
        Self {
            model_name,
            current_version: version,
            last_upgrade_time: Some(Utc::now()),
            last_downgrade_time: None,
        }
    }
}

/// 版本变更记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionChange {
    /// 模型名称
    pub model_name: String,
    /// 从哪个版本变更
    pub from_version: u32,
    /// 变更为哪个版本
    pub to_version: u32,
    /// 变更类型
    pub change_type: VersionChangeType,
    /// 变更时间
    pub timestamp: DateTime<Utc>,
}

/// 变更类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VersionChangeType {
    /// 升级
    Upgrade,
    /// 降级
    Downgrade,
}

/// 升级/降级结果
#[derive(Debug, Clone)]
pub struct VersionUpgradeResult {
    /// 旧版本号
    pub old_version: u32,
    /// 新版本号
    pub new_version: u32,
    /// 升级 DDL
    pub upgrade_ddl: String,
    /// 降级 DDL（可用于回滚）
    pub downgrade_ddl: String,
}
