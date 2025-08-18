use crossbeam::queue::SegQueue;
use pyo3::prelude::*;
use rat_quickdb::config::{DatabaseConfigBuilder, PoolConfigBuilder};
use rat_quickdb::manager::{add_database, get_global_pool_manager};
use rat_quickdb::odm::{get_odm_manager, OdmOperations};
use rat_quickdb::types::DatabaseConfig;
use rat_quickdb::types::{
    ConnectionConfig, DataValue, DatabaseType, IdStrategy, PaginationConfig, PoolConfig,
    QueryCondition, QueryOperator, QueryOptions, SortConfig, SortDirection,
};
use rat_quickdb::{QuickDbError, QuickDbResult};
use serde_json::{Map as JsonMap, Value as JsonValue};
use std::collections::HashMap;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tokio::runtime::Runtime;
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;
use zerg_creep::{debug, error, info, warn};

// 导入模型绑定模块
mod model_bindings;
use model_bindings::*;

/// 获取版本信息
#[pyfunction]
fn get_version() -> String {
    "0.1.1".to_string()
}

/// 获取库信息
#[pyfunction]
fn get_info() -> String {
    "Rat QuickDB Python Bindings".to_string()
}

/// 获取库名称
#[pyfunction]
fn get_name() -> String {
    "rat_quickdb_py".to_string()
}

/// Python请求消息结构
#[derive(Debug, Clone)]
pub struct PyRequestMessage {
    pub id: String,
    pub request_type: String,
    pub data: String,
}

/// Python响应消息结构
#[derive(Debug, Clone)]
pub struct PyResponseMessage {
    pub id: String,
    pub success: bool,
    pub data: String,
    pub error: Option<String>,
}

/// 数据库请求结构
#[derive(Debug)]
pub struct PyDbRequest {
    /// 请求ID
    pub request_id: String,
    /// 操作类型
    pub operation: String,
    /// 集合名称
    pub collection: String,
    /// 数据
    pub data: Option<HashMap<String, DataValue>>,
    /// 查询条件
    pub conditions: Option<Vec<QueryCondition>>,
    /// 查询选项
    pub options: Option<QueryOptions>,
    /// 更新数据
    pub updates: Option<HashMap<String, DataValue>>,
    /// ID
    pub id: Option<String>,
    /// 数据库别名
    pub alias: Option<String>,
    /// 数据库配置（用于add_database操作）
    pub database_config: Option<DatabaseConfig>,
    /// 响应发送器
    pub response_sender: oneshot::Sender<PyDbResponse>,
}

/// 数据库响应结构
#[derive(Debug, Clone)]
pub struct PyDbResponse {
    /// 请求ID
    pub request_id: String,
    /// 是否成功
    pub success: bool,
    /// 响应数据
    pub data: String,
    /// 错误信息
    pub error: Option<String>,
}

/// Python数据库队列桥接器
#[pyclass(name = "DbQueueBridge")]
pub struct PyDbQueueBridge {
    /// 请求发送器
    request_sender: Arc<std::sync::Mutex<Option<mpsc::UnboundedSender<PyDbRequest>>>>,
    /// 默认别名
    default_alias: Arc<std::sync::Mutex<Option<String>>>,
    /// 任务句柄
    _task_handle: Arc<std::sync::Mutex<Option<std::thread::JoinHandle<()>>>>,
    /// 初始化状态
    initialized: Arc<std::sync::Mutex<bool>>,
}

