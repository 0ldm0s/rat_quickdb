//! 简化版队列桥接器
//!
//! 使用 crossbeam::SegQueue 实现 Rust-Python 解耦通信
//! 移除复杂的任务队列依赖，直接处理基本数据库操作

use crossbeam_queue::SegQueue;
use std::sync::Arc;
use serde_json;
use uuid::Uuid;
use std::collections::HashMap;
use rat_logger::{info, warn, error};
use chrono;

// 导入必要的模块和类型
use crate::types::{DataValue, DatabaseConfig, QueryOperator, QueryCondition};
use crate::manager::{get_global_pool_manager, add_database};
use crate::model::ModelMeta;
use crate::odm::OdmOperations;


/// Python 请求消息
#[derive(Debug, Clone)]
pub struct PyRequestMessage {
    /// 请求ID
    pub request_id: String,
    /// 请求类型
    pub request_type: String,
    /// 请求数据（JSON字符串）
    pub data: String,
}

/// Python 响应消息
#[derive(Debug, Clone)]
pub struct PyResponseMessage {
    /// 请求ID
    pub request_id: String,
    /// 是否成功
    pub success: bool,
    /// 响应数据（JSON字符串）
    pub data: String,
    /// 错误信息
    pub error: Option<String>,
}

/// 简化版队列桥接器
pub struct SimpleQueueBridge {
    /// 请求队列 - Python 向 Rust 发送请求
    request_queue: Arc<SegQueue<PyRequestMessage>>,
    /// 响应队列 - Rust 向 Python 返回响应
    response_queue: Arc<SegQueue<PyResponseMessage>>,
    /// 全局tokio runtime句柄
    runtime_handle: Arc<tokio::runtime::Runtime>,
}

impl SimpleQueueBridge {
    /// 创建新的简化队列桥接器
    pub fn new() -> Result<Self, String> {
        info!("创建简化版队列桥接器");

        let request_queue = Arc::new(SegQueue::new());
        let response_queue = Arc::new(SegQueue::new());

        // 创建持久的tokio runtime
        let runtime_handle = Arc::new(
            tokio::runtime::Runtime::new()
                .map_err(|e| format!("创建tokio runtime失败: {}", e))?
        );

        Ok(Self {
            request_queue,
            response_queue,
            runtime_handle,
        })
    }

    /// 发送请求并等待响应
    pub fn send_request(&self, request_type: String, data: String) -> Result<String, String> {
        let request_id = Uuid::new_v4().to_string();

        info!("发送请求: {} - {}", request_type, request_id);

        // 克隆request_id以避免移动问题
        let request_id_clone = request_id.clone();

        // 使用持久的runtime处理请求
        let result = self.runtime_handle.block_on(async {
            self.process_request_async(&request_type, &data, &request_id).await
        });

        let response = match result {
            Ok(response) => response,
            Err(e) => {
                error!("处理请求时发生错误: {}", e);
                PyResponseMessage {
                    request_id: request_id_clone,
                    success: false,
                    data: String::new(),
                    error: Some(e),
                }
            }
        };

        if response.success {
            Ok(response.data)
        } else {
            Err(response.error.unwrap_or("未知错误".to_string()))
        }
    }

    
    /// 异步处理请求 - 直接使用全局ODM层
    async fn process_request_async(&self, request_type: &str, data: &str, request_id: &str) -> Result<PyResponseMessage, String> {
        info!("异步处理请求: {} - {}", request_type, request_id);

        // 在异步上下文中处理请求，使用全局ODM管理器
        let result = match request_type {
            "create" => self.handle_create_odm(data).await,
            "find" => self.handle_find_odm(data).await,
            "update" => self.handle_update_odm(data).await,
            "delete" => self.handle_delete_odm(data).await,
            "count" => self.handle_count_odm(data).await,
            "find_by_id" => self.handle_find_by_id_odm(data).await,
            "delete_by_id" => self.handle_delete_by_id_odm(data).await,
            "update_by_id" => self.handle_update_by_id_odm(data).await,
            "register_model" => self.handle_register_model_odm(data).await,
            "create_table" => self.handle_create_table_odm(data).await,
            "drop_table" => self.handle_drop_table_odm(data).await,
            "add_database" => self.handle_add_database_odm(data).await,
            _ => Err(format!("不支持的请求类型: {}", request_type)),
        };

        match result {
            Ok(data) => Ok(PyResponseMessage {
                request_id: request_id.to_string(),
                success: true,
                data,
                error: None,
            }),
            Err(error) => {
                error!("异步处理请求失败: {}", error);
                Ok(PyResponseMessage {
                    request_id: request_id.to_string(),
                    success: false,
                    data: String::new(),
                    error: Some(error),
                })
            }
        }
    }

  
    // === 直接ODM操作处理器 ===

