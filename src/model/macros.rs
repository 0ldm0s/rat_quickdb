//! 模型相关的宏定义
//!
//! 提供便捷的宏来定义模型和字段类型

/// 便捷宏：定义模型字段类型
#[macro_export]
macro_rules! field_types {
    (string) => {
        $crate::model::field_types::FieldType::String {
            max_length: None,
            min_length: None,
            regex: None,
        }
    };
    (string, max_length = $max:expr) => {
        $crate::model::field_types::FieldType::String {
            max_length: Some($max),
            min_length: None,
            regex: None,
        }
    };
    (string, min_length = $min:expr) => {
        $crate::model::field_types::FieldType::String {
            max_length: None,
            min_length: Some($min),
            regex: None,
        }
    };
    (string, max_length = $max:expr, min_length = $min:expr) => {
        $crate::model::field_types::FieldType::String {
            max_length: Some($max),
            min_length: Some($min),
            regex: None,
        }
    };
    (integer) => {
        $crate::model::field_types::FieldType::Integer {
            min_value: None,
            max_value: None,
        }
    };
    (integer, min = $min:expr) => {
        $crate::model::field_types::FieldType::Integer {
            min_value: Some($min),
            max_value: None,
        }
    };
    (integer, max = $max:expr) => {
        $crate::model::field_types::FieldType::Integer {
            min_value: None,
            max_value: Some($max),
        }
    };
    (integer, min = $min:expr, max = $max:expr) => {
        $crate::model::field_types::FieldType::Integer {
            min_value: Some($min),
            max_value: Some($max),
        }
    };
    (float) => {
        $crate::model::field_types::FieldType::Float {
            min_value: None,
            max_value: None,
        }
    };
    (boolean) => {
        $crate::model::field_types::FieldType::Boolean
    };
    (datetime) => {
        $crate::model::field_types::FieldType::DateTime
    };
    (uuid) => {
        $crate::model::field_types::FieldType::Uuid
    };
    (json) => {
        $crate::model::field_types::FieldType::Json
    };
    (array, $item_type:expr) => {
        $crate::model::field_types::FieldType::Array {
            item_type: Box::new($item_type),
            max_items: None,
            min_items: None,
        }
    };
    (reference, $target:expr) => {
        $crate::model::field_types::FieldType::Reference {
            target_collection: $target.to_string(),
        }
    };
}

