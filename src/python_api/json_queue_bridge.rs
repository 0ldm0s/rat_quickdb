//! JSON队列桥接器
//!
//! 使用JSON字符串与Python进行通信，通过全局任务队列系统执行数据库操作

use pyo3::prelude::*;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::oneshot;

use crate::error::{QuickDbResult, QuickDbError};
use crate::task_queue::{get_global_task_queue, DbTask};
use crate::types::{DataValue, QueryCondition, QueryOptions};

/// JSON队列桥接器 - 使用JSON字符串与Python通信
#[pyclass(name = "JsonQueueBridge")]
pub struct PyJsonQueueBridge {
    // 桥接器本身不需要状态，所有操作都通过全局任务队列
}

#[pymethods]
impl PyJsonQueueBridge {
    #[new]
    pub fn new() -> Self {
        Self {}
    }

    /// 创建记录
    pub fn create(&self, table: String, data_json: String) -> PyResult<String> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("无法创建运行时: {}", e))
        })?;

        rt.block_on(async {
            // 解析JSON数据
            let data: HashMap<String, Value> = serde_json::from_str(&data_json)
                .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("无效的JSON数据: {}", e)))?;

            // 转换为DataValue
            let data_values: HashMap<String, DataValue> = data
                .into_iter()
                .map(|(k, v)| (k, json_value_to_data_value(v)))
                .collect();

            // 通过全局任务队列执行
            let task_queue = get_global_task_queue();
            let result = task_queue.create(table, data_values, None).await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("数据库操作失败: {}", e)))?;

            Ok(result)
        })
    }

    /// 查询记录
    pub fn find(&self, table: String, conditions_json: String, options_json: Option<String>) -> PyResult<String> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("无法创建运行时: {}", e))
        })?;

        rt.block_on(async {
            // 解析查询条件
            let conditions: Vec<Value> = serde_json::from_str(&conditions_json)
                .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("无效的查询条件JSON: {}", e)))?;

            let query_conditions: Vec<QueryCondition> = conditions
                .into_iter()
                .map(|v| parse_query_condition(v))
                .collect::<Result<Vec<_>, _>>()?;

            // 解析查询选项
            let options = if let Some(options_json) = options_json {
                let opts: Value = serde_json::from_str(&options_json)
                    .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("无效的查询选项JSON: {}", e)))?;
                Some(parse_query_options(opts)?)
            } else {
                None
            };

            // 通过全局任务队列执行
            let task_queue = get_global_task_queue();
            let result = task_queue.find(table, query_conditions, options).await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("数据库查询失败: {}", e)))?;

            Ok(result)
        })
    }

    /// 根据ID查询记录
    pub fn find_by_id(&self, table: String, id: String, options_json: Option<String>) -> PyResult<Option<String>> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("无法创建运行时: {}", e))
        })?;

        rt.block_on(async {
            // 解析查询选项
            let options = if let Some(options_json) = options_json {
                let opts: Value = serde_json::from_str(&options_json)
                    .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("无效的查询选项JSON: {}", e)))?;
                Some(parse_query_options(opts)?)
            } else {
                None
            };

            // 通过全局任务队列执行
            let task_queue = get_global_task_queue();
            let result = task_queue.find_by_id(table, id, options).await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("数据库查询失败: {}", e)))?;

            Ok(result)
        })
    }

    /// 更新记录
    pub fn update(&self, table: String, conditions_json: String, data_json: String, options_json: Option<String>) -> PyResult<u64> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("无法创建运行时: {}", e))
        })?;

        rt.block_on(async {
            // 解析查询条件
            let conditions: Vec<Value> = serde_json::from_str(&conditions_json)
                .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("无效的查询条件JSON: {}", e)))?;

            let query_conditions: Vec<QueryCondition> = conditions
                .into_iter()
                .map(|v| parse_query_condition(v))
                .collect::<Result<Vec<_>, _>>()?;

            // 解析更新数据
            let data: HashMap<String, Value> = serde_json::from_str(&data_json)
                .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("无效的更新数据JSON: {}", e)))?;

            let data_values: HashMap<String, DataValue> = data
                .into_iter()
                .map(|(k, v)| (k, json_value_to_data_value(v)))
                .collect();

            // 通过全局任务队列执行
            let task_queue = get_global_task_queue();
            let result = task_queue.update(table, query_conditions, data_values, None).await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("数据库更新失败: {}", e)))?;

            Ok(result)
        })
    }

    /// 根据ID更新记录
    pub fn update_by_id(&self, table: String, id: String, data_json: String, options_json: Option<String>) -> PyResult<bool> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("无法创建运行时: {}", e))
        })?;

        rt.block_on(async {
            // 解析更新数据
            let data: HashMap<String, Value> = serde_json::from_str(&data_json)
                .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("无效的更新数据JSON: {}", e)))?;

            let data_values: HashMap<String, DataValue> = data
                .into_iter()
                .map(|(k, v)| (k, json_value_to_data_value(v)))
                .collect();

            // 通过全局任务队列执行
            let task_queue = get_global_task_queue();
            let result = task_queue.update_by_id(table, id, data_values, None).await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("数据库更新失败: {}", e)))?;

            Ok(result)
        })
    }

    /// 删除记录
    pub fn delete(&self, table: String, conditions_json: String, options_json: Option<String>) -> PyResult<u64> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("无法创建运行时: {}", e))
        })?;

        rt.block_on(async {
            // 解析查询条件
            let conditions: Vec<Value> = serde_json::from_str(&conditions_json)
                .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("无效的查询条件JSON: {}", e)))?;

            let query_conditions: Vec<QueryCondition> = conditions
                .into_iter()
                .map(|v| parse_query_condition(v))
                .collect::<Result<Vec<_>, _>>()?;

            // 通过全局任务队列执行
            let task_queue = get_global_task_queue();
            let result = task_queue.delete(table, query_conditions, None).await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("数据库删除失败: {}", e)))?;

            Ok(result)
        })
    }

    /// 根据ID删除记录
    pub fn delete_by_id(&self, table: String, id: String, options_json: Option<String>) -> PyResult<bool> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("无法创建运行时: {}", e))
        })?;

        rt.block_on(async {
            // 通过全局任务队列执行
            let task_queue = get_global_task_queue();
            let result = task_queue.delete_by_id(table, id, None).await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("数据库删除失败: {}", e)))?;

            Ok(result)
        })
    }

    /// 计数记录
    pub fn count(&self, table: String, conditions_json: String, options_json: Option<String>) -> PyResult<u64> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("无法创建运行时: {}", e))
        })?;

        rt.block_on(async {
            // 解析查询条件
            let conditions: Vec<Value> = serde_json::from_str(&conditions_json)
                .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("无效的查询条件JSON: {}", e)))?;

            let query_conditions: Vec<QueryCondition> = conditions
                .into_iter()
                .map(|v| parse_query_condition(v))
                .collect::<Result<Vec<_>, _>>()?;

            // 通过全局任务队列执行
            let task_queue = get_global_task_queue();
            let result = task_queue.count(table, query_conditions, None).await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("数据库计数失败: {}", e)))?;

            Ok(result)
        })
    }

    /// 检查记录是否存在
    pub fn exists(&self, table: String, conditions_json: String, options_json: Option<String>) -> PyResult<bool> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("无法创建运行时: {}", e))
        })?;

        rt.block_on(async {
            // 解析查询条件
            let conditions: Vec<Value> = serde_json::from_str(&conditions_json)
                .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("无效的查询条件JSON: {}", e)))?;

            let query_conditions: Vec<QueryCondition> = conditions
                .into_iter()
                .map(|v| parse_query_condition(v))
                .collect::<Result<Vec<_>, _>>()?;

            // 通过全局任务队列执行
            let task_queue = get_global_task_queue();
            let result = task_queue.exists(table, query_conditions, None).await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("数据库存在性检查失败: {}", e)))?;

            Ok(result)
        })
    }

    /// 检查表是否存在
    pub fn check_table(&self, table: String) -> PyResult<bool> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("无法创建运行时: {}", e))
        })?;

        rt.block_on(async {
            // 通过全局任务队列执行
            let task_queue = get_global_task_queue();
            let result = task_queue.check_table(table).await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("表检查失败: {}", e)))?;

            Ok(result)
        })
    }
}

