---
name: rat-quickdb
description: rat_quickdb 跨数据库 ODM 库的深度知识 - 架构、define_model! 宏、ODM 层、适配器、缓存系统及扩展指南
---

使用此 skill 快速理解项目核心架构、define_model! 宏用法、ODM 操作流程、数据库适配器扩展以及日常开发操作。

## 项目概述

`rat_quickdb` (v0.5.3) 是一个跨数据库 ODM（Object-Document Mapping）库，支持 SQLite、PostgreSQL、MySQL、MongoDB 四种数据库，提供统一 API。

**核心设计理念**：
- **声明式模型定义**：`define_model!` 宏一次定义结构体 + Model trait + CRUD 方法 + 自动注册
- **ODM-First 架构**：所有数据库操作必须通过 ODM 层，禁止直接访问适配器或连接池
- **消息传递架构**：ODM 层通过 `mpsc::UnboundedChannel` 发送请求，后台 tokio 任务异步处理
- **跨数据库一致性**：统一 API 屏蔽四种数据库差异

**关键依赖**：
- `tokio` (full) - 异步运行时
- `rat_memcache` (full) - L1 内存 + L2 磁盘缓存
- `rat_logger` - 日志系统（项目规范强制要求）
- `rat_embed_lang` - 多语言错误消息（zh-CN/en-US/ja-JP）
- `crossbeam-queue` / `dashmap` / `parking_lot` - 并发原语
- `chrono` / `uuid` / `regex` / `serde` / `serde_json` - 核心工具库

**数据库驱动**（条件编译）：
- `sqlite-support` → `sqlx` (sqlite)
- `postgres-support` → `tokio-postgres` + `sqlx/postgres`
- `mysql-support` → `mysql_async` + `sqlx/mysql`
- `mongodb-support` → `mongodb`

## 强制初始化

**必须在使用任何数据库操作前调用 `rat_quickdb::init()`**：

```rust
fn main() {
    rat_quickdb::init();  // 必须！初始化 ODM 系统、线程管理器、i18n

    // 之后才可使用数据库操作
    rat_quickdb::add_database(config).await;
}
```

## 核心架构

### 分层架构

```
用户代码
  ↓
Model 层 (define_model! 宏)        → 定义数据结构、自动注册、CRUD 实例方法
  ↓
ODM 层 (AsyncOdmManager)           → 全局单例、消息传递、请求分发
  ↓
Manager 层 (PoolManager)           → 连接池管理、模型注册、缓存管理
  ↓
Adapter 层 (DatabaseAdapter trait) → 数据库抽象、SQL 生成、类型转换
  ↓
Pool 层 (ConnectionPool)           → 连接工作线程、操作队列
  ↓
数据库驱动                          → sqlx / tokio-postgres / mysql_async / mongodb
```

### 关键模块

| 模块 | 路径 | 职责 |
|------|------|------|
| **Model** | `src/model/` | `define_model!` 宏、Model trait、ModelManager<T>、字段类型定义 |
| **ODM** | `src/odm/` | AsyncOdmManager、OdmOperations trait、请求处理器 |
| **Manager** | `src/manager/` | PoolManager、数据库/模型/缓存操作、别名映射 |
| **Adapter** | `src/adapter/` | DatabaseAdapter trait、四种数据库适配器、缓存装饰器 |
| **Pool** | `src/pool/` | ConnectionPool、SqliteWorker、MultiConnectionManager |
| **Cache** | `src/cache/` | CacheManager (L1+L2)、缓存键生成、查询/记录缓存 |
| **Types** | `src/types/` | DataValue、QueryCondition、DatabaseConfig、IdStrategy 等 |
| **Config** | `src/config/` | AppConfig、构建器模式、便捷配置函数 |
| **Stored Procedure** | `src/stored_procedure/` | 跨数据库存储过程、JOIN 查询、MongoDB 聚合管道 |
| **Table** | `src/table/` | 表管理、Schema 定义、版本迁移 |
| **Field Versioning** | `src/field_versioning/` | 字段版本管理、DDL 生成 |
| **i18n** | `src/i18n/` | 多语言错误消息 (zh-CN/en-US/ja-JP) |
| **Security** | `src/security.rs` | DatabaseSecurityValidator (防 SQL/NoSQL 注入) |

## define_model! 宏（核心）

所有模型定义**必须**使用 `define_model!` 宏，没有例外。

### 基础用法