#[pymethods]
impl PyDbQueueBridge {
    /// 创建新的数据库队列桥接器
    #[new]
    pub fn new() -> PyResult<Self> {
        info!("创建新的数据库队列桥接器");
        let (request_sender, request_receiver) = mpsc::unbounded_channel::<PyDbRequest>();

        // 启动后台守护任务线程
        info!("启动后台守护任务线程");
        let task_handle = thread::spawn(move || {
            info!("守护任务线程开始运行");
            let rt = Runtime::new().expect("创建Tokio运行时失败");
            rt.block_on(async {
                info!("守护任务异步运行时启动");
                PyDbQueueBridgeAsync::daemon_task(request_receiver).await;
                info!("守护任务异步运行时结束");
            });
            info!("守护任务线程结束");
        });

        // 等待一小段时间确保守护任务启动
        thread::sleep(Duration::from_millis(100));

        let bridge = PyDbQueueBridge {
            request_sender: Arc::new(std::sync::Mutex::new(Some(request_sender))),
            default_alias: Arc::new(std::sync::Mutex::new(None)),
            _task_handle: Arc::new(std::sync::Mutex::new(Some(task_handle))),
            initialized: Arc::new(std::sync::Mutex::new(true)),
        };

        info!("数据库队列桥接器创建完成");
        Ok(bridge)
    }