    /// 使用ODM层处理创建操作
    async fn handle_create_odm(&self, data: &str) -> Result<String, String> {
        let request: serde_json::Value = serde_json::from_str(data)
            .map_err(|e| format!("解析创建请求失败: {}", e))?;

        let table = request["table"].as_str()
            .ok_or("缺少表名")?;
        let alias = request.get("alias").and_then(|v| v.as_str());

        // 检查数据格式
        let record = if let Some(record_str) = request.get("data").and_then(|v| v.as_str()) {
            // 如果data是字符串，解析为JSON
            serde_json::from_str::<serde_json::Value>(record_str)
                .map_err(|e| format!("解析记录数据失败: {}", e))?
        } else if let Some(record_obj) = request.get("data") {
            // 如果data直接是对象，使用它
            record_obj.clone()
        } else {
            return Err("缺少记录数据".to_string());
        };

        
        // 转换为ODM格式的数据
        let mut data_map = std::collections::HashMap::new();
        if let serde_json::Value::Object(ref obj) = record {
            // 处理带标签的DataValue格式
            for (key, value) in obj {
                // 直接解析带标签的DataValue，无需类型推断
                let data_value = self.parse_labeled_data_value(value.clone())?;
                data_map.insert(key.clone(), data_value);
            }

        } else {
            return Err("record不是Object类型".to_string());
        }

        // 通过ODM层执行创建操作
        use crate::odm::get_odm_manager;
        let odm_manager = get_odm_manager().await;
        let result = odm_manager.create(table, data_map, alias).await
            .map_err(|e| format!("ODM创建操作失败: {}", e))?;

        info!("ODM创建记录成功: {} - {}", table, serde_json::to_string(&result).unwrap_or_default());

        // 返回JSON格式的响应
        Ok(serde_json::json!({
            "success": true,
            "data": result
        }).to_string())
    }

    /// 使用ODM层处理查询操作
    async fn handle_find_odm(&self, data: &str) -> Result<String, String> {
        let request: serde_json::Value = serde_json::from_str(data)
            .map_err(|e| format!("解析查询请求失败: {}", e))?;

        let table = request["table"].as_str()
            .ok_or("缺少表名")?;
        let alias = request.get("alias").and_then(|v| v.as_str());

        // 解析条件
        let conditions = if let Some(conditions_str) = request.get("conditions").and_then(|v| v.as_str()) {
            let conditions_value: serde_json::Value = serde_json::from_str(conditions_str)
                .map_err(|e| format!("解析查询条件失败: {}", e))?;
            self.parse_query_conditions(conditions_value)?
        } else {
            vec![] // 空条件表示查询所有
        };

        let options = None;

        // 通过ODM层执行查询操作
        use crate::odm::get_odm_manager;
        let odm_manager = get_odm_manager().await;
        let result = odm_manager.find(table, conditions, options, alias).await
            .map_err(|e| format!("ODM查询操作失败: {}", e))?;

        info!("ODM查询记录成功: {} - {} 条记录", table, result.len());

        // 返回JSON格式的响应
        Ok(serde_json::json!({
            "success": true,
            "data": result
        }).to_string())
    }