```rust
use rat_quickdb::{define_model, string_field, integer_field, float_field, boolean_field, datetime_field, json_field, IdStrategy, DatabaseConfig, DatabaseType, CacheStrategy};

// 定义模型（自动生成结构体 + Model trait 实现 + CRUD 方法）
define_model! {
    User {
        fields: [
            string_field!("username", 50),           // String(max_length)
            string_field!("email", 100),              // String(max_length)
            integer_field!("age"),                     // i32
            float_field!("score"),                     // f64
            boolean_field!("is_active", true),          // bool, 默认值 true
            datetime_field!("created_at"),             // DateTime
            json_field!("metadata"),                   // JSON
        ],
        table_name: "users",                          // 表名/集合名
        id_strategy: IdStrategy::AutoIncrement,       // ID 策略
        database_alias: "default",                    // 数据库别名
        indexes: [                                     // 索引
            { fields: ["email"], unique: true },
            { fields: ["username"], unique: true },
        ],
    }
}
```

### 宏展开后生成的内容

1. **结构体定义** - 带 `#[derive(Debug, Clone, Serialize, Deserialize)]`
2. **Model trait 实现**：
   - `meta()` → 返回 `ModelMeta`（collection_name, database_alias, fields, indexes, version）
   - 自动注册模型元数据（`std::sync::Once` 保证只注册一次）
   - `to_data_map_direct()` → 高性能直接转换，避免 JSON 序列化
3. **实例方法**：
   - `save()` → 验证 → 确保表存在 → ODM create → 返回 ID
   - `update(updates)` → 自动提取 id/_id → ODM update_by_id
   - `delete()` → 自动提取 id/_id → ODM delete_by_id
   - `update_many()` / `delete_many()` → 批量操作
4. **静态方法**（通过 `ModelManager<T>`）：
   - `User::find()`, `User::find_by_id()`, `User::count()`
   - `User::create_table()`, `User::find_with_groups()`
   - `User::create_stored_procedure()`, `User::execute_stored_procedure()`

### 可用字段类型

```rust
string_field!("name", max_length)           // String
integer_field!("count")                      // i32
bigint_field!("big_id")                      // i64
float_field!("price")                        // f64
boolean_field!("active", default_value)      // bool
datetime_field!("created_at")                // DateTime
datetime_with_tz_field!("event_time", tz)    // DateTimeWithTz（自动时区转换）
json_field!("metadata")                      // JSON
uuid_field!("token")                         // UUID
array_field!("tags", "String")               // Array<String>（简单值列表）
list_field!("scores", "Float")               // List<Float>
reference_field!("user_id")                  // 外键引用
```

### ID 策略

```rust
use rat_quickdb::IdStrategy;

IdStrategy::AutoIncrement                    // 自增 ID（SQLite/MySQL 推荐）
IdStrategy::Uuid                             // UUID v4（PostgreSQL 推荐）
IdStrategy::Snowflake { machine_id: 1, datacenter_id: 1 }  // 雪花算法
IdStrategy::ObjectId                         // MongoDB ObjectId（MongoDB 推荐）
IdStrategy::Custom("my_custom_id".into())    // 自定义
```

## 数据库配置

### 便捷配置函数

```rust
use rat_quickdb::{sqlite_config, postgres_config, mysql_config, mongodb_config, add_database, CacheStrategy};

// SQLite
let config = sqlite_config("sqlite:test.db")
    .cache_strategy(CacheStrategy::ReadWrite);

// PostgreSQL
let config = postgres_config("postgres://user:pass@localhost:5432/mydb")
    .max_connections(10)
    .cache_strategy(CacheStrategy::ReadOnly);

// MySQL
let config = mysql_config("mysql://user:pass@localhost:3306/mydb")
    .max_connections(10);

// MongoDB
let config = mongodb_config("mongodb://localhost:27017/mydb")
    .max_connections(5);
```

### 构建器模式

```rust
use rat_quickdb::{AppConfigBuilder, DatabaseConfigBuilder, LoggingConfigBuilder, Environment, LogLevel};

let config = AppConfigBuilder::new()
    .environment(Environment::Development)
    .logging(LoggingConfigBuilder::new()
        .level(LogLevel::Debug)
        .build())
    .database(DatabaseConfigBuilder::new()
        .db_type(DatabaseType::PostgreSQL)
        .connection_string("postgres://localhost/mydb")
        .alias("default")
        .max_connections(10)
        .cache_strategy(CacheStrategy::ReadWrite)
        .id_strategy(IdStrategy::Uuid)
        .build())
    .build();
```

### 缓存策略

