# rat_quickdb 适配器层文档

## 概述

适配器层（`src/adapter/`）封装了各数据库的差异，提供统一的 `DatabaseAdapter` trait 接口。每个数据库有独立的子目录实现。

## 核心 Trait

位置：`src/adapter/mod.rs`

```rust
#[async_trait]
pub trait DatabaseAdapter: Send + Sync {
    async fn create(&self, table: &str, data: DataMap) -> QuickDbResult<DataMap>;
    async fn find_by_id(&self, table: &str, id: &DataValue) -> QuickDbResult<Option<DataMap>>;
    async fn find(&self, table: &str, conditions: Vec<QueryConditionWithConfig>, options: Option<QueryOptions>) -> QuickDbResult<Vec<DataMap>>;
    async fn update(&self, table: &str, id: &DataValue, operations: Vec<UpdateOperation>) -> QuickDbResult<()>;
    async fn delete(&self, table: &str, id: &DataValue) -> QuickDbResult<()>;
    async fn count(&self, table: &str, conditions: Vec<QueryConditionWithConfig>) -> QuickDbResult<u64>;
    async fn create_table(&self, meta: &ModelMeta) -> QuickDbResult<()>;
    async fn create_index(&self, table: &str, index: &IndexDefinition) -> QuickDbResult<()>;
    async fn get_server_version(&self) -> QuickDbResult<String>;
    // ... 其他方法
}
```

## 工厂函数

位置：`src/adapter/mod.rs`

```rust
pub fn create_adapter(db_type: &DatabaseType) -> QuickDbResult<Box<dyn DatabaseAdapter>>
pub fn create_adapter_with_cache(db_type: &DatabaseType, cache_manager: Arc<CacheManager>) -> QuickDbResult<Box<dyn DatabaseAdapter>>
```

## 各数据库适配器

### SQLite 适配器

位置：`src/adapter/sqlite/`

**文件结构**：
- `adapter.rs` - SqliteAdapter 实现
- `operations.rs` - SQL 操作构建
- `query_builder.rs` - 查询条件构建
- `query.rs` - 查询执行
- `schema.rs` - 表结构管理
- `utils.rs` - 工具函数

**特点**：
- 单文件数据库，使用 WAL 模式
- 自动处理布尔值（0/1 ↔ bool）
- 支持 JSON 字段查询

**特殊处理**：
- 布尔值：SQLite 存储为 0/1，适配器自动转换
- 日期时间：支持字符串和时间戳两种存储格式

### PostgreSQL 适配器

位置：`src/adapter/postgres/`

**文件结构**：
- `adapter.rs` - PostgresAdapter 实现
- `operations.rs` - SQL 操作构建
- `query_builder.rs` - 查询条件构建
- `query.rs` - 查询执行
- `schema.rs` - 表结构管理
- `utils.rs` - 工具函数

**特点**：
- 原生 JSONB 支持
- 强类型系统
- 支持数组类型
- 支持 UUID 类型

**JSON 查询**：
- `build_json_query_condition()` - 构建 JSON 查询条件
- `convert_to_jsonb_value()` - 转换为 JSONB 值

### MySQL 适配器

位置：`src/adapter/mysql/`

**文件结构**：
- `adapter.rs` - MysqlAdapter 实现
- `operations.rs` - SQL 操作构建
- `query_builder.rs` - 查询条件构建
- `query.rs` - 查询执行
- `schema.rs` - 表结构管理
- `utils.rs` - 工具函数

**特点**：
- JSON 类型支持
- 自动生成索引
- 支持 ENUM 类型

**限制**：
- 不支持原生数组类型
- JSON 查询语法与 PostgreSQL 不同

### MongoDB 适配器

位置：`src/adapter/mongodb/`

**文件结构**：
- `adapter.rs` - MongoAdapter 实现
- `operations.rs` - 聚合管道构建
- `query_builder.rs` - 查询条件构建
- `query.rs` - 查询执行
- `schema.rs` - 集合管理
- `utils.rs` - 工具函数

**特点**：
- 文档数据库，无固定模式
- 支持聚合管道
- 自动生成集合
- 支持嵌套文档查询

**聚合操作**：
- `MongoAggregationOperation` - 聚合操作枚举
- 支持 Project、Match、Lookup、Unwind、Group、Sort 等阶段

## 缓存装饰器

位置：`src/adapter/cached.rs`

```rust
pub struct CachedDatabaseAdapter {
    inner: Box<dyn DatabaseAdapter>,
    cache_manager: Arc<CacheManager>,
}
```

**功能**：
- 透明地为任何适配器添加缓存支持
- 自动处理缓存读写
- 支持缓存绕过

## 查询构建

### SQL 查询构建

各数据库适配器的 `query_builder.rs` 负责将 `QueryConditionWithConfig` 转换为 SQL WHERE 子句。

**大小写不敏感处理**：
- PostgreSQL/MySQL/SQLite：使用 `LOWER(field) = LOWER(value)`
- MongoDB：使用正则表达式 `$regex: "^value$", $options: "i"`

### NoSQL 查询构建

MongoDB 适配器将查询条件转换为 BSON 文档。

## 表结构管理

各适配器的 `schema.rs` 负责：
- 根据 `ModelMeta` 创建表
- 创建索引
- 检查表是否存在
- 获取表结构信息

## 错误处理

各适配器统一使用 `QuickDbError` 错误类型，特定数据库错误会被转换为统一错误。

### PostgreSQL 错误码

位置：`src/adapter/postgres/`

支持将 PostgreSQL 错误码转换为 `QuickDbError` 的具体变体。

### MySQL 错误码

位置：`src/adapter/mysql/`

支持将 MySQL 错误码转换为 `QuickDbError` 的具体变体。

### SQLite 错误码

位置：`src/adapter/sqlite/`

支持将 SQLite 错误码转换为 `QuickDbError` 的具体变体。

### MongoDB 错误结构

位置：`src/adapter/mongodb/`

支持将 MongoDB 错误文档转换为 `QuickDbError` 的具体变体。
