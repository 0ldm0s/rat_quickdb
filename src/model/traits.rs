//! Model trait 定义模块
//!
//! 定义模型的核心接口和操作特征

use crate::error::{QuickDbError, QuickDbResult};
use crate::model::conversion::ToDataValue;
use crate::model::field_types::{FieldDefinition, FieldType, ModelMeta};
use crate::types::*;
use async_trait::async_trait;
use base64;
use rat_logger::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::marker::PhantomData;

/// 模型特征
///
/// 所有模型都必须实现这个特征
pub trait Model: Serialize + for<'de> Deserialize<'de> + Send + Sync {
    /// 获取模型元数据
    fn meta() -> ModelMeta;

    /// 获取集合/表名
    fn collection_name() -> String {
        Self::meta().collection_name
    }

    /// 获取数据库别名
    fn database_alias() -> Option<String> {
        Self::meta().database_alias
    }

    /// 验证模型数据
    fn validate(&self) -> QuickDbResult<()> {
        let meta = Self::meta();
        let data = self.to_data_map()?;

        // 调试信息：打印序列化后的数据
        debug!("🔍 验证数据映射: {:?}", data);

        for (field_name, field_def) in &meta.fields {
            let field_value = data.get(field_name).unwrap_or(&DataValue::Null);
            debug!("🔍 验证字段 {}: {:?}", field_name, field_value);
            field_def.validate_with_field_name(field_value, field_name)?;
        }

        Ok(())
    }

    /// 转换为数据映射（直接转换，避免 JSON 序列化开销）
    /// 子类应该重写此方法以提供高性能的直接转换
    fn to_data_map_direct(&self) -> QuickDbResult<HashMap<String, DataValue>> {
        // 默认回退到 JSON 序列化方式，但建议子类重写
        warn!("使用默认的 JSON 序列化方式，建议重写 to_data_map_direct 方法以提高性能");
        self.to_data_map_legacy()
    }

    /// 转换为数据映射（传统 JSON 序列化方式）
    /// 保留此方法用于向后兼容和调试
    fn to_data_map_legacy(&self) -> QuickDbResult<HashMap<String, DataValue>> {
        let json_str =
            serde_json::to_string(self).map_err(|e| QuickDbError::SerializationError {
                message: format!("序列化失败: {}", e),
            })?;
        debug!("🔍 序列化后的JSON字符串: {}", json_str);

        let json_value: JsonValue =
            serde_json::from_str(&json_str).map_err(|e| QuickDbError::SerializationError {
                message: format!("解析JSON失败: {}", e),
            })?;
        debug!("🔍 解析后的JsonValue: {:?}", json_value);

        let mut data_map = HashMap::new();
        if let JsonValue::Object(obj) = json_value {
            for (key, value) in obj {
                let data_value = DataValue::from_json(value.clone());
                debug!("🔍 字段 {} 转换: {:?} -> {:?}", key, value, data_value);
                data_map.insert(key, data_value);
            }
        }

        Ok(data_map)
    }

    /// 将模型转换为数据映射（高性能版本）
    fn to_data_map(&self) -> QuickDbResult<HashMap<String, DataValue>> {
        self.to_data_map_direct()
    }

    /// 将模型转换为带类型信息的数据映射（专门用于 PyO3 兼容序列化）
    /// 对于 None 值，会根据字段类型生成带类型标签的 DataValue
    fn to_data_map_with_types(&self) -> QuickDbResult<HashMap<String, DataValue>> {
        let json_map = self.to_data_map_with_types_json()?;
        // 将 HashMap<String, JsonValue> 转换为 HashMap<String, DataValue>
        let mut data_map = HashMap::new();
        for (key, json_value) in json_map {
            data_map.insert(key, DataValue::Json(json_value));
        }
        Ok(data_map)
    }