    /// 创建数据
    pub fn create(
        &self,
        table: String,
        data_json: String,
        alias: Option<String>,
    ) -> PyResult<String> {
        self.check_initialized()?;
        let data = self.parse_json_to_data_map(&data_json)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("解析数据失败: {}", e)))?;
        
        let (response_sender, response_receiver) = oneshot::channel();
        let request = PyDbRequest {
            request_id: Uuid::new_v4().to_string(),
            operation: "create".to_string(),
            collection: table,
            data: Some(data),
            conditions: None,
            options: None,
            updates: None,
            id: None,
            alias,
            database_config: None,
            response_sender,
        };
        
        self.send_request(request)?;
        self.wait_for_response(response_receiver)
    }

    /// 查询数据
    pub fn find(
        &self,
        table: String,
        query_json: String,
        alias: Option<String>,
    ) -> PyResult<String> {
        self.check_initialized()?;
        let conditions = self.parse_conditions_json(&query_json)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("解析查询条件失败: {}", e)))?;
        
        let (response_sender, response_receiver) = oneshot::channel();
        let request = PyDbRequest {
            request_id: Uuid::new_v4().to_string(),
            operation: "find".to_string(),
            collection: table,
            data: None,
            conditions: Some(conditions),
            options: None,
            updates: None,
            id: None,
            alias,
            database_config: None,
            response_sender,
        };
        
        self.send_request(request)?;
        self.wait_for_response(response_receiver)
    }

    /// 根据ID查询数据
    pub fn find_by_id(&self, table: String, id: String, alias: Option<String>) -> PyResult<String> {
        self.check_initialized()?;
        
        let (response_sender, response_receiver) = oneshot::channel();
        let request = PyDbRequest {
            request_id: Uuid::new_v4().to_string(),
            operation: "find_by_id".to_string(),
            collection: table,
            data: None,
            conditions: None,
            options: None,
            updates: None,
            id: Some(id),
            alias,
            database_config: None,
            response_sender,
        };
        
        self.send_request(request)?;
        self.wait_for_response(response_receiver)
    }

    /// 添加SQLite数据库
    pub fn add_sqlite_database(
        &self,
        alias: String,
        path: String,
        max_connections: Option<u32>,
        min_connections: Option<u32>,
        connection_timeout: Option<u64>,
        idle_timeout: Option<u64>,
        max_lifetime: Option<u64>,
    ) -> PyResult<String> {
        info!("添加SQLite数据库: alias={}, path={}", alias, path);
        
        let pool_config = match PoolConfigBuilder::new()
            .max_connections(max_connections.unwrap_or(10))
            .min_connections(min_connections.unwrap_or(1))
            .connection_timeout(connection_timeout.unwrap_or(30))
            .idle_timeout(idle_timeout.unwrap_or(600))
            .max_lifetime(max_lifetime.unwrap_or(3600))
            .build() {
                Ok(config) => config,
                Err(e) => {
                    return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                        format!("连接池配置构建失败: {}", e)
                    ));
                }
            };
        
        let db_config = match DatabaseConfigBuilder::new()
            .db_type(DatabaseType::SQLite)
            .connection(ConnectionConfig::SQLite { 
                path: path.clone(),
                create_if_missing: true,
            })
            .pool(pool_config)
            .alias(alias.clone())
            .id_strategy(IdStrategy::Uuid)
            .build() {
                Ok(config) => config,
                Err(e) => {
                    return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                        format!("数据库配置构建失败: {}", e)
                    ));
                }
            };
        
        // 通过守护任务添加数据库，确保在正确的运行时中启动
        let (response_tx, response_rx) = oneshot::channel();
        let request_id = Uuid::new_v4().to_string();
        
        let request = PyDbRequest {
            request_id: request_id.clone(),
            operation: "add_database".to_string(),
            collection: "".to_string(), // 不需要集合名
            data: None,
            conditions: None,
            options: None,
            updates: None,
            id: None,
            alias: Some(alias.clone()),
            database_config: Some(db_config),
            response_sender: response_tx,
        };
        
        // 发送请求到守护任务
        self.send_request(request)?;
        
        // 等待响应
        let response_json = self.wait_for_response(response_rx)?;
        let response: serde_json::Value = serde_json::from_str(&response_json)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("解析响应失败: {}", e)
            ))?;
        
        if response["success"].as_bool().unwrap_or(false) {
            // 更新默认别名
            *self.default_alias.lock().unwrap() = Some(alias.clone());
            Ok(response_json)
        } else {
            let error_msg = response["error"].as_str().unwrap_or("未知错误");
            Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(error_msg.to_string()))
        }
    }

    /// 添加PostgreSQL数据库
    pub fn add_postgresql_database(
        &self,
        alias: String,
        host: String,
        port: u16,
        database: String,
        username: String,
        password: String,
        ssl_mode: Option<String>,
        max_connections: Option<u32>,
        min_connections: Option<u32>,
        connection_timeout: Option<u64>,
        idle_timeout: Option<u64>,
        max_lifetime: Option<u64>,
    ) -> PyResult<String> {
        info!("添加PostgreSQL数据库: alias={}, host={}, port={}, database={}", alias, host, port, database);
        
        let pool_config = match PoolConfigBuilder::new()
            .max_connections(max_connections.unwrap_or(10))
            .min_connections(min_connections.unwrap_or(1))
            .connection_timeout(connection_timeout.unwrap_or(30))
            .idle_timeout(idle_timeout.unwrap_or(600))
            .max_lifetime(max_lifetime.unwrap_or(3600))
            .build() {
                Ok(config) => config,
                Err(e) => {
                    return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                        format!("连接池配置构建失败: {}", e)
                    ));
                }
            };
        
        let db_config = match DatabaseConfigBuilder::new()
            .db_type(DatabaseType::PostgreSQL)
            .connection(ConnectionConfig::PostgreSQL { 
                host: host.clone(),
                port,
                database: database.clone(),
                username: username.clone(),
                password: password.clone(),
                ssl_mode: ssl_mode.or_else(|| Some("prefer".to_string())),
                tls_config: None,
            })
            .pool(pool_config)
            .alias(alias.clone())
            .id_strategy(IdStrategy::Uuid)
            .build() {
                Ok(config) => config,
                Err(e) => {
                    return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                        format!("数据库配置构建失败: {}", e)
                    ));
                }
            };
        
        // 通过守护任务添加数据库
        let (response_tx, response_rx) = oneshot::channel();
        let request_id = Uuid::new_v4().to_string();
        
        let request = PyDbRequest {
            request_id: request_id.clone(),
            operation: "add_database".to_string(),
            collection: "".to_string(),
            data: None,
            conditions: None,
            options: None,
            updates: None,
            id: None,
            alias: Some(alias.clone()),
            database_config: Some(db_config),
            response_sender: response_tx,
        };
        
        self.send_request(request)?;
        
        let response_json = self.wait_for_response(response_rx)?;
        let response: serde_json::Value = serde_json::from_str(&response_json)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("解析响应失败: {}", e)
            ))?;
        
        if response["success"].as_bool().unwrap_or(false) {
            *self.default_alias.lock().unwrap() = Some(alias.clone());
            Ok(response_json)
        } else {
            let error_msg = response["error"].as_str().unwrap_or("未知错误");
            Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(error_msg.to_string()))
        }
    }

    /// 添加MySQL数据库
    pub fn add_mysql_database(
        &self,
        alias: String,
        host: String,
        port: u16,
        database: String,
        username: String,
        password: String,
        max_connections: Option<u32>,
        min_connections: Option<u32>,
        connection_timeout: Option<u64>,
        idle_timeout: Option<u64>,
        max_lifetime: Option<u64>,
    ) -> PyResult<String> {
        info!("添加MySQL数据库: alias={}, host={}, port={}, database={}", alias, host, port, database);
        
        let pool_config = match PoolConfigBuilder::new()
            .max_connections(max_connections.unwrap_or(10))
            .min_connections(min_connections.unwrap_or(1))
            .connection_timeout(connection_timeout.unwrap_or(30))
            .idle_timeout(idle_timeout.unwrap_or(600))
            .max_lifetime(max_lifetime.unwrap_or(3600))
            .build() {
                Ok(config) => config,
                Err(e) => {
                    return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                        format!("连接池配置构建失败: {}", e)
                    ));
                }
            };
        
        let db_config = match DatabaseConfigBuilder::new()
            .db_type(DatabaseType::MySQL)
            .connection(ConnectionConfig::MySQL { 
                host: host.clone(),
                port,
                database: database.clone(),
                username: username.clone(),
                password: password.clone(),
                ssl_opts: None,
                tls_config: None,
            })
            .pool(pool_config)
            .alias(alias.clone())
            .id_strategy(IdStrategy::Uuid)
            .build() {
                Ok(config) => config,
                Err(e) => {
                    return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                        format!("数据库配置构建失败: {}", e)
                    ));
                }
            };
        
        // 通过守护任务添加数据库
        let (response_tx, response_rx) = oneshot::channel();
        let request_id = Uuid::new_v4().to_string();
        
        let request = PyDbRequest {
            request_id: request_id.clone(),
            operation: "add_database".to_string(),
            collection: "".to_string(),
            data: None,
            conditions: None,
            options: None,
            updates: None,
            id: None,
            alias: Some(alias.clone()),
            database_config: Some(db_config),
            response_sender: response_tx,
        };
        
        self.send_request(request)?;
        
        let response_json = self.wait_for_response(response_rx)?;
        let response: serde_json::Value = serde_json::from_str(&response_json)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("解析响应失败: {}", e)
            ))?;
        
        if response["success"].as_bool().unwrap_or(false) {
            *self.default_alias.lock().unwrap() = Some(alias.clone());
            Ok(response_json)
        } else {
            let error_msg = response["error"].as_str().unwrap_or("未知错误");
            Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(error_msg.to_string()))
        }
    }

    /// 添加MongoDB数据库
    pub fn add_mongodb_database(
        &self,
        alias: String,
        host: String,
        port: u16,
        database: String,
        username: Option<String>,
        password: Option<String>,
        auth_source: Option<String>,
        direct_connection: Option<bool>,
        max_connections: Option<u32>,
        min_connections: Option<u32>,
        connection_timeout: Option<u64>,
        idle_timeout: Option<u64>,
        max_lifetime: Option<u64>,
    ) -> PyResult<String> {
        info!("添加MongoDB数据库: alias={}, host={}, port={}, database={}", alias, host, port, database);
        
        let pool_config = match PoolConfigBuilder::new()
            .max_connections(max_connections.unwrap_or(10))
            .min_connections(min_connections.unwrap_or(1))
            .connection_timeout(connection_timeout.unwrap_or(30))
            .idle_timeout(idle_timeout.unwrap_or(600))
            .max_lifetime(max_lifetime.unwrap_or(3600))
            .build() {
                Ok(config) => config,
                Err(e) => {
                    return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                        format!("连接池配置构建失败: {}", e)
                    ));
                }
            };
        
        let db_config = match DatabaseConfigBuilder::new()
            .db_type(DatabaseType::MongoDB)
            .connection(ConnectionConfig::MongoDB { 
                host: host.clone(),
                port,
                database: database.clone(),
                username,
                password,
                auth_source,
                direct_connection: direct_connection.unwrap_or(false),
                tls_config: None,
                zstd_config: None,
                options: None,
            })
            .pool(pool_config)
            .alias(alias.clone())
            .id_strategy(IdStrategy::Uuid)
            .build() {
                Ok(config) => config,
                Err(e) => {
                    return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                        format!("数据库配置构建失败: {}", e)
                    ));
                }
            };
        
        // 通过守护任务添加数据库
        let (response_tx, response_rx) = oneshot::channel();
        let request_id = Uuid::new_v4().to_string();
        
        let request = PyDbRequest {
            request_id: request_id.clone(),
            operation: "add_database".to_string(),
            collection: "".to_string(),
            data: None,
            conditions: None,
            options: None,
            updates: None,
            id: None,
            alias: Some(alias.clone()),
            database_config: Some(db_config),
            response_sender: response_tx,
        };
        
        self.send_request(request)?;
        
        let response_json = self.wait_for_response(response_rx)?;
        let response: serde_json::Value = serde_json::from_str(&response_json)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("解析响应失败: {}", e)
            ))?;
        
        if response["success"].as_bool().unwrap_or(false) {
            *self.default_alias.lock().unwrap() = Some(alias.clone());
            Ok(response_json)
        } else {
            let error_msg = response["error"].as_str().unwrap_or("未知错误");
            Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(error_msg.to_string()))
        }
    }

    /// 设置默认别名
    pub fn set_default_alias(&self, alias: String) -> PyResult<()> {
        let mut default_alias = self.default_alias.lock().unwrap();
        *default_alias = Some(alias);
        Ok(())
    }
}

