//! 数据库队列桥接器模块
//! 提供Python与Rust数据库操作的桥接功能

use crate::config::*;
use pyo3::prelude::*;
use rat_quickdb::config::DatabaseConfigBuilder;
use crate::model_bindings::PyModelMeta;
use rat_quickdb::types::{
    ConnectionConfig, DatabaseType, IdStrategy, PoolConfig, TlsConfig, ZstdConfig,
};
use serde_json::Value as JsonValue;
use std::sync::Arc;

// 导入JSON队列桥接器
use rat_quickdb::python_api::json_queue_bridge::PyJsonQueueBridge;

/// Python数据库队列桥接器
/// 提供异步数据库操作的Python接口
#[pyclass(name = "DbQueueBridge")]
pub struct PyDbQueueBridge {
    /// 默认数据库别名
    default_alias: Arc<std::sync::Mutex<Option<String>>>,
    /// 初始化状态
    initialized: Arc<std::sync::Mutex<bool>>,
    /// 持有的SimpleQueueBridge实例
    simple_bridge: Arc<rat_quickdb::python_api::simple_queue_bridge::SimpleQueueBridge>,
}

#[pymethods]
impl PyDbQueueBridge {
    /// 创建新的数据库队列桥接器
    #[new]
    pub fn new() -> PyResult<Self> {
        // 创建持久的SimpleQueueBridge实例
        let simple_bridge = rat_quickdb::python_api::simple_queue_bridge::SimpleQueueBridge::new()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("创建SimpleQueueBridge失败: {}", e)))?;

        Ok(PyDbQueueBridge {
            default_alias: Arc::new(std::sync::Mutex::new(None)),
            initialized: Arc::new(std::sync::Mutex::new(true)),
            simple_bridge: Arc::new(simple_bridge),
        })
    }

    /// 创建数据记录
    pub fn create(
        &self,
        table: String,
        data_json: String,
        alias: Option<String>,
    ) -> PyResult<String> {
        self.check_initialized()?;

        let body = serde_json::json!({
            "table": table,
            "data": serde_json::from_str::<serde_json::Value>(&data_json)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("解析数据JSON失败: {}", e)))?,
            "alias": alias
        }).to_string();

        self.send_action_request("create", &body)
    }

    /// 查找数据记录（智能检测查询类型）
    pub fn find(
        &self,
        table: String,
        query_json: String,
        alias: Option<String>,
    ) -> PyResult<String> {
        self.check_initialized()?;

        // 智能检测查询类型
        if self.is_condition_groups_query(&query_json) {
            // 条件组合查询
            let body = serde_json::json!({
                "table": table,
                "condition_groups": serde_json::from_str::<serde_json::Value>(&query_json)
                    .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("解析查询条件失败: {}", e)))?,
                "alias": alias
            }).to_string();

            self.send_action_request("find_with_groups", &body)
        } else {
            // 普通查询
            let body = serde_json::json!({
                "table": table,
                "conditions": serde_json::from_str::<serde_json::Value>(&query_json)
                    .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("解析查询条件失败: {}", e)))?,
                "alias": alias
            }).to_string();

            self.send_action_request("find", &body)
        }
    }

    /// 使用条件组合查找数据记录
    pub fn find_with_groups(
        &self,
        table: String,
        query_groups_json: String,
        alias: Option<String>,
    ) -> PyResult<String> {
        self.check_initialized()?;

        let body = serde_json::json!({
            "table": table,
            "condition_groups": serde_json::from_str::<serde_json::Value>(&query_groups_json)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("解析查询条件组失败: {}", e)))?,
            "alias": alias
        }).to_string();

        self.send_action_request("find_with_groups", &body)
    }

    /// 根据ID查找数据记录
    pub fn find_by_id(&self, table: String, id: String, alias: Option<String>) -> PyResult<String> {
        self.check_initialized()?;

        let body = serde_json::json!({
            "table": table,
            "id": id,
            "alias": alias
        }).to_string();

        self.send_action_request("find_by_id", &body)
    }

    /// 根据ID查找数据记录（Python原生格式）
    /// 自动转换DataValue格式为Python原生类型
    pub fn find_by_id_native(&self, table: String, id: String, alias: Option<String>) -> PyResult<String> {
        self.check_initialized()?;

        let body = serde_json::json!({
            "table": table,
            "id": id,
            "alias": alias
        }).to_string();

        let response = self.send_action_request("find_by_id", &body)?;

        // 在Python层进行DataValue到原生类型的转换
        // 这里标记需要转换，实际转换在Python层完成
        Ok(serde_json::json!({
            "success": true,
            "data": serde_json::from_str::<serde_json::Value>(&response).unwrap_or(serde_json::Value::Null),
            "_convert": true  // 标记需要自动转换
        }).to_string())
    }

    /// 统计符合条件的记录数量
    pub fn count(
        &self,
        table: String,
        conditions_json: String,
        alias: Option<String>,
    ) -> PyResult<String> {
        self.check_initialized()?;

        let body = serde_json::json!({
            "table": table,
            "conditions": serde_json::from_str::<serde_json::Value>(&conditions_json)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("解析查询条件失败: {}", e)))?,
            "alias": alias
        }).to_string();

        self.send_action_request("count", &body)
    }

    /// 删除数据记录
    pub fn delete(
        &self,
        table: String,
        conditions_json: String,
        alias: Option<String>,
    ) -> PyResult<String> {
        self.check_initialized()?;

        let body = serde_json::json!({
            "table": table,
            "conditions": serde_json::from_str::<serde_json::Value>(&conditions_json)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("解析删除条件失败: {}", e)))?,
            "alias": alias
        }).to_string();

        self.send_action_request("delete", &body)
    }

    /// 根据ID删除数据记录
    pub fn delete_by_id(&self, table: String, id: String, alias: Option<String>) -> PyResult<String> {
        self.check_initialized()?;

        let body = serde_json::json!({
            "table": table,
            "id": id,
            "alias": alias
        }).to_string();

        self.send_action_request("delete_by_id", &body)
    }

    /// 更新数据记录
    pub fn update(
        &self,
        table: String,
        conditions_json: String,
        updates_json: String,
        alias: Option<String>,
    ) -> PyResult<String> {
        self.check_initialized()?;

        let body = serde_json::json!({
            "table": table,
            "conditions": serde_json::from_str::<serde_json::Value>(&conditions_json)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("解析更新条件失败: {}", e)))?,
            "updates": serde_json::from_str::<serde_json::Value>(&updates_json)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("解析更新数据失败: {}", e)))?,
            "alias": alias
        }).to_string();

        self.send_action_request("update", &body)
    }

    /// 根据ID更新数据记录
    pub fn update_by_id(
        &self,
        table: String,
        id: String,
        updates_json: String,
        alias: Option<String>,
    ) -> PyResult<String> {
        self.check_initialized()?;

        let body = serde_json::json!({
            "table": table,
            "id": id,
            "updates": serde_json::from_str::<serde_json::Value>(&updates_json)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("解析更新数据失败: {}", e)))?,
            "alias": alias
        }).to_string();

        self.send_action_request("update_by_id", &body)
    }

    /// 添加SQLite数据库
    pub fn add_sqlite_database(
        &self,
        alias: String,
        path: String,
        create_if_missing: Option<bool>,
        max_connections: Option<u32>,
        min_connections: Option<u32>,
        connection_timeout: Option<u64>,
        idle_timeout: Option<u64>,
        max_lifetime: Option<u64>,
        cache_config: Option<PyCacheConfig>,
        id_strategy: Option<String>,
    ) -> PyResult<String> {
        let mut pool_config_builder = PoolConfig::builder();

        if let Some(max_conn) = max_connections {
            pool_config_builder = pool_config_builder.max_connections(max_conn);
        }
        if let Some(min_conn) = min_connections {
            pool_config_builder = pool_config_builder.min_connections(min_conn);
        }
        if let Some(timeout) = connection_timeout {
            pool_config_builder = pool_config_builder.connection_timeout(timeout);
        }
        if let Some(idle) = idle_timeout {
            pool_config_builder = pool_config_builder.idle_timeout(idle);
        }
        if let Some(lifetime) = max_lifetime {
            pool_config_builder = pool_config_builder.max_lifetime(lifetime);
        }

        let pool_config = pool_config_builder.build()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("构建连接池配置失败: {}", e)))?;

        let create_if_missing_value = create_if_missing.unwrap_or(true);

        // 解析ID策略，默认使用AutoIncrement
        let id_strategy_enum = match id_strategy.as_deref() {
            Some("Uuid") => IdStrategy::Uuid,
            Some("Snowflake") => IdStrategy::Snowflake { machine_id: 1, datacenter_id: 1 },
            Some("ObjectId") => IdStrategy::ObjectId,
            Some(custom) if custom.starts_with("Custom:") => {
                let prefix = custom.strip_prefix("Custom:").unwrap_or(custom);
                IdStrategy::Custom(prefix.to_string())
            },
            _ => IdStrategy::AutoIncrement,
        };

        let mut db_config_builder = DatabaseConfigBuilder::new()
            .db_type(DatabaseType::SQLite)
            .connection(ConnectionConfig::SQLite {
                path,
                create_if_missing: create_if_missing_value,
            })
            .pool(pool_config)
            .alias(alias.clone())
            .id_strategy(id_strategy_enum);

        if let Some(cache_cfg) = cache_config {
            db_config_builder = db_config_builder.cache(cache_cfg.to_rust_config());
        }

        let database_config = db_config_builder.build()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("构建数据库配置失败: {}", e)))?;

        // 构建数据库配置JSON
        let db_config_json = serde_json::to_string(&database_config)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("序列化数据库配置失败: {}", e)))?;

        let body = serde_json::json!({
            "database_config": serde_json::from_str::<serde_json::Value>(&db_config_json)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("解析数据库配置失败: {}", e)))?,
            "alias": alias
        }).to_string();

        let response = self.send_action_request("add_database", &body)?;

        // 如果这是第一个数据库，设置为默认别名
        {
            let mut default_alias_guard = self.default_alias.lock().unwrap();
            if default_alias_guard.is_none() {
                *default_alias_guard = Some(alias);
            }
        }

        Ok(response)
    }

    /// 添加MongoDB数据库
    pub fn add_mongodb_database(
        &self,
        alias: String,
        host: String,
        port: u16,
        database: String,
        username: String,
        password: String,
        auth_source: Option<String>,
        direct_connection: Option<bool>,
        max_connections: Option<u32>,
        min_connections: Option<u32>,
        connection_timeout: Option<u64>,
        idle_timeout: Option<u64>,
        max_lifetime: Option<u64>,
        cache_config: Option<PyCacheConfig>,
        tls_config: Option<PyTlsConfig>,
        zstd_config: Option<PyZstdConfig>,
    ) -> PyResult<String> {
        let mut pool_config_builder = PoolConfig::builder();

        if let Some(max_conn) = max_connections {
            pool_config_builder = pool_config_builder.max_connections(max_conn);
        }
        if let Some(min_conn) = min_connections {
            pool_config_builder = pool_config_builder.min_connections(min_conn);
        }
        if let Some(timeout) = connection_timeout {
            pool_config_builder = pool_config_builder.connection_timeout(timeout);
        }
        if let Some(idle) = idle_timeout {
            pool_config_builder = pool_config_builder.idle_timeout(idle);
        }
        if let Some(lifetime) = max_lifetime {
            pool_config_builder = pool_config_builder.max_lifetime(lifetime);
        }

        let pool_config = pool_config_builder.build()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("构建连接池配置失败: {}", e)))?;

        let auth_source_value = auth_source.unwrap_or_else(|| "admin".to_string());
        let direct_connection_value = direct_connection.unwrap_or(true);

        // 构建TLS配置
        let final_tls_config = if let Some(tls_cfg) = tls_config {
            Some(TlsConfig {
                enabled: tls_cfg.enabled,
                ca_cert_path: tls_cfg.ca_cert_path.clone(),
                client_cert_path: tls_cfg.client_cert_path.clone(),
                client_key_path: tls_cfg.client_key_path.clone(),
                verify_server_cert: tls_cfg.verify_server_cert,
                verify_hostname: tls_cfg.verify_hostname,
                min_tls_version: tls_cfg.min_tls_version.clone(),
                cipher_suites: tls_cfg.cipher_suites.clone(),
            })
        } else {
            None
        };

        // 构建ZSTD配置
        let final_zstd_config = if let Some(zstd_cfg) = zstd_config {
            Some(ZstdConfig {
                enabled: zstd_cfg.enabled,
                compression_level: zstd_cfg.compression_level,
                compression_threshold: zstd_cfg.compression_threshold,
            })
        } else {
            None
        };

        let mut db_config_builder = DatabaseConfigBuilder::new()
            .db_type(DatabaseType::MongoDB)
            .connection(ConnectionConfig::MongoDB {
                host,
                port,
                database,
                username: Some(username),
                password: Some(password),
                auth_source: Some(auth_source_value),
                direct_connection: direct_connection_value,
                tls_config: final_tls_config,
                zstd_config: final_zstd_config,
                options: None,
            })
            .pool(pool_config)
            .alias(alias.clone())
            .id_strategy(IdStrategy::ObjectId);

        if let Some(cache_cfg) = cache_config {
            db_config_builder = db_config_builder.cache(cache_cfg.to_rust_config());
        }

        // TLS和ZSTD配置已经包含在ConnectionConfig中

        let database_config = db_config_builder.build()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("构建数据库配置失败: {}", e)))?;

        // 构建数据库配置JSON
        let db_config_json = serde_json::to_string(&database_config)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("序列化数据库配置失败: {}", e)))?;

        let body = serde_json::json!({
            "database_config": serde_json::from_str::<serde_json::Value>(&db_config_json)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("解析数据库配置失败: {}", e)))?,
            "alias": alias
        }).to_string();

        let response = self.send_action_request("add_database", &body)?;

        // 如果这是第一个数据库，设置为默认别名
        {
            let mut default_alias_guard = self.default_alias.lock().unwrap();
            if default_alias_guard.is_none() {
                *default_alias_guard = Some(alias);
            }
        }

        Ok(response)
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
        cache_config: Option<PyCacheConfig>,
        id_strategy: Option<String>,
    ) -> PyResult<String> {
        let mut pool_config_builder = PoolConfig::builder();

        if let Some(max_conn) = max_connections {
            pool_config_builder = pool_config_builder.max_connections(max_conn);
        }
        if let Some(min_conn) = min_connections {
            pool_config_builder = pool_config_builder.min_connections(min_conn);
        }
        if let Some(timeout) = connection_timeout {
            pool_config_builder = pool_config_builder.connection_timeout(timeout);
        }
        if let Some(idle) = idle_timeout {
            pool_config_builder = pool_config_builder.idle_timeout(idle);
        }
        if let Some(lifetime) = max_lifetime {
            pool_config_builder = pool_config_builder.max_lifetime(lifetime);
        }

        let pool_config = pool_config_builder.build()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("构建连接池配置失败: {}", e)))?;

        // 解析ID策略，默认使用AutoIncrement
        let id_strategy_enum = match id_strategy.as_deref() {
            Some("Uuid") => IdStrategy::Uuid,
            Some("Snowflake") => IdStrategy::Snowflake { machine_id: 1, datacenter_id: 1 },
            Some("ObjectId") => IdStrategy::ObjectId,
            Some(custom) if custom.starts_with("Custom:") => {
                let prefix = custom.strip_prefix("Custom:").unwrap_or(custom);
                IdStrategy::Custom(prefix.to_string())
            },
            _ => IdStrategy::AutoIncrement,
        };

        let mut db_config_builder = DatabaseConfigBuilder::new()
            .db_type(DatabaseType::MySQL)
            .connection(ConnectionConfig::MySQL {
                host,
                port,
                database,
                username,
                password,
                ssl_opts: None,
                tls_config: None,
            })
            .pool(pool_config)
            .alias(alias.clone())
            .id_strategy(id_strategy_enum);

        if let Some(cache_cfg) = cache_config {
            db_config_builder = db_config_builder.cache(cache_cfg.to_rust_config());
        }

        let database_config = db_config_builder.build()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("构建数据库配置失败: {}", e)))?;

        // 构建数据库配置JSON
        let db_config_json = serde_json::to_string(&database_config)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("序列化数据库配置失败: {}", e)))?;

        let body = serde_json::json!({
            "database_config": serde_json::from_str::<serde_json::Value>(&db_config_json)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("解析数据库配置失败: {}", e)))?,
            "alias": alias
        }).to_string();

        let response = self.send_action_request("add_database", &body)?;

        // 如果这是第一个数据库，设置为默认别名
        {
            let mut default_alias_guard = self.default_alias.lock().unwrap();
            if default_alias_guard.is_none() {
                *default_alias_guard = Some(alias);
            }
        }

        Ok(response)
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
        max_connections: Option<u32>,
        min_connections: Option<u32>,
        connection_timeout: Option<u64>,
        idle_timeout: Option<u64>,
        max_lifetime: Option<u64>,
        cache_config: Option<PyCacheConfig>,
        id_strategy: Option<String>,
    ) -> PyResult<String> {
        let mut pool_config_builder = PoolConfig::builder();

        if let Some(max_conn) = max_connections {
            pool_config_builder = pool_config_builder.max_connections(max_conn);
        }
        if let Some(min_conn) = min_connections {
            pool_config_builder = pool_config_builder.min_connections(min_conn);
        }
        if let Some(timeout) = connection_timeout {
            pool_config_builder = pool_config_builder.connection_timeout(timeout);
        }
        if let Some(idle) = idle_timeout {
            pool_config_builder = pool_config_builder.idle_timeout(idle);
        }
        if let Some(lifetime) = max_lifetime {
            pool_config_builder = pool_config_builder.max_lifetime(lifetime);
        }

        let pool_config = pool_config_builder.build()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("构建连接池配置失败: {}", e)))?;

        // 解析ID策略，默认使用AutoIncrement
        let id_strategy_enum = match id_strategy.as_deref() {
            Some("Uuid") => IdStrategy::Uuid,
            Some("Snowflake") => IdStrategy::Snowflake { machine_id: 1, datacenter_id: 1 },
            Some("ObjectId") => IdStrategy::ObjectId,
            Some(custom) if custom.starts_with("Custom:") => {
                let prefix = custom.strip_prefix("Custom:").unwrap_or(custom);
                IdStrategy::Custom(prefix.to_string())
            },
            _ => IdStrategy::AutoIncrement,
        };

        let mut db_config_builder = DatabaseConfigBuilder::new()
            .db_type(DatabaseType::PostgreSQL)
            .connection(ConnectionConfig::PostgreSQL {
                host,
                port,
                database,
                username,
                password,
                ssl_mode: None,
                tls_config: None,
            })
            .pool(pool_config)
            .alias(alias.clone())
            .id_strategy(id_strategy_enum);

        if let Some(cache_cfg) = cache_config {
            db_config_builder = db_config_builder.cache(cache_cfg.to_rust_config());
        }

        let database_config = db_config_builder.build()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("构建数据库配置失败: {}", e)))?;

        // 构建数据库配置JSON
        let db_config_json = serde_json::to_string(&database_config)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("序列化数据库配置失败: {}", e)))?;

        let body = serde_json::json!({
            "database_config": serde_json::from_str::<serde_json::Value>(&db_config_json)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("解析数据库配置失败: {}", e)))?,
            "alias": alias
        }).to_string();

        let response = self.send_action_request("add_database", &body)?;

        // 如果这是第一个数据库，设置为默认别名
        {
            let mut default_alias_guard = self.default_alias.lock().unwrap();
            if default_alias_guard.is_none() {
                *default_alias_guard = Some(alias);
            }
        }

        Ok(response)
    }

    /// 设置默认数据库别名
    pub fn set_default_alias(&self, alias: String) -> PyResult<()> {
        let mut default_alias_guard = self.default_alias.lock().unwrap();
        *default_alias_guard = Some(alias);
        Ok(())
    }

    /// 注册ODM模型
    pub fn register_model(&self, model_meta: &PyModelMeta) -> PyResult<String> {
        self.check_initialized()?;

        let body = serde_json::json!({
            "model_meta": serde_json::from_str::<serde_json::Value>(&serde_json::to_string(&model_meta.inner).unwrap())
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("解析模型元数据失败: {}", e)))?
        }).to_string();

        self.send_action_request("register_model", &body)
    }

    /// 删除表
    pub fn drop_table(
        &self,
        table: String,
        alias: Option<String>,
    ) -> PyResult<String> {
        self.check_initialized()?;

        let body = serde_json::json!({
            "table": table,
            "alias": alias
        }).to_string();

        self.send_action_request("drop_table", &body)
    }

    /// 创建表
    pub fn create_table(
        &self,
        table: String,
        fields_json: String,
        alias: Option<String>,
    ) -> PyResult<String> {
        self.check_initialized()?;

        let body = serde_json::json!({
            "table": table,
            "fields": serde_json::from_str::<serde_json::Value>(&fields_json)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("解析字段定义失败: {}", e)))?,
            "alias": alias
        }).to_string();

        self.send_action_request("create_table", &body)
    }
}