    /// 将模型转换为带类型信息的 JSON 映射（专门用于 PyO3 兼容序列化）
    /// 对于 None 值，会根据字段类型生成带类型标签的 JsonValue
    /// 这个方法直接返回 JsonValue，避免 DataValue 的额外嵌套
    fn to_data_map_with_types_json(&self) -> QuickDbResult<HashMap<String, JsonValue>> {
        let meta = Self::meta();
        let mut data_map = HashMap::new();

        // 遍历模型的所有字段
        let json_str =
            serde_json::to_string(self).map_err(|e| QuickDbError::SerializationError {
                message: format!("序列化失败: {}", e),
            })?;

        debug!("🔍 to_data_map_with_types_json 序列化的JSON: {}", json_str);

        let json_value: JsonValue =
            serde_json::from_str(&json_str).map_err(|e| QuickDbError::SerializationError {
                message: format!("解析JSON失败: {}", e),
            })?;

        debug!(
            "🔍 to_data_map_with_types_json 解析后的JSON: {:?}",
            json_value
        );

        if let JsonValue::Object(obj) = json_value {
            for (key, value) in obj {
                // 检查字段是否在元数据中定义
                if let Some(field_def) = meta.fields.get(&key) {
                    // 对所有字段都生成带类型标签的 JsonValue
                    let type_name = match &field_def.field_type {
                        FieldType::String { .. } => "String",
                        FieldType::Integer { .. } => "Int",
                        FieldType::Float { .. } => "Float",
                        FieldType::BigInteger => "Int",
                        FieldType::Double => "Float",
                        FieldType::Text => "String",
                        FieldType::Boolean => "Bool",
                        FieldType::DateTime => "DateTime",
                        FieldType::DateTimeWithTz { .. } => "DateTime", // 带时区的DateTime
                        FieldType::Date => "DateTime",
                        FieldType::Time => "DateTime",
                        FieldType::Uuid => "Uuid",
                        FieldType::Json => "Json",
                        FieldType::Binary => "Bytes",
                        FieldType::Decimal { .. } => "Float",
                        FieldType::Array { .. } => "Array",
                        FieldType::Object { .. } => "Object",
                        FieldType::Reference { .. } => "String",
                    };

                    // 直接创建带类型标签的 JsonValue，避免嵌套
                    let typed_json = match value {
                        JsonValue::Null => {
                            // 对于 null 值，创建 {类型名: null}
                            let mut type_obj = serde_json::Map::new();
                            type_obj.insert(type_name.to_string(), JsonValue::Null);
                            JsonValue::Object(type_obj)
                        }
                        JsonValue::String(s) => {
                            // 对于字符串值，创建 {类型名: "value"}
                            let mut type_obj = serde_json::Map::new();
                            type_obj.insert(type_name.to_string(), JsonValue::String(s));
                            JsonValue::Object(type_obj)
                        }
                        JsonValue::Number(n) => {
                            // 对于数字值，根据类型包装
                            let mut type_obj = serde_json::Map::new();
                            type_obj.insert(type_name.to_string(), JsonValue::Number(n));
                            JsonValue::Object(type_obj)
                        }
                        JsonValue::Bool(b) => {
                            // 对于布尔值，创建 {类型名: boolean}
                            let mut type_obj = serde_json::Map::new();
                            type_obj.insert(type_name.to_string(), JsonValue::Bool(b));
                            JsonValue::Object(type_obj)
                        }
                        JsonValue::Array(arr) => {
                            // 对于数组，需要根据字段类型为每个元素添加类型标记
                            if let FieldType::Array { item_type, .. } = &field_def.field_type {
                                let item_type_name = match &**item_type {
                                    FieldType::String { .. } => "String",
                                    FieldType::Integer { .. } => "Int",
                                    FieldType::Float { .. } => "Float",
                                    FieldType::BigInteger => "Int",
                                    FieldType::Double => "Float",
                                    FieldType::Text => "String",
                                    FieldType::Boolean => "Bool",
                                    FieldType::DateTime => "DateTime",
                                    FieldType::DateTimeWithTz { .. } => "DateTime", // 带时区的DateTime
                                    FieldType::Date => "DateTime",
                                    FieldType::Time => "DateTime",
                                    FieldType::Uuid => "Uuid",
                                    FieldType::Json => "Json",
                                    FieldType::Binary => "Bytes",
                                    FieldType::Decimal { .. } => "Float",
                                    FieldType::Array { .. } => "Array",
                                    FieldType::Object { .. } => "Object",
                                    FieldType::Reference { .. } => "String",
                                };

                                let processed_array: Vec<JsonValue> = arr
                                    .into_iter()
                                    .map(|v| {
                                        // 为每个数组元素添加类型标记
                                        let mut item_type_obj = serde_json::Map::new();
                                        match v {
                                            JsonValue::String(s) => {
                                                item_type_obj.insert(
                                                    item_type_name.to_string(),
                                                    JsonValue::String(s),
                                                );
                                            }
                                            JsonValue::Number(n) => {
                                                item_type_obj.insert(
                                                    item_type_name.to_string(),
                                                    JsonValue::Number(n),
                                                );
                                            }
                                            JsonValue::Bool(b) => {
                                                item_type_obj.insert(
                                                    item_type_name.to_string(),
                                                    JsonValue::Bool(b),
                                                );
                                            }
                                            JsonValue::Null => {
                                                item_type_obj.insert(
                                                    item_type_name.to_string(),
                                                    JsonValue::Null,
                                                );
                                            }
                                            JsonValue::Array(nested_arr) => {
                                                // 嵌套数组暂时保持原样，实际使用中应该递归处理
                                                item_type_obj.insert(
                                                    item_type_name.to_string(),
                                                    JsonValue::Array(nested_arr),
                                                );
                                            }
                                            JsonValue::Object(nested_obj) => {
                                                // 嵌套对象暂时保持原样，实际使用中应该递归处理
                                                item_type_obj.insert(
                                                    item_type_name.to_string(),
                                                    JsonValue::Object(nested_obj),
                                                );
                                            }
                                        }
                                        JsonValue::Object(item_type_obj)
                                    })
                                    .collect();
                                let mut type_obj = serde_json::Map::new();
                                type_obj.insert(
                                    type_name.to_string(),
                                    JsonValue::Array(processed_array),
                                );
                                JsonValue::Object(type_obj)
                            } else {
                                // 如果不是数组类型，保持原有逻辑
                                let processed_array: Vec<JsonValue> = arr
                                    .into_iter()
                                    .map(|v| match v {
                                        JsonValue::String(s) => JsonValue::String(s),
                                        JsonValue::Number(n) => JsonValue::Number(n),
                                        JsonValue::Bool(b) => JsonValue::Bool(b),
                                        JsonValue::Null => JsonValue::Null,
                                        JsonValue::Array(_) => v,
                                        JsonValue::Object(_) => v,
                                    })
                                    .collect();
                                let mut type_obj = serde_json::Map::new();
                                type_obj.insert(
                                    type_name.to_string(),
                                    JsonValue::Array(processed_array),
                                );
                                JsonValue::Object(type_obj)
                            }
                        }
                        JsonValue::Object(obj) => {
                            // 对于对象，递归处理每个字段，然后包装类型
                            let processed_obj: serde_json::Map<String, JsonValue> = obj
                                .into_iter()
                                .map(|(k, v)| {
                                    let processed_value = match v {
                                        JsonValue::String(s) => JsonValue::String(s),
                                        JsonValue::Number(n) => JsonValue::Number(n),
                                        JsonValue::Bool(b) => JsonValue::Bool(b),
                                        JsonValue::Null => JsonValue::Null,
                                        JsonValue::Array(_) => v,
                                        JsonValue::Object(_) => v,
                                    };
                                    (k, processed_value)
                                })
                                .collect();
                            let mut type_obj = serde_json::Map::new();
                            type_obj
                                .insert(type_name.to_string(), JsonValue::Object(processed_obj));
                            JsonValue::Object(type_obj)
                        }
                    };

                    data_map.insert(key, typed_json);
                } else {
                    // 字段不在元数据中 - 这在 v0.3.0 中不应该发生，报错退出
                    return Err(QuickDbError::ValidationError {
                        field: key.clone(),
                        message: format!(
                            "字段 '{}' 未在模型元数据中定义，这在 v0.3.0 中是不允许的",
                            key
                        ),
                    });
                }
            }
        }

        Ok(data_map)
    }

