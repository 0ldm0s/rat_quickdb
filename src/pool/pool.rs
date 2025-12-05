//! 连接池核心模块

use crossbeam_queue::SegQueue;
use rat_logger::{debug, error, info, warn};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot};

#[cfg(feature = "sqlite-support")]
use super::SqliteWorker;
use super::{
    DatabaseConnection, DatabaseOperation, ExtendedPoolConfig, MultiConnectionManager,
    PooledConnection,
};
use crate::adapter::DatabaseAdapter;
use crate::error::{QuickDbError, QuickDbResult};
use crate::model::FieldDefinition;
use crate::types::*;

/// 新的连接池 - 基于生产者/消费者模式
#[derive(Debug)]
pub struct ConnectionPool {
    /// 数据库配置
    pub db_config: DatabaseConfig,
    /// 扩展连接池配置
    pub config: ExtendedPoolConfig,
    /// 操作请求发送器
    pub operation_sender: mpsc::UnboundedSender<DatabaseOperation>,
    /// 数据库类型
    pub db_type: DatabaseType,
    /// 缓存管理器（可选）
    pub cache_manager: Option<Arc<crate::cache::CacheManager>>,
}

impl ConnectionPool {
    /// 使用配置创建连接池
    pub async fn with_config(
        db_config: DatabaseConfig,
        config: ExtendedPoolConfig,
    ) -> QuickDbResult<Self> {
        Self::with_config_and_cache(db_config, config, None).await
    }

    /// 使用配置和缓存管理器创建连接池
    pub async fn with_config_and_cache(
        db_config: DatabaseConfig,
        config: ExtendedPoolConfig,
        cache_manager: Option<Arc<crate::cache::CacheManager>>,
    ) -> QuickDbResult<Self> {
        let (operation_sender, operation_receiver) = mpsc::unbounded_channel();

        let pool = Self {
            db_type: db_config.db_type.clone(),
            db_config: db_config.clone(),
            config: config.clone(),
            operation_sender,
            cache_manager: cache_manager.clone(),
        };

        // 根据数据库类型启动对应的工作器
        match &db_config.db_type {
            #[cfg(feature = "sqlite-support")]
            DatabaseType::SQLite => {
                pool.start_sqlite_worker(operation_receiver, db_config, config)
                    .await?;
            }
            #[cfg(feature = "postgres-support")]
            DatabaseType::PostgreSQL => {
                pool.start_multi_connection_manager(operation_receiver, db_config, config)
                    .await?;
            }
            #[cfg(feature = "mysql-support")]
            DatabaseType::MySQL => {
                pool.start_multi_connection_manager(operation_receiver, db_config, config)
                    .await?;
            }
            #[cfg(feature = "mongodb-support")]
            DatabaseType::MongoDB => {
                pool.start_multi_connection_manager(operation_receiver, db_config, config)
                    .await?;
            }
            _ => Err(QuickDbError::ConfigError {
                message: "不支持的数据库类型（可能需要启用相应的feature）".to_string(),
            })?,
        }

        Ok(pool)
    }

    /// 设置缓存管理器
    pub fn set_cache_manager(&mut self, cache_manager: Arc<crate::cache::CacheManager>) {
        self.cache_manager = Some(cache_manager);
    }

    /// 启动SQLite工作器
    #[cfg(feature = "sqlite-support")]
    async fn start_sqlite_worker(
        &self,
        operation_receiver: mpsc::UnboundedReceiver<DatabaseOperation>,
        db_config: DatabaseConfig,
        config: ExtendedPoolConfig,
    ) -> QuickDbResult<()> {
        let connection = self.create_sqlite_connection().await?;

        // 创建启动同步通道
        let (startup_tx, startup_rx) = oneshot::channel();

        // 创建适配器
        use crate::adapter::{create_adapter, create_adapter_with_cache};
        let (adapter, adapter_type) = if let Some(cache_manager) = &self.cache_manager {
            let adapter = create_adapter_with_cache(&db_config.db_type, cache_manager.clone())?;
            (adapter, "缓存适配器")
        } else {
            let adapter = create_adapter(&db_config.db_type)?;
            (adapter, "普通适配器")
        };

        info!("数据库 '{}' 使用 {}", db_config.alias, adapter_type);

        let worker = SqliteWorker {
            connection,
            operation_receiver,
            db_config: db_config.clone(),
            retry_count: 0,
            max_retries: config.max_retries,
            retry_interval_ms: config.retry_interval_ms,
            health_check_interval_sec: config.health_check_timeout_sec, // 复用健康检查超时作为间隔
            last_health_check: Instant::now(),
            is_healthy: true,
            cache_manager: self.cache_manager.clone(),
            adapter,
        };

        // 启动工作器
        tokio::spawn(async move {
            // 发送启动完成信号
            let _ = startup_tx.send(());
            worker.run().await;
        });

        // 等待工作器启动完成
        startup_rx
            .await
            .map_err(|_| QuickDbError::ConnectionError {
                message: crate::i18n::tf(
                    "error.sqlite_worker_startup",
                    &[("alias", &db_config.alias)],
                ),
            })?;

        info!("SQLite工作器启动完成: 别名={}", db_config.alias);
        Ok(())
    }