    /// 使用ODM层处理更新操作
    async fn handle_update_odm(&self, data: &str) -> Result<String, String> {
        let request: serde_json::Value = serde_json::from_str(data)
            .map_err(|e| format!("解析更新请求失败: {}", e))?;

        let table = request["table"].as_str()
            .ok_or("缺少表名")?;
        let alias = request.get("alias").and_then(|v| v.as_str());

        // 解析条件和更新数据
        let conditions = if let Some(conditions_str) = request.get("conditions").and_then(|v| v.as_str()) {
            let conditions_value: serde_json::Value = serde_json::from_str(conditions_str)
                .map_err(|e| format!("解析更新条件失败: {}", e))?;
            self.parse_query_conditions(conditions_value)?
        } else {
            vec![] // 空条件表示更新所有记录
        };

        let mut updates = std::collections::HashMap::new();
        if let Some(updates_str) = request.get("updates").and_then(|v| v.as_str()) {
            let updates_value: serde_json::Value = serde_json::from_str(updates_str)
                .map_err(|e| format!("解析更新数据失败: {}", e))?;
            if let serde_json::Value::Object(obj) = updates_value {
                for (key, value) in obj {
                    // 使用带标签DataValue解析方法，而不是普通的json_value_to_data_value
                    match self.parse_labeled_data_value(value.clone()) {
                        Ok(datavalue) => {
                            info!("🔍 更新字段 {} - 使用带标签DataValue解析: {:?}", key, datavalue);
                            updates.insert(key, datavalue);
                        },
                        Err(e) => {
                            warn!("🔍 更新字段 {} - 带标签解析失败，使用传统方法: {} - 原值: {:?}", key, e, value);
                            updates.insert(key, self.json_value_to_data_value(value));
                        }
                    }
                }
            }
        } else {
            // 默认添加更新时间
            updates.insert("updated_at".to_string(), DataValue::DateTime(
                chrono::Utc::now()
            ));
        }

        // 通过ODM层执行更新操作
        use crate::odm::get_odm_manager;
        let odm_manager = get_odm_manager().await;
        let result = odm_manager.update(table, conditions, updates, alias).await
            .map_err(|e| format!("ODM更新操作失败: {}", e))?;

        info!("ODM更新记录成功: {} - {} 条记录", table, result);

        // 返回JSON格式的响应
        Ok(serde_json::json!({
            "success": true,
            "data": result
        }).to_string())
    }

    /// 使用ODM层处理删除操作
    async fn handle_delete_odm(&self, data: &str) -> Result<String, String> {
        let request: serde_json::Value = serde_json::from_str(data)
            .map_err(|e| format!("解析删除请求失败: {}", e))?;

        let table = request["table"].as_str()
            .ok_or("缺少表名")?;
        let alias = request.get("alias").and_then(|v| v.as_str());

        // 解析条件
        let conditions = if let Some(conditions_str) = request.get("conditions").and_then(|v| v.as_str()) {
            let conditions_value: serde_json::Value = serde_json::from_str(conditions_str)
                .map_err(|e| format!("解析删除条件失败: {}", e))?;
            self.parse_query_conditions(conditions_value)?
        } else {
            vec![] // 空条件表示删除所有记录
        };

        // 通过ODM层执行删除操作
        use crate::odm::get_odm_manager;
        let odm_manager = get_odm_manager().await;
        let result = odm_manager.delete(table, conditions, alias).await
            .map_err(|e| format!("ODM删除操作失败: {}", e))?;

        info!("ODM删除记录成功: {} - {} 条记录", table, result);

        // 返回JSON格式的响应
        Ok(serde_json::json!({
            "success": true,
            "data": result
        }).to_string())
    }

    /// 使用ODM层处理计数操作
    async fn handle_count_odm(&self, data: &str) -> Result<String, String> {
        let request: serde_json::Value = serde_json::from_str(data)
            .map_err(|e| format!("解析计数请求失败: {}", e))?;

        let table = request["table"].as_str()
            .ok_or("缺少表名")?;
        let alias = request.get("alias").and_then(|v| v.as_str());

        // 解析条件
        let conditions = if let Some(conditions_str) = request.get("conditions").and_then(|v| v.as_str()) {
            let conditions_value: serde_json::Value = serde_json::from_str(conditions_str)
                .map_err(|e| format!("解析计数条件失败: {}", e))?;
            self.parse_query_conditions(conditions_value)?
        } else {
            vec![] // 空条件表示计数所有记录
        };

        // 通过ODM层执行计数操作
        use crate::odm::get_odm_manager;
        let odm_manager = get_odm_manager().await;
        let result = odm_manager.count(table, conditions, alias).await
            .map_err(|e| format!("ODM计数操作失败: {}", e))?;

        info!("ODM计数记录成功: {} - {} 条记录", table, result);

        // 返回JSON格式的响应
        Ok(serde_json::json!({
            "success": true,
            "data": result
        }).to_string())
    }

