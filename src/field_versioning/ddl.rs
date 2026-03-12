//! DDL 生成器

use crate::model::field_types::{FieldDefinition, FieldType, IndexDefinition, ModelMeta};
use crate::types::DatabaseType;

/// 生成模型 DDL
pub fn generate_ddl(
    model: &ModelMeta,
    db_type: DatabaseType,
) -> String {
    match db_type {
        DatabaseType::SQLite => generate_sqlite_ddl(model),
        DatabaseType::PostgreSQL => generate_postgres_ddl(model),
        DatabaseType::MySQL => generate_mysql_ddl(model),
        DatabaseType::MongoDB => generate_mongodb_ddl(model),
    }
}

/// 生成 SQLite DDL
fn generate_sqlite_ddl(model: &ModelMeta) -> String {
    let mut ddl = format!("CREATE TABLE IF NOT EXISTS {} (\n", model.collection_name);

    let fields: Vec<_> = model.fields.iter().collect();
    for (i, (name, def)) in fields.iter().enumerate() {
        ddl.push_str(&format!("    {} {}", name, field_type_to_sqlite(def)));

        if def.required {
            ddl.push_str(" NOT NULL");
        }

        if def.unique {
            ddl.push_str(" UNIQUE");
        }

        if i < fields.len() - 1 {
            ddl.push(',');
        }
        ddl.push('\n');
    }

    // 添加主键
    if let Some(id_field) = model.fields.get("id") {
        if matches!(id_field.field_type, FieldType::String { .. }) {
            ddl.push_str("    ,PRIMARY KEY (id)\n");
        }
    }

    ddl.push_str(");\n");

    // 添加索引
    for index in &model.indexes {
        let unique_str = if index.unique { " UNIQUE" } else { "" };
        let default_name = format!("idx_{}_{}", model.collection_name, index.fields.join("_"));
        let index_name = index.name.as_ref().unwrap_or(&default_name);
        ddl.push_str(&format!(
            "CREATE{} INDEX IF NOT EXISTS {} ON {} ({});\n",
            unique_str,
            index_name,
            model.collection_name,
            index.fields.join(", ")
        ));
    }

    ddl
}

/// 生成 PostgreSQL DDL
fn generate_postgres_ddl(model: &ModelMeta) -> String {
    let mut ddl = format!("CREATE TABLE IF NOT EXISTS {} (\n", model.collection_name);

    let fields: Vec<_> = model.fields.iter().collect();
    for (i, (name, def)) in fields.iter().enumerate() {
        ddl.push_str(&format!("    {} {}", name, field_type_to_postgres(def)));

        if def.required {
            ddl.push_str(" NOT NULL");
        }

        if def.unique {
            ddl.push_str(" UNIQUE");
        }

        if i < fields.len() - 1 {
            ddl.push(',');
        }
        ddl.push('\n');
    }

    ddl.push_str(");\n");

    // 添加索引
    for index in &model.indexes {
        let unique_str = if index.unique { " UNIQUE" } else { "" };
        let default_name = format!("idx_{}_{}", model.collection_name, index.fields.join("_"));
        let index_name = index.name.as_ref().unwrap_or(&default_name);
        ddl.push_str(&format!(
            "CREATE{} INDEX IF NOT EXISTS {} ON {} ({});\n",
            unique_str,
            index_name,
            model.collection_name,
            index.fields.join(", ")
        ));
    }

    ddl
}

/// 生成 MySQL DDL
fn generate_mysql_ddl(model: &ModelMeta) -> String {
    let mut ddl = format!("CREATE TABLE IF NOT EXISTS {} (\n", model.collection_name);

    let fields: Vec<_> = model.fields.iter().collect();
    for (i, (name, def)) in fields.iter().enumerate() {
        ddl.push_str(&format!("    {} {}", name, field_type_to_mysql(def)));

        if def.required {
            ddl.push_str(" NOT NULL");
        }

        if def.unique {
            ddl.push_str(" UNIQUE");
        }

        if i < fields.len() - 1 {
            ddl.push(',');
        }
        ddl.push('\n');
    }

    ddl.push_str(") ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;\n");

    // 添加索引
    for index in &model.indexes {
        let unique_str = if index.unique { " UNIQUE" } else { "" };
        let default_name = format!("idx_{}_{}", model.collection_name, index.fields.join("_"));
        let index_name = index.name.as_ref().unwrap_or(&default_name);
        ddl.push_str(&format!(
            "CREATE{} INDEX {} ON {} ({});\n",
            unique_str,
            index_name,
            model.collection_name,
            index.fields.join(", ")
        ));
    }

    ddl
}