/// 异步数据库队列桥接器实现
struct PyDbQueueBridgeAsync;

impl PyDbQueueBridgeAsync {
    /// 守护任务主循环
    async fn daemon_task(mut request_receiver: mpsc::UnboundedReceiver<PyDbRequest>) {
        info!("启动数据库队列守护任务");

        while let Some(request) = request_receiver.recv().await {
            debug!("收到请求: operation={}, collection={}", request.operation, request.collection);
            
            // 直接处理请求，不使用ODM管理器
            let response = Self::process_request(&request).await;
            
            if let Err(e) = request.response_sender.send(response) {
                error!("发送响应失败: {:?}", e);
            }
        }
        
        info!("数据库队列守护任务结束");
    }

    /// 直接处理请求
    async fn process_request(request: &PyDbRequest) -> PyDbResponse {
        let request_id = request.request_id.clone();
        
        match request.operation.as_str() {
            "add_database" => {
                if let Some(db_config) = &request.database_config {
                    match add_database(db_config.clone()).await {
                        Ok(_) => PyDbResponse {
                            request_id,
                            success: true,
                            data: serde_json::json!({
                                "success": true,
                                "message": format!("数据库 '{}' 添加成功", db_config.alias)
                            }).to_string(),
                            error: None,
                        },
                        Err(e) => PyDbResponse {
                            request_id,
                            success: false,
                            data: String::new(),
                            error: Some(format!("添加数据库失败: {}", e)),
                        },
                    }
                } else {
                    PyDbResponse {
                        request_id,
                        success: false,
                        data: String::new(),
                        error: Some("缺少数据库配置".to_string()),
                    }
                }
            }
            "create" => {
                if let Some(data) = &request.data {
                    match Self::handle_create_direct(&request.collection, data.clone(), request.alias.clone()).await {
                        Ok(result) => PyDbResponse {
                            request_id,
                            success: true,
                            data: result,
                            error: None,
                        },
                        Err(e) => PyDbResponse {
                            request_id,
                            success: false,
                            data: String::new(),
                            error: Some(format!("创建失败: {}", e)),
                        },
                    }
                } else {
                    PyDbResponse {
                        request_id,
                        success: false,
                        data: String::new(),
                        error: Some("缺少数据".to_string()),
                    }
                }
            }
            "find_by_id" => {
                if let Some(id) = &request.id {
                    match Self::handle_find_by_id_direct(&request.collection, id, request.alias.clone()).await {
                        Ok(result) => PyDbResponse {
                            request_id,
                            success: true,
                            data: result.unwrap_or_else(|| "null".to_string()),
                            error: None,
                        },
                        Err(e) => PyDbResponse {
                            request_id,
                            success: false,
                            data: String::new(),
                            error: Some(format!("查询失败: {}", e)),
                        },
                    }
                } else {
                    PyDbResponse {
                        request_id,
                        success: false,
                        data: String::new(),
                        error: Some("缺少ID".to_string()),
                    }
                }
            }
            "find" => {
                if let Some(conditions) = &request.conditions {
                    match Self::handle_find_direct(&request.collection, conditions.clone(), request.options.clone(), request.alias.clone()).await {
                        Ok(result) => PyDbResponse {
                            request_id,
                            success: true,
                            data: result,
                            error: None,
                        },
                        Err(e) => PyDbResponse {
                            request_id,
                            success: false,
                            data: String::new(),
                            error: Some(format!("查询失败: {}", e)),
                        },
                    }
                } else {
                    PyDbResponse {
                        request_id,
                        success: false,
                        data: String::new(),
                        error: Some("缺少查询条件".to_string()),
                    }
                }
            }
            _ => PyDbResponse {
                request_id,
                success: false,
                data: String::new(),
                error: Some(format!("不支持的操作: {}", request.operation)),
            },
        }
    }