    /// 从数据映射创建模型实例
    fn from_data_map(data: HashMap<String, DataValue>) -> QuickDbResult<Self> {
        // 使用模型元数据后处理数据字段，修复复杂类型字段反序列化问题
        let meta = Self::meta();
        let processed_data = crate::process_data_fields_from_metadata(data, &meta.fields);

        // 直接从HashMap<String, DataValue>转换为模型实例，避免JSON中转
        crate::model::data_conversion::create_model_from_data_map::<Self>(&processed_data)
    }
}

/// 记录操作特征
///
/// 提供记录的CRUD操作
#[async_trait]
pub trait ModelOperations<T: Model> {
    /// 保存记录
    async fn save(&self) -> QuickDbResult<String>;

    /// 根据ID查找记录
    async fn find_by_id(id: &str) -> QuickDbResult<Option<T>>;

    /// 查找多条记录
    async fn find(
        conditions: Vec<QueryCondition>,
        options: Option<QueryOptions>,
    ) -> QuickDbResult<Vec<T>> {
        Self::find_with_cache_control(conditions, options, false).await
    }

    /// 查找多条记录（支持缓存控制）
    async fn find_with_cache_control(
        conditions: Vec<QueryCondition>,
        options: Option<QueryOptions>,
        bypass_cache: bool,
    ) -> QuickDbResult<Vec<T>>;