/// 便捷宏：定义模型
#[macro_export]
macro_rules! define_model {
    (
        $(#[$meta:meta])*
        struct $name:ident {
            $(
                $(#[$field_meta:meta])*
                $field:ident: $field_type:ty,
            )*
        }

        collection = $collection:expr,
        $(
            database = $database:expr,
        )?
        fields = {
            $(
                $field_name:ident: $field_def:expr,
            )*
        }
        $(
            indexes = [
                $(
                    { fields: [$($index_field:expr),*], unique: $unique:expr $(, name: $index_name:expr)? },
                )*
            ],
        )?
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        pub struct $name {
            $(
                $(#[$field_meta])*
                pub $field: $field_type,
            )*
        }

        impl $crate::model::traits::Model for $name {
            fn meta() -> $crate::model::field_types::ModelMeta {
                let mut fields = std::collections::HashMap::new();
                $(
                    fields.insert(stringify!($field_name).to_string(), $field_def);
                )*

                let mut indexes = Vec::new();
                $(
                    $(
                        indexes.push($crate::model::field_types::IndexDefinition {
                            fields: vec![$($index_field.to_string()),*],
                            unique: $unique,
                            name: None $(.or(Some($index_name.to_string())))?,
                        });
                    )*
                )?

                let model_meta = $crate::model::field_types::ModelMeta {
                    collection_name: $collection.to_string(),
                    database_alias: None $(.or(Some($database.to_string())))?,
                    fields,
                    indexes,
                    description: None,
                };

                // 自动注册模型元数据（仅在首次调用时注册）
                static ONCE: std::sync::Once = std::sync::Once::new();
                ONCE.call_once(|| {
                    if let Err(e) = $crate::manager::register_model(model_meta.clone()) {
                        panic!("❌ 模型注册失败: {}", e);
                    } else {
                        $crate::debug_log!("✅ 模型自动注册成功: {}", model_meta.collection_name);
                    }
                });

                model_meta
            }

            /// 高性能直接转换实现，避免 JSON 序列化开销
            fn to_data_map_direct(&self) -> $crate::error::QuickDbResult<std::collections::HashMap<String, $crate::types::DataValue>> {
                use $crate::model::conversion::ToDataValue;
                let mut data_map = std::collections::HashMap::new();

                $crate::debug_log!("🔍 开始 to_data_map_direct 转换...");

                // 获取字段元数据，用于智能转换
                let meta = Self::meta();

                $(
                    $crate::debug_log!("🔍 转换字段 {}: {:?}", stringify!($field), self.$field);

                    // 根据字段类型进行智能转换
                    let field_name = stringify!($field).to_string();
                    let field_def = meta.fields.get(&field_name);

                    let data_value = if let Some(field_type) = field_def.map(|f| &f.field_type) {
                        // 有字段类型定义，进行元数据感知的转换
                        match field_type {
                            $crate::model::field_types::FieldType::DateTimeWithTz { timezone_offset } => {
                                // 获取数据库别名，如果为None则是严重框架错误，立即panic
                                let alias = Self::database_alias().expect("严重错误：模型没有数据库别名！这表明框架内部存在严重问题！");
                                let db_type = $crate::manager::get_database_type_by_alias(&alias);

                                // 使用数据库感知的转换函数
                                $crate::convert_datetime_with_tz_aware(&self.$field, timezone_offset, db_type)?
                            },
                            _ => {
                                // 其他字段类型使用默认转换
                                self.$field.to_data_value()
                            }
                        }
                    } else {
                        // 没有字段类型定义，使用默认转换
                        self.$field.to_data_value()
                    };

                    $crate::debug_log!("🔍 字段 {} 转换为: {:?}", stringify!($field), data_value);
                    data_map.insert(field_name, data_value);
                )*

                // 移除为None的id字段，让数据库自动生成ID
                if let Some(id_value) = data_map.get("id") {
                    if matches!(id_value, $crate::types::DataValue::Null) {
                        data_map.remove("id");
                    }
                }

                // 移除为None的_id字段，让MongoDB自动生成
                if let Some(id_value) = data_map.get("_id") {
                    if matches!(id_value, $crate::types::DataValue::Null) {
                        data_map.remove("_id");
                    }
                }

                $crate::debug_log!("🔍 to_data_map_direct 转换完成");
                Ok(data_map)
            }
        }

        impl $name {
            /// 保存模型到数据库
            pub async fn save(&self) -> $crate::error::QuickDbResult<String> {
                self.validate()?;
                let data = self.to_data_map()?;
                let collection_name = Self::collection_name();
                let database_alias = Self::database_alias();

                // 确保表和索引存在（静默处理，这是预期行为）
                let alias = database_alias.as_deref().unwrap_or("default");
                let _ = $crate::manager::ensure_table_and_indexes(&collection_name, alias).await;

                // 调用ODM创建记录

                let result = $crate::odm::create(
                    &collection_name,
                    data,
                    database_alias.as_deref(),
                ).await?;

  
                // 将 DataValue 转换为 String（通常是 ID）
                match result {
                    $crate::types::DataValue::String(id) => Ok(id),
                    $crate::types::DataValue::Int(id) => Ok(id.to_string()),
                    $crate::types::DataValue::Uuid(id) => Ok(id.to_string()),
                    $crate::types::DataValue::Object(obj) => {
                        // 如果返回的是对象，尝试提取_id字段（MongoDB）或id字段（SQL）
                        if let Some(id_value) = obj.get("_id").or_else(|| obj.get("id")) {
                            match id_value {
                                $crate::types::DataValue::String(id) => Ok(id.clone()),
                                $crate::types::DataValue::Int(id) => Ok(id.to_string()),
                                $crate::types::DataValue::Uuid(id) => Ok(id.to_string()),
                                _ => Ok(format!("{:?}", id_value))
                            }
                        } else {
                            // 如果对象中没有id字段，序列化整个对象
                            match serde_json::to_string(&obj) {
                                Ok(json_str) => Ok(json_str),
                                Err(_) => Ok(format!("{:?}", obj))
                            }
                        }
                    },
                    other => {
                        // 如果返回的不是简单的 ID 类型，尝试序列化为 JSON
                        match serde_json::to_string(&other) {
                            Ok(json_str) => Ok(json_str),
                            Err(_) => Ok(format!("{:?}", other))
                        }
                    }
                }
            }

            /// 更新模型
            pub async fn update(&self, updates: std::collections::HashMap<String, $crate::types::DataValue>) -> $crate::error::QuickDbResult<bool> {
                // 尝试从模型中获取ID字段，兼容 MongoDB 的 _id 和 SQL 的 id
                let data_map = self.to_data_map()?;
                let (id_field_name, id_value) = data_map.get("_id")
                    .map(|v| ("_id", v))
                    .or_else(|| data_map.get("id").map(|v| ("id", v)))
                    .ok_or_else(|| $crate::error::QuickDbError::ValidationError {
                        field: "id".to_string(),
                        message: "模型缺少ID字段（id 或 _id），无法更新".to_string()
                    })?;

                // 将ID转换为字符串
                let id_str = match id_value {
                    $crate::types::DataValue::String(s) => s.clone(),
                    $crate::types::DataValue::Int(i) => i.to_string(),
                    $crate::types::DataValue::Uuid(u) => u.to_string(),
                    // MongoDB 的 ObjectId 可能存储在 Object 中
                    $crate::types::DataValue::Object(obj) => {
                        if let Some($crate::types::DataValue::String(oid)) = obj.get("$oid") {
                            oid.clone()
                        } else {
                            return Err($crate::error::QuickDbError::ValidationError {
                                field: id_field_name.to_string(),
                                message: format!("不支持的MongoDB ObjectId格式: {:?}", obj)
                            });
                        }
                    }
                    _ => return Err($crate::error::QuickDbError::ValidationError {
                        field: id_field_name.to_string(),
                        message: format!("不支持的ID类型: {:?}", id_value)
                    })
                };

                let collection_name = Self::collection_name();
                let database_alias = Self::database_alias();

                $crate::odm::update_by_id(&collection_name, &id_str, updates, database_alias.as_deref()).await
            }

            /// 删除模型
            pub async fn delete(&self) -> $crate::error::QuickDbResult<bool> {
                // 尝试从模型中获取ID字段，兼容 MongoDB 的 _id 和 SQL 的 id
                let data_map = self.to_data_map()?;
                let (id_field_name, id_value) = data_map.get("_id")
                    .map(|v| ("_id", v))
                    .or_else(|| data_map.get("id").map(|v| ("id", v)))
                    .ok_or_else(|| $crate::error::QuickDbError::ValidationError {
                        field: "id".to_string(),
                        message: "模型缺少ID字段（id 或 _id），无法删除".to_string()
                    })?;

                // 将ID转换为字符串
                let id_str = match id_value {
                    $crate::types::DataValue::String(s) => s.clone(),
                    $crate::types::DataValue::Int(i) => i.to_string(),
                    $crate::types::DataValue::Uuid(u) => u.to_string(),
                    // MongoDB 的 ObjectId 可能存储在 Object 中
                    $crate::types::DataValue::Object(obj) => {
                        if let Some($crate::types::DataValue::String(oid)) = obj.get("$oid") {
                            oid.clone()
                        } else {
                            return Err($crate::error::QuickDbError::ValidationError {
                                field: id_field_name.to_string(),
                                message: format!("不支持的MongoDB ObjectId格式: {:?}", obj)
                            });
                        }
                    }
                    _ => return Err($crate::error::QuickDbError::ValidationError {
                        field: id_field_name.to_string(),
                        message: format!("不支持的ID类型: {:?}", id_value)
                    })
                };

                let collection_name = Self::collection_name();
                let database_alias = Self::database_alias();

                $crate::odm::delete_by_id(&collection_name, &id_str, database_alias.as_deref()).await
            }

            /// 批量更新模型（静态方法）
            ///
            /// 根据条件批量更新多条记录，返回受影响的行数
            pub async fn update_many(conditions: Vec<$crate::types::QueryCondition>, updates: std::collections::HashMap<String, $crate::types::DataValue>) -> $crate::error::QuickDbResult<u64> {
                let collection_name = Self::collection_name();
                let database_alias = Self::database_alias();

                $crate::odm::update(
                    &collection_name,
                    conditions,
                    updates,
                    database_alias.as_deref(),
                ).await
            }

            /// 使用操作数组批量更新模型（静态方法）
            ///
            /// 根据条件使用操作数组批量更新多条记录，支持原子性增减操作，返回受影响的行数
            pub async fn update_many_with_operations(conditions: Vec<$crate::types::QueryCondition>, operations: Vec<$crate::types::UpdateOperation>) -> $crate::error::QuickDbResult<u64> {
                let collection_name = Self::collection_name();
                let database_alias = Self::database_alias();

                $crate::odm::update_with_operations(
                    &collection_name,
                    conditions,
                    operations,
                    database_alias.as_deref(),
                ).await
            }

            /// 批量删除模型（静态方法）
            ///
            /// 根据条件批量删除多条记录，返回受影响的行数
            pub async fn delete_many(conditions: Vec<$crate::types::QueryCondition>) -> $crate::error::QuickDbResult<u64> {
                let collection_name = Self::collection_name();
                let database_alias = Self::database_alias();

                $crate::odm::delete(
                    &collection_name,
                    conditions,
                    database_alias.as_deref(),
                ).await
            }
        }
    };
}