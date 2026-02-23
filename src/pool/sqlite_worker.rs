//! SQLite工作器模块

use rat_logger::{debug, error, info, warn};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot};
use tokio::time::timeout;

use super::{DatabaseConnection, DatabaseOperation, ExtendedPoolConfig};
use crate::adapter::DatabaseAdapter;
use crate::error::{QuickDbError, QuickDbResult};
use crate::types::*;

/// SQLite 单线程工作器
#[cfg(feature = "sqlite-support")]
pub struct SqliteWorker {
    /// 数据库连接
    pub(crate) connection: DatabaseConnection,
    /// 操作接收器
    pub(crate) operation_receiver: mpsc::UnboundedReceiver<DatabaseOperation>,
    /// 数据库配置
    pub(crate) db_config: DatabaseConfig,
    /// 重试计数
    pub(crate) retry_count: u32,
    /// 最大重试次数
    pub(crate) max_retries: u32,
    /// 重试间隔（毫秒）
    pub(crate) retry_interval_ms: u64,
    /// 健康检查间隔（秒）
    pub(crate) health_check_interval_sec: u64,
    /// 上次健康检查时间
    pub(crate) last_health_check: Instant,
    /// 连接是否健康
    pub(crate) is_healthy: bool,
    /// 缓存管理器（可选）
    pub(crate) cache_manager: Option<Arc<crate::cache::CacheManager>>,
    /// 数据库适配器（持久化，避免重复创建）
    pub(crate) adapter: Box<dyn crate::adapter::DatabaseAdapter + Send + Sync>,
}

#[cfg(feature = "sqlite-support")]
impl std::fmt::Debug for SqliteWorker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SqliteWorker")
            .field("connection", &self.connection)
            .field("db_config", &self.db_config)
            .field("retry_count", &self.retry_count)
            .field("max_retries", &self.max_retries)
            .field("retry_interval_ms", &self.retry_interval_ms)
            .field("health_check_interval_sec", &self.health_check_interval_sec)
            .field("last_health_check", &self.last_health_check)
            .field("is_healthy", &self.is_healthy)
            .field("cache_manager", &self.cache_manager)
            .field("adapter", &"<DatabaseAdapter>")
            .finish()
    }
}
#[cfg(feature = "sqlite-support")]
impl SqliteWorker {
    /// 运行SQLite工作器
    pub async fn run(mut self) {
        info!("SQLite工作器开始运行: 别名={}", self.db_config.alias);

        // 启动健康检查任务
        let health_check_handle = self.start_health_check_task().await;

        while let Some(operation) = self.operation_receiver.recv().await {
            // 检查连接健康状态
            if !self.is_healthy {
                warn!("SQLite连接不健康，尝试重新连接");
                if let Err(e) = self.reconnect().await {
                    error!("SQLite重新连接失败: {}", e);
                    continue;
                }
            }

            match self.handle_operation(operation).await {
                Ok(_) => {
                    self.retry_count = 0; // 重置重试计数
                    self.is_healthy = true; // 标记连接健康
                }
                Err(e) => {
                    error!("SQLite操作处理失败: {}", e);
                    self.is_healthy = false; // 标记连接不健康

                    // 智能重试逻辑
                    if self.retry_count < self.max_retries {
                        self.retry_count += 1;
                        let backoff_delay = self.calculate_backoff_delay();
                        warn!(
                            "SQLite操作重试 {}/{}, 延迟{}ms",
                            self.retry_count, self.max_retries, backoff_delay
                        );
                        tokio::time::sleep(Duration::from_millis(backoff_delay)).await;

                        // 尝试重新连接
                        if let Err(reconnect_err) = self.reconnect().await {
                            error!("SQLite重新连接失败: {}", reconnect_err);
                        }
                    } else {
                        error!("SQLite操作重试次数超限，标记连接为不健康状态");
                        self.is_healthy = false;
                        // 不再直接退出程序，而是继续运行但标记为不健康
                    }
                }
            }
        }

        // 清理健康检查任务
        health_check_handle.abort();
        info!("SQLite工作器停止运行");
    }