    /// 直接使用连接池处理创建请求
    async fn handle_create_direct(
        collection: &str,
        data: HashMap<String, DataValue>,
        alias: Option<String>,
    ) -> QuickDbResult<String> {
        use rat_quickdb::pool::DatabaseOperation;
        use tokio::sync::oneshot;
        
        let actual_alias = alias.unwrap_or_else(|| "default".to_string());
        debug!("直接处理创建请求: collection={}, alias={}", collection, actual_alias);
        
        let manager = get_global_pool_manager();
        let connection_pools = manager.get_connection_pools();
        let connection_pool = connection_pools.get(&actual_alias)
            .ok_or_else(|| QuickDbError::AliasNotFound {
                alias: actual_alias.clone(),
            })?;
        
        // 创建oneshot通道用于接收响应
        let (response_tx, response_rx) = oneshot::channel();
        
        // 发送DatabaseOperation::Create请求到连接池
        let operation = DatabaseOperation::Create {
            table: collection.to_string(),
            data,
            response: response_tx,
        };
        
        connection_pool.operation_sender.send(operation)
            .map_err(|_| QuickDbError::ConnectionError {
                message: "连接池操作通道已关闭".to_string(),
            })?;
        
        // 等待响应
        let result = response_rx.await
            .map_err(|_| QuickDbError::ConnectionError {
                message: "等待连接池响应超时".to_string(),
            })??;
        
        Ok(serde_json::to_string(&result)
            .map_err(|e| QuickDbError::SerializationError { message: format!("序列化失败: {}", e) })?)
    }
    