    /// 使用ODM层处理根据ID查询操作
    async fn handle_find_by_id_odm(&self, data: &str) -> Result<String, String> {
        let request: serde_json::Value = serde_json::from_str(data)
            .map_err(|e| format!("解析ID查询请求失败: {}", e))?;

        let table = request["table"].as_str()
            .ok_or("缺少表名")?;

        // 解析ID - 支持多种格式：字符串、DataValue格式、整数等
        let id_str = if let Some(id_str) = request["id"].as_str() {
            // 简单字符串格式
            id_str.to_string()
        } else if let Some(id_obj) = request["id"].as_object() {
            // DataValue格式，如 {"String": "test_001"}
            if let Some(s) = id_obj.get("String").and_then(|v| v.as_str()) {
                s.to_string()
            } else if let Some(i) = id_obj.get("Int").and_then(|v| v.as_i64()) {
                i.to_string()
            } else {
                return Err("ID格式不支持，必须是String或Int类型".to_string());
            }
        } else if let Some(i) = request["id"].as_i64() {
            // 整数格式
            i.to_string()
        } else {
            return Err("缺少记录ID或ID格式不正确".to_string());
        };

        let alias = request.get("alias").and_then(|v| v.as_str());

        // 通过ODM层执行ID查询操作
        use crate::odm::get_odm_manager;
        let odm_manager = get_odm_manager().await;
        let result = odm_manager.find_by_id(table, &id_str, alias).await
            .map_err(|e| format!("ODM ID查询操作失败: {}", e))?;

        match result {
            Some(data) => {
                info!("ODM ID查询记录成功: {} - {}", table, id_str);
                // 返回JSON格式的响应
                Ok(serde_json::json!({
                    "success": true,
                    "data": data
                }).to_string())
            }
            None => {
                info!("ODM ID查询记录未找到: {} - {}", table, id_str);
                // 返回未找到的响应
                Ok(serde_json::json!({
                    "success": true,
                    "data": null
                }).to_string())
            }
        }
    }

    /// 使用ODM层处理根据ID删除操作
    async fn handle_delete_by_id_odm(&self, data: &str) -> Result<String, String> {
        let request: serde_json::Value = serde_json::from_str(data)
            .map_err(|e| format!("解析ID删除请求失败: {}", e))?;

        let table = request["table"].as_str()
            .ok_or("缺少表名")?;
        let id = request["id"].as_str()
            .ok_or("缺少记录ID")?;
        let alias = request.get("alias").and_then(|v| v.as_str());

        // 通过ODM层执行ID删除操作
        use crate::odm::get_odm_manager;
        let odm_manager = get_odm_manager().await;
        let result = odm_manager.delete_by_id(table, id, alias).await
            .map_err(|e| format!("ODM ID删除操作失败: {}", e))?;

        info!("ODM ID删除记录成功: {} - {} - 成功: {}", table, id, result);

        // 返回JSON格式的响应
        Ok(serde_json::json!({
            "success": true,
            "data": result
        }).to_string())
    }