    /// 更新记录
    async fn update(&self, updates: HashMap<String, DataValue>) -> QuickDbResult<bool>;

    /// 删除记录
    async fn delete(&self) -> QuickDbResult<bool>;

    /// 统计记录数量
    async fn count(conditions: Vec<QueryCondition>) -> QuickDbResult<u64>;

    /// 使用条件组查找多条记录（支持复杂的AND/OR逻辑组合）
    async fn find_with_groups(
        condition_groups: Vec<QueryConditionGroup>,
        options: Option<QueryOptions>,
    ) -> QuickDbResult<Vec<T>> {
        Self::find_with_groups_with_cache_control(condition_groups, options, false).await
    }

    /// 使用条件组查找多条记录（支持缓存控制和复杂的AND/OR逻辑组合）
    async fn find_with_groups_with_cache_control(
        condition_groups: Vec<QueryConditionGroup>,
        options: Option<QueryOptions>,
        bypass_cache: bool,
    ) -> QuickDbResult<Vec<T>>;

    /// 批量更新记录
    ///
    /// 根据条件批量更新多条记录，返回受影响的行数
    async fn update_many(
        conditions: Vec<QueryCondition>,
        updates: HashMap<String, DataValue>,
    ) -> QuickDbResult<u64>;

    /// 使用操作数组批量更新模型
    ///
    /// 根据条件使用操作数组批量更新多条记录，支持原子性增减操作，返回受影响的行数
    async fn update_many_with_operations(
        conditions: Vec<QueryCondition>,
        operations: Vec<crate::types::UpdateOperation>,
    ) -> QuickDbResult<u64>;

    /// 批量删除模型
    ///
    /// 根据条件批量删除多条记录，返回受影响的行数
    async fn delete_many(conditions: Vec<QueryCondition>) -> QuickDbResult<u64>;

    /// 创建表
    ///
    /// 使用模型的元数据直接创建表，无需插入数据
    async fn create_table() -> QuickDbResult<()>;

    /// 创建存储过程
    ///
    /// 通过模型管理器创建跨模型的存储过程，以当前模型作为基表
    async fn create_stored_procedure(
        config: crate::stored_procedure::StoredProcedureConfig,
    ) -> QuickDbResult<crate::stored_procedure::StoredProcedureCreateResult>;

    /// 执行存储过程查询
    ///
    /// 通过模型管理器执行存储过程查询，使用当前模型的数据库别名
    async fn execute_stored_procedure(
        procedure_name: &str,
        params: Option<std::collections::HashMap<String, crate::types::DataValue>>,
    ) -> QuickDbResult<crate::stored_procedure::StoredProcedureQueryResult>;
}
