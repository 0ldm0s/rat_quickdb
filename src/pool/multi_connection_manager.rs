//! 多连接管理器模块

use crossbeam_queue::SegQueue;
use rat_logger::{debug, error, info, warn};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot};
use tokio::time::timeout;
use uuid::Uuid;

use super::{ConnectionWorker, DatabaseConnection, DatabaseOperation, ExtendedPoolConfig};
use crate::adapter::DatabaseAdapter;
use crate::error::{QuickDbError, QuickDbResult};
use crate::types::*;

/// 多连接工作器管理器（用于MySQL/PostgreSQL/MongoDB）
pub struct MultiConnectionManager {
    /// 工作器列表
    pub(crate) workers: Vec<ConnectionWorker>,
    /// 可用工作器队列
    pub(crate) available_workers: SegQueue<usize>,
    /// 操作接收器
    pub(crate) operation_receiver: mpsc::UnboundedReceiver<DatabaseOperation>,
    /// 数据库配置
    pub(crate) db_config: DatabaseConfig,
    /// 扩展配置
    pub(crate) config: ExtendedPoolConfig,
    /// 保活任务句柄
    pub(crate) keepalive_handle: Option<tokio::task::JoinHandle<()>>,
    /// 缓存管理器（可选）
    pub(crate) cache_manager: Option<Arc<crate::cache::CacheManager>>,
}
impl MultiConnectionManager {
    /// 创建初始连接
    pub async fn create_initial_connections(&mut self) -> QuickDbResult<()> {
        // 只创建1个worker进行测试
        let worker = self.create_connection_worker(0).await?;
        self.workers.push(worker);
        self.available_workers.push(0);

        Ok(())
    }

    /// 创建连接工作器
    async fn create_connection_worker(&self, index: usize) -> QuickDbResult<ConnectionWorker> {
        let connection = self.create_database_connection().await?;

        // 创建适配器
        use crate::adapter::{create_adapter, create_adapter_with_cache};
        let (adapter, adapter_type) = if let Some(cache_manager) = &self.cache_manager {
            let adapter =
                create_adapter_with_cache(&self.db_config.db_type, cache_manager.clone())?;
            (adapter, "缓存适配器")
        } else {
            let adapter = create_adapter(&self.db_config.db_type)?;
            (adapter, "普通适配器")
        };

        debug!("数据库 '{}' 使用 {}", self.db_config.alias, adapter_type);

        Ok(ConnectionWorker {
            id: format!("{}-worker-{}", self.db_config.alias, index),
            connection,
            pool_config: self.config.clone(),
            created_at: Instant::now(),
            last_used: Instant::now(),
            retry_count: 0,
            db_type: self.db_config.db_type.clone(),
            adapter,
        })
    }