// PyDbQueueBridge的内部实现方法
impl PyDbQueueBridge {
    /// 发送action请求
    fn send_action_request(&self, action: &str, body: &str) -> PyResult<String> {
        // 使用持久的simple_queue_bridge进行JSON字符串通信
        // 构建请求数据 - 将action和body合并
        let mut request_data = serde_json::json!({});
        if let Ok(mut body_obj) = serde_json::from_str::<serde_json::Value>(body) {
            if let Some(obj) = body_obj.as_object_mut() {
                request_data.as_object_mut().unwrap().extend(obj.clone());
            }
        }

        let request_json = serde_json::to_string(&request_data)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("序列化请求数据失败: {}", e)))?;

        // 通过持久的simple_queue_bridge发送请求
        self.simple_bridge.send_request(action.to_string(), request_json)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("请求失败: {}", e)))
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

    /// 检测是否为条件组合查询（包含operator字段的OR/AND逻辑查询）
    fn is_condition_groups_query(&self, json_str: &str) -> bool {
        if let Ok(json_value) = serde_json::from_str::<JsonValue>(json_str) {
            match json_value {
                // 检查单个对象是否包含operator字段
                JsonValue::Object(ref obj) => {
                    obj.contains_key("operator") && obj.contains_key("conditions")
                },
                // 检查数组中是否有任何元素包含operator字段
                JsonValue::Array(ref arr) => {
                    arr.iter().any(|item| {
                        if let JsonValue::Object(ref obj) = item {
                            obj.contains_key("operator") && obj.contains_key("conditions")
                        } else {
                            false
                        }
                    })
                },
                _ => false,
            }
        } else {
            false
        }
    }
}