/// 生成 MongoDB DDL（集合创建）
fn generate_mongodb_ddl(model: &ModelMeta) -> String {
    let mut ddl = format!("// MongoDB 集合: {}\n", model.collection_name);
    ddl.push_str("// MongoDB 使用灵活的 schema，不需要预定义结构\n");
    ddl.push_str("// 以下是建议的索引定义：\n\n");

    for index in &model.indexes {
        let unique_str = if index.unique { ", unique: true" } else { "" };
        ddl.push_str(&format!(
            "db.{}.createIndex({{ {}: 1 {} }});\n",
            model.collection_name,
            index.fields.join(": 1, "),
            unique_str
        ));
    }

    ddl
}

/// 将字段类型转换为 SQLite 类型
fn field_type_to_sqlite(def: &FieldDefinition) -> String {
    match &def.field_type {
        FieldType::String { max_length, .. } => {
            if let Some(len) = max_length {
                format!("VARCHAR({})", len)
            } else {
                "TEXT".to_string()
            }
        }
        FieldType::Integer { .. } => "INTEGER".to_string(),
        FieldType::BigInteger => "BIGINT".to_string(),
        FieldType::Float { .. } => "REAL".to_string(),
        FieldType::Double => "DOUBLE".to_string(),
        FieldType::Text => "TEXT".to_string(),
        FieldType::Boolean => "INTEGER".to_string(), // SQLite 用整数存储布尔值
        FieldType::DateTime => "TEXT".to_string(),
        FieldType::DateTimeWithTz { .. } => "TEXT".to_string(),
        FieldType::Date => "TEXT".to_string(),
        FieldType::Time => "TEXT".to_string(),
        FieldType::Uuid => "TEXT".to_string(),
        FieldType::Json => "TEXT".to_string(),
        FieldType::Binary => "BLOB".to_string(),
        FieldType::Decimal { precision, scale } => {
            format!("DECIMAL({},{})", precision, scale)
        }
        FieldType::Array { .. } => "TEXT".to_string(), // SQLite 用 JSON 文本存储数组
        FieldType::Object { .. } => "TEXT".to_string(), // SQLite 用 JSON 文本存储对象
        FieldType::Reference { .. } => "TEXT".to_string(),
    }
}

/// 将字段类型转换为 PostgreSQL 类型
fn field_type_to_postgres(def: &FieldDefinition) -> String {
    match &def.field_type {
        FieldType::String { max_length, .. } => {
            if let Some(len) = max_length {
                format!("VARCHAR({})", len)
            } else {
                "TEXT".to_string()
            }
        }
        FieldType::Integer { .. } => "INTEGER".to_string(),
        FieldType::BigInteger => "BIGINT".to_string(),
        FieldType::Float { .. } => "DOUBLE PRECISION".to_string(),
        FieldType::Double => "DOUBLE PRECISION".to_string(),
        FieldType::Text => "TEXT".to_string(),
        FieldType::Boolean => "BOOLEAN".to_string(),
        FieldType::DateTime => "TIMESTAMP".to_string(),
        FieldType::DateTimeWithTz { .. } => "TIMESTAMP WITH TIME ZONE".to_string(),
        FieldType::Date => "DATE".to_string(),
        FieldType::Time => "TIME".to_string(),
        FieldType::Uuid => "UUID".to_string(),
        FieldType::Json => "JSONB".to_string(),
        FieldType::Binary => "BYTEA".to_string(),
        FieldType::Decimal { precision, scale } => {
            format!("DECIMAL({},{})", precision, scale)
        }
        FieldType::Array { item_type, .. } => {
            format!("{}[]", boxed_type_to_postgres(item_type))
        }
        FieldType::Object { .. } => "JSONB".to_string(),
        FieldType::Reference { target_collection } => {
            format!("VARCHAR(255)  -- 引用: {}", target_collection)
        }
    }
}