    /// 创建数据库连接
    async fn create_database_connection(&self) -> QuickDbResult<DatabaseConnection> {
        match &self.db_config.db_type {
            #[cfg(feature = "postgres-support")]
            DatabaseType::PostgreSQL => {
                let connection_string = match &self.db_config.connection {
                    crate::types::ConnectionConfig::PostgreSQL {
                        host,
                        port,
                        database,
                        username,
                        password,
                        ssl_mode: _,
                        tls_config: _,
                    } => {
                        // 对密码进行 URL 编码以处理特殊字符
                        let encoded_password = urlencoding::encode(password);
                        format!(
                            "postgresql://{}:{}@{}:{}/{}",
                            username, encoded_password, host, port, database
                        )
                    }
                    _ => {
                        return Err(QuickDbError::ConfigError {
                            message: "PostgreSQL连接配置类型不匹配".to_string(),
                        });
                    }
                };

                // 使用PgPoolOptions创建连接池 - 使用配置值
                let pool = sqlx::postgres::PgPoolOptions::new()
                    .max_connections(self.config.base.max_connections)
                    .min_connections(self.config.base.min_connections)
                    .max_lifetime(std::time::Duration::from_secs(
                        self.config.base.max_lifetime,
                    ))
                    .idle_timeout(std::time::Duration::from_secs(
                        self.config.base.idle_timeout,
                    ))
                    .acquire_timeout(std::time::Duration::from_millis(
                        self.config.base.connection_timeout,
                    ))
                    .connect(&connection_string)
                    .await
                    .map_err(|e| QuickDbError::ConnectionError {
                        message: format!("PostgreSQL连接池创建失败: {}", e),
                    })?;

                Ok(DatabaseConnection::PostgreSQL(pool))
            }
            #[cfg(feature = "mysql-support")]
            DatabaseType::MySQL => {
                let connection_string = match &self.db_config.connection {
                    crate::types::ConnectionConfig::MySQL {
                        host,
                        port,
                        database,
                        username,
                        password,
                        ssl_opts: _,
                        tls_config: _,
                    } => {
                        // 对密码进行 URL 编码以处理特殊字符
                        let encoded_password = urlencoding::encode(password);
                        format!(
                            "mysql://{}:{}@{}:{}/{}",
                            username, encoded_password, host, port, database
                        )
                    }
                    _ => {
                        return Err(QuickDbError::ConfigError {
                            message: "MySQL连接配置类型不匹配".to_string(),
                        });
                    }
                };

                // 创建带有连接池配置的MySQL连接池
                let mysql_pool = sqlx::mysql::MySqlPoolOptions::new()
                    .min_connections(self.config.base.min_connections)
                    .max_connections(self.config.base.max_connections)
                    .acquire_timeout(std::time::Duration::from_millis(
                        self.config.base.connection_timeout,
                    ))
                    .idle_timeout(std::time::Duration::from_millis(
                        self.config.base.idle_timeout,
                    ))
                    .max_lifetime(std::time::Duration::from_millis(
                        self.config.base.max_lifetime,
                    ))
                    .connect(&connection_string)
                    .await
                    .map_err(|e| QuickDbError::ConnectionError {
                        message: format!("MySQL连接池创建失败: {}", e),
                    })?;

                Ok(DatabaseConnection::MySQL(mysql_pool))
            }
            #[cfg(feature = "mongodb-support")]
            DatabaseType::MongoDB => {
                let connection_uri = match &self.db_config.connection {
                    crate::types::ConnectionConfig::MongoDB {
                        host,
                        port,
                        database,
                        username,
                        password,
                        auth_source,
                        direct_connection,
                        tls_config,
                        zstd_config,
                        options,
                    } => {
                        // 使用构建器生成连接URI
                        let mut builder = crate::types::MongoDbConnectionBuilder::new(
                            host.clone(),
                            *port,
                            database.clone(),
                        );

                        // 设置认证信息
                        if let (Some(user), Some(pass)) = (username, password) {
                            builder = builder.with_auth(user.clone(), pass.clone());
                        }

                        // 设置认证数据库
                        if let Some(auth_src) = auth_source {
                            builder = builder.with_auth_source(auth_src.clone());
                        }

                        // 设置直接连接
                        builder = builder.with_direct_connection(*direct_connection);

                        // 设置TLS配置
                        if let Some(tls) = tls_config {
                            builder = builder.with_tls_config(tls.clone());
                        }

                        // 设置ZSTD压缩配置
                        if let Some(zstd) = zstd_config {
                            builder = builder.with_zstd_config(zstd.clone());
                        }

                        // 添加自定义选项
                        if let Some(opts) = options {
                            for (key, value) in opts {
                                builder = builder.with_option(key.clone(), value.clone());
                            }
                        }

                        builder.build_uri()
                    }
                    _ => {
                        return Err(QuickDbError::ConfigError {
                            message: "MongoDB连接配置类型不匹配".to_string(),
                        });
                    }
                };

                debug!("MongoDB连接URI: {}", connection_uri);

                let client = mongodb::Client::with_uri_str(&connection_uri)
                    .await
                    .map_err(|e| QuickDbError::ConnectionError {
                        message: format!("MongoDB连接失败: {}", e),
                    })?;

                let database_name = match &self.db_config.connection {
                    crate::types::ConnectionConfig::MongoDB { database, .. } => database.clone(),
                    _ => unreachable!(),
                };

                let db = client.database(&database_name);
                Ok(DatabaseConnection::MongoDB(db))
            }
            _ => Err(QuickDbError::ConfigError {
                message: "不支持的数据库类型用于多连接管理器（可能需要启用相应的feature）"
                    .to_string(),
            }),
        }
    }