    /// 直接使用连接池处理根据ID查询请求
    async fn handle_find_by_id_direct(
        collection: &str,
        id: &str,
        alias: Option<String>,
    ) -> QuickDbResult<Option<String>> {
        use rat_quickdb::pool::DatabaseOperation;
        use tokio::sync::oneshot;
        
        let actual_alias = alias.unwrap_or_else(|| "default".to_string());
        debug!("直接处理根据ID查询请求: collection={}, id={}, alias={}", collection, id, actual_alias);
        
        let manager = get_global_pool_manager();
        let connection_pools = manager.get_connection_pools();
        let connection_pool = connection_pools.get(&actual_alias)
            .ok_or_else(|| QuickDbError::AliasNotFound {
                alias: actual_alias.clone(),
            })?;
        
        // 创建oneshot通道用于接收响应
        let (response_tx, response_rx) = oneshot::channel();
        
        // 发送DatabaseOperation::FindById请求到连接池
        let operation = DatabaseOperation::FindById {
            table: collection.to_string(),
            id: DataValue::String(id.to_string()),
            response: response_tx,
        };
        
        connection_pool.operation_sender.send(operation)
            .map_err(|_| QuickDbError::ConnectionError {
                message: "连接池操作通道已关闭".to_string(),
            })?;
        
        // 等待响应
        let result = response_rx.await
            .map_err(|_| QuickDbError::ConnectionError {
                message: "等待连接池响应超时".to_string(),
            })??;
        
        if let Some(value) = result {
            Ok(Some(serde_json::to_string(&value)
                .map_err(|e| QuickDbError::SerializationError { message: format!("序列化失败: {}", e) })?))
        } else {
            Ok(None)
        }
    }
    