/// 将字段类型转换为 MySQL 类型
fn field_type_to_mysql(def: &FieldDefinition) -> String {
    match &def.field_type {
        FieldType::String { max_length, .. } => {
            if let Some(len) = max_length {
                format!("VARCHAR({})", len)
            } else {
                "TEXT".to_string()
            }
        }
        FieldType::Integer { .. } => "INT".to_string(),
        FieldType::BigInteger => "BIGINT".to_string(),
        FieldType::Float { .. } => "DOUBLE".to_string(),
        FieldType::Double => "DOUBLE".to_string(),
        FieldType::Text => "TEXT".to_string(),
        FieldType::Boolean => "TINYINT(1)".to_string(),
        FieldType::DateTime => "DATETIME".to_string(),
        FieldType::DateTimeWithTz { .. } => "DATETIME".to_string(),
        FieldType::Date => "DATE".to_string(),
        FieldType::Time => "TIME".to_string(),
        FieldType::Uuid => "CHAR(36)".to_string(),
        FieldType::Json => "JSON".to_string(),
        FieldType::Binary => "BLOB".to_string(),
        FieldType::Decimal { precision, scale } => {
            format!("DECIMAL({},{})", precision, scale)
        }
        FieldType::Array { .. } => "JSON".to_string(),
        FieldType::Object { .. } => "JSON".to_string(),
        FieldType::Reference { .. } => "VARCHAR(255)".to_string(),
    }
}

/// 将嵌套字段类型转换为 PostgreSQL 类型
fn boxed_type_to_postgres(field_type: &FieldType) -> String {
    match field_type {
        FieldType::String { max_length, .. } => {
            if let Some(len) = max_length {
                format!("VARCHAR({})", len)
            } else {
                "TEXT".to_string()
            }
        }
        FieldType::Integer { .. } => "INTEGER".to_string(),
        FieldType::BigInteger => "BIGINT".to_string(),
        FieldType::Float { .. } => "DOUBLE PRECISION".to_string(),
        FieldType::Double => "DOUBLE PRECISION".to_string(),
        FieldType::Boolean => "BOOLEAN".to_string(),
        FieldType::DateTime => "TIMESTAMP".to_string(),
        FieldType::Uuid => "UUID".to_string(),
        _ => "TEXT".to_string(),
    }
}

/// 计算两个模型之间的差异 DDL（从旧版本升级到新版本）
pub fn generate_diff_ddl(
    old_model: &ModelMeta,
    new_model: &ModelMeta,
    db_type: DatabaseType,
) -> String {
    let mut ddl = String::new();

    // 找出新增的字段
    for (name, def) in &new_model.fields {
        if !old_model.fields.contains_key(name) {
            let type_str = match db_type {
                DatabaseType::SQLite => field_type_to_sqlite(def),
                DatabaseType::PostgreSQL => field_type_to_postgres(def),
                DatabaseType::MySQL => field_type_to_mysql(def),
                DatabaseType::MongoDB => "N/A".to_string(),
            };
            ddl.push_str(&format!(
                "-- 新增字段: {}\nALTER TABLE {} ADD COLUMN {} {};\n",
                name,
                new_model.collection_name,
                name,
                type_str
            ));
        }
    }

    ddl
}

/// 生成降级 DDL（从新版本回到旧版本）
pub fn generate_downgrade_ddl(
    old_model: &ModelMeta,
    new_model: &ModelMeta,
    _db_type: DatabaseType,
) -> String {
    let mut ddl = String::new();

    // 找出新增的字段（在降级时需要删除）
    for name in new_model.fields.keys() {
        if !old_model.fields.contains_key(name) {
            ddl.push_str(&format!(
                "-- 删除字段: {}\nALTER TABLE {} DROP COLUMN {};\n",
                name,
                new_model.collection_name,
                name
            ));
        }
    }

    ddl
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_generate_sqlite_ddl() {
        let mut fields = HashMap::new();
        fields.insert(
            "id".to_string(),
            FieldDefinition {
                field_type: FieldType::String {
                    max_length: Some(36),
                    min_length: None,
                    regex: None,
                },
                required: true,
                unique: true,
                default: None,
                description: None,
                indexed: false,
                validator: None,
                sqlite_compatibility: false,
            },
        );
        fields.insert(
            "name".to_string(),
            FieldDefinition {
                field_type: FieldType::String {
                    max_length: Some(100),
                    min_length: None,
                    regex: None,
                },
                required: true,
                unique: false,
                default: None,
                description: None,
                indexed: false,
                validator: None,
                sqlite_compatibility: false,
            },
        );

        let model = ModelMeta {
            collection_name: "users".to_string(),
            database_alias: Some("default".to_string()),
            fields,
            indexes: vec![],
            description: None,
            version: Some(1),
        };

        let ddl = generate_sqlite_ddl(&model);
        assert!(ddl.contains("CREATE TABLE"));
        assert!(ddl.contains("VARCHAR(36)"));
        assert!(ddl.contains("VARCHAR(100)"));
    }
}
