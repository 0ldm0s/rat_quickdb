//! 字段版本管理器

use crate::error::{QuickDbError, QuickDbResult};
use crate::field_versioning::ddl::{generate_ddl, generate_diff_ddl, generate_downgrade_ddl};
use crate::field_versioning::types::{ModelVersionMeta, VersionChange, VersionChangeType, VersionUpgradeResult};
use crate::model::field_types::ModelMeta;
use crate::types::DatabaseType;

use chrono::Utc;
use parking_lot::RwLock;
use sled::Db;
use std::collections::HashMap;
use std::path::PathBuf;

/// 字段版本管理器
pub struct FieldVersionManager {
    /// sled 数据库
    db: Db,
    /// 存储路径
    storage_path: PathBuf,
    /// 数据库类型
    db_type: DatabaseType,
    /// 缓存的版本元数据
    cache: RwLock<HashMap<String, ModelVersionMeta>>,
}

impl FieldVersionManager {
    /// 创建新的版本管理器
    pub fn new(storage_path: PathBuf, db_type: DatabaseType) -> QuickDbResult<Self> {
        // 确保目录存在
        std::fs::create_dir_all(&storage_path)?;

        let db = sled::open(&storage_path)
            .map_err(|e| QuickDbError::ConfigError {
                message: format!("无法打开版本管理存储: {}", e),
            })?;

        Ok(Self {
            db,
            storage_path,
            db_type,
            cache: RwLock::new(HashMap::new()),
        })
    }

    /// 获取默认存储路径
    pub fn default_storage_path(alias: &str) -> PathBuf {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        home.join(".rat_quickdb").join(alias)
    }

    /// 注册模型（创建初始版本）
    pub fn register_model(&self, model: &ModelMeta) -> QuickDbResult<()> {
        let version = model.version.unwrap_or(1);
        let model_name = &model.collection_name;

        // 检查是否已存在
        if self.get_version(model_name)?.is_some() {
            return Err(QuickDbError::ValidationError {
                field: "model".to_string(),
                message: format!("模型 {} 已存在，请使用 upgrade_model 进行升级", model_name),
            });
        }

        // 创建版本元数据
        let meta = ModelVersionMeta::new(model_name.clone(), version);

        // 保存到 sled
        self.save_version_meta(model_name, &meta)?;

        // 生成并保存 DDL 文件
        let ddl = generate_ddl(model, self.db_type);
        let ddl_path = self.storage_path.join(format!("{}_upgrade.ddl", model_name));
        std::fs::write(&ddl_path, &ddl)?;

        // 保存初始模型定义
        self.save_model_definition(model_name, version, model)?;

        // 更新缓存
        self.cache.write().insert(model_name.clone(), meta);

        Ok(())
    }

