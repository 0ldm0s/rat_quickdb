//! 字段类型定义模块
//!
//! 定义模型字段的类型、验证和元数据

use crate::error::{QuickDbError, QuickDbResult};
use crate::types::DataValue;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use rat_logger::debug;

/// 字段类型枚举
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FieldType {
    /// 字符串类型
    String {
        max_length: Option<usize>,
        min_length: Option<usize>,
        regex: Option<String>,
    },
    /// 整数类型
    Integer {
        min_value: Option<i64>,
        max_value: Option<i64>,
    },
    /// 大整数类型
    BigInteger,
    /// 浮点数类型
    Float {
        min_value: Option<f64>,
        max_value: Option<f64>,
    },
    /// 双精度浮点数类型
    Double,
    /// 文本类型
    Text,
    /// 布尔类型
    Boolean,
    /// 日期时间类型
    DateTime,
    /// 日期类型
    Date,
    /// 时间类型
    Time,
    /// UUID类型
    Uuid,
    /// JSON类型
    Json,
    /// 二进制类型
    Binary,
    /// 十进制类型
    Decimal {
        precision: u8,
        scale: u8,
    },
    /// 数组类型
    Array {
        item_type: Box<FieldType>,
        max_items: Option<usize>,
        min_items: Option<usize>,
    },
    /// 对象类型
    Object {
        fields: HashMap<String, FieldDefinition>,
    },
    /// 引用类型（外键）
    Reference {
        target_collection: String,
    },
}

/// 字段定义
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldDefinition {
    /// 字段类型
    pub field_type: FieldType,
    /// 是否必填
    pub required: bool,
    /// 默认值
    pub default: Option<DataValue>,
    /// 是否唯一
    pub unique: bool,
    /// 是否建立索引
    pub indexed: bool,
    /// 字段描述
    pub description: Option<String>,
    /// 自定义验证函数名
    pub validator: Option<String>,
    /// SQLite 布尔值兼容性
    pub sqlite_compatibility: bool,
}

impl FieldDefinition {
    /// 创建新的字段定义
    pub fn new(field_type: FieldType) -> Self {
        Self {
            field_type,
            required: false,
            default: None,
            unique: false,
            indexed: false,
            description: None,
            validator: None,
            sqlite_compatibility: false,
        }
    }

    /// 设置为必填字段
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    /// 设置默认值
    pub fn default_value(mut self, value: DataValue) -> Self {
        self.default = Some(value);
        self
    }

    /// 设置为唯一字段
    pub fn unique(mut self) -> Self {
        self.unique = true;
        self
    }

    /// 设置为索引字段
    pub fn indexed(mut self) -> Self {
        self.indexed = true;
        self
    }

    /// 设置字段描述
    pub fn description(mut self, desc: &str) -> Self {
        self.description = Some(desc.to_string());
        self
    }

    /// 设置验证函数
    pub fn validator(mut self, validator_name: &str) -> Self {
        self.validator = Some(validator_name.to_string());
        self
    }

    /// 设置 SQLite 兼容性
    pub fn with_sqlite_compatibility(mut self, compatible: bool) -> Self {
        self.sqlite_compatibility = compatible;
        self
    }

    /// 设置默认值（别名方法，提供更直观的API）
    pub fn with_default(mut self, value: DataValue) -> Self {
        self.default = Some(value);
        self
    }

    /// 验证字段值
    pub fn validate(&self, value: &DataValue) -> QuickDbResult<()> {
        self.validate_with_field_name(value, "unknown")
    }

