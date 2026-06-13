# rat_quickdb 模型层文档

## 概述

模型层（`src/model/`）定义数据模型结构，提供模型操作接口。通过宏驱动的代码生成减少样板代码。

## 核心组件

### Model trait

位置：`src/model/traits.rs`

模型核心 trait，定义元数据和数据转换方法。

```rust
#[async_trait]
pub trait Model: Send + Sync + 'static {
    fn meta() -> ModelMeta;
    fn collection_name() -> String;
    async fn validate(&self) -> QuickDbResult<()>;
    fn to_data_map(&self) -> DataMap;
    fn from_data_map(data: &DataMap) -> QuickDbResult<Self> where Self: Sized;
}
```

### ModelOperations trait

位置：`src/model/traits.rs`

模型操作 trait，定义 CRUD 操作。

```rust
#[async_trait]
pub trait ModelOperations<T: Model>: Send + Sync {
    async fn save(&self) -> QuickDbResult<()>;
    async fn update(&self) -> QuickDbResult<()>;
    async fn upsert(&self) -> QuickDbResult<()>;
    async fn delete(&self) -> QuickDbResult<()>;
}
```

### ModelManager<T>

位置：`src/model/traits.rs`

泛型模型管理器，提供静态方法进行数据库操作。

```rust
pub struct ModelManager<T: Model> {
    _phantom: PhantomData<T>,
}
```

**关键方法**：
- `find_by_id(id)` - 按 ID 查询
- `find(conditions, options)` - 条件查询
- `find_with_config(conditions, options)` - 带配置的查询
- `update(id, operations)` - 更新记录
- `delete(id)` - 删除记录
- `count(conditions)` - 统计数量

### define_model! 宏

位置：`src/model/macros.rs`

自动生成模型样板代码的宏。

```rust
define_model! {
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct User {
        #[id(strategy = "uuid")]
        pub id: String,
        #[field(type = "string")]
        pub username: String,
        #[field(type = "string")]
        pub email: String,
        #[field(type = "boolean")]
        pub active: bool,
        #[field(type = "datetime")]
        pub created_at: DateTime<Utc>,
    }
}
```

**自动生成**：
- `Model` trait 实现
- `ModelOperations` trait 实现
- 字段访问器方法
- 序列化/反序列化支持

### FieldDefinition

位置：`src/model/field_types.rs`

字段定义结构体。

```rust
pub struct FieldDefinition {
    pub name: String,
    pub field_type: FieldType,
    pub required: bool,
    pub default_value: Option<DataValue>,
    pub index: bool,
    pub unique: bool,
}
```

### FieldType 枚举

位置：`src/model/field_types.rs`

字段类型枚举。

```rust
pub enum FieldType {
    String,
    Integer,
    BigInteger,
    Float,
    Double,
    Text,
    Boolean,
    DateTime,
    DateTimeWithTz,
    Date,
    Time,
    Uuid,
    Json,
    Binary,
    Decimal,
    Array,
    Object,
    Reference(String),
}
```

### ModelMeta

位置：`src/model/field_types.rs`

模型元数据。

```rust
pub struct ModelMeta {
    pub table_name: String,
    pub fields: Vec<FieldDefinition>,
    pub indexes: Vec<IndexDefinition>,
}
```

### IndexDefinition

位置：`src/model/field_types.rs`

索引定义。

```rust
pub struct IndexDefinition {
    pub name: String,
    pub fields: Vec<String>,
    pub unique: bool,
}
```

## 数据转换

### ToDataValue trait

位置：`src/model/conversion/to_data_value.rs`

用于将 Rust 类型转换为 `DataValue`。

实现的类型：
- 基本类型：`bool`, `i32`, `i64`, `f32`, `f64`, `String`, `&str`
- 时间类型：`DateTime<Utc>`, `DateTime<FixedOffset>`
- UUID：`Uuid`
- 容器类型：`Vec<T>`, `HashMap<String, T>`, `Option<T>`
- 序列化类型：任何实现 `Serialize` 的类型

### create_model_from_data_map()

位置：`src/model/data_conversion.rs`

从 DataMap 创建模型实例的工厂函数。

## 便捷字段函数

位置：`src/model/convenience.rs`

提供便捷的字段构造函数：

```rust
pub fn string_field(name: &str) -> FieldDefinition
pub fn integer_field(name: &str) -> FieldDefinition
pub fn bigint_field(name: &str) -> FieldDefinition
pub fn float_field(name: &str) -> FieldDefinition
pub fn boolean_field(name: &str) -> FieldDefinition
pub fn datetime_field(name: &str) -> FieldDefinition
pub fn datetime_with_tz_field(name: &str) -> FieldDefinition
pub fn uuid_field(name: &str) -> FieldDefinition
pub fn json_field(name: &str) -> FieldDefinition
pub fn array_field(name: &str) -> FieldDefinition
pub fn dict_field(name: &str) -> FieldDefinition
pub fn reference_field(name: &str, reference: &str) -> FieldDefinition
```

## 数据转换模块

位置：`src/model/conversion/`

- `primitive_impls.rs` - 基本类型转换实现
- `collection_impls.rs` - 容器类型转换实现
- `complex_impls.rs` - 复杂类型转换实现
- `database_aware.rs` - 数据库感知的转换
- `datetime_conversion.rs` - 时间类型转换
- `to_data_value.rs` - ToDataValue trait 实现