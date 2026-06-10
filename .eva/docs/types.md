# rat_quickdb 类型系统文档

## 概述

rat_quickdb 的类型系统定义了所有核心数据结构，包括数据值、查询条件、更新操作、ID 策略、数据库配置和缓存配置。

## DataValue 枚举

位置：`src/types/data_value/mod.rs`

统一的数据值类型，用于跨数据库的数据表示。

```rust
pub enum DataValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Json(String),           // JSON 字符串
    Array(Vec<DataValue>),  // 数组
    Object(HashMap<String, DataValue>),  // 对象
    DateTime(DateTime<Utc>),
    DateTimeWithTz(DateTime<FixedOffset>),
    Binary(Vec<u8>),
}
```

**关键方法**：
- `type_name()` - 获取类型名称
- `is_null()` - 检查是否为空
- `to_json_string()` - 转换为 JSON 字符串
- `deserialize_to<T>()` - 反序列化为指定类型

## 查询条件

位置：`src/types/query/mod.rs`

### QueryCondition（简化版）

默认大小写敏感，适合大多数场景。

```rust
pub struct QueryCondition {
    pub field: String,
    pub operator: QueryOperator,
    pub value: DataValue,
}
```

### QueryConditionWithConfig（完整版）

支持大小写不敏感等高级配置。

```rust
pub struct QueryConditionWithConfig {
    pub field: String,
    pub operator: QueryOperator,
    pub value: DataValue,
    pub case_insensitive: bool,
}
```

**自动转换**：简化版可通过 `.into()` 转换为完整版。

### QueryOperator 枚举

```rust
pub enum QueryOperator {
    Eq,         // 等于
    Ne,         // 不等于
    Gt,         // 大于
    Gte,        // 大于等于
    Lt,         // 小于
    Lte,        // 小于等于
    Like,       // 模糊匹配
    NotLike,    // 非模糊匹配
    In,         // 包含在列表中
    NotIn,      // 不包含在列表中
    IsNull,     // 为空
    IsNotNull,  // 不为空
    Contains,   // 包含（数组/字符串）
    StartsWith, // 以...开头
    EndsWith,   // 以...结尾
}
```

### 分页配置

```rust
pub struct PaginationConfig {
    pub page: u32,
    pub page_size: u32,
}
```

### 查询选项

```rust
pub struct QueryOptions {
    pub pagination: Option<PaginationConfig>,
    pub order_by: Option<String>,
    pub order_desc: bool,
}
```

## 更新操作

位置：`src/types/update_operations/mod.rs`

### UpdateOperation

```rust
pub struct UpdateOperation {
    pub field: String,
    pub operator: UpdateOperator,
    pub value: DataValue,
}
```

### UpdateOperator 枚举

```rust
pub enum UpdateOperator {
    Set,        // 设置值
    Increment,  // 自增
    Decrement,  // 自减
    Push,       // 数组追加
    Pull,       // 数组移除
    AddToSet,   // 集合添加（去重）
    Pop,        // 数组弹出
}
```

**工厂方法**：
- `UpdateOperation::set(field, value)` - 创建 Set 操作
- `UpdateOperation::increment(field, amount)` - 创建 Increment 操作
- `UpdateOperation::decrement(field, amount)` - 创建 Decrement 操作

## ID 策略

位置：`src/types/id_types/mod.rs`

### IdStrategy 枚举

```rust
pub enum IdStrategy {
    AutoIncrement,     // 自增 ID
    UUID,              // UUID v4
    Snowflake,         // 雪花算法
    ObjectId,          // MongoDB ObjectId
    Custom(String),    // 自定义前缀
}
```

### IdType 枚举

```rust
pub enum IdType {
    Integer,
    String,
}
```

## 数据库配置

位置：`src/types/database_config/mod.rs`

### DatabaseType 枚举

```rust
pub enum DatabaseType {
    SQLite,
    PostgreSQL,
    MySQL,
    MongoDB,
}
```

### DatabaseConfig

```rust
pub struct DatabaseConfig {
    pub alias: String,
    pub db_type: DatabaseType,
    pub connection: ConnectionConfig,
}
```

### ConnectionConfig 枚举

```rust
pub enum ConnectionConfig {
    SQLite {
        db_path: String,
    },
    PostgreSQL {
        host: String,
        port: u16,
        database: String,
        username: String,
        password: String,
        ssl_mode: Option<String>,
        tls_config: Option<String>,
    },
    MySQL {
        host: String,
        port: u16,
        database: String,
        username: String,
        password: String,
    },
    MongoDB {
        uri: String,
        database: String,
    },
}
```

## 缓存配置

位置：`src/types/cache_config/mod.rs`

### CacheConfig

```rust
pub struct CacheConfig {
    pub enabled: bool,
    pub strategy: CacheStrategy,
    pub ttl_seconds: Option<u64>,
    pub max_size: Option<usize>,
}
```

### CacheStrategy 枚举

```rust
pub enum CacheStrategy {
    None,           // 不缓存
    Memory,         // 仅内存缓存（L1）
    Disk,           // 仅磁盘缓存（L2）
    MemoryAndDisk,  // 双层缓存（L1 + L2）
}
```

## MongoDB 查询构建器

位置：`src/types/mongo_builder.rs`

### MongoDbConnectionBuilder

用于构建 MongoDB 连接 URI。

```rust
pub struct MongoDbConnectionBuilder {
    // 构建 MongoDB 连接字符串
}
```

## 类型转换

### From 实现

- `QueryCondition` → `QueryConditionWithConfig`（自动转换）
- `DataValue` → `serde_json::Value`
- `FieldType` → `ColumnType`

### ToDataValue trait

位置：`src/model/conversion/to_data_value.rs`

用于将 Rust 类型转换为 `DataValue`。

实现的类型：
- 基本类型：`bool`, `i32`, `i64`, `f32`, `f64`, `String`, `&str`
- 时间类型：`DateTime<Utc>`, `DateTime<FixedOffset>`
- UUID：`Uuid`
- 容器类型：`Vec<T>`, `HashMap<String, T>`, `Option<T>`
- 序列化类型：任何实现 `Serialize` 的类型

## Serde 辅助

位置：`src/types/serde_helpers.rs`

提供自定义序列化/反序列化辅助函数，用于处理特殊类型的 JSON 转换。