    /// 直接使用连接池处理查询请求
    async fn handle_find_direct(
        collection: &str,
        conditions: Vec<QueryCondition>,
        options: Option<QueryOptions>,
        alias: Option<String>,
    ) -> QuickDbResult<String> {
        use rat_quickdb::pool::DatabaseOperation;
        use tokio::sync::oneshot;
        
        let actual_alias = alias.unwrap_or_else(|| "default".to_string());
        debug!("直接处理查询请求: collection={}, alias={}", collection, actual_alias);
        
        let manager = get_global_pool_manager();
        let connection_pools = manager.get_connection_pools();
        let connection_pool = connection_pools.get(&actual_alias)
            .ok_or_else(|| QuickDbError::AliasNotFound {
                alias: actual_alias.clone(),
            })?;
        
        // 创建oneshot通道用于接收响应
        let (response_tx, response_rx) = oneshot::channel();
        
        // 发送DatabaseOperation::Find请求到连接池
        let operation = DatabaseOperation::Find {
            table: collection.to_string(),
            conditions,
            options: options.unwrap_or_default(),
            response: response_tx,
        };
        
        connection_pool.operation_sender.send(operation)
            .map_err(|_| QuickDbError::ConnectionError {
                message: "连接池操作通道已关闭".to_string(),
            })?;
        
        // 等待响应
        let result = response_rx.await
            .map_err(|_| QuickDbError::ConnectionError {
                message: "等待连接池响应超时".to_string(),
            })??;
        
        serde_json::to_string(&result)
            .map_err(|e| QuickDbError::SerializationError { message: format!("序列化失败: {}", e) })
    }
}