    /// 升级模型
    pub fn upgrade_model(
        &self,
        model_name: &str,
        new_model: &ModelMeta,
    ) -> QuickDbResult<VersionUpgradeResult> {
        let new_version = new_model.version.unwrap_or(1);

        // 获取旧版本信息
        let old_meta = self.get_version_meta(model_name)?
            .ok_or_else(|| QuickDbError::ValidationError {
                field: "model".to_string(),
                message: format!("模型 {} 不存在，请先注册", model_name),
            })?;

        let old_version = old_meta.current_version;

        // 确保新版本大于旧版本
        if new_version <= old_version {
            return Err(QuickDbError::ValidationError {
                field: "version".to_string(),
                message: format!(
                    "新版本号 {} 必须大于旧版本号 {}",
                    new_version, old_version
                ),
            });
        }

        // 获取旧模型定义（用于生成差异 DDL）
        let old_model = self.load_model_definition(model_name, old_version)?;

        // 生成 DDL
        let upgrade_ddl = generate_diff_ddl(&old_model, new_model, self.db_type);
        let downgrade_ddl = generate_downgrade_ddl(&old_model, new_model, self.db_type);

        // 写入 DDL 文件（覆盖之前的）
        let upgrade_ddl_path = self.storage_path.join(format!("{}_upgrade.ddl", model_name));
        let downgrade_ddl_path = self.storage_path.join(format!("{}_downgrade.ddl", model_name));

        std::fs::write(&upgrade_ddl_path, &upgrade_ddl)?;
        std::fs::write(&downgrade_ddl_path, &downgrade_ddl)?;

        // 更新版本元数据
        let mut new_meta = ModelVersionMeta::new(model_name.to_string(), new_version);
        new_meta.last_upgrade_time = Some(Utc::now());
        new_meta.last_downgrade_time = old_meta.last_downgrade_time;

        self.save_version_meta(model_name, &new_meta)?;

        // 保存新的模型定义
        self.save_model_definition(model_name, new_version, new_model)?;

        // 记录变更
        let change = VersionChange {
            model_name: model_name.to_string(),
            from_version: old_version,
            to_version: new_version,
            change_type: VersionChangeType::Upgrade,
            timestamp: Utc::now(),
        };
        self.save_change(model_name, &change)?;

        // 更新缓存
        self.cache.write().insert(model_name.to_string(), new_meta);

        Ok(VersionUpgradeResult {
            old_version,
            new_version,
            upgrade_ddl,
            downgrade_ddl,
        })
    }

    /// 回滚模型（降级到上一版本）
    pub fn rollback_model(&self, model_name: &str) -> QuickDbResult<VersionUpgradeResult> {
        // 获取当前版本信息
        let current_meta = self.get_version_meta(model_name)?
            .ok_or_else(|| QuickDbError::ValidationError {
                field: "model".to_string(),
                message: format!("模型 {} 不存在", model_name),
            })?;

        if current_meta.current_version <= 1 {
            return Err(QuickDbError::ValidationError {
                field: "version".to_string(),
                message: "无法回滚，已是初始版本".to_string(),
            });
        }

        // 获取上一版本的模型定义
        let previous_version = current_meta.current_version - 1;
        let previous_model = self.load_model_definition(model_name, previous_version)?;
        let current_model = self.load_model_definition(model_name, current_meta.current_version)?;

        // 生成 DDL（previous -> current 是降级）
        let upgrade_ddl = generate_downgrade_ddl(&previous_model, &current_model, self.db_type);
        let downgrade_ddl = generate_diff_ddl(&previous_model, &current_model, self.db_type);

        // 写入 DDL 文件
        let upgrade_ddl_path = self.storage_path.join(format!("{}_upgrade.ddl", model_name));
        let downgrade_ddl_path = self.storage_path.join(format!("{}_downgrade.ddl", model_name));

        std::fs::write(&upgrade_ddl_path, &upgrade_ddl)?;
        std::fs::write(&downgrade_ddl_path, &downgrade_ddl)?;

        // 更新版本元数据
        let mut new_meta = ModelVersionMeta::new(model_name.to_string(), previous_version);
        new_meta.last_upgrade_time = current_meta.last_upgrade_time;
        new_meta.last_downgrade_time = Some(Utc::now());

        self.save_version_meta(model_name, &new_meta)?;

        // 记录变更
        let change = VersionChange {
            model_name: model_name.to_string(),
            from_version: current_meta.current_version,
            to_version: previous_version,
            change_type: VersionChangeType::Downgrade,
            timestamp: Utc::now(),
        };
        self.save_change(model_name, &change)?;

        // 更新缓存
        self.cache.write().insert(model_name.to_string(), new_meta);

        Ok(VersionUpgradeResult {
            old_version: current_meta.current_version,
            new_version: previous_version,
            upgrade_ddl,
            downgrade_ddl,
        })
    }

    /// 获取模型当前版本
    pub fn get_version(&self, model_name: &str) -> QuickDbResult<Option<u32>> {
        Ok(self.get_version_meta(model_name)?.map(|m| m.current_version))
    }