/// 创建JSON队列桥接器
#[pyfunction]
pub fn create_json_queue_bridge() -> PyResult<PyJsonQueueBridge> {
    Ok(PyJsonQueueBridge::new())
}

// === 辅助函数 ===

/// 将JSON值转换为DataValue
pub(crate) fn json_value_to_data_value(value: Value) -> DataValue {
    match value {
        Value::Null => DataValue::Null,
        Value::Bool(b) => DataValue::Bool(b),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                DataValue::Int(i)
            } else if let Some(f) = n.as_f64() {
                DataValue::Float(f)
            } else {
                DataValue::Json(Value::Number(n))
            }
        },
        Value::String(s) => DataValue::String(s),
        Value::Array(arr) => {
            let data_array: Vec<DataValue> = arr.into_iter()
                .map(json_value_to_data_value)
                .collect();
            DataValue::Array(data_array)
        },
        Value::Object(obj) => {
            let data_object: HashMap<String, DataValue> = obj.into_iter()
                .map(|(k, v)| (k, json_value_to_data_value(v)))
                .collect();
            DataValue::Object(data_object)
        }
    }
}

/// 解析查询条件
pub fn parse_query_condition(value: Value) -> PyResult<QueryCondition> {
    let obj = value.as_object().ok_or_else(||
        pyo3::exceptions::PyValueError::new_err("查询条件必须是JSON对象")
    )?;

    let field = obj.get("field")
        .and_then(|v| v.as_str())
        .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("缺少字段名"))?
        .to_string();

    let operator_str = obj.get("operator")
        .and_then(|v| v.as_str())
        .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("缺少操作符"))?;

    let operator = match operator_str {
        "eq" => crate::types::QueryOperator::Eq,
        "ne" => crate::types::QueryOperator::Ne,
        "gt" => crate::types::QueryOperator::Gt,
        "gte" => crate::types::QueryOperator::Gte,
        "lt" => crate::types::QueryOperator::Lt,
        "lte" => crate::types::QueryOperator::Lte,
        "contains" => crate::types::QueryOperator::Contains,
        "startsWith" => crate::types::QueryOperator::StartsWith,
        "endsWith" => crate::types::QueryOperator::EndsWith,
        "in" => crate::types::QueryOperator::In,
        "notIn" => crate::types::QueryOperator::NotIn,
        "regex" => crate::types::QueryOperator::Regex,
        "exists" => crate::types::QueryOperator::Exists,
        "isNull" => crate::types::QueryOperator::IsNull,
        "isNotNull" => crate::types::QueryOperator::IsNotNull,
        _ => return Err(pyo3::exceptions::PyValueError::new_err(format!("不支持的操作符: {}", operator_str))),
    };

    let value = obj.get("value")
        .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("缺少值"))
        .map(|v| json_value_to_data_value(v.clone()))?;

    Ok(QueryCondition {
        field,
        operator,
        value,
    })
}