```rust
use rat_quickdb::CacheStrategy;

CacheStrategy::ReadWrite    // 读写缓存（默认）
CacheStrategy::ReadOnly     // 只读缓存
CacheStrategy::Bypass       // 绕过缓存（实时数据场景，性能提升 2-20x）
```

## ODM 操作（核心 API）

### 基本操作

```rust
use rat_quickdb::{define_model, string_field, integer_field, IdStrategy, QueryCondition, QueryOperator};

// 创建
let user = User {
    username: "alice".into(),
    email: "alice@example.com".into(),
    age: 30,
    // ...
};
let id = user.save().await?;

// 查询单条
let found = User::find_by_id(&id).await?;

// 条件查询
let results = User::find()
    .condition(QueryCondition::new("age", QueryOperator::Gte, 18))
    .condition(QueryCondition::new("is_active", QueryOperator::Eq, true))
    .execute()
    .await?;

// 更新
let updated = User {
    id: Some(id.clone()),
    username: "alice_updated".into(),
    // ...
};
updated.update(updated.clone()).await?;

// 删除
updated.delete().await?;

// 计数
let count = User::count().await?;
```

### 查询操作符

```rust
use rat_quickdb::{QueryCondition, QueryOperator};

// 比较操作
QueryCondition::new("age", QueryOperator::Eq, 30)
QueryCondition::new("age", QueryOperator::Ne, 0)
QueryCondition::new("age", QueryOperator::Gt, 18)
QueryCondition::new("age", QueryOperator::Gte, 18)
QueryCondition::new("age", QueryOperator::Lt, 65)
QueryCondition::new("age", QueryOperator::Lte, 65)

// 字符串操作
QueryCondition::new("name", QueryOperator::Contains, "alice")
QueryCondition::new("name", QueryOperator::StartsWith, "al")
QueryCondition::new("name", QueryOperator::EndsWith, "ce")
QueryCondition::new("name", QueryOperator::Regex, r"^al.*ce$")

// 集合操作
QueryCondition::new("status", QueryOperator::In, vec!["active", "pending"])
QueryCondition::new("status", QueryOperator::NotIn, vec!["deleted"])

// JSON 操作
QueryCondition::new("metadata", QueryOperator::JsonContains, json!({"key": "value"}))

// 空值检查
QueryCondition::new("email", QueryOperator::IsNull, ())
QueryCondition::new("email", QueryOperator::IsNotNull, ())
QueryCondition::new("email", QueryOperator::Exists, true)

// 条件组合
let condition = QueryCondition::group(LogicalOperator::And, vec![
    QueryCondition::new("age", QueryOperator::Gte, 18),
    QueryCondition::new("status", QueryOperator::Eq, "active"),
]);
```

### 更新操作

```rust
use rat_quickdb::{UpdateOperation, UpdateOperator};

let updates = vec![
    UpdateOperation::new("age", UpdateOperator::Set, 31),
    UpdateOperation::new("score", UpdateOperator::Increment, 5),
    UpdateOperation::new("price", UpdateOperator::Decrement, 10),
    UpdateOperation::new("quantity", UpdateOperator::Multiply, 2),
];
```

### 分页与排序

```rust
use rat_quickdb::{QueryOptions, SortConfig, SortDirection, PaginationConfig};

let results = User::find()
    .condition(QueryCondition::new("age", QueryOperator::Gte, 18))
    .options(QueryOptions {
        sort: Some(SortConfig {
            field: "created_at".into(),
            direction: SortDirection::Desc,
        }),
        pagination: Some(PaginationConfig {
            page: 1,
            page_size: 20,
        }),
        ..Default::default()
    })
    .execute()
    .await?;
```

## 数据库适配器

### DatabaseAdapter trait

统一接口，约 20 个方法：

| 类别 | 方法 |
|------|------|
| **CRUD** | `create`, `find_by_id`, `find`, `find_with_cache_control`, `update`, `update_by_id`, `delete`, `delete_by_id`, `count` |
| **分组查询** | `find_with_groups`, `count_with_groups` |
| **DDL** | `create_table`, `create_index`, `table_exists`, `drop_table` |
| **存储过程** | `create_stored_procedure`, `execute_stored_procedure` |
| **工具** | `get_server_version` |

### 各适配器结构（一致）

```
src/adapter/{db}/
├── mod.rs              # 模块导出
├── adapter.rs          # DatabaseAdapter trait 实现
├── operations.rs       # 底层操作实现
├── query.rs            # 查询构建辅助
├── query_builder.rs    # 查询构建器
├── schema.rs           # 表结构管理
├── utils.rs            # 工具函数
└── data_conversion.rs  # 数据转换（SQLite/MySQL 独有）
```