    /// 获取版本元数据
    fn get_version_meta(&self, model_name: &str) -> QuickDbResult<Option<ModelVersionMeta>> {
        // 先从缓存获取
        if let Some(meta) = self.cache.read().get(model_name) {
            return Ok(Some(meta.clone()));
        }

        // 从 sled 获取
        let key = format!("meta:{}", model_name);
        if let Some(data) = self.db.get(key.as_bytes())? {
            let meta: ModelVersionMeta = serde_json::from_slice(&data)
                .map_err(|e| QuickDbError::SerializationError {
                    message: format!("无法解析版本元数据: {}", e),
                })?;
            self.cache.write().insert(model_name.to_string(), meta.clone());
            return Ok(Some(meta));
        }

        Ok(None)
    }

    /// 保存版本元数据
    fn save_version_meta(&self, model_name: &str, meta: &ModelVersionMeta) -> QuickDbResult<()> {
        let key = format!("meta:{}", model_name);
        let data = serde_json::to_vec(meta)
            .map_err(|e| QuickDbError::SerializationError {
                message: format!("无法序列化版本元数据: {}", e),
            })?;
        self.db.insert(key.as_bytes(), data)?;
        self.db.flush()?;
        Ok(())
    }

    /// 保存模型定义
    fn save_model_definition(
        &self,
        model_name: &str,
        version: u32,
        model: &ModelMeta,
    ) -> QuickDbResult<()> {
        let key = format!("def:{}:{}", model_name, version);
        let data = serde_json::to_vec(model)
            .map_err(|e| QuickDbError::SerializationError {
                message: format!("无法序列化模型定义: {}", e),
            })?;
        self.db.insert(key.as_bytes(), data)?;
        self.db.flush()?;
        Ok(())
    }

    /// 加载模型定义
    fn load_model_definition(&self, model_name: &str, version: u32) -> QuickDbResult<ModelMeta> {
        let key = format!("def:{}:{}", model_name, version);
        let data = self.db.get(key.as_bytes())?
            .ok_or_else(|| QuickDbError::NotFound {
                message: format!("模型 {} 版本 {} 的定义不存在", model_name, version),
            })?;
        let model: ModelMeta = serde_json::from_slice(&data)
            .map_err(|e| QuickDbError::SerializationError {
                message: format!("无法解析模型定义: {}", e),
            })?;
        Ok(model)
    }

    /// 保存变更记录
    fn save_change(&self, model_name: &str, change: &VersionChange) -> QuickDbResult<()> {
        let key = format!("change:{}", model_name);
        let data = serde_json::to_vec(change)
            .map_err(|e| QuickDbError::SerializationError {
                message: format!("无法序列化变更记录: {}", e),
            })?;
        self.db.insert(key.as_bytes(), data)?;
        self.db.flush()?;
        Ok(())
    }

    /// 获取变更记录
    pub fn get_changes(&self, model_name: &str) -> QuickDbResult<Vec<VersionChange>> {
        let key = format!("change:{}", model_name);
        if let Some(data) = self.db.get(key.as_bytes())? {
            let change: VersionChange = serde_json::from_slice(&data)
                .map_err(|e| QuickDbError::SerializationError {
                    message: format!("无法解析变更记录: {}", e),
                })?;
            return Ok(vec![change]);
        }
        Ok(vec![])
    }

    /// 获取 DDL 文件路径
    pub fn get_ddl_path(&self, model_name: &str, upgrade: bool) -> PathBuf {
        let filename = if upgrade {
            format!("{}_upgrade.ddl", model_name)
        } else {
            format!("{}_downgrade.ddl", model_name)
        };
        self.storage_path.join(filename)
    }

    /// 读取 DDL 内容
    pub fn read_ddl(&self, model_name: &str, upgrade: bool) -> QuickDbResult<String> {
        let path = self.get_ddl_path(model_name, upgrade);
        Ok(std::fs::read_to_string(&path)?)
    }
}