/// PyDbQueueBridge的辅助方法实现
impl PyDbQueueBridge {
    /// 发送请求
    fn send_request(&self, request: PyDbRequest) -> PyResult<()> {
        let sender_guard = self.request_sender.lock().unwrap();
        if let Some(sender) = sender_guard.as_ref() {
            sender.send(request)
                .map_err(|_| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("发送请求失败"))?;
            Ok(())
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("请求发送器未初始化"))
        }
    }

    /// 等待响应
    fn wait_for_response(
        &self,
        response_receiver: oneshot::Receiver<PyDbResponse>,
    ) -> PyResult<String> {
        let rt = Runtime::new().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("创建运行时失败: {}", e))
        })?;
        
        let response = rt.block_on(response_receiver).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("等待响应失败: {}", e))
        })?;
        
        if response.success {
            Ok(response.data)
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                response.error.unwrap_or_else(|| "未知错误".to_string())
            ))
        }
    }

    /// 检查初始化状态
    fn check_initialized(&self) -> PyResult<()> {
        let initialized = self.initialized.lock().unwrap();
        if *initialized {
            Ok(())
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("桥接器未初始化"))
        }
    }

    /// 解析JSON到数据映射
    fn parse_json_to_data_map(&self, json_str: &str) -> Result<HashMap<String, DataValue>, String> {
        let json_value: JsonValue = serde_json::from_str(json_str)
            .map_err(|e| format!("JSON解析失败: {}", e))?;
        
        if let JsonValue::Object(map) = json_value {
            let mut data_map = HashMap::new();
            for (key, value) in map {
                let data_value = match value {
                    JsonValue::String(s) => DataValue::String(s),
                    JsonValue::Number(n) => {
                        if let Some(i) = n.as_i64() {
                            DataValue::Int(i)
                        } else if let Some(f) = n.as_f64() {
                            DataValue::Float(f)
                        } else {
                            return Err(format!("不支持的数字类型: {}", n));
                        }
                    }
                    JsonValue::Bool(b) => DataValue::Bool(b),
                    JsonValue::Null => DataValue::Null,
                    _ => return Err(format!("不支持的数据类型: {:?}", value)),
                };
                data_map.insert(key, data_value);
            }
            Ok(data_map)
        } else {
            Err("JSON必须是对象类型".to_string())
        }
    }

    /// 解析查询条件JSON
    fn parse_conditions_json(&self, json_str: &str) -> Result<Vec<QueryCondition>, String> {
        let json_value: JsonValue = serde_json::from_str(json_str)
            .map_err(|e| format!("JSON解析失败: {}", e))?;
        
        if let JsonValue::Object(map) = json_value {
            let mut conditions = Vec::new();
            for (field, value) in map {
                let condition = QueryCondition {
                    field,
                    operator: QueryOperator::Eq,
                    value: match value {
                        JsonValue::String(s) => DataValue::String(s),
                        JsonValue::Number(n) => {
                            if let Some(i) = n.as_i64() {
                                DataValue::Int(i)
                            } else if let Some(f) = n.as_f64() {
                                DataValue::Float(f)
                            } else {
                                return Err(format!("不支持的数字类型: {}", n));
                            }
                        }
                        JsonValue::Bool(b) => DataValue::Bool(b),
                        JsonValue::Null => DataValue::Null,
                        _ => return Err(format!("不支持的数据类型: {:?}", value)),
                    },
                };
                conditions.push(condition);
            }
            Ok(conditions)
        } else {
            Err("查询条件JSON必须是对象类型".to_string())
        }
    }
}

/// 创建数据库队列桥接器
#[pyfunction]
pub fn create_db_queue_bridge() -> PyResult<PyDbQueueBridge> {
    PyDbQueueBridge::new()
}

/// Python模块定义
#[pymodule]
fn rat_quickdb_py(_py: Python, m: &PyModule) -> PyResult<()> {
    // 基础函数
    m.add_function(wrap_pyfunction!(get_version, m)?)?;
    m.add_function(wrap_pyfunction!(get_info, m)?)?;
    m.add_function(wrap_pyfunction!(get_name, m)?)?;
    m.add_function(wrap_pyfunction!(create_db_queue_bridge, m)?)?;
    
    // 数据库桥接器
    m.add_class::<PyDbQueueBridge>()?;
    
    // ODM 模型系统类
    m.add_class::<PyFieldType>()?;
    m.add_class::<PyFieldDefinition>()?;
    m.add_class::<PyIndexDefinition>()?;
    m.add_class::<PyModelMeta>()?;
    
    // 便捷字段创建函数
    m.add_function(wrap_pyfunction!(string_field, m)?)?;
    m.add_function(wrap_pyfunction!(integer_field, m)?)?;
    m.add_function(wrap_pyfunction!(boolean_field, m)?)?;
    m.add_function(wrap_pyfunction!(datetime_field, m)?)?;
    m.add_function(wrap_pyfunction!(uuid_field, m)?)?;
    m.add_function(wrap_pyfunction!(reference_field, m)?)?;
    m.add_function(wrap_pyfunction!(array_field, m)?)?;
    m.add_function(wrap_pyfunction!(json_field, m)?)?;
    
    Ok(())
}
