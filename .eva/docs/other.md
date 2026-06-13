# rat_quickdb 其他模块文档

## 存储过程

位置：`src/stored_procedure/`

### StoredProcedureConfig

位置：`src/stored_procedure/config.rs`

存储过程配置。

```rust
pub struct StoredProcedureConfig {
    pub name: String,
    pub parameters: Vec<StoredProcedureParameter>,
    pub return_type: Option<FieldType>,
}
```

### StoredProcedureInfo

位置：`src/stored_procedure/types.rs`

存储过程信息。

```rust
pub struct StoredProcedureInfo {
    pub name: String,
    pub parameters: Vec<StoredProcedureParameter>,
    pub return_type: Option<FieldType>,
    pub database_type: DatabaseType,
}
```

### JoinRelation

位置：`src/stored_procedure/types.rs`

JOIN 关系定义。

```rust
pub struct JoinRelation {
    pub from_table: String,
    pub from_field: String,
    pub to_table: String,
    pub to_field: String,
    pub join_type: JoinType,
}
```

### MongoAggregationOperation

位置：`src/stored_procedure/types.rs`

MongoDB 聚合管道操作。

```rust
pub enum MongoAggregationOperation {
    Project(Document),
    Match(Document),
    Lookup(Document),
    Unwind(String),
    Group(Document),
    Sort(Document),
    // ... 其他操作
}
```

## 字段版本管理

位置：`src/field_versioning/`

### ModelVersionMeta

位置：`src/field_versioning/types.rs`

模型版本元数据。

```rust
pub struct ModelVersionMeta {
    pub model_name: String,
    pub current_version: u32,
    pub fields: Vec<FieldVersionInfo>,
}
```

### VersionChange

位置：`src/field_versioning/types.rs`

版本变更记录。

```rust
pub struct VersionChange {
    pub from_version: u32,
    pub to_version: u32,
    pub changes: Vec<FieldChange>,
    pub timestamp: DateTime<Utc>,
}
```

### FieldVersionManager

位置：`src/field_versioning/manager.rs`

字段版本管理器。

```rust
pub struct FieldVersionManager {
    // 版本管理器
}
```

**关键方法**：
- `get_current_version(model)` - 获取当前版本
- `get_version_history(model)` - 获取版本历史
- `upgrade(model, target_version)` - 升级到目标版本
- `rollback(model, target_version)` - 回滚到目标版本
- `generate_upgrade_ddl(model, target_version)` - 生成升级 DDL
- `generate_rollback_ddl(model, target_version)` - 生成回滚 DDL

### DDL 生成

位置：`src/field_versioning/ddl.rs`

生成升级/回滚的 DDL 语句。

## 表管理

位置：`src/table/`

### TableSchema

位置：`src/table/schema.rs`

表模式定义。

```rust
pub struct TableSchema {
    pub name: String,
    pub columns: Vec<ColumnDefinition>,
    pub indexes: Vec<IndexDefinition>,
}
```

### ColumnDefinition

位置：`src/table/schema.rs`

列定义。

```rust
pub struct ColumnDefinition {
    pub name: String,
    pub column_type: ColumnType,
    pub nullable: bool,
    pub default_value: Option<String>,
    pub primary_key: bool,
}
```

### ColumnType 枚举

位置：`src/table/schema.rs`

列类型枚举。

```rust
pub enum ColumnType {
    Integer,
    BigInteger,
    Float,
    Double,
    Text,
    Boolean,
    DateTime,
    Json,
    Binary,
    // ... 其他类型
}
```

### TableManager

位置：`src/table/manager.rs`

表管理器。

```rust
pub struct TableManager {
    // 表管理器
}
```

**关键方法**：
- `create_table(schema)` - 创建表
- `drop_table(name)` - 删除表
- `table_exists(name)` - 检查表是否存在
- `get_table_schema(name)` - 获取表模式
- `create_index(table, index)` - 创建索引

### SchemaVersion

位置：`src/table/version.rs`

模式版本管理。

```rust
pub struct SchemaVersion {
    pub version: u32,
    pub applied_at: DateTime<Utc>,
    pub description: String,
}
```

## 工具函数

### 时区转换

位置：`src/utils/timezone.rs`

DateTimeWithTz 时区处理工具。

```rust
pub fn convert_datetime_with_tz(
    dt: DateTime<Utc>,
    target_tz: FixedOffset,
) -> DateTime<FixedOffset>

pub fn parse_datetime_with_tz(s: &str) -> QuickDbResult<DateTime<FixedOffset>>
```

## 错误处理

位置：`src/error.rs`

### QuickDbError

统一错误类型。

```rust
pub enum QuickDbError {
    TableNotExistError { table: String, message: String },
    ConnectionError(String),
    QueryError(String),
    ValidationError(String),
    CacheError(String),
    SerializationError(String),
    DatabaseError(String),
    ConfigurationError(String),
    // ... 其他错误变体
}
```

### QuickDbResult<T>

统一结果类型。

```rust
pub type QuickDbResult<T> = Result<T, QuickDbError>;
```

## 安全

位置：`src/security.rs`

### DatabaseSecurityValidator

数据库安全验证器。

```rust
pub struct DatabaseSecurityValidator {
    // 安全验证器
}
```

**功能**：
- SQL 注入防护
- 参数化查询
- 输入验证
- 查询长度限制

## 序列化

位置：`src/serializer.rs`

### DataSerializer

数据序列化器。

```rust
pub struct DataSerializer {
    // 序列化器
}
```

**关键方法**：
- `serialize(data, format)` - 序列化数据
- `deserialize(data, format)` - 反序列化数据

### OutputFormat 枚举

```rust
pub enum OutputFormat {
    Json,
    Toml,
    Binary,
}
```

### SerializationResult

序列化结果。

```rust
pub struct SerializationResult {
    pub data: Vec<u8>,
    pub format: OutputFormat,
    pub checksum: String,
}
```

## 日志

位置：`src/lib.rs`

### debug_log! 宏

调试日志宏。

```rust
debug_log!("调试信息: {}", value);
```

## i18n

位置：`src/i18n/mod.rs`

多语言错误消息系统。

**支持语言**：
- 中文（zh-CN）
- 英文（en-US）
- 日文（ja-JP）

**使用方式**：
```rust
// 初始化 i18n 系统
rat_quickdb::init();

// 错误消息会自动根据语言环境显示
```

## 其他模块

### id_generator

位置：`src/id_generator.rs`

ID 生成器。

```rust
pub trait IdGenerator {
    fn generate_id(&self) -> QuickDbResult<DataValue>;
}

pub struct MongoAutoIncrementGenerator {
    // MongoDB 自增 ID 生成器
}
```

### join_macro

位置：`src/join_macro.rs`

JOIN 宏，用于构建跨表查询。

```rust
// JOIN 宏示例
join!(User, Order, user_id)
```