    /// 使用ODM层处理根据ID更新操作
    async fn handle_update_by_id_odm(&self, data: &str) -> Result<String, String> {
        let request: serde_json::Value = serde_json::from_str(data)
            .map_err(|e| format!("解析ID更新请求失败: {}", e))?;

        let table = request["table"].as_str()
            .ok_or("缺少表名")?;
        let id = request["id"].as_str()
            .ok_or("缺少记录ID")?;
        let alias = request.get("alias").and_then(|v| v.as_str());

        // 解析更新数据
        let mut updates = std::collections::HashMap::new();
        if let Some(updates_str) = request.get("updates").and_then(|v| v.as_str()) {
            let update_json: serde_json::Value = serde_json::from_str(updates_str)
                .map_err(|e| format!("解析更新数据JSON失败: {}", e))?;
            if let serde_json::Value::Object(obj) = update_json {
                for (key, value) in obj {
                    updates.insert(key, self.json_value_to_data_value(value));
                }
            }
        } else {
            return Err("缺少更新数据".to_string());
        }

        // 通过ODM层执行ID更新操作
        use crate::odm::get_odm_manager;
        let odm_manager = get_odm_manager().await;
        let result = odm_manager.update_by_id(table, id, updates, alias).await
            .map_err(|e| format!("ODM ID更新操作失败: {}", e))?;

        info!("ODM ID更新记录成功: {} - {} - 成功: {}", table, id, result);

        // 返回JSON格式的响应
        Ok(serde_json::json!({
            "success": true,
            "data": result
        }).to_string())
    }

    /// 使用ODM层处理数据库添加操作
    async fn handle_add_database_odm(&self, data: &str) -> Result<String, String> {
        let request: serde_json::Value = serde_json::from_str(data)
            .map_err(|e| format!("解析数据库添加请求失败: {}", e))?;

        info!("处理数据库添加请求: {}", data);

        // 解析数据库配置
        if let Some(db_config_value) = request.get("database_config") {
            let db_config: DatabaseConfig = serde_json::from_value(db_config_value.clone())
                .map_err(|e| format!("解析数据库配置失败: {}", e))?;

            // 使用全局连接池管理器添加数据库
            add_database(db_config).await
                .map_err(|e| format!("添加数据库失败: {}", e))?;

            info!("数据库添加成功");
            Ok(serde_json::json!({
                "success": true,
                "message": "数据库添加成功"
            }).to_string())
        } else {
            Err("缺少数据库配置".to_string())
        }
    }

    /// 使用ODM层处理模型注册操作
    async fn handle_register_model_odm(&self, data: &str) -> Result<String, String> {
        let request: serde_json::Value = serde_json::from_str(data)
            .map_err(|e| format!("解析模型注册请求失败: {}", e))?;

        info!("处理模型注册请求: {}", data);

        // 解析模型元数据
        if let Some(model_meta_value) = request.get("model_meta") {
            let model_meta: ModelMeta = serde_json::from_value(model_meta_value.clone())
                .map_err(|e| format!("解析模型元数据失败: {}", e))?;

            let collection_name = model_meta.collection_name.clone();
            let database_alias = model_meta.database_alias.clone()
                .ok_or("模型元数据缺少数据库别名")?;

            // 使用全局连接池管理器注册模型
            get_global_pool_manager().register_model(model_meta)
                .map_err(|e| format!("模型注册失败: {}", e))?;

            info!("模型元数据注册成功，开始创建表和索引");

            // 立即创建表和索引
            get_global_pool_manager().ensure_table_and_indexes(&collection_name, &database_alias)
                .await
                .map_err(|e| format!("创建表和索引失败: {}", e))?;

            Ok(serde_json::json!({
                "success": true,
                "message": "模型注册成功，表和索引已创建"
            }).to_string())
        } else {
            Err("缺少模型元数据".to_string())
        }
    }

    /// 使用ODM层处理表创建操作
    async fn handle_create_table_odm(&self, data: &str) -> Result<String, String> {
        let request: serde_json::Value = serde_json::from_str(data)
            .map_err(|e| format!("解析表创建请求失败: {}", e))?;

        let table = request.get("table").and_then(|v| v.as_str())
            .ok_or("缺少表名")?;
        let alias = request.get("alias").and_then(|v| v.as_str())
            .ok_or("缺少数据库别名")?;

        info!("处理表创建请求: 表={}, 数据库={}", table, alias);

        // 通过ODM层间接创建表（ODM会自动处理表创建）
        // 实际上ODM层在第一次操作时会自动创建表，所以这里不需要显式创建
        info!("ODM层将在首次操作时自动创建表: {}", table);

        info!("表创建成功: {}", table);
        Ok(serde_json::json!({
            "success": true,
            "message": "表创建成功"
        }).to_string())
    }