    /// 启动连接保活任务
    pub fn start_keepalive_task(&mut self) {
        let keepalive_interval = Duration::from_secs(self.config.keepalive_interval_sec);
        let health_check_timeout = Duration::from_secs(self.config.health_check_timeout_sec);

        // 这里需要实现保活逻辑的占位符
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(keepalive_interval);

            loop {
                interval.tick().await;
                debug!("执行连接保活检查");
                // TODO: 实现具体的保活逻辑
            }
        });

        self.keepalive_handle = Some(handle);
    }

    /// 运行多连接管理器
    pub async fn run(mut self) {
        debug!("多连接管理器开始运行: 别名={}", self.db_config.alias);

        // 创建初始连接
        if let Err(e) = self.create_initial_connections().await {
            error!("创建初始连接失败: {}", e);
            return;
        }

        // 启动保活任务
        self.start_keepalive_task();

        while let Some(operation) = self.operation_receiver.recv().await {
            if let Err(e) = self.handle_operation(operation).await {
                error!("多连接操作处理失败: {}", e);
            }
        }

        debug!("多连接管理器停止运行");
    }

    /// 处理数据库操作
    async fn handle_operation(&mut self, operation: DatabaseOperation) -> QuickDbResult<()> {
        // 获取可用工作器
        let worker_index = match self.available_workers.pop() {
            Some(index) => index,
            None => {
                // 所有工作器都在使用中，等待或创建新连接
                return Err(QuickDbError::ConnectionError {
                    message: "所有连接都在使用中".to_string(),
                });
            }
        };

        // 获取工作器的连接
        let worker = &mut self.workers[worker_index];
        worker.last_used = Instant::now();

        // 处理具体操作
        let result = match operation {
            DatabaseOperation::Create {
                table,
                data,
                id_strategy,
                alias,
                response,
            } => {
                let result = worker
                    .adapter
                    .create(&worker.connection, &table, &data, &id_strategy, &alias)
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
                let result = worker
                    .adapter
                    .find_by_id(&worker.connection, &table, &id, &alias)
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
                let result = worker
                    .adapter
                    .find(&worker.connection, &table, &conditions, &options, &alias)
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
                let result = worker
                    .adapter
                    .find_with_groups(
                        &worker.connection,
                        &table,
                        &condition_groups,
                        &options,
                        &alias,
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
                let result = worker
                    .adapter
                    .update(&worker.connection, &table, &conditions, &data, &alias)
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
                let result = worker
                    .adapter
                    .update_with_operations(
                        &worker.connection,
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
                let result = worker
                    .adapter
                    .update_by_id(&worker.connection, &table, &id, &data, &alias)
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
                let result = worker
                    .adapter
                    .delete(&worker.connection, &table, &conditions, &alias)
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
                let result = worker
                    .adapter
                    .delete_by_id(&worker.connection, &table, &id, &alias)
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
                let result = worker
                    .adapter
                    .count(&worker.connection, &table, &conditions, &alias)
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
                let result = worker
                    .adapter
                    .create_table(&worker.connection, &table, &fields, &id_strategy, &alias)
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
                let result = worker
                    .adapter
                    .create_index(&worker.connection, &table, &index_name, &fields, unique)
                    .await;
                let _ = response.send(result);
                Ok(())
            }
            DatabaseOperation::TableExists { table, response } => {
                let result = worker
                    .adapter
                    .table_exists(&worker.connection, &table)
                    .await;
                let _ = response.send(result);
                Ok(())
            }
            DatabaseOperation::DropTable { table, response } => {
                let result = worker.adapter.drop_table(&worker.connection, &table).await;
                let _ = response.send(result);
                Ok(())
            }
            DatabaseOperation::GetServerVersion { response } => {
                let result = worker.adapter.get_server_version(&worker.connection).await;
                let _ = response.send(result);
                Ok(())
            }
            DatabaseOperation::CreateStoredProcedure { config, response } => {
                let result = worker
                    .adapter
                    .create_stored_procedure(&worker.connection, &config)
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
                let result = worker
                    .adapter
                    .execute_stored_procedure(
                        &worker.connection,
                        &procedure_name,
                        &database,
                        params,
                    )
                    .await;
                let _ = response.send(result);
                Ok(())
            }
        };

        // 处理连接错误和重试逻辑
        if let Err(ref e) = result {
            let worker_id = worker.id.clone();
            worker.retry_count += 1;
            let retry_count = worker.retry_count;

            error!(
                "工作器 {} 操作失败 ({}/{}): {}",
                worker_id, retry_count, self.config.max_retries, e
            );

            if retry_count > self.config.max_retries {
                warn!("工作器 {} 重试次数超限，尝试重新创建连接", worker_id);

                // 释放对 worker 的借用，然后重新创建连接
                drop(worker);

                // 尝试重新创建连接，但不退出程序
                match self.create_connection_worker(worker_index).await {
                    Ok(new_worker) => {
                        self.workers[worker_index] = new_worker;
                        debug!("工作器 {} 连接已重新创建", worker_index);
                    }
                    Err(create_err) => {
                        error!("重新创建工作器 {} 连接失败: {}", worker_index, create_err);
                        // 重新获取 worker 引用并重置计数
                        if let Some(worker) = self.workers.get_mut(worker_index) {
                            worker.retry_count = 0; // 重置计数，下次再试
                        }
                        // 延迟一段时间再重试
                        tokio::time::sleep(Duration::from_millis(
                            self.config.retry_interval_ms * 2,
                        ))
                        .await;
                    }
                }
            }
        } else {
            // 操作成功，重置重试计数
            worker.retry_count = 0;
        }

        // 归还工作器
        self.available_workers.push(worker_index);

        result
    }
}