### 缓存装饰器

`CachedDatabaseAdapter` 包装任何适配器，添加 L1/L2 缓存层：

```
用户请求 → CachedDatabaseAdapter → L1 缓存(命中?) → L2 缓存(命中?) → 底层适配器 → 数据库
```

## 缓存系统

### 两级缓存

- **L1 缓存**：内存缓存（LRU/LFU/FIFO 策略），基于 `rat_memcache`
- **L2 缓存**：磁盘持久缓存（LZ4 压缩），基于 `rat_memcache`

### 缓存操作

```rust
use rat_quickdb::{clear_cache, clear_all_caches, get_cache_stats};

// 清除特定模型缓存
clear_cache("default:users").await?;

// 清除所有缓存
clear_all_caches().await?;

// 获取缓存统计
let stats = get_cache_stats().await?;
```

### 性能参考

| 数据库 | 缓存绕过提升 |
|--------|-------------|
| MySQL | 16x |
| PostgreSQL | 2.25x |
| MongoDB (批量) | 20x |

## 存储过程系统

### 跨数据库 JOIN 查询

```rust
use rat_quickdb::{StoredProcedureBuilder, JoinType, create_stored_procedure, execute_stored_procedure};

// 定义存储过程
let config = StoredProcedureBuilder::new("user_orders")
    .database("default")
    .table("users")
    .join(JoinType::Inner, "orders", "users.id", "orders.user_id")
    .join(JoinType::Left, "products", "orders.product_id", "products.id")
    .field("users.username")
    .field("orders.total")
    .field("products.name")
    .build();

// 创建
create_stored_procedure(&config).await?;

// 执行
let results = execute_stored_procedure("user_orders", Some(vec![
    QueryCondition::new("users.age", QueryOperator::Gte, 18),
])).await?;
```

### MongoDB 聚合管道

```rust
use rat_quickdb::{MongoPipelineBuilder, MongoAggregationOperation};

let pipeline = MongoPipelineBuilder::new()
    .match_(MongoAggregationOperation::Match {
        conditions: vec![QueryCondition::new("status", QueryOperator::Eq, "active")],
    })
    .group(MongoAggregationOperation::Group {
        group_key: "category".into(),
        accumulators: vec![/* ... */],
    })
    .sort("total", -1)
    .build();
```

## 连接池架构

- **SQLite**：单 `SqliteWorker`，文件级锁
- **PostgreSQL/MySQL/MongoDB**：`MultiConnectionManager`，多连接池

```
ODM Request → mpsc::UnboundedSender → ConnectionPool
                                          ↓
                                    ConnectionWorker
                                    (持有 DatabaseConnection + Adapter)
                                          ↓
                                    DatabaseConnection (条件编译枚举)
                                    ├── sqlx::SqlitePool
                                    ├── sqlx::PgPool
                                    ├── sqlx::MySqlPool
                                    └── mongodb::Database
```

## 类型系统

### DataValue（核心统一类型）

```rust
pub enum DataValue {
    Null,
    Bool(bool),
    Int(i32),
    UInt(u32),
    Float(f64),
    String(String),
    Bytes(Vec<u8>),
    DateTime(chrono::NaiveDateTime),
    DateTimeUTC(chrono::DateTime<chrono::Utc>),
    Uuid(uuid::Uuid),
    Json(serde_json::Value),
    Array(Vec<DataValue>),
    Object(serde_json::Map<String, serde_json::Value>),
}
```

### FieldType（模型字段类型）

`String | Integer | BigInteger | Float | Double | Text | Boolean | DateTime | DateTimeWithTz | Date | Time | Uuid | Json | Binary | Decimal | Array | Object | Reference`

### 全局操作锁

首次查询操作后，`AtomicBool` 锁定，禁止添加新数据库。这是为了防止运行时修改连接池导致的数据不一致。

## 多数据库别名

```rust
use rat_quickdb::{add_database, sqlite_config, postgres_config, set_default_alias};

// 添加多个数据库
add_database(sqlite_config("sqlite:local.db").alias("local")).await?;
add_database(postgres_config("postgres://remote/db").alias("remote")).await?;

// 设置默认别名
set_default_alias("remote").await?;

// 模型指定数据库别名
define_model! {
    User {
        fields: [string_field!("name", 50)],
        table_name: "users",
        id_strategy: IdStrategy::AutoIncrement,
        database_alias: "local",  // 使用 local 数据库
    }
}
```