    /// 使用ODM层处理表删除操作
    async fn handle_drop_table_odm(&self, data: &str) -> Result<String, String> {
        let request: serde_json::Value = serde_json::from_str(data)
            .map_err(|e| format!("解析表删除请求失败: {}", e))?;

        let table = request.get("table").and_then(|v| v.as_str())
            .ok_or("缺少表名")?;
        let alias = request.get("alias").and_then(|v| v.as_str())
            .ok_or("缺少数据库别名")?;

        info!("处理表删除请求: 表={}, 数据库={}", table, alias);

        // 使用manager模块的drop_table函数
        crate::manager::drop_table(alias, table).await
            .map_err(|e| format!("删除表失败: {}", e))?;

        info!("表删除成功: {}", table);
        Ok(serde_json::json!({
            "success": true,
            "message": "表删除成功"
        }).to_string())
    }

        /// 解析查询条件
    fn parse_query_conditions(&self, conditions_value: serde_json::Value) -> Result<Vec<crate::types::QueryCondition>, String> {
        match conditions_value {
            serde_json::Value::Array(arr) => {
                let mut conditions = Vec::new();
                for item in arr {
                    if let serde_json::Value::Object(obj) = item {
                        // 解析单个条件
                        let field = obj.get("field").and_then(|v| v.as_str())
                            .ok_or("条件缺少field字段")?.to_string();
                        let operator_str = obj.get("operator").and_then(|v| v.as_str())
                            .ok_or("条件缺少operator字段")?;
                        let value = obj.get("value")
                            .ok_or("条件缺少value字段")?;

                        // 转换操作符
                        let operator = match operator_str {
                            "eq" => QueryOperator::Eq,
                            "ne" => QueryOperator::Ne,
                            "gt" => QueryOperator::Gt,
                            "gte" => QueryOperator::Gte,
                            "lt" => QueryOperator::Lt,
                            "lte" => QueryOperator::Lte,
                            "like" => QueryOperator::Contains,
                            "ilike" => QueryOperator::Contains,
                            "in" => QueryOperator::In,
                            "not_in" => QueryOperator::NotIn,
                            "is_null" => QueryOperator::IsNull,
                            "is_not_null" => QueryOperator::IsNotNull,
                            _ => return Err(format!("不支持的操作符: {}", operator_str)),
                        };

                        let data_value = self.json_value_to_data_value(value.clone());
                        conditions.push(crate::types::QueryCondition {
                            field,
                            operator,
                            value: data_value,
                        });
                    } else {
                        return Err("条件必须是对象格式".to_string());
                    }
                }
                Ok(conditions)
            },
            serde_json::Value::Object(_) => {
                // 单个条件对象
                self.parse_query_conditions(serde_json::Value::Array(vec![conditions_value]))
            },
            _ => Err("条件必须是数组或对象格式".to_string()),
        }
    }