    /// 启动健康检查任务
    async fn start_health_check_task(&self) -> tokio::task::JoinHandle<()> {
        let health_check_interval = Duration::from_secs(self.health_check_interval_sec);
        let db_config = self.db_config.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(health_check_interval);

            loop {
                interval.tick().await;
                debug!("执行SQLite连接健康检查: 别名={}", db_config.alias);
                // 健康检查逻辑在主循环中处理
            }
        })
    }

    /// 重新连接数据库
    async fn reconnect(&mut self) -> QuickDbResult<()> {
        info!("正在重新连接SQLite数据库: 别名={}", self.db_config.alias);

        let new_connection = self.create_sqlite_connection().await?;
        self.connection = new_connection;
        self.is_healthy = true;
        self.retry_count = 0;

        info!("SQLite数据库重新连接成功: 别名={}", self.db_config.alias);
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
                    message: "SQLite连接配置类型不匹配".to_string(),
                });
            }
        };

        // 特殊处理内存数据库：直接连接，不创建文件
        if path == ":memory:" {
            info!("连接SQLite内存数据库: 别名={}", self.db_config.alias);
            let pool = sqlx::SqlitePool::connect(&path).await.map_err(|e| {
                QuickDbError::ConnectionError {
                    message: format!("SQLite内存数据库连接失败: {}", e),
                }
            })?;
            return Ok(DatabaseConnection::SQLite(pool));
        }

        // 检查数据库文件是否存在
        let file_exists = std::path::Path::new(&path).exists();

        // 如果文件不存在且不允许创建，则返回错误
        if !file_exists && !create_if_missing {
            return Err(QuickDbError::ConnectionError {
                message: format!("SQLite数据库文件不存在且未启用自动创建: {}", path),
            });
        }

        // 如果需要创建文件且文件不存在，则创建父目录
        if create_if_missing && !file_exists {
            if let Some(parent) = std::path::Path::new(&path).parent() {
                tokio::fs::create_dir_all(parent).await.map_err(|e| {
                    QuickDbError::ConnectionError {
                        message: format!("创建SQLite数据库目录失败: {}", e),
                    }
                })?;
            }

            // 创建空的数据库文件
            tokio::fs::File::create(&path)
                .await
                .map_err(|e| QuickDbError::ConnectionError {
                    message: format!("创建SQLite数据库文件失败: {}", e),
                })?;
        }

        let pool =
            sqlx::SqlitePool::connect(&path)
                .await
                .map_err(|e| QuickDbError::ConnectionError {
                    message: format!("SQLite连接失败: {}", e),
                })?;
        Ok(DatabaseConnection::SQLite(pool))
    }

    /// 计算退避延迟（指数退避）
    fn calculate_backoff_delay(&self) -> u64 {
        let base_delay = self.retry_interval_ms;
        let exponential_delay = base_delay * (2_u64.pow(self.retry_count.min(10))); // 限制最大指数
        let max_delay = 30000; // 最大延迟30秒
        exponential_delay.min(max_delay)
    }

    /// 执行连接健康检查
    async fn perform_health_check(&mut self) -> bool {
        if self.last_health_check.elapsed() < Duration::from_secs(self.health_check_interval_sec) {
            return self.is_healthy;
        }

        debug!("执行SQLite连接健康检查: 别名={}", self.db_config.alias);

        // 执行简单的查询来检查连接健康状态
        let health_check_result = match &self.connection {
            #[cfg(feature = "sqlite-support")]
            DatabaseConnection::SQLite(pool) => {
                sqlx::query("SELECT 1").fetch_optional(pool).await.is_ok()
            }
            _ => false,
        };

        self.last_health_check = Instant::now();
        self.is_healthy = health_check_result;

        if !self.is_healthy {
            warn!("SQLite连接健康检查失败: 别名={}", self.db_config.alias);
        } else {
            debug!("SQLite连接健康检查通过: 别名={}", self.db_config.alias);
        }

        self.is_healthy
    }

    /// 处理数据库操作（带 panic 捕获）
    async fn handle_operation(&mut self, operation: DatabaseOperation) -> QuickDbResult<()> {
        // 执行健康检查
        self.perform_health_check().await;

        // 执行数据库操作，使用 Result 来处理错误而不是 panic 捕获
        let operation_result = match operation {
            DatabaseOperation::Create {
                table,
                data,
                id_strategy,
                alias,
                response,
            } => {
                let result = self
                    .adapter
                    .create(&self.connection, &table, &data, &id_strategy, &alias)
                    .await;
                let _ = response.send(result);
                Ok(())
            }
            DatabaseOperation::FindById {
                table,
                id,
                alias,
                response,
            } => {
                let result = self
                    .adapter
                    .find_by_id(&self.connection, &table, &id, &alias)
                    .await;
                let _ = response.send(result);
                Ok(())
            }
            DatabaseOperation::Find {
                table,
                conditions,
                options,
                alias,
                response,
            } => {
                let result = self
                    .adapter
                    .find(&self.connection, &table, &conditions, &options, &alias)
                    .await;
                let _ = response.send(result);
                Ok(())
            }
            DatabaseOperation::FindWithGroups {
                table,
                condition_groups,
                options,
                alias,
                response,
            } => {
                let result = self
                    .adapter
                    .find_with_groups(
                        &self.connection,
                        &table,
                        &condition_groups,
                        &options,
                        &alias,
                    )
                    .await;
                let _ = response.send(result);
                Ok(())
            }
            DatabaseOperation::FindWithBypassCache {
                table,
                conditions,
                options,
                alias,
                bypass_cache,
                response,
            } => {
                let result = self
                    .adapter
                    .find_with_cache_control(
                        &self.connection,
                        &table,
                        &conditions,
                        &options,
                        &alias,
                        bypass_cache,
                    )
                    .await;
                let _ = response.send(result);
                Ok(())
            }
            DatabaseOperation::FindWithGroupsWithBypassCache {
                table,
                condition_groups,
                options,
                alias,
                bypass_cache,
                response,
            } => {
                let result = self
                    .adapter
                    .find_with_groups_with_cache_control_and_config(
                        &self.connection,
                        &table,
                        &condition_groups,
                        &options,
                        &alias,
                        bypass_cache,
                    )
                    .await;
                let _ = response.send(result);
                Ok(())
            }
            DatabaseOperation::Update {
                table,
                conditions,
                data,
                alias,
                response,
            } => {
                let result = self
                    .adapter
                    .update(&self.connection, &table, &conditions, &data, &alias)
                    .await;
                let _ = response.send(result);
                Ok(())
            }
            DatabaseOperation::UpdateWithOperations {
                table,
                conditions,
                operations,
                alias,
                response,
            } => {
                let result = self
                    .adapter
                    .update_with_operations(
                        &self.connection,
                        &table,
                        &conditions,
                        &operations,
                        &alias,
                    )
                    .await;
                let _ = response.send(result);
                Ok(())
            }
            DatabaseOperation::UpdateById {
                table,
                id,
                data,
                alias,
                response,
            } => {
                let result = self
                    .adapter
                    .update_by_id(&self.connection, &table, &id, &data, &alias)
                    .await;
                let _ = response.send(result);
                Ok(())
            }
            DatabaseOperation::Delete {
                table,
                conditions,
                alias,
                response,
            } => {
                let result = self
                    .adapter
                    .delete(&self.connection, &table, &conditions, &alias)
                    .await;
                let _ = response.send(result);
                Ok(())
            }
            DatabaseOperation::DeleteById {
                table,
                id,
                alias,
                response,
            } => {
                let result = self
                    .adapter
                    .delete_by_id(&self.connection, &table, &id, &alias)
                    .await;
                let _ = response.send(result);
                Ok(())
            }
            DatabaseOperation::Count {
                table,
                conditions,
                alias,
                response,
            } => {
                let result = self
                    .adapter
                    .count(&self.connection, &table, &conditions, &alias)
                    .await;
                let _ = response.send(result);
                Ok(())
            }
            DatabaseOperation::CountWithGroups {
                table,
                condition_groups,
                alias,
                response,
            } => {
                let result = self
                    .adapter
                    .count_with_groups(&self.connection, &table, &condition_groups, &alias)
                    .await;
                let _ = response.send(result);
                Ok(())
            }
            DatabaseOperation::CreateTable {
                table,
                fields,
                id_strategy,
                alias,
                response,
            } => {
                let result = self
                    .adapter
                    .create_table(&self.connection, &table, &fields, &id_strategy, &alias)
                    .await;
                let _ = response.send(result);
                Ok(())
            }
            DatabaseOperation::CreateIndex {
                table,
                index_name,
                fields,
                unique,
                response,
            } => {
                let result = self
                    .adapter
                    .create_index(&self.connection, &table, &index_name, &fields, unique)
                    .await;
                let _ = response.send(result);
                Ok(())
            }
            DatabaseOperation::TableExists { table, response } => {
                let result = self.adapter.table_exists(&self.connection, &table).await;
                let _ = response.send(result);
                Ok(())
            }
            DatabaseOperation::DropTable { table, response } => {
                let result = self.adapter.drop_table(&self.connection, &table).await;
                let _ = response.send(result);
                Ok(())
            }
            DatabaseOperation::GetServerVersion { response } => {
                let result = self.adapter.get_server_version(&self.connection).await;
                let _ = response.send(result);
                Ok(())
            }
            DatabaseOperation::CreateStoredProcedure { config, response } => {
                let result = self
                    .adapter
                    .create_stored_procedure(&self.connection, &config)
                    .await;
                let _ = response.send(result);
                Ok(())
            }
            DatabaseOperation::ExecuteStoredProcedure {
                procedure_name,
                database,
                params,
                response,
            } => {
                let result = self
                    .adapter
                    .execute_stored_procedure(&self.connection, &procedure_name, &database, params)
                    .await;
                let _ = response.send(result);
                Ok(())
            }
        };

        operation_result
    }
}
