//! 字段类型定义模块
//!
//! 定义模型字段的类型、验证和元数据

use crate::error::{QuickDbError, QuickDbResult};
use crate::types::DataValue;
use rat_logger::debug;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    /// 带时区的日期时间类型（存储为Unix时间戳）
    DateTimeWithTz {
        timezone_offset: String, // 格式："+00:00", "+08:00", "-05:00"
    },
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
    Decimal { precision: u8, scale: u8 },
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
    Reference { target_collection: String },
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

    pub fn validate_with_field_name(
        &self,
        value: &DataValue,
        field_name: &str,
    ) -> QuickDbResult<()> {
        // 检查必填字段
        if self.required && matches!(value, DataValue::Null) {
            return Err(QuickDbError::ValidationError {
                field: field_name.to_string(),
                message: crate::i18n::t("validation.required_empty"),
            });
        }

        // 如果值为空且不是必填字段，则跳过验证
        if matches!(value, DataValue::Null) {
            return Ok(());
        }

        // 根据字段类型进行验证
        match &self.field_type {
            FieldType::String {
                max_length,
                min_length,
                regex,
            } => {
                if let DataValue::String(s) = value {
                    if let Some(max_len) = max_length {
                        if s.len() > *max_len {
                            return Err(QuickDbError::ValidationError {
                                field: "string_length".to_string(),
                                message: crate::i18n::tf("validation.string_max_length", &[("max", &max_len.to_string())]),
                            });
                        }
                    }
                    if let Some(min_len) = min_length {
                        if s.len() < *min_len {
                            return Err(QuickDbError::ValidationError {
                                field: "string_length".to_string(),
                                message: crate::i18n::tf("validation.string_min_length", &[("min", &min_len.to_string())]),
                            });
                        }
                    }
                    if let Some(pattern) = regex {
                        let regex = regex::Regex::new(pattern).map_err(|e| {
                            QuickDbError::ValidationError {
                                field: "regex".to_string(),
                                message: crate::i18n::tf("validation.regex_invalid", &[("error", &e.to_string())]),
                            }
                        })?;
                        if !regex.is_match(s) {
                            return Err(QuickDbError::ValidationError {
                                field: "regex_match".to_string(),
                                message: crate::i18n::t("validation.regex_not_match"),
                            });
                        }
                    }
                } else {
                    return Err(QuickDbError::ValidationError {
                        field: "type_mismatch".to_string(),
                        message: crate::i18n::t("validation.type_string"),
                    });
                }
            }
            FieldType::Integer {
                min_value,
                max_value,
            } => {
                if let DataValue::Int(i) = value {
                    if let Some(min_val) = min_value {
                        if *i < *min_val {
                            return Err(QuickDbError::ValidationError {
                                field: "integer_range".to_string(),
                                message: crate::i18n::tf("validation.integer_min", &[("min", &min_val.to_string())]),
                            });
                        }
                    }
                    if let Some(max_val) = max_value {
                        if *i > *max_val {
                            return Err(QuickDbError::ValidationError {
                                field: "integer_range".to_string(),
                                message: crate::i18n::tf("validation.integer_max", &[("max", &max_val.to_string())]),
                            });
                        }
                    }
                } else {
                    return Err(QuickDbError::ValidationError {
                        field: "type_mismatch".to_string(),
                        message: crate::i18n::t("validation.type_integer"),
                    });
                }
            }
            FieldType::Float {
                min_value,
                max_value,
            } => {
                if let DataValue::Float(f) = value {
                    if let Some(min_val) = min_value {
                        if *f < *min_val {
                            return Err(QuickDbError::ValidationError {
                                field: "float_range".to_string(),
                                message: crate::i18n::tf("validation.float_min", &[("min", &min_val.to_string())]),
                            });
                        }
                    }
                    if let Some(max_val) = max_value {
                        if *f > *max_val {
                            return Err(QuickDbError::ValidationError {
                                field: "float_range".to_string(),
                                message: crate::i18n::tf("validation.float_max", &[("max", &max_val.to_string())]),
                            });
                        }
                    }
                } else {
                    return Err(QuickDbError::ValidationError {
                        field: "type_mismatch".to_string(),
                        message: crate::i18n::t("validation.type_float"),
                    });
                }
            }
            FieldType::Boolean => {
                if !matches!(value, DataValue::Bool(_)) {
                    return Err(QuickDbError::ValidationError {
                        field: "type_mismatch".to_string(),
                        message: crate::i18n::t("validation.type_boolean"),
                    });
                }
            }
            FieldType::DateTime => {
                if !matches!(value, DataValue::DateTime(_)) {
                    return Err(QuickDbError::ValidationError {
                        field: "type_mismatch".to_string(),
                        message: crate::i18n::t("validation.type_datetime"),
                    });
                }
            }
            FieldType::DateTimeWithTz { timezone_offset } => {
                match value {
                    DataValue::DateTime(_) => {
                        // DateTime类型可以直接接受
                        debug!(
                            "✅ DateTimeWithTz字段验证通过 - DateTime类型 (字段: {}, 时区: {})",
                            field_name, timezone_offset
                        );
                    }
                    DataValue::String(s) => {
                        // 验证字符串格式的日期时间（RFC3339或本地时间格式）
                        if s.is_empty() {
                            debug!(
                                "✅ DateTimeWithTz字段验证通过 - 空字符串（将自动生成当前时间） (字段: {}, 时区: {})",
                                field_name, timezone_offset
                            );
                        } else {
                            // 尝试解析RFC3339格式
                            if s.contains('T')
                                && (s.contains('+') || s.contains('Z') || s.contains('-'))
                            {
                                if chrono::DateTime::parse_from_rfc3339(s).is_ok() {
                                    debug!(
                                        "✅ DateTimeWithTz字段验证通过 - RFC3339格式: '{}' (字段: {}, 时区: {})",
                                        s, field_name, timezone_offset
                                    );
                                } else {
                                    return Err(QuickDbError::ValidationError {
                                        field: "datetime_format".to_string(),
                                        message: crate::i18n::tf("validation.rfc3339_invalid", &[("value", s), ("field", field_name)]),
                                    });
                                }
                            } else {
                                // 尝试解析本地时间格式（如 "2024-06-15 12:00:00"）
                                if chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
                                    .is_ok()
                                {
                                    debug!(
                                        "✅ DateTimeWithTz字段验证通过 - 本地时间格式: '{}' (字段: {}, 时区: {})",
                                        s, field_name, timezone_offset
                                    );
                                } else {
                                    return Err(QuickDbError::ValidationError {
                                        field: "datetime_format".to_string(),
                                        message: crate::i18n::tf("validation.datetime_invalid", &[("value", s), ("field", field_name)]),
                                    });
                                }
                            }
                        }
                    }
                    DataValue::Int(_) => {
                        // Unix时间戳也可以接受
                        debug!(
                            "✅ DateTimeWithTz字段验证通过 - Unix时间戳 (字段: {}, 时区: {})",
                            field_name, timezone_offset
                        );
                    }
                    _ => {
                        return Err(QuickDbError::ValidationError {
                            field: "type_mismatch".to_string(),
                            message: crate::i18n::tf("validation.type_datetime_or_string", &[("field", field_name)]),
                        });
                    }
                }

                // 验证时区偏移格式
                if !is_valid_timezone_offset(timezone_offset) {
                    return Err(QuickDbError::ValidationError {
                        field: "timezone_offset".to_string(),
                        message: crate::i18n::tf("validation.timezone_offset_invalid", &[("offset", timezone_offset)]),
                    });
                }
            }
            FieldType::Uuid => {
                match value {
                    DataValue::String(s) => {
                        // 验证字符串格式的UUID
                        debug!(
                            "🔍 UUID字段验证 - 字符串格式: '{}' (字段: {})",
                            s, field_name
                        );
                        // 空字符串表示需要自动生成UUID，允许通过
                        if s.is_empty() {
                            debug!(
                                "✅ UUID字段验证通过 - 空字符串（将自动生成UUID） (字段: {})",
                                field_name
                            );
                        } else if uuid::Uuid::parse_str(s).is_err() {
                            debug!(
                                "❌ UUID字段验证失败 - 无效的UUID格式: '{}' (字段: {})",
                                s, field_name
                            );
                            return Err(QuickDbError::ValidationError {
                                field: "uuid_format".to_string(),
                                message: crate::i18n::tf("validation.uuid_invalid", &[("value", s), ("field", field_name)]),
                            });
                        } else {
                            debug!(
                                "✅ UUID字段验证通过 - 字符串格式: '{}' (字段: {})",
                                s, field_name
                            );
                        }
                    }
                    DataValue::Uuid(u) => {
                        // DataValue::Uuid类型本身就是有效的，无需验证
                        debug!(
                            "✅ UUID字段验证通过 - UUID类型: {} (字段: {})",
                            u, field_name
                        );
                    }
                    _ => {
                        debug!(
                            "❌ UUID字段验证失败 - 类型不匹配: {:?} (字段: {})",
                            value, field_name
                        );
                        return Err(QuickDbError::ValidationError {
                            field: "type_mismatch".to_string(),
                            message: crate::i18n::tf("validation.type_uuid", &[("actual", &format!("{:?}", value)), ("field", field_name)]),
                        });
                    }
                }
            }
            FieldType::Json => {
                // JSON类型可以接受任何值
            }
            FieldType::Array {
                item_type,
                max_items,
                min_items,
            } => {
                match value {
                    DataValue::Array(arr) => {
                        // 处理DataValue::Array格式
                        if let Some(max_items) = max_items {
                            if arr.len() > *max_items {
                                return Err(QuickDbError::ValidationError {
                                    field: "array_size".to_string(),
                                    message: crate::i18n::tf("validation.array_max_items", &[("max", &max_items.to_string())]),
                                });
                            }
                        }
                        if let Some(min_items) = min_items {
                            if arr.len() < *min_items {
                                return Err(QuickDbError::ValidationError {
                                    field: "array_size".to_string(),
                                    message: crate::i18n::tf("validation.array_min_items", &[("min", &min_items.to_string())]),
                                });
                            }
                        }
                        // 验证数组中的每个元素
                        let item_field = FieldDefinition::new((**item_type).clone());
                        for item in arr {
                            item_field.validate(item)?;
                        }
                    }
                    DataValue::String(json_str) => {
                        // 处理JSON字符串格式的数组
                        if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(json_str)
                        {
                            if let Some(arr) = json_value.as_array() {
                                if let Some(max_items) = max_items {
                                    if arr.len() > *max_items {
                                        return Err(QuickDbError::ValidationError {
                                            field: "array_size".to_string(),
                                            message: crate::i18n::tf("validation.array_max_items", &[("max", &max_items.to_string())]),
                                        });
                                    }
                                }
                                if let Some(min_items) = min_items {
                                    if arr.len() < *min_items {
                                        return Err(QuickDbError::ValidationError {
                                            field: "array_size".to_string(),
                                            message: crate::i18n::tf("validation.array_min_items", &[("min", &min_items.to_string())]),
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
                                    message: crate::i18n::t("validation.type_array_json_invalid"),
                                });
                            }
                        } else {
                            return Err(QuickDbError::ValidationError {
                                field: "type_mismatch".to_string(),
                                message: crate::i18n::t("validation.json_parse_failed"),
                            });
                        }
                    }
                    _ => {
                        return Err(QuickDbError::ValidationError {
                            field: "type_mismatch".to_string(),
                            message: crate::i18n::t("validation.type_array"),
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
                        message: crate::i18n::t("validation.type_object"),
                    });
                }
            }
            FieldType::Reference {
                target_collection: _,
            } => {
                // 引用类型通常是字符串ID
                if !matches!(value, DataValue::String(_)) {
                    return Err(QuickDbError::ValidationError {
                        field: "reference_type".to_string(),
                        message: crate::i18n::t("validation.type_reference"),
                    });
                }
            }
            FieldType::BigInteger => {
                if !matches!(value, DataValue::Int(_)) {
                    return Err(QuickDbError::ValidationError {
                        field: "type_mismatch".to_string(),
                        message: crate::i18n::t("validation.type_biginteger"),
                    });
                }
            }
            FieldType::Double => {
                if !matches!(value, DataValue::Float(_)) {
                    return Err(QuickDbError::ValidationError {
                        field: "type_mismatch".to_string(),
                        message: crate::i18n::t("validation.type_double"),
                    });
                }
            }
            FieldType::Text => {
                if !matches!(value, DataValue::String(_)) {
                    return Err(QuickDbError::ValidationError {
                        field: "type_mismatch".to_string(),
                        message: crate::i18n::t("validation.type_text"),
                    });
                }
            }
            FieldType::Date => {
                if !matches!(value, DataValue::DateTime(_)) {
                    return Err(QuickDbError::ValidationError {
                        field: "type_mismatch".to_string(),
                        message: crate::i18n::t("validation.type_date"),
                    });
                }
            }
            FieldType::Time => {
                if !matches!(value, DataValue::DateTime(_)) {
                    return Err(QuickDbError::ValidationError {
                        field: "type_mismatch".to_string(),
                        message: crate::i18n::t("validation.type_time"),
                    });
                }
            }
            FieldType::Binary => {
                if !matches!(value, DataValue::String(_)) {
                    return Err(QuickDbError::ValidationError {
                        field: "type_mismatch".to_string(),
                        message: crate::i18n::t("validation.type_binary"),
                    });
                }
            }
            FieldType::Decimal {
                precision: _,
                scale: _,
            } => {
                if !matches!(value, DataValue::Float(_)) {
                    return Err(QuickDbError::ValidationError {
                        field: "type_mismatch".to_string(),
                        message: crate::i18n::t("validation.type_decimal"),
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
    /// 模型版本号（用于字段版本控制）
    pub version: Option<u32>,
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

/// 验证时区偏移格式是否有效
///
/// 有效格式：+00:00, +08:00, -05:00 等
#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: initialize i18n and set the given language.
    fn setup_i18n(lang: &str) {
        crate::i18n::ErrorMessageI18n::init_i18n();
        crate::i18n::set_language(lang);
    }

    /// Extract the `message` string from a `QuickDbError::ValidationError`.
    fn validation_message(err: &QuickDbError) -> &str {
        match err {
            QuickDbError::ValidationError { message, .. } => message.as_str(),
            _ => panic!("expected ValidationError"),
        }
    }

    // =========================================================================
    // Required field
    // =========================================================================

    #[test]
    fn test_required_empty_zh_cn() {
        setup_i18n("zh-CN");
        let field = FieldDefinition::new(FieldType::String {
            max_length: None,
            min_length: None,
            regex: None,
        })
        .required();
        let err = field.validate_with_field_name(&DataValue::Null, "username").unwrap_err();
        assert_eq!(validation_message(&err), "必填字段不能为空");
    }

    #[test]
    fn test_required_empty_en_us() {
        setup_i18n("en-US");
        let field = FieldDefinition::new(FieldType::String {
            max_length: None,
            min_length: None,
            regex: None,
        })
        .required();
        let err = field.validate_with_field_name(&DataValue::Null, "username").unwrap_err();
        assert_eq!(
            validation_message(&err),
            "Required field cannot be empty"
        );
    }

    #[test]
    fn test_required_empty_ja_jp() {
        setup_i18n("ja-JP");
        let field = FieldDefinition::new(FieldType::String {
            max_length: None,
            min_length: None,
            regex: None,
        })
        .required();
        let err = field.validate_with_field_name(&DataValue::Null, "username").unwrap_err();
        assert_eq!(
            validation_message(&err),
            "必須フィールドは空にできません"
        );
    }

    // =========================================================================
    // String type
    // =========================================================================

    #[test]
    fn test_string_max_length_zh_cn() {
        setup_i18n("zh-CN");
        let field = FieldDefinition::new(FieldType::String {
            max_length: Some(5),
            min_length: None,
            regex: None,
        });
        let err = field
            .validate_with_field_name(&DataValue::String("hello world".into()), "name")
            .unwrap_err();
        assert_eq!(validation_message(&err), "字符串长度不能超过5");
    }

    #[test]
    fn test_string_max_length_en_us() {
        setup_i18n("en-US");
        let field = FieldDefinition::new(FieldType::String {
            max_length: Some(5),
            min_length: None,
            regex: None,
        });
        let err = field
            .validate_with_field_name(&DataValue::String("hello world".into()), "name")
            .unwrap_err();
        assert_eq!(validation_message(&err), "String length cannot exceed 5");
    }

    #[test]
    fn test_string_min_length_zh_cn() {
        setup_i18n("zh-CN");
        let field = FieldDefinition::new(FieldType::String {
            max_length: None,
            min_length: Some(3),
            regex: None,
        });
        let err = field
            .validate_with_field_name(&DataValue::String("ab".into()), "name")
            .unwrap_err();
        assert_eq!(validation_message(&err), "字符串长度不能少于3");
    }

    #[test]
    fn test_string_min_length_en_us() {
        setup_i18n("en-US");
        let field = FieldDefinition::new(FieldType::String {
            max_length: None,
            min_length: Some(3),
            regex: None,
        });
        let err = field
            .validate_with_field_name(&DataValue::String("ab".into()), "name")
            .unwrap_err();
        assert_eq!(
            validation_message(&err),
            "String length cannot be less than 3"
        );
    }

    #[test]
    fn test_string_regex_invalid_zh_cn() {
        setup_i18n("zh-CN");
        let field = FieldDefinition::new(FieldType::String {
            max_length: None,
            min_length: None,
            regex: Some("[invalid(".into()),
        });
        let err = field
            .validate_with_field_name(&DataValue::String("test".into()), "name")
            .unwrap_err();
        let msg = validation_message(&err);
        assert!(msg.starts_with("正则表达式无效:"));
    }

    #[test]
    fn test_string_regex_invalid_en_us() {
        setup_i18n("en-US");
        let field = FieldDefinition::new(FieldType::String {
            max_length: None,
            min_length: None,
            regex: Some("[invalid(".into()),
        });
        let err = field
            .validate_with_field_name(&DataValue::String("test".into()), "name")
            .unwrap_err();
        let msg = validation_message(&err);
        assert!(msg.starts_with("Invalid regex:"));
    }

    #[test]
    fn test_string_regex_not_match_zh_cn() {
        setup_i18n("zh-CN");
        let field = FieldDefinition::new(FieldType::String {
            max_length: None,
            min_length: None,
            regex: Some(r"^\d+$".into()),
        });
        let err = field
            .validate_with_field_name(&DataValue::String("abc".into()), "name")
            .unwrap_err();
        assert_eq!(validation_message(&err), "字符串不匹配正则表达式");
    }

    #[test]
    fn test_string_regex_not_match_en_us() {
        setup_i18n("en-US");
        let field = FieldDefinition::new(FieldType::String {
            max_length: None,
            min_length: None,
            regex: Some(r"^\d+$".into()),
        });
        let err = field
            .validate_with_field_name(&DataValue::String("abc".into()), "name")
            .unwrap_err();
        assert_eq!(
            validation_message(&err),
            "String does not match regex pattern"
        );
    }

    #[test]
    fn test_string_type_mismatch_zh_cn() {
        setup_i18n("zh-CN");
        let field = FieldDefinition::new(FieldType::String {
            max_length: None,
            min_length: None,
            regex: None,
        });
        let err = field
            .validate_with_field_name(&DataValue::Int(42), "name")
            .unwrap_err();
        assert_eq!(
            validation_message(&err),
            "字段类型不匹配，期望字符串类型"
        );
    }

    #[test]
    fn test_string_type_mismatch_en_us() {
        setup_i18n("en-US");
        let field = FieldDefinition::new(FieldType::String {
            max_length: None,
            min_length: None,
            regex: None,
        });
        let err = field
            .validate_with_field_name(&DataValue::Int(42), "name")
            .unwrap_err();
        assert_eq!(
            validation_message(&err),
            "Type mismatch, expected string type"
        );
    }

    // =========================================================================
    // Integer type
    // =========================================================================

    #[test]
    fn test_integer_min_zh_cn() {
        setup_i18n("zh-CN");
        let field = FieldDefinition::new(FieldType::Integer {
            min_value: Some(0),
            max_value: None,
        });
        let err = field
            .validate_with_field_name(&DataValue::Int(-1), "age")
            .unwrap_err();
        assert_eq!(validation_message(&err), "整数值不能小于0");
    }

    #[test]
    fn test_integer_min_en_us() {
        setup_i18n("en-US");
        let field = FieldDefinition::new(FieldType::Integer {
            min_value: Some(0),
            max_value: None,
        });
        let err = field
            .validate_with_field_name(&DataValue::Int(-1), "age")
            .unwrap_err();
        assert_eq!(validation_message(&err), "Integer value cannot be less than 0");
    }

    #[test]
    fn test_integer_max_zh_cn() {
        setup_i18n("zh-CN");
        let field = FieldDefinition::new(FieldType::Integer {
            min_value: None,
            max_value: Some(100),
        });
        let err = field
            .validate_with_field_name(&DataValue::Int(101), "age")
            .unwrap_err();
        assert_eq!(validation_message(&err), "整数值不能大于100");
    }

    #[test]
    fn test_integer_max_en_us() {
        setup_i18n("en-US");
        let field = FieldDefinition::new(FieldType::Integer {
            min_value: None,
            max_value: Some(100),
        });
        let err = field
            .validate_with_field_name(&DataValue::Int(101), "age")
            .unwrap_err();
        assert_eq!(
            validation_message(&err),
            "Integer value cannot be greater than 100"
        );
    }

    #[test]
    fn test_integer_type_mismatch_zh_cn() {
        setup_i18n("zh-CN");
        let field = FieldDefinition::new(FieldType::Integer {
            min_value: None,
            max_value: None,
        });
        let err = field
            .validate_with_field_name(&DataValue::String("not an int".into()), "age")
            .unwrap_err();
        assert_eq!(
            validation_message(&err),
            "字段类型不匹配，期望整数类型"
        );
    }

    #[test]
    fn test_integer_type_mismatch_en_us() {
        setup_i18n("en-US");
        let field = FieldDefinition::new(FieldType::Integer {
            min_value: None,
            max_value: None,
        });
        let err = field
            .validate_with_field_name(&DataValue::String("not an int".into()), "age")
            .unwrap_err();
        assert_eq!(
            validation_message(&err),
            "Type mismatch, expected integer type"
        );
    }

    // =========================================================================
    // Float type
    // =========================================================================

    #[test]
    fn test_float_min_zh_cn() {
        setup_i18n("zh-CN");
        let field = FieldDefinition::new(FieldType::Float {
            min_value: Some(0.0),
            max_value: None,
        });
        let err = field
            .validate_with_field_name(&DataValue::Float(-0.1), "score")
            .unwrap_err();
        assert_eq!(validation_message(&err), "浮点数值不能小于0");
    }

    #[test]
    fn test_float_min_en_us() {
        setup_i18n("en-US");
        let field = FieldDefinition::new(FieldType::Float {
            min_value: Some(0.0),
            max_value: None,
        });
        let err = field
            .validate_with_field_name(&DataValue::Float(-0.1), "score")
            .unwrap_err();
        assert_eq!(validation_message(&err), "Float value cannot be less than 0");
    }

    #[test]
    fn test_float_max_zh_cn() {
        setup_i18n("zh-CN");
        let field = FieldDefinition::new(FieldType::Float {
            min_value: None,
            max_value: Some(1.0),
        });
        let err = field
            .validate_with_field_name(&DataValue::Float(1.5), "score")
            .unwrap_err();
        assert_eq!(validation_message(&err), "浮点数值不能大于1");
    }

    #[test]
    fn test_float_max_en_us() {
        setup_i18n("en-US");
        let field = FieldDefinition::new(FieldType::Float {
            min_value: None,
            max_value: Some(1.0),
        });
        let err = field
            .validate_with_field_name(&DataValue::Float(1.5), "score")
            .unwrap_err();
        assert_eq!(validation_message(&err), "Float value cannot be greater than 1");
    }

    #[test]
    fn test_float_type_mismatch_zh_cn() {
        setup_i18n("zh-CN");
        let field = FieldDefinition::new(FieldType::Float {
            min_value: None,
            max_value: None,
        });
        let err = field
            .validate_with_field_name(&DataValue::String("not a float".into()), "score")
            .unwrap_err();
        assert_eq!(
            validation_message(&err),
            "字段类型不匹配，期望浮点数类型"
        );
    }

    #[test]
    fn test_float_type_mismatch_en_us() {
        setup_i18n("en-US");
        let field = FieldDefinition::new(FieldType::Float {
            min_value: None,
            max_value: None,
        });
        let err = field
            .validate_with_field_name(&DataValue::String("not a float".into()), "score")
            .unwrap_err();
        assert_eq!(
            validation_message(&err),
            "Type mismatch, expected float type"
        );
    }

    // =========================================================================
    // Boolean type
    // =========================================================================

    #[test]
    fn test_boolean_type_mismatch_zh_cn() {
        setup_i18n("zh-CN");
        let field = FieldDefinition::new(FieldType::Boolean);
        let err = field
            .validate_with_field_name(&DataValue::Int(1), "active")
            .unwrap_err();
        assert_eq!(
            validation_message(&err),
            "字段类型不匹配，期望布尔类型"
        );
    }

    #[test]
    fn test_boolean_type_mismatch_en_us() {
        setup_i18n("en-US");
        let field = FieldDefinition::new(FieldType::Boolean);
        let err = field
            .validate_with_field_name(&DataValue::Int(1), "active")
            .unwrap_err();
        assert_eq!(
            validation_message(&err),
            "Type mismatch, expected boolean type"
        );
    }

    // =========================================================================
    // DateTime type
    // =========================================================================

    #[test]
    fn test_datetime_type_mismatch_zh_cn() {
        setup_i18n("zh-CN");
        let field = FieldDefinition::new(FieldType::DateTime);
        let err = field
            .validate_with_field_name(&DataValue::String("not datetime".into()), "created_at")
            .unwrap_err();
        assert_eq!(
            validation_message(&err),
            "字段类型不匹配，期望日期时间类型"
        );
    }

    #[test]
    fn test_datetime_type_mismatch_en_us() {
        setup_i18n("en-US");
        let field = FieldDefinition::new(FieldType::DateTime);
        let err = field
            .validate_with_field_name(&DataValue::String("not datetime".into()), "created_at")
            .unwrap_err();
        assert_eq!(
            validation_message(&err),
            "Type mismatch, expected datetime type"
        );
    }

    // =========================================================================
    // DateTimeWithTz type
    // =========================================================================

    #[test]
    fn test_datetime_with_tz_rfc3339_invalid_zh_cn() {
        setup_i18n("zh-CN");
        let field = FieldDefinition::new(FieldType::DateTimeWithTz {
            timezone_offset: "+08:00".into(),
        });
        let err = field
            .validate_with_field_name(
                &DataValue::String("2024-13-01T12:00:00+08:00".into()),
                "created_at",
            )
            .unwrap_err();
        let msg = validation_message(&err);
        assert!(msg.contains("无效的RFC3339日期时间格式:"));
        assert!(msg.contains("'2024-13-01T12:00:00+08:00'"));
        assert!(msg.contains("created_at"));
    }

    #[test]
    fn test_datetime_with_tz_rfc3339_invalid_en_us() {
        setup_i18n("en-US");
        let field = FieldDefinition::new(FieldType::DateTimeWithTz {
            timezone_offset: "+08:00".into(),
        });
        let err = field
            .validate_with_field_name(
                &DataValue::String("2024-13-01T12:00:00+08:00".into()),
                "created_at",
            )
            .unwrap_err();
        let msg = validation_message(&err);
        assert!(msg.contains("Invalid RFC3339 datetime format:"));
        assert!(msg.contains("'2024-13-01T12:00:00+08:00'"));
        assert!(msg.contains("created_at"));
    }

    #[test]
    fn test_datetime_with_tz_datetime_invalid_zh_cn() {
        setup_i18n("zh-CN");
        let field = FieldDefinition::new(FieldType::DateTimeWithTz {
            timezone_offset: "+08:00".into(),
        });
        // A string without 'T' or '+'/'Z'/'-' -- enters the NaiveDateTime branch
        let err = field
            .validate_with_field_name(
                &DataValue::String("not-a-valid-datetime".into()),
                "created_at",
            )
            .unwrap_err();
        let msg = validation_message(&err);
        assert!(msg.contains("无效的日期时间格式，期望RFC3339或YYYY-MM-DD HH:MM:SS格式:"));
        assert!(msg.contains("created_at"));
    }

    #[test]
    fn test_datetime_with_tz_datetime_invalid_en_us() {
        setup_i18n("en-US");
        let field = FieldDefinition::new(FieldType::DateTimeWithTz {
            timezone_offset: "+08:00".into(),
        });
        let err = field
            .validate_with_field_name(
                &DataValue::String("not-a-valid-datetime".into()),
                "created_at",
            )
            .unwrap_err();
        let msg = validation_message(&err);
        assert!(msg.contains("Invalid datetime format, expected RFC3339 or YYYY-MM-DD HH:MM:SS:"));
        assert!(msg.contains("created_at"));
    }

    #[test]
    fn test_datetime_with_tz_type_mismatch_zh_cn() {
        setup_i18n("zh-CN");
        let field = FieldDefinition::new(FieldType::DateTimeWithTz {
            timezone_offset: "+08:00".into(),
        });
        let err = field
            .validate_with_field_name(&DataValue::Bool(true), "created_at")
            .unwrap_err();
        let msg = validation_message(&err);
        assert!(msg.contains("字段类型不匹配，期望日期时间类型或字符串或整数"));
        assert!(msg.contains("created_at"));
    }

    #[test]
    fn test_datetime_with_tz_type_mismatch_en_us() {
        setup_i18n("en-US");
        let field = FieldDefinition::new(FieldType::DateTimeWithTz {
            timezone_offset: "+08:00".into(),
        });
        let err = field
            .validate_with_field_name(&DataValue::Bool(true), "created_at")
            .unwrap_err();
        let msg = validation_message(&err);
        assert!(msg.contains("Type mismatch, expected datetime, string or integer type"));
        assert!(msg.contains("created_at"));
    }

    #[test]
    fn test_datetime_with_tz_timezone_offset_invalid_zh_cn() {
        setup_i18n("zh-CN");
        let field = FieldDefinition::new(FieldType::DateTimeWithTz {
            timezone_offset: "bad_offset".into(),
        });
        let err = field
            .validate_with_field_name(
                &DataValue::String("2024-06-15T12:00:00+08:00".into()),
                "created_at",
            )
            .unwrap_err();
        let msg = validation_message(&err);
        assert!(msg.contains("无效的时区偏移格式: 'bad_offset'"));
        assert!(msg.contains("+00:00, +08:00, -05:00"));
    }

    #[test]
    fn test_datetime_with_tz_timezone_offset_invalid_en_us() {
        setup_i18n("en-US");
        let field = FieldDefinition::new(FieldType::DateTimeWithTz {
            timezone_offset: "bad_offset".into(),
        });
        let err = field
            .validate_with_field_name(
                &DataValue::String("2024-06-15T12:00:00+08:00".into()),
                "created_at",
            )
            .unwrap_err();
        let msg = validation_message(&err);
        assert!(msg.contains("Invalid timezone offset format: 'bad_offset'"));
        assert!(msg.contains("+00:00, +08:00, -05:00"));
    }

    // =========================================================================
    // UUID type
    // =========================================================================

    #[test]
    fn test_uuid_invalid_zh_cn() {
        setup_i18n("zh-CN");
        let field = FieldDefinition::new(FieldType::Uuid);
        let err = field
            .validate_with_field_name(&DataValue::String("not-a-uuid".into()), "id")
            .unwrap_err();
        let msg = validation_message(&err);
        assert!(msg.contains("无效的UUID格式: 'not-a-uuid'"));
        assert!(msg.contains("id"));
    }

    #[test]
    fn test_uuid_invalid_en_us() {
        setup_i18n("en-US");
        let field = FieldDefinition::new(FieldType::Uuid);
        let err = field
            .validate_with_field_name(&DataValue::String("not-a-uuid".into()), "id")
            .unwrap_err();
        let msg = validation_message(&err);
        assert!(msg.contains("Invalid UUID format: 'not-a-uuid'"));
        assert!(msg.contains("id"));
    }

    #[test]
    fn test_uuid_type_mismatch_zh_cn() {
        setup_i18n("zh-CN");
        let field = FieldDefinition::new(FieldType::Uuid);
        let err = field
            .validate_with_field_name(&DataValue::Int(123), "id")
            .unwrap_err();
        let msg = validation_message(&err);
        assert!(msg.contains("字段类型不匹配，期望UUID字符串或UUID类型"));
        // format!("{:?}", value) on DataValue::Int(123) produces "123"
        assert!(msg.contains("123"));
        assert!(msg.contains("id"));
    }

    #[test]
    fn test_uuid_type_mismatch_en_us() {
        setup_i18n("en-US");
        let field = FieldDefinition::new(FieldType::Uuid);
        let err = field
            .validate_with_field_name(&DataValue::Int(123), "id")
            .unwrap_err();
        let msg = validation_message(&err);
        assert!(msg.contains("Type mismatch, expected UUID string or UUID type"));
        assert!(msg.contains("123"));
        assert!(msg.contains("id"));
    }

    // =========================================================================
    // Array type
    // =========================================================================

    #[test]
    fn test_array_max_items_zh_cn() {
        setup_i18n("zh-CN");
        let field = FieldDefinition::new(FieldType::Array {
            item_type: Box::new(FieldType::Integer {
                min_value: None,
                max_value: None,
            }),
            max_items: Some(2),
            min_items: None,
        });
        let arr = vec![
            DataValue::Int(1),
            DataValue::Int(2),
            DataValue::Int(3),
        ];
        let err = field
            .validate_with_field_name(&DataValue::Array(arr), "tags")
            .unwrap_err();
        assert_eq!(validation_message(&err), "数组元素数量不能超过2");
    }

    #[test]
    fn test_array_max_items_en_us() {
        setup_i18n("en-US");
        let field = FieldDefinition::new(FieldType::Array {
            item_type: Box::new(FieldType::Integer {
                min_value: None,
                max_value: None,
            }),
            max_items: Some(2),
            min_items: None,
        });
        let arr = vec![
            DataValue::Int(1),
            DataValue::Int(2),
            DataValue::Int(3),
        ];
        let err = field
            .validate_with_field_name(&DataValue::Array(arr), "tags")
            .unwrap_err();
        assert_eq!(validation_message(&err), "Array item count cannot exceed 2");
    }

    #[test]
    fn test_array_min_items_zh_cn() {
        setup_i18n("zh-CN");
        let field = FieldDefinition::new(FieldType::Array {
            item_type: Box::new(FieldType::Integer {
                min_value: None,
                max_value: None,
            }),
            max_items: None,
            min_items: Some(2),
        });
        let arr = vec![DataValue::Int(1)];
        let err = field
            .validate_with_field_name(&DataValue::Array(arr), "tags")
            .unwrap_err();
        assert_eq!(validation_message(&err), "数组元素数量不能少于2");
    }

    #[test]
    fn test_array_min_items_en_us() {
        setup_i18n("en-US");
        let field = FieldDefinition::new(FieldType::Array {
            item_type: Box::new(FieldType::Integer {
                min_value: None,
                max_value: None,
            }),
            max_items: None,
            min_items: Some(2),
        });
        let arr = vec![DataValue::Int(1)];
        let err = field
            .validate_with_field_name(&DataValue::Array(arr), "tags")
            .unwrap_err();
        assert_eq!(validation_message(&err), "Array item count cannot be less than 2");
    }

    #[test]
    fn test_array_json_not_array_zh_cn() {
        setup_i18n("zh-CN");
        let field = FieldDefinition::new(FieldType::Array {
            item_type: Box::new(FieldType::Integer {
                min_value: None,
                max_value: None,
            }),
            max_items: None,
            min_items: None,
        });
        // A valid JSON string but not an array (it is an object)
        let err = field
            .validate_with_field_name(&DataValue::String(r#"{"key": "value"}"#.into()), "tags")
            .unwrap_err();
        assert_eq!(
            validation_message(&err),
            "JSON字符串不是有效的数组格式"
        );
    }

    #[test]
    fn test_array_json_not_array_en_us() {
        setup_i18n("en-US");
        let field = FieldDefinition::new(FieldType::Array {
            item_type: Box::new(FieldType::Integer {
                min_value: None,
                max_value: None,
            }),
            max_items: None,
            min_items: None,
        });
        let err = field
            .validate_with_field_name(&DataValue::String(r#"{"key": "value"}"#.into()), "tags")
            .unwrap_err();
        assert_eq!(
            validation_message(&err),
            "JSON string is not a valid array format"
        );
    }

    #[test]
    fn test_array_json_parse_failed_zh_cn() {
        setup_i18n("zh-CN");
        let field = FieldDefinition::new(FieldType::Array {
            item_type: Box::new(FieldType::Integer {
                min_value: None,
                max_value: None,
            }),
            max_items: None,
            min_items: None,
        });
        let err = field
            .validate_with_field_name(&DataValue::String("not valid json {{".into()), "tags")
            .unwrap_err();
        assert_eq!(validation_message(&err), "无法解析JSON字符串");
    }

    #[test]
    fn test_array_json_parse_failed_en_us() {
        setup_i18n("en-US");
        let field = FieldDefinition::new(FieldType::Array {
            item_type: Box::new(FieldType::Integer {
                min_value: None,
                max_value: None,
            }),
            max_items: None,
            min_items: None,
        });
        let err = field
            .validate_with_field_name(&DataValue::String("not valid json {{".into()), "tags")
            .unwrap_err();
        assert_eq!(validation_message(&err), "Failed to parse JSON string");
    }

    #[test]
    fn test_array_type_mismatch_zh_cn() {
        setup_i18n("zh-CN");
        let field = FieldDefinition::new(FieldType::Array {
            item_type: Box::new(FieldType::Integer {
                min_value: None,
                max_value: None,
            }),
            max_items: None,
            min_items: None,
        });
        let err = field
            .validate_with_field_name(&DataValue::Int(42), "tags")
            .unwrap_err();
        assert_eq!(
            validation_message(&err),
            "字段类型不匹配，期望数组类型或JSON字符串"
        );
    }

    #[test]
    fn test_array_type_mismatch_en_us() {
        setup_i18n("en-US");
        let field = FieldDefinition::new(FieldType::Array {
            item_type: Box::new(FieldType::Integer {
                min_value: None,
                max_value: None,
            }),
            max_items: None,
            min_items: None,
        });
        let err = field
            .validate_with_field_name(&DataValue::Int(42), "tags")
            .unwrap_err();
        assert_eq!(
            validation_message(&err),
            "Type mismatch, expected array type or JSON string"
        );
    }

    // =========================================================================
    // Object type
    // =========================================================================

    #[test]
    fn test_object_type_mismatch_zh_cn() {
        setup_i18n("zh-CN");
        let field = FieldDefinition::new(FieldType::Object {
            fields: HashMap::new(),
        });
        let err = field
            .validate_with_field_name(&DataValue::String("not object".into()), "metadata")
            .unwrap_err();
        assert_eq!(
            validation_message(&err),
            "字段类型不匹配，期望对象类型"
        );
    }

    #[test]
    fn test_object_type_mismatch_en_us() {
        setup_i18n("en-US");
        let field = FieldDefinition::new(FieldType::Object {
            fields: HashMap::new(),
        });
        let err = field
            .validate_with_field_name(&DataValue::String("not object".into()), "metadata")
            .unwrap_err();
        assert_eq!(
            validation_message(&err),
            "Type mismatch, expected object type"
        );
    }

    // =========================================================================
    // Reference type
    // =========================================================================

    #[test]
    fn test_reference_type_mismatch_zh_cn() {
        setup_i18n("zh-CN");
        let field = FieldDefinition::new(FieldType::Reference {
            target_collection: "users".into(),
        });
        let err = field
            .validate_with_field_name(&DataValue::Int(42), "user_id")
            .unwrap_err();
        assert_eq!(validation_message(&err), "引用字段必须是字符串ID");
    }

    #[test]
    fn test_reference_type_mismatch_en_us() {
        setup_i18n("en-US");
        let field = FieldDefinition::new(FieldType::Reference {
            target_collection: "users".into(),
        });
        let err = field
            .validate_with_field_name(&DataValue::Int(42), "user_id")
            .unwrap_err();
        assert_eq!(validation_message(&err), "Reference field must be a string ID");
    }

    // =========================================================================
    // BigInteger type
    // =========================================================================

    #[test]
    fn test_biginteger_type_mismatch_zh_cn() {
        setup_i18n("zh-CN");
        let field = FieldDefinition::new(FieldType::BigInteger);
        let err = field
            .validate_with_field_name(&DataValue::String("not int".into()), "count")
            .unwrap_err();
        assert_eq!(
            validation_message(&err),
            "字段类型不匹配，期望大整数类型"
        );
    }

    #[test]
    fn test_biginteger_type_mismatch_en_us() {
        setup_i18n("en-US");
        let field = FieldDefinition::new(FieldType::BigInteger);
        let err = field
            .validate_with_field_name(&DataValue::String("not int".into()), "count")
            .unwrap_err();
        assert_eq!(
            validation_message(&err),
            "Type mismatch, expected big integer type"
        );
    }

    // =========================================================================
    // Double type
    // =========================================================================

    #[test]
    fn test_double_type_mismatch_zh_cn() {
        setup_i18n("zh-CN");
        let field = FieldDefinition::new(FieldType::Double);
        let err = field
            .validate_with_field_name(&DataValue::Int(42), "price")
            .unwrap_err();
        assert_eq!(
            validation_message(&err),
            "字段类型不匹配，期望双精度浮点数类型"
        );
    }

    #[test]
    fn test_double_type_mismatch_en_us() {
        setup_i18n("en-US");
        let field = FieldDefinition::new(FieldType::Double);
        let err = field
            .validate_with_field_name(&DataValue::Int(42), "price")
            .unwrap_err();
        assert_eq!(
            validation_message(&err),
            "Type mismatch, expected double type"
        );
    }

    // =========================================================================
    // Text type
    // =========================================================================

    #[test]
    fn test_text_type_mismatch_zh_cn() {
        setup_i18n("zh-CN");
        let field = FieldDefinition::new(FieldType::Text);
        let err = field
            .validate_with_field_name(&DataValue::Int(42), "body")
            .unwrap_err();
        assert_eq!(
            validation_message(&err),
            "字段类型不匹配，期望文本类型"
        );
    }

    #[test]
    fn test_text_type_mismatch_en_us() {
        setup_i18n("en-US");
        let field = FieldDefinition::new(FieldType::Text);
        let err = field
            .validate_with_field_name(&DataValue::Int(42), "body")
            .unwrap_err();
        assert_eq!(
            validation_message(&err),
            "Type mismatch, expected text type"
        );
    }

    // =========================================================================
    // Date type
    // =========================================================================

    #[test]
    fn test_date_type_mismatch_zh_cn() {
        setup_i18n("zh-CN");
        let field = FieldDefinition::new(FieldType::Date);
        let err = field
            .validate_with_field_name(&DataValue::String("2024-01-01".into()), "birthday")
            .unwrap_err();
        assert_eq!(
            validation_message(&err),
            "字段类型不匹配，期望日期类型"
        );
    }

    #[test]
    fn test_date_type_mismatch_en_us() {
        setup_i18n("en-US");
        let field = FieldDefinition::new(FieldType::Date);
        let err = field
            .validate_with_field_name(&DataValue::String("2024-01-01".into()), "birthday")
            .unwrap_err();
        assert_eq!(
            validation_message(&err),
            "Type mismatch, expected date type"
        );
    }

    // =========================================================================
    // Time type
    // =========================================================================

    #[test]
    fn test_time_type_mismatch_zh_cn() {
        setup_i18n("zh-CN");
        let field = FieldDefinition::new(FieldType::Time);
        let err = field
            .validate_with_field_name(&DataValue::String("12:00:00".into()), "alarm")
            .unwrap_err();
        assert_eq!(
            validation_message(&err),
            "字段类型不匹配，期望时间类型"
        );
    }

    #[test]
    fn test_time_type_mismatch_en_us() {
        setup_i18n("en-US");
        let field = FieldDefinition::new(FieldType::Time);
        let err = field
            .validate_with_field_name(&DataValue::String("12:00:00".into()), "alarm")
            .unwrap_err();
        assert_eq!(
            validation_message(&err),
            "Type mismatch, expected time type"
        );
    }

    // =========================================================================
    // Binary type
    // =========================================================================

    #[test]
    fn test_binary_type_mismatch_zh_cn() {
        setup_i18n("zh-CN");
        let field = FieldDefinition::new(FieldType::Binary);
        let err = field
            .validate_with_field_name(&DataValue::Int(42), "avatar")
            .unwrap_err();
        assert_eq!(
            validation_message(&err),
            "字段类型不匹配，期望二进制数据（Base64字符串）"
        );
    }

    #[test]
    fn test_binary_type_mismatch_en_us() {
        setup_i18n("en-US");
        let field = FieldDefinition::new(FieldType::Binary);
        let err = field
            .validate_with_field_name(&DataValue::Int(42), "avatar")
            .unwrap_err();
        assert_eq!(
            validation_message(&err),
            "Type mismatch, expected binary data (Base64 string)"
        );
    }

    // =========================================================================
    // Decimal type
    // =========================================================================

    #[test]
    fn test_decimal_type_mismatch_zh_cn() {
        setup_i18n("zh-CN");
        let field = FieldDefinition::new(FieldType::Decimal {
            precision: 10,
            scale: 2,
        });
        let err = field
            .validate_with_field_name(&DataValue::String("not decimal".into()), "amount")
            .unwrap_err();
        assert_eq!(
            validation_message(&err),
            "字段类型不匹配，期望十进制数类型"
        );
    }

    #[test]
    fn test_decimal_type_mismatch_en_us() {
        setup_i18n("en-US");
        let field = FieldDefinition::new(FieldType::Decimal {
            precision: 10,
            scale: 2,
        });
        let err = field
            .validate_with_field_name(&DataValue::String("not decimal".into()), "amount")
            .unwrap_err();
        assert_eq!(
            validation_message(&err),
            "Type mismatch, expected decimal type"
        );
    }

    // =========================================================================
    // ja-JP representative tests (5 keys)
    // =========================================================================

    #[test]
    fn test_ja_jp_required_empty() {
        setup_i18n("ja-JP");
        let field = FieldDefinition::new(FieldType::String {
            max_length: None,
            min_length: None,
            regex: None,
        })
        .required();
        let err = field
            .validate_with_field_name(&DataValue::Null, "username")
            .unwrap_err();
        assert_eq!(
            validation_message(&err),
            "必須フィールドは空にできません"
        );
    }

    #[test]
    fn test_ja_jp_string_max_length() {
        setup_i18n("ja-JP");
        let field = FieldDefinition::new(FieldType::String {
            max_length: Some(5),
            min_length: None,
            regex: None,
        });
        let err = field
            .validate_with_field_name(&DataValue::String("hello world".into()), "name")
            .unwrap_err();
        assert_eq!(
            validation_message(&err),
            "文字列の長さは5を超えることはできません"
        );
    }

    #[test]
    fn test_ja_jp_integer_type_mismatch() {
        setup_i18n("ja-JP");
        let field = FieldDefinition::new(FieldType::Integer {
            min_value: None,
            max_value: None,
        });
        let err = field
            .validate_with_field_name(&DataValue::String("not an int".into()), "age")
            .unwrap_err();
        assert_eq!(
            validation_message(&err),
            "フィールドタイプが一致しません、整数型を期待"
        );
    }

    #[test]
    fn test_ja_jp_array_max_items() {
        setup_i18n("ja-JP");
        let field = FieldDefinition::new(FieldType::Array {
            item_type: Box::new(FieldType::Integer {
                min_value: None,
                max_value: None,
            }),
            max_items: Some(2),
            min_items: None,
        });
        let arr = vec![
            DataValue::Int(1),
            DataValue::Int(2),
            DataValue::Int(3),
        ];
        let err = field
            .validate_with_field_name(&DataValue::Array(arr), "tags")
            .unwrap_err();
        assert_eq!(
            validation_message(&err),
            "配列要素数は2を超えることはできません"
        );
    }

    #[test]
    fn test_ja_jp_uuid_invalid() {
        setup_i18n("ja-JP");
        let field = FieldDefinition::new(FieldType::Uuid);
        let err = field
            .validate_with_field_name(&DataValue::String("not-a-uuid".into()), "id")
            .unwrap_err();
        let msg = validation_message(&err);
        assert!(msg.contains("無効なUUID形式: 'not-a-uuid'"));
        assert!(msg.contains("id"));
    }

    // =========================================================================
    // Edge cases: non-required Null passes, valid values pass
    // =========================================================================

    #[test]
    fn test_non_required_null_passes() {
        setup_i18n("zh-CN");
        let field = FieldDefinition::new(FieldType::String {
            max_length: Some(5),
            min_length: None,
            regex: None,
        });
        // Not required, Null is OK
        assert!(field.validate_with_field_name(&DataValue::Null, "name").is_ok());
    }

    #[test]
    fn test_valid_string_within_bounds() {
        setup_i18n("zh-CN");
        let field = FieldDefinition::new(FieldType::String {
            max_length: Some(10),
            min_length: Some(1),
            regex: None,
        });
        assert!(field
            .validate_with_field_name(&DataValue::String("hello".into()), "name")
            .is_ok());
    }

    #[test]
    fn test_valid_integer_within_bounds() {
        setup_i18n("zh-CN");
        let field = FieldDefinition::new(FieldType::Integer {
            min_value: Some(0),
            max_value: Some(100),
        });
        assert!(field
            .validate_with_field_name(&DataValue::Int(50), "age")
            .is_ok());
    }

    #[test]
    fn test_json_type_accepts_anything() {
        setup_i18n("zh-CN");
        let field = FieldDefinition::new(FieldType::Json);
        assert!(field
            .validate_with_field_name(&DataValue::Int(42), "data")
            .is_ok());
    }

    #[test]
    fn test_uuid_empty_string_passes() {
        setup_i18n("zh-CN");
        let field = FieldDefinition::new(FieldType::Uuid);
        assert!(field
            .validate_with_field_name(&DataValue::String("".into()), "id")
            .is_ok());
    }
}

/// 验证时区偏移格式是否有效
///
/// 有效格式：+00:00, +08:00, -05:00 等
fn is_valid_timezone_offset(offset: &str) -> bool {
    // 正则表达式匹配时区偏移格式
    // 格式：+或-，后跟两位数的小时，冒号，两位数的分钟
    use regex::Regex;

    if let Ok(re) = Regex::new(r"^[+-]\d{2}:\d{2}$") {
        if !re.is_match(offset) {
            return false;
        }

        // 解析小时和分钟，验证范围
        let parts: Vec<&str> = offset[1..].split(':').collect();
        if parts.len() != 2 {
            return false;
        }

        if let (Ok(hours), Ok(minutes)) = (parts[0].parse::<i32>(), parts[1].parse::<i32>()) {
            // 小时范围：0-23，分钟范围：0-59
            if hours > 23 || minutes > 59 {
                return false;
            }
        } else {
            return false;
        }

        true
    } else {
        false
    }
}