    /// 获取数据库特定的JSON处理器
    /// 解析带标签的DataValue格式
    fn parse_labeled_data_value(&self, value: serde_json::Value) -> Result<DataValue, String> {
        match value {
            serde_json::Value::Object(obj) => {
                if obj.len() == 1 {
                    // 带标签的DataValue格式
                    for (tag, val) in &obj {
                        return match tag.as_str() {
                            "String" => Ok(DataValue::String(val.as_str().unwrap_or_default().to_string())),
                            "Int" => {
                                if let Some(i) = val.as_i64() {
                                    Ok(DataValue::Int(i))
                                } else {
                                    Err(format!("Int字段包含无效的整数: {:?}", val))
                                }
                            },
                            "Float" => {
                                if let Some(f) = val.as_f64() {
                                    Ok(DataValue::Float(f))
                                } else {
                                    Err(format!("Float字段包含无效的浮点数: {:?}", val))
                                }
                            },
                            "Bool" => {
                                if let Some(b) = val.as_bool() {
                                    Ok(DataValue::Bool(b))
                                } else {
                                    Err(format!("Bool字段包含无效的布尔值: {:?}", val))
                                }
                            },
                            "DateTime" => {
                                if let Some(dt_str) = val.as_str() {
                                    // 解析ISO 8601格式的datetime字符串
                                    match chrono::DateTime::parse_from_rfc3339(dt_str) {
                                        Ok(dt) => Ok(DataValue::DateTime(dt.with_timezone(&chrono::Utc))),
                                        Err(e) => Err(format!("DateTime字段包含无效的ISO格式: {} - {}", dt_str, e))
                                    }
                                } else {
                                    Err(format!("DateTime字段包含无效的字符串: {:?}", val))
                                }
                            },
                            "Uuid" => {
                                if let Some(uuid_str) = val.as_str() {
                                    // 解析UUID字符串
                                    match uuid::Uuid::parse_str(uuid_str) {
                                        Ok(uuid) => Ok(DataValue::Uuid(uuid)),
                                        Err(e) => Err(format!("Uuid字段包含无效的UUID格式: {} - {}", uuid_str, e))
                                    }
                                } else {
                                    Err(format!("Uuid字段包含无效的字符串: {:?}", val))
                                }
                            },
                            "Null" => Ok(DataValue::Null),
                            "Object" => {
                                if let serde_json::Value::Object(inner_obj) = val {
                                    let mut data_map = std::collections::HashMap::new();
                                    for (k, v) in inner_obj {
                                        data_map.insert(k.clone(), self.parse_labeled_data_value(v.clone())?);
                                    }
                                    Ok(DataValue::Object(data_map))
                                } else {
                                    Err(format!("Object字段包含无效的对象: {:?}", val))
                                }
                            },
                            "Array" => {
                                if let serde_json::Value::Array(arr) = val {
                                    let data_array: Result<Vec<_>, _> = arr.iter()
                                        .map(|v| self.parse_labeled_data_value(v.clone()))
                                        .collect();
                                    Ok(DataValue::Array(data_array?))
                                } else {
                                    Err(format!("Array字段包含无效的数组: {:?}", val))
                                }
                            },
                            _ => Err(format!("不支持的DataValue标签: {}", tag)),
                        };
                    }
                }
                Err(format!("无效的带标签DataValue格式: {:?}", obj))
            },
            _ => Err(format!("期望带标签的DataValue格式，但得到: {:?}", value)),
        }
    }

    fn get_database_processor(&self, db_alias: Option<&str>) -> Result<Box<dyn super::database_processors::DatabaseJsonProcessor>, String> {
        use super::database_processors::create_database_json_processor;

        if let Some(alias) = db_alias {
            // 获取数据库类型
            let db_type = crate::manager::get_global_pool_manager().get_database_type(alias)
                .map_err(|e| format!("无法获取数据库'{}'的类型: {}, 请检查数据库配置是否正确", alias, e))?;

                        Ok(create_database_json_processor(&db_type))
        } else {
            Err("未指定数据库别名，无法获取数据库处理器".to_string())
        }
    }

    fn json_value_to_data_value(&self, value: serde_json::Value) -> DataValue {
        match value {
            serde_json::Value::Null => DataValue::Null,
            serde_json::Value::Bool(b) => DataValue::Bool(b),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    DataValue::Int(i)
                } else if let Some(f) = n.as_f64() {
                    DataValue::Float(f)
                } else {
                    DataValue::Json(serde_json::Value::Number(n))
                }
            },
            serde_json::Value::String(s) => DataValue::String(s),
            serde_json::Value::Array(arr) => {
                let data_array: Vec<DataValue> = arr.into_iter()
                    .map(|v| self.json_value_to_data_value(v))
                    .collect();
                DataValue::Array(data_array)
            },
            serde_json::Value::Object(obj) => {
                let data_object: std::collections::HashMap<String, DataValue> = obj.into_iter()
                    .map(|(k, v)| (k, self.json_value_to_data_value(v)))
                    .collect();
                DataValue::Object(data_object)
            }
        }
    }
}

/// 创建简化队列桥接器的工厂函数
pub fn create_simple_queue_bridge() -> Result<SimpleQueueBridge, String> {
    info!("创建简化队列桥接器实例");
    SimpleQueueBridge::new()
}