/// 创建数据库队列桥接器
#[pyfunction]
pub fn create_db_queue_bridge() -> PyResult<PyDbQueueBridge> {
    PyDbQueueBridge::new()
}

/// 创建JSON队列桥接器 - 使用JSON字符串与全局任务队列系统交互
#[pyfunction]
pub fn create_json_queue_bridge() -> PyResult<PyJsonQueueBridge> {
    Ok(PyJsonQueueBridge::new())
}

/// 注册ODM模型
#[pyfunction]
pub fn register_model(model_meta: &PyModelMeta) -> PyResult<String> {
    // 这个函数保持为独立函数，但内部使用队列机制
    // 创建一个临时的桥接器实例用于模型注册
    let bridge = match PyDbQueueBridge::new() {
        Ok(bridge) => bridge,
        Err(e) => {
            let response = serde_json::json!({
                "success": false,
                "message": "创建桥接器失败",
                "error": format!("{}", e)
            });
            return Ok(response.to_string());
        }
    };

    // 通过桥接器发送模型注册消息
    match bridge.register_model(model_meta) {
        Ok(response) => Ok(response),
        Err(e) => {
            let response = serde_json::json!({
                "success": false,
                "message": "模型注册失败",
                "error": format!("{}", e)
            });
            Ok(response.to_string())
        }
    }
}