/// 解析查询选项
fn parse_query_options(value: Value) -> PyResult<QueryOptions> {
    let mut options = QueryOptions::new();

    if let Some(obj) = value.as_object() {
        // 解析排序配置
        if let Some(sort_array) = obj.get("sort") {
            if let Value::Array(arr) = sort_array {
                let sort_configs: Vec<crate::types::SortConfig> = arr.iter()
                    .map(|v| {
                        let sort_obj = v.as_object().ok_or_else(||
                            pyo3::exceptions::PyValueError::new_err("排序配置必须是JSON对象")
                        )?;

                        let field = sort_obj.get("field")
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("缺少排序字段名"))?
                            .to_string();

                        let direction_str = sort_obj.get("direction")
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("缺少排序方向"))?;

                        let direction = match direction_str {
                            "asc" => crate::types::SortDirection::Asc,
                            "desc" => crate::types::SortDirection::Desc,
                            _ => return Err(pyo3::exceptions::PyValueError::new_err(format!("不支持的排序方向: {}", direction_str))),
                        };

                        Ok(crate::types::SortConfig {
                            field,
                            direction,
                        })
                    })
                    .collect::<PyResult<Vec<_>>>()?;

                options = options.with_sort(sort_configs);
            }
        }

        // 解析分页配置
        if let Some(pagination_obj) = obj.get("pagination") {
            if let Value::Object(pag_obj) = pagination_obj {
                let skip = pag_obj.get("skip")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);

                let limit = pag_obj.get("limit")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(100);

                let pagination = crate::types::PaginationConfig {
                    skip,
                    limit,
                };

                options = options.with_pagination(pagination);
            }
        }

        // 解析字段选择
        if let Some(fields_array) = obj.get("fields") {
            if let Value::Array(arr) = fields_array {
                let fields: Vec<String> = arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();

                options = options.with_fields(fields);
            }
        }
    }

    Ok(options)
}