    /// 启动多连接管理器
    async fn start_multi_connection_manager(
        &self,
        operation_receiver: mpsc::UnboundedReceiver<DatabaseOperation>,
        db_config: DatabaseConfig,
        config: ExtendedPoolConfig,
    ) -> QuickDbResult<()> {
        let manager = MultiConnectionManager {
            workers: Vec::new(),
            available_workers: SegQueue::new(),
            operation_receiver,
            db_config,
            config,
            keepalive_handle: None,
            cache_manager: self.cache_manager.clone(),
        };

        // 启动管理器
        tokio::spawn(async move {
            manager.run().await;
        });

        Ok(())
    }

    /// 创建SQLite连接
    #[cfg(feature = "sqlite-support")]
    async fn create_sqlite_connection(&self) -> QuickDbResult<DatabaseConnection> {
        let (path, create_if_missing) = match &self.db_config.connection {
            crate::types::ConnectionConfig::SQLite {
                path,
                create_if_missing,
            } => (path.clone(), *create_if_missing),
            _ => {
                return Err(QuickDbError::ConfigError {
                    message: crate::i18n::t("error.sqlite_config_mismatch"),
                });
            }
        };

        // 特殊处理内存数据库：直接连接，不创建文件
        if path == ":memory:" {
            info!("连接SQLite内存数据库: 别名={}", self.db_config.alias);
            let pool = sqlx::SqlitePool::connect(&path).await.map_err(|e| {
                QuickDbError::ConnectionError {
                    message: crate::i18n::tf("error.sqlite_memory", &[("message", &e.to_string())]),
                }
            })?;
            return Ok(DatabaseConnection::SQLite(pool));
        }

        // 检查数据库文件是否存在
        let file_exists = std::path::Path::new(&path).exists();

        // 如果文件不存在且不允许创建，则返回错误
        if !file_exists && !create_if_missing {
            return Err(QuickDbError::ConnectionError {
                message: crate::i18n::tf("error.sqlite_file_not_found", &[("path", &path)]),
            });
        }

        // 如果需要创建文件且文件不存在，则创建父目录
        if create_if_missing && !file_exists {
            if let Some(parent) = std::path::Path::new(&path).parent() {
                tokio::fs::create_dir_all(parent).await.map_err(|e| {
                    QuickDbError::ConnectionError {
                        message: crate::i18n::tf(
                            "error.sqlite_dir_create",
                            &[("message", &e.to_string())],
                        ),
                    }
                })?;
            }

            // 创建空的数据库文件
            tokio::fs::File::create(&path)
                .await
                .map_err(|e| QuickDbError::ConnectionError {
                    message: crate::i18n::tf(
                        "error.sqlite_file_create",
                        &[("message", &e.to_string())],
                    ),
                })?;
        }

        let pool =
            sqlx::SqlitePool::connect(&path)
                .await
                .map_err(|e| QuickDbError::ConnectionError {
                    message: crate::i18n::tf(
                        "error.sqlite_connection",
                        &[("message", &e.to_string())],
                    ),
                })?;
        Ok(DatabaseConnection::SQLite(pool))
    }

    /// 发送操作请求并等待响应
    async fn send_operation<T>(&self, operation: DatabaseOperation) -> QuickDbResult<T>
    where
        T: Send + 'static,
    {
        // 这是一个泛型占位符，实际实现需要根据具体操作类型来处理
        Err(QuickDbError::QueryError {
            message: "操作发送未实现".to_string(),
        })
    }

    /// 创建记录
    pub async fn create(
        &self,
        table: &str,
        data: &HashMap<String, DataValue>,
        id_strategy: &IdStrategy,
    ) -> QuickDbResult<DataValue> {
        let (response_sender, response_receiver) = oneshot::channel();

        let operation = DatabaseOperation::Create {
            table: table.to_string(),
            data: data.clone(),
            id_strategy: id_strategy.clone(),
            alias: self.db_config.alias.clone(),
            response: response_sender,
        };

        self.operation_sender
            .send(operation)
            .map_err(|_| QuickDbError::QueryError {
                message: "发送操作失败".to_string(),
            })?;

        response_receiver
            .await
            .map_err(|_| QuickDbError::QueryError {
                message: "接收响应失败".to_string(),
            })?
    }

    /// 根据ID查找记录
    pub async fn find_by_id(
        &self,
        table: &str,
        id: &DataValue,
    ) -> QuickDbResult<Option<DataValue>> {
        let (response_sender, response_receiver) = oneshot::channel();

        let operation = DatabaseOperation::FindById {
            table: table.to_string(),
            id: id.clone(),
            alias: self.db_config.alias.clone(),
            response: response_sender,
        };

        self.operation_sender
            .send(operation)
            .map_err(|_| QuickDbError::QueryError {
                message: "发送操作失败".to_string(),
            })?;

        response_receiver
            .await
            .map_err(|_| QuickDbError::QueryError {
                message: "接收响应失败".to_string(),
            })?
    }

    /// 查找记录
    pub async fn find(
        &self,
        table: &str,
        conditions: &[QueryCondition],
        options: &QueryOptions,
    ) -> QuickDbResult<Vec<DataValue>> {
        let (response_sender, response_receiver) = oneshot::channel();

        let operation = DatabaseOperation::Find {
            table: table.to_string(),
            conditions: conditions.to_vec(),
            options: options.clone(),
            alias: self.db_config.alias.clone(),
            response: response_sender,
        };

        self.operation_sender
            .send(operation)
            .map_err(|_| QuickDbError::QueryError {
                message: "发送操作失败".to_string(),
            })?;

        response_receiver
            .await
            .map_err(|_| QuickDbError::QueryError {
                message: "接收响应失败".to_string(),
            })?
    }

    /// 更新记录
    pub async fn update(
        &self,
        table: &str,
        conditions: &[QueryCondition],
        data: &HashMap<String, DataValue>,
    ) -> QuickDbResult<u64> {
        let (response_sender, response_receiver) = oneshot::channel();

        let operation = DatabaseOperation::Update {
            table: table.to_string(),
            conditions: conditions.to_vec(),
            data: data.clone(),
            alias: self.db_config.alias.clone(),
            response: response_sender,
        };

        self.operation_sender
            .send(operation)
            .map_err(|_| QuickDbError::QueryError {
                message: "发送操作失败".to_string(),
            })?;

        response_receiver
            .await
            .map_err(|_| QuickDbError::QueryError {
                message: "接收响应失败".to_string(),
            })?
    }

    /// 根据ID更新记录
    pub async fn update_by_id(
        &self,
        table: &str,
        id: &DataValue,
        data: &HashMap<String, DataValue>,
    ) -> QuickDbResult<bool> {
        let (response_sender, response_receiver) = oneshot::channel();

        let operation = DatabaseOperation::UpdateById {
            table: table.to_string(),
            id: id.clone(),
            data: data.clone(),
            alias: self.db_config.alias.clone(),
            response: response_sender,
        };

        self.operation_sender
            .send(operation)
            .map_err(|_| QuickDbError::QueryError {
                message: "发送操作失败".to_string(),
            })?;

        response_receiver
            .await
            .map_err(|_| QuickDbError::QueryError {
                message: "接收响应失败".to_string(),
            })?
    }

    /// 删除记录
    pub async fn delete(
        &self,
        table: &str,
        conditions: &[QueryCondition],
        alias: &str,
    ) -> QuickDbResult<u64> {
        let (response_sender, response_receiver) = oneshot::channel();

        let operation = DatabaseOperation::Delete {
            table: table.to_string(),
            conditions: conditions.to_vec(),
            alias: alias.to_string(),
            response: response_sender,
        };

        self.operation_sender
            .send(operation)
            .map_err(|_| QuickDbError::QueryError {
                message: "发送操作失败".to_string(),
            })?;

        response_receiver
            .await
            .map_err(|_| QuickDbError::QueryError {
                message: "接收响应失败".to_string(),
            })?
    }

    /// 根据ID删除记录
    pub async fn delete_by_id(
        &self,
        table: &str,
        id: &DataValue,
        alias: &str,
    ) -> QuickDbResult<bool> {
        let (response_sender, response_receiver) = oneshot::channel();

        let operation = DatabaseOperation::DeleteById {
            table: table.to_string(),
            id: id.clone(),
            alias: alias.to_string(),
            response: response_sender,
        };

        self.operation_sender
            .send(operation)
            .map_err(|_| QuickDbError::QueryError {
                message: "发送操作失败".to_string(),
            })?;

        response_receiver
            .await
            .map_err(|_| QuickDbError::QueryError {
                message: "接收响应失败".to_string(),
            })?
    }

    /// 统计记录
    pub async fn count(
        &self,
        table: &str,
        conditions: &[QueryCondition],
        alias: &str,
    ) -> QuickDbResult<u64> {
        let (response_sender, response_receiver) = oneshot::channel();

        let operation = DatabaseOperation::Count {
            table: table.to_string(),
            conditions: conditions.to_vec(),
            alias: alias.to_string(),
            response: response_sender,
        };

        self.operation_sender
            .send(operation)
            .map_err(|_| QuickDbError::QueryError {
                message: "发送操作失败".to_string(),
            })?;

        response_receiver
            .await
            .map_err(|_| QuickDbError::QueryError {
                message: "接收响应失败".to_string(),
            })?
    }

    /// 创建表
    pub async fn create_table(
        &self,
        table: &str,
        fields: &HashMap<String, FieldDefinition>,
        id_strategy: &IdStrategy,
    ) -> QuickDbResult<()> {
        let (response_sender, response_receiver) = oneshot::channel();

        let operation = DatabaseOperation::CreateTable {
            table: table.to_string(),
            fields: fields.clone(),
            id_strategy: id_strategy.clone(),
            alias: self.db_config.alias.clone(),
            response: response_sender,
        };

        self.operation_sender
            .send(operation)
            .map_err(|_| QuickDbError::QueryError {
                message: "发送操作失败".to_string(),
            })?;

        response_receiver
            .await
            .map_err(|_| QuickDbError::QueryError {
                message: "接收响应失败".to_string(),
            })?
    }

    /// 创建索引
    pub async fn create_index(
        &self,
        table: &str,
        index_name: &str,
        fields: &[String],
        unique: bool,
    ) -> QuickDbResult<()> {
        let (response_sender, response_receiver) = oneshot::channel();

        let operation = DatabaseOperation::CreateIndex {
            table: table.to_string(),
            index_name: index_name.to_string(),
            fields: fields.to_vec(),
            unique,
            response: response_sender,
        };

        self.operation_sender
            .send(operation)
            .map_err(|_| QuickDbError::QueryError {
                message: "发送操作失败".to_string(),
            })?;

        response_receiver
            .await
            .map_err(|_| QuickDbError::QueryError {
                message: "接收响应失败".to_string(),
            })?
    }

    /// 检查表是否存在
    pub async fn table_exists(&self, table: &str) -> QuickDbResult<bool> {
        let (response_sender, response_receiver) = oneshot::channel();

        let operation = DatabaseOperation::TableExists {
            table: table.to_string(),
            response: response_sender,
        };

        self.operation_sender
            .send(operation)
            .map_err(|_| QuickDbError::QueryError {
                message: "发送操作失败".to_string(),
            })?;

        response_receiver
            .await
            .map_err(|_| QuickDbError::QueryError {
                message: "接收响应失败".to_string(),
            })?
    }

    /// 删除表
    pub async fn drop_table(&self, table: &str) -> QuickDbResult<()> {
        let (response_sender, response_receiver) = oneshot::channel();

        let operation = DatabaseOperation::DropTable {
            table: table.to_string(),
            response: response_sender,
        };

        self.operation_sender
            .send(operation)
            .map_err(|_| QuickDbError::QueryError {
                message: "发送操作失败".to_string(),
            })?;

        response_receiver
            .await
            .map_err(|_| QuickDbError::QueryError {
                message: "接收响应失败".to_string(),
            })?
    }

    /// 获取数据库类型
    pub fn get_database_type(&self) -> &DatabaseType {
        &self.db_config.db_type
    }

    /// 获取连接（兼容旧接口）
    pub async fn get_connection(&self) -> QuickDbResult<PooledConnection> {
        // 在新架构中，我们不再直接返回连接
        // 这个方法主要用于兼容性，返回一个虚拟连接
        Ok(PooledConnection {
            id: format!("{}-virtual", self.db_config.alias),
            db_type: self.db_config.db_type.clone(),
            alias: self.db_config.alias.clone(),
        })
    }

    /// 释放连接（兼容旧接口）
    pub async fn release_connection(&self, _connection_id: &str) -> QuickDbResult<()> {
        // 在新架构中，连接由工作器自动管理，这个方法为空实现
        Ok(())
    }

    /// 清理过期连接（兼容旧接口）
    pub async fn cleanup_expired_connections(&self) {
        // 在新架构中，连接由工作器自动管理，这个方法为空实现
        debug!("清理过期连接（新架构中自动管理）");
    }
}