## 错误处理

```rust
use rat_quickdb::{QuickDbError, QuickDbResult};

pub type QuickDbResult<T> = Result<T, QuickDbError>;

// 错误类型支持多语言消息（zh-CN/en-US/ja-JP）
// 基于 rat_embed_lang 实现国际化
```

## 常用命令

```bash
# 构建
cargo build
cargo build --features sqlite-support
cargo build --features postgres-support
cargo build --features mysql-support
cargo build --features mongodb-support
cargo build --features full

# 测试
cargo test
cargo test --features sqlite-support
cargo test --features postgres-support
cargo test --features full

# 代码质量
cargo fmt --check
cargo clippy
cargo check

# 运行示例（需要启用对应 feature）
cargo run --example model_operations_sqlite --features sqlite-support
cargo run --example model_operations_pgsql --features postgres-support
cargo run --example model_operations_mysql --features mysql-support
cargo run --example model_operations_mongodb --features mongodb-support
cargo run --example query_operations_sqlite --features sqlite-support

# 性能测试（release 模式）
cargo run --release --example simple_concurrent_test --features sqlite-support
```

## 关键限制（CRITICAL）

1. **必须调用 `init()`** — 所有数据库操作前
2. **必须使用 `define_model!`** — 禁止手动定义模型结构体
3. **禁止直接访问适配器/连接池** — 所有操作通过 ODM 层
4. **必须启用对应 feature** — 运行示例或测试时
5. **必须显式指定 ID 策略** — 无默认值
6. **必须使用 `rat_logger`** — 禁止其他日志库

## 扩展新数据库

分步指南：添加新的数据库支持（例如 ClickHouse）

1. **`Cargo.toml`** 添加 feature 和依赖
2. **`src/types/database_config/mod.rs`** 添加 `DatabaseType` 变体
3. **`src/pool/types.rs`** 添加 `DatabaseConnection` 变体（条件编译）
4. **`src/adapter/`** 创建 `clickhouse/` 目录，实现 `DatabaseAdapter` trait
5. **`src/adapter/mod.rs`** 在 `create_adapter()` 工厂函数中添加分支
6. **`src/config/core.rs`** 添加连接配置支持
7. **`src/config/convenience.rs`** 添加 `clickhouse_config()` 便捷函数

参考现有适配器结构，所有文件组织方式一致。

## 字段版本管理

支持字段级别的 Schema 版本管理：

```rust
use rat_quickdb::{FieldVersionManager, ModelVersionMeta, VersionChange};

let manager = FieldVersionManager::new("sqlite:versions.db");
manager.register_model(&ModelVersionMeta::new("users", "default", 1, vec![
    VersionChange::AddField { name: "nickname".into(), field_type: "String".into() },
]))?;
manager.apply_version_changes("default:users").await?;
```

## 示例参考

| 示例 | Feature | 说明 |
|------|---------|------|
| `model_operations_sqlite.rs` | sqlite | 模型定义与 CRUD |
| `model_operations_pgsql.rs` | postgres | PostgreSQL 模型操作 |
| `model_operations_mysql.rs` | mysql | MySQL 模型操作 |
| `model_operations_mongodb.rs` | mongodb | MongoDB 模型操作 |
| `query_operations_sqlite.rs` | sqlite | 查询操作演示 |
| `id_strategy_test.rs` | sqlite | ID 策略测试 |
| `cache_performance_comparison.rs` | sqlite | 缓存性能对比 |
| `cache_bypass_comparison_sqlite.rs` | sqlite | 缓存绕过性能 |
| `test_datetime_with_tz_field.rs` | sqlite | 时区字段处理 |
| `timezone_complex_query_demo.rs` | sqlite | 时区复杂查询 |
| `string_fuzzy_search_sqlite.rs` | sqlite | 字符串模糊搜索 |
| `test_array_in_query_sqlite.rs` | sqlite | 数组 IN 查询 |
| `test_stored_procedure.rs` | sqlite | 存储过程 |
| `test_complex_join.rs` | sqlite | 复杂 JOIN 查询 |
| `test_join_macro.rs` | sqlite | JOIN 宏 |
| `field_versioning_sqlite.rs` | sqlite | 字段版本管理 |
| `test_i18n_errors.rs` | sqlite | 多语言错误消息 |
| `json_field_array_test.rs` | sqlite | JSON 数组字段 |
| `test_global_lock.rs` | sqlite | 全局操作锁 |
| `test_model_database_alias.rs` | sqlite | 多数据库别名 |