    pub fn validate_with_field_name(&self, value: &DataValue, field_name: &str) -> QuickDbResult<()> {
        // 检查必填字段
        if self.required && matches!(value, DataValue::Null) {
            return Err(QuickDbError::ValidationError { field: field_name.to_string(), message: "必填字段不能为空".to_string() });
        }

        // 如果值为空且不是必填字段，则跳过验证
        if matches!(value, DataValue::Null) {
            return Ok(());
        }

        // 根据字段类型进行验证
        match &self.field_type {
            FieldType::String { max_length, min_length, regex } => {
                if let DataValue::String(s) = value {
                    if let Some(max_len) = max_length {
                        if s.len() > *max_len {
                            return Err(QuickDbError::ValidationError {
                                field: "string_length".to_string(),
                                message: format!("字符串长度不能超过{}", max_len)
                            });
                        }
                    }
                    if let Some(min_len) = min_length {
                        if s.len() < *min_len {
                            return Err(QuickDbError::ValidationError {
                                field: "string_length".to_string(),
                                message: format!("字符串长度不能少于{}", min_len)
                            });
                        }
                    }
                    if let Some(pattern) = regex {
                        let regex = regex::Regex::new(pattern)
                            .map_err(|e| QuickDbError::ValidationError {
                                field: "regex".to_string(),
                                message: format!("正则表达式无效: {}", e)
                            })?;
                        if !regex.is_match(s) {
                            return Err(QuickDbError::ValidationError {
                                field: "regex_match".to_string(),
                                message: "字符串不匹配正则表达式".to_string()
                            });
                        }
                    }
                } else {
                    return Err(QuickDbError::ValidationError {
                        field: "type_mismatch".to_string(),
                        message: "字段类型不匹配，期望字符串类型".to_string()
                    });
                }
            }
            FieldType::Integer { min_value, max_value } => {
                if let DataValue::Int(i) = value {
                    if let Some(min_val) = min_value {
                        if *i < *min_val {
                            return Err(QuickDbError::ValidationError {
                                field: "integer_range".to_string(),
                                message: format!("整数值不能小于{}", min_val)
                            });
                        }
                    }
                    if let Some(max_val) = max_value {
                        if *i > *max_val {
                            return Err(QuickDbError::ValidationError {
                                field: "integer_range".to_string(),
                                message: format!("整数值不能大于{}", max_val)
                            });
                        }
                    }
                } else {
                    return Err(QuickDbError::ValidationError {
                        field: "type_mismatch".to_string(),
                        message: "字段类型不匹配，期望整数类型".to_string()
                    });
                }
            }
            FieldType::Float { min_value, max_value } => {
                if let DataValue::Float(f) = value {
                    if let Some(min_val) = min_value {
                        if *f < *min_val {
                            return Err(QuickDbError::ValidationError {
                                field: "float_range".to_string(),
                                message: format!("浮点数值不能小于{}", min_val)
                            });
                        }
                    }
                    if let Some(max_val) = max_value {
                        if *f > *max_val {
                            return Err(QuickDbError::ValidationError {
                                field: "float_range".to_string(),
                                message: format!("浮点数值不能大于{}", max_val)
                            });
                        }
                    }
                } else {
                    return Err(QuickDbError::ValidationError {
                        field: "type_mismatch".to_string(),
                        message: "字段类型不匹配，期望浮点数类型".to_string()
                    });
                }
            }
            FieldType::Boolean => {
                if !matches!(value, DataValue::Bool(_)) {
                    return Err(QuickDbError::ValidationError {
                        field: "type_mismatch".to_string(),
                        message: "字段类型不匹配，期望布尔类型".to_string()
                    });
                }
            }
            FieldType::DateTime => {
                if !matches!(value, DataValue::DateTime(_)) {
                    return Err(QuickDbError::ValidationError {
                        field: "type_mismatch".to_string(),
                        message: "字段类型不匹配，期望日期时间类型".to_string()
                    });
                }
            }
            FieldType::Uuid => {
                match value {
                    DataValue::String(s) => {
                        // 验证字符串格式的UUID
                        debug!("🔍 UUID字段验证 - 字符串格式: '{}' (字段: {})", s, field_name);
                        if uuid::Uuid::parse_str(s).is_err() {
                            debug!("❌ UUID字段验证失败 - 无效的UUID格式: '{}' (字段: {})", s, field_name);
                            return Err(QuickDbError::ValidationError {
                                field: "uuid_format".to_string(),
                                message: format!("无效的UUID格式: '{}' (字段: {})", s, field_name)
                            });
                        }
                        debug!("✅ UUID字段验证通过 - 字符串格式: '{}' (字段: {})", s, field_name);
                    },
                    DataValue::Uuid(u) => {
                        // DataValue::Uuid类型本身就是有效的，无需验证
                        debug!("✅ UUID字段验证通过 - UUID类型: {} (字段: {})", u, field_name);
                    },
                    _ => {
                        debug!("❌ UUID字段验证失败 - 类型不匹配: {:?} (字段: {})", value, field_name);
                        return Err(QuickDbError::ValidationError {
                            field: "type_mismatch".to_string(),
                            message: format!("字段类型不匹配，期望UUID字符串或UUID类型，实际收到: {:?} (字段: {})", value, field_name)
                        });
                    }
                }
            }
            FieldType::Json => {
                // JSON类型可以接受任何值
            }
            FieldType::Array { item_type, max_items, min_items } => {
                match value {
                    DataValue::Array(arr) => {
                        // 处理DataValue::Array格式
                        if let Some(max_items) = max_items {
                            if arr.len() > *max_items {
                                return Err(QuickDbError::ValidationError {
                                    field: "array_size".to_string(),
                                    message: format!("数组元素数量不能超过{}", max_items)
                                });
                            }
                        }
                        if let Some(min_items) = min_items {
                            if arr.len() < *min_items {
                                return Err(QuickDbError::ValidationError {
                                    field: "array_size".to_string(),
                                    message: format!("数组元素数量不能少于{}", min_items)
                                });
                            }
                        }
                        // 验证数组中的每个元素
                        let item_field = FieldDefinition::new((**item_type).clone());
                        for item in arr {
                            item_field.validate(item)?;
                        }
                    },
                    DataValue::String(json_str) => {
                        // 处理JSON字符串格式的数组
                        if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(json_str) {
                            if let Some(arr) = json_value.as_array() {
                                if let Some(max_items) = max_items {
                                    if arr.len() > *max_items {
                                        return Err(QuickDbError::ValidationError {
                                            field: "array_size".to_string(),
                                            message: format!("数组元素数量不能超过{}", max_items)
                                        });
                                    }
                                }
                                if let Some(min_items) = min_items {
                                    if arr.len() < *min_items {
                                        return Err(QuickDbError::ValidationError {
                                            field: "array_size".to_string(),
                                            message: format!("数组元素数量不能少于{}", min_items)
                                        });
                                    }
                                }
                                // 验证数组中的每个元素
                                let item_field = FieldDefinition::new((**item_type).clone());
                                for item_json in arr {
                                    let item_data_value = DataValue::from_json(item_json.clone());
                                    item_field.validate(&item_data_value)?;
                                }
                            } else {
                                return Err(QuickDbError::ValidationError {
                                    field: "type_mismatch".to_string(),
                                    message: "JSON字符串不是有效的数组格式".to_string()
                                });
                            }
                        } else {
                            return Err(QuickDbError::ValidationError {
                                field: "type_mismatch".to_string(),
                                message: "无法解析JSON字符串".to_string()
                            });
                        }
                    },
                    _ => {
                        return Err(QuickDbError::ValidationError {
                            field: "type_mismatch".to_string(),
                            message: "字段类型不匹配，期望数组类型或JSON字符串".to_string()
                        });
                    }
                }
            }
            FieldType::Object { fields } => {
                if let DataValue::Object(obj) = value {
                    // 验证对象中的每个字段
                    for (field_name, field_def) in fields {
                        let field_value = obj.get(field_name).unwrap_or(&DataValue::Null);
                        field_def.validate(field_value)?;
                    }
                } else {
                    return Err(QuickDbError::ValidationError {
                        field: "type_mismatch".to_string(),
                        message: "字段类型不匹配，期望对象类型".to_string()
                    });
                }
            }
            FieldType::Reference { target_collection: _ } => {
                // 引用类型通常是字符串ID
                if !matches!(value, DataValue::String(_)) {
                    return Err(QuickDbError::ValidationError {
                        field: "reference_type".to_string(),
                        message: "引用字段必须是字符串ID".to_string()
                    });
                }
            }
            FieldType::BigInteger => {
                if !matches!(value, DataValue::Int(_)) {
                    return Err(QuickDbError::ValidationError {
                        field: "type_mismatch".to_string(),
                        message: "字段类型不匹配，期望大整数类型".to_string()
                    });
                }
            }
            FieldType::Double => {
                if !matches!(value, DataValue::Float(_)) {
                    return Err(QuickDbError::ValidationError {
                        field: "type_mismatch".to_string(),
                        message: "字段类型不匹配，期望双精度浮点数类型".to_string()
                    });
                }
            }
            FieldType::Text => {
                if !matches!(value, DataValue::String(_)) {
                    return Err(QuickDbError::ValidationError {
                        field: "type_mismatch".to_string(),
                        message: "字段类型不匹配，期望文本类型".to_string()
                    });
                }
            }
            FieldType::Date => {
                if !matches!(value, DataValue::DateTime(_)) {
                    return Err(QuickDbError::ValidationError {
                        field: "type_mismatch".to_string(),
                        message: "字段类型不匹配，期望日期类型".to_string()
                    });
                }
            }
            FieldType::Time => {
                if !matches!(value, DataValue::DateTime(_)) {
                    return Err(QuickDbError::ValidationError {
                        field: "type_mismatch".to_string(),
                        message: "字段类型不匹配，期望时间类型".to_string()
                    });
                }
            }
            FieldType::Binary => {
                if !matches!(value, DataValue::String(_)) {
                    return Err(QuickDbError::ValidationError {
                        field: "type_mismatch".to_string(),
                        message: "字段类型不匹配，期望二进制数据（Base64字符串）".to_string()
                    });
                }
            }
            FieldType::Decimal { precision: _, scale: _ } => {
                if !matches!(value, DataValue::Float(_)) {
                    return Err(QuickDbError::ValidationError {
                        field: "type_mismatch".to_string(),
                        message: "字段类型不匹配，期望十进制数类型".to_string()
                    });
                }
            }
        }

        Ok(())
    }
}

/// 模型元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMeta {
    /// 集合/表名
    pub collection_name: String,
    /// 数据库别名
    pub database_alias: Option<String>,
    /// 字段定义
    pub fields: HashMap<String, FieldDefinition>,
    /// 索引定义
    pub indexes: Vec<IndexDefinition>,
    /// 模型描述
    pub description: Option<String>,
}

/// 索引定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexDefinition {
    /// 索引字段
    pub fields: Vec<String>,
    /// 是否唯一索引
    pub unique: bool,
    /// 索引名称
    pub name: Option<String>,
}