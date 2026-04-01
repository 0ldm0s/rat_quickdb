# rat_quickdb

[![Crates.io](https://img.shields.io/crates/v/rat_quickdb.svg)](https://crates.io/crates/rat_quickdb)
[![Documentation](https://docs.rs/rat_quickdb/badge.svg)](https://docs.rs/rat_quickdb)
[![License: LGPL-3.0](https://img.shields.io/badge/License-LGPL--3.0-blue.svg)](https://www.gnu.org/licenses/lgpl-3.0)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://rust-lang.org)
[![Downloads](https://img.shields.io/crates/d/rat_quickdb.svg)](https://crates.io/crates/rat_quickdb)

🚀 强大的跨数据库ODM库，支持SQLite、PostgreSQL、MySQL、MongoDB的统一接口

**🌐 语言版本**: [中文](README.md) | [English](README.en.md) | [日本語](README.ja.md)

## ✨ 核心特性

- **🎯 自动索引创建**: 基于模型定义自动创建表和索引，无需手动干预
- **🗄️ 多数据库支持**: SQLite、PostgreSQL、MySQL、MongoDB
- **🔗 统一API**: 一致的接口操作不同数据库
- **🔒 SQLite布尔值兼容**: 自动处理SQLite布尔值存储差异，零配置兼容
- **🏊 连接池管理**: 高效的连接池和无锁队列架构
- **⚡ 异步支持**: 基于Tokio的异步运行时
- **🧠 智能缓存**: 内置缓存支持（基于rat_memcache），支持TTL过期、回退机制和缓存绕过
- **🆔 多种ID生成策略**: AutoIncrement、UUID、Snowflake、ObjectId、Custom前缀
- **📝 日志控制**: 由调用者完全控制日志初始化，避免库自动初始化冲突
- **🐍 Python绑定**: 可选Python API支持
- **📋 任务队列**: 内置异步任务队列系统
- **🔍 类型安全**: 强类型模型定义和验证
- **📋 存储过程**: 跨数据库的统一存储过程API，支持多表JOIN和聚合查询

## 🔄 版本变更说明

### v0.5.1 - 版本更新

**新功能：**
- 🎯 **大小写不敏感查询**：所有数据库适配器现在支持大小写不敏感的字符串查询
- 🔄 **双类型系统**：提供简化版和完整版查询条件类型，满足不同使用场景
- 📊 **跨数据库支持**：MongoDB、MySQL、PostgreSQL、SQLite 全部支持
- 🔄 **自动类型转换**：简化版自动转换为完整版，无需手动处理

**类型说明：**

本版本提供两种查询条件类型：

1. **`QueryCondition`（简化版）**：适用于大多数场景
   - 不包含 `case_insensitive` 字段
   - 默认大小写敏感
   - 使用更简洁，代码更清晰

2. **`QueryConditionWithConfig`（完整版）**：适用于需要配置的场景
   - 包含 `case_insensitive` 字段，可控制大小写敏感性
   - 支持未来更多配置选项
   - 用于需要大小写不敏感等高级功能的查询

**使用示例：**

```rust
use rat_quickdb::*;

// ===== 简化版：默认大小写敏感查询（推荐用于日常使用）=====
let results = ModelManager::<User>::find(
    vec![QueryCondition {
        field: "username".to_string(),
        operator: QueryOperator::Eq,
        value: DataValue::String("admin".to_string()),
        // 无 case_insensitive 字段，默认大小写敏感
    }],
    None
).await?;

// ===== 完整版：大小写不敏感查询 =====
let insensitive_results = ModelManager::<User>::find_with_config(
    vec![QueryConditionWithConfig {
        field: "username".to_string(),
        operator: QueryOperator::Eq,
        value: DataValue::String("admin".to_string()),
        case_insensitive: true,  // 启用大小写不敏感
    }],
    None
).await?;

// ===== 完整版：大小写敏感查询（明确指定）=====
let sensitive_results = ModelManager::<User>::find_with_config(
    vec![QueryConditionWithConfig {
        field: "username".to_string(),
        operator: QueryOperator::Eq,
        value: DataValue::String("ADMIN".to_string()),
        case_insensitive: false,  // 明确禁用大小写不敏感
    }],
    None
).await?;
```

**方法对应关系：**

| 简化版方法 | 完整版方法 | 说明 |
|-----------|-----------|------|
| `find(conditions)` | `find_with_config(conditions)` | 查找记录 |
| `count(conditions)` | `count_with_config(conditions)` | 统计记录 |
| `delete_many(conditions)` | `delete_many_with_config(conditions)` | 批量删除 |
| `find_with_cache_control(conditions, options, bypass)` | （内部方法） | 缓存控制 |

**自动转换机制：**

所有简化版方法内部会自动将 `QueryCondition` 转换为 `QueryConditionWithConfig`（`case_insensitive` 默认为 `false`），无需手动处理。

**实现方式：**
- **MongoDB**: 使用正则表达式 `$regex: "^value$", $options: "i"`
- **MySQL**: 使用 `LOWER(field) = LOWER(value)`
- **PostgreSQL**: 使用 `LOWER(field) = LOWER(value)`
- **SQLite**: 使用 `LOWER(field) = LOWER(value)`

**适用场景：**
- 📧 用户名/邮箱查询（用户可能输入任意大小写）
- 🔍 产品名称搜索（不区分大小写）
- 🏷️ 标签和分类查询（提高查询友好性）
- 🌍 多语言文本搜索（适应不同语言的大小写规则）

**性能说明：**
- 对字符串字段启用大小写不敏感查询会略微降低查询性能
- 建议对需要模糊匹配的字段使用，对精确匹配字段保持默认大小写敏感
- 可以通过创建函数索引（如 `LOWER(field)`）来优化性能

**测试验证：**
```bash
# MongoDB
cargo run --example query_operations_mongodb --features mongodb-support

# MySQL
cargo run --example query_operations_mysql --features mysql-support

# PostgreSQL
cargo run --example query_operations_pgsql --features postgres-support

# SQLite
cargo run --example query_operations_sqlite --features sqlite-support
```

### v0.4.5 - 统一表不存在错误处理

**新功能：**
- 🎯 **统一TableNotExistError**：所有数据库适配器现在提供一致的表不存在错误识别
- 🔄 **MongoDB特殊处理**：针对MongoDB的集合自动创建特性，采用实用主义策略
- 📊 **统一接口**：调用者无需区分数据库类型，获得一致的错误处理体验
- 🎛️ **业务友好**：明确的错误预期，便于业务逻辑处理

**核心改进：**
```rust
// 统一的表不存在错误处理
match ModelManager::<User>::find_by_id("non-existent-id").await {
    Err(QuickDbError::TableNotExistError { table, message }) => {
        println!("表不存在: {}", table);
        // 调用者可以明确知道需要初始化数据
    }
    // ... 其他错误处理
}
```

**MongoDB特殊策略：**
- 查询不存在的集合或空集合都返回 `TableNotExistError`
- 调用者收到错误后插入数据会自动创建集合
- 提供统一的错误接口，隐藏MongoDB的语义差异

### v0.4.2 - 缓存绕过功能

**新功能：**
- 🎯 **缓存绕过支持**：新增 `find_with_cache_control` 方法，支持强制跳过缓存查询
- 🔄 **向后兼容**：原有 `find` 方法保持不变，作为新方法的包装器
- 📊 **性能对比**：提供缓存绕过性能测试示例，展示实际性能差异
- 🎛️ **灵活控制**：可根据业务需求选择使用缓存或强制查询数据库

**使用示例：**
```rust
// 强制跳过缓存查询（适用于金融等实时数据场景）
let results = ModelManager::<User>::find_with_cache_control(
    conditions,
    None,
    true  // bypass_cache = true
).await?;

// 普通缓存查询（默认行为）
let results = ModelManager::<User>::find(conditions, None).await?;
```

**性能测试示例：**
```bash
# 运行缓存绕过性能测试
cargo run --example cache_bypass_comparison_mysql --features mysql-support
cargo run --example cache_bypass_comparison_pgsql --features postgres-support
cargo run --example cache_bypass_comparison_sqlite --features sqlite-support
cargo run --example cache_bypass_comparison_mongodb --features mongodb-support
```

### v0.3.6 - 存储过程虚拟表系统

⚠️ **重要变更：连接池配置参数单位变更**

**v0.3.6** 对连接池配置进行了重大改进，**所有超时参数现在使用秒为单位**：

```rust
// v0.3.6 新写法（推荐）
let pool_config = PoolConfig::builder()
    .connection_timeout(30)        // 30秒（之前是5000毫秒）
    .idle_timeout(300)             // 300秒（之前是300000毫秒）
    .max_lifetime(1800)            // 1800秒（之前是1800000毫秒）
    .max_retries(3)                // 新增：最大重试次数
    .retry_interval_ms(1000)       // 新增：重试间隔（毫秒）
    .keepalive_interval_sec(60)    // 新增：保活间隔（秒）
    .health_check_timeout_sec(10)  // 新增：健康检查超时（秒）
    .build()?;
```

⚠️ **重要变更：强制使用 Builder 模式**

**v0.5.3+** 强制要求使用 `PoolConfig::builder()` 创建连接池配置，**禁止直接构造结构体**：

```rust
// ✅ 正确：使用 builder 模式
let pool_config = PoolConfig::builder()
    .max_connections(10)
    .min_connections(2)
    .connection_timeout(30)
    .build()?;

// ❌ 错误：直接构造结构体会导致编译错误
// let pool_config = PoolConfig {
//     max_connections: 10,
//     min_connections: 2,
//     ...
// };
```

**原因**：builder 模式确保所有配置参数都经过验证，避免使用无效的配置值。

**新功能：**
- 🎯 **存储过程虚拟表系统**：跨四种数据库的统一存储过程API
- 🔗 **多表JOIN支持**：自动生成JOIN语句和聚合管道
- 📊 **聚合查询优化**：自动GROUP BY子句生成（SQL数据库）
- 🧠 **类型安全存储过程**：编译时验证和类型检查

## 📦 安装

在`Cargo.toml`中添加依赖：

```toml
[dependencies]
rat_quickdb = "0.5.1"
```

### 🔧 特性控制

rat_quickdb 使用 Cargo 特性来控制不同数据库的支持和功能。默认情况下只包含核心功能，你需要根据使用的数据库类型启用相应的特性：

```toml
[dependencies]
rat_quickdb = { version = "0.5.1", features = [
    "sqlite-support",    # 支持SQLite数据库
    "postgres-support",  # 支持PostgreSQL数据库
    "mysql-support",     # 支持MySQL数据库
    "mongodb-support",   # 支持MongoDB数据库
] }
```

#### 可用特性列表

| 特性名称 | 描述 | 默认启用 |
|---------|------|---------|
| `sqlite-support` | SQLite数据库支持 | ❌ |
| `postgres-support` | PostgreSQL数据库支持 | ❌ |
| `mysql-support` | MySQL数据库支持 | ❌ |
| `mongodb-support` | MongoDB数据库支持 | ❌ |
| `melange-storage` | 已弃用：L2缓存功能已内置在rat_memcache中 | ❌ |
| `python-bindings` | Python API绑定 | ❌ |
| `full` | 启用所有数据库支持 | ❌ |

#### 数据库版本要求

**重要**：不同数据库对JSON操作和正则表达式的支持版本要求不同：

| 数据库 | 最低版本要求 | JSON支持 | Contains操作符实现 | JsonContains操作符实现 | Regex操作符实现 |
|--------|-------------|----------|-------------------|-------------------------|-----------------|
| **MySQL** | 5.7+ / MariaDB 10.2+ | ✅ 完整支持 | 字符串字段使用LIKE，JSON字段使用JSON_CONTAINS() | ❌ 暂时不支持 | ✅ REGEXP 操作符 |
| **PostgreSQL** | 9.2+ | ✅ 完整支持 | 字符串字段使用LIKE，JSON字段使用@>操作符 | ✅ 完全支持 | ✅ `~` 操作符 |
| **SQLite** | 3.38.0+ | ✅ 基础支持 | 仅字符串字段支持LIKE操作 | ❌ 不支持 | ❌ 不支持 |
| **MongoDB** | 7.0+ | ✅ 原生支持 | 原生$regex操作符 | ✅ 完全支持 | ✅ 原生 $regex 操作符 |

⚠️ **版本兼容性注意事项**：
- MySQL 5.6及以下版本不支持JSON_CONTAINS函数，会导致运行时错误
- PostgreSQL早期版本可能需要启用JSON扩展
- SQLite JSON功能是可选的，需要在编译时启用

#### 🔍 正则表达式查询（Regex）

rat_quickdb 支持跨数据库的正则表达式查询，使用 `QueryOperator::Regex` 操作符：

**数据库特定实现**：

| 数据库 | 操作符 | 语法示例 | 状态 |
|--------|--------|----------|------|
| **PostgreSQL** | `~` | `WHERE field ~ 'pattern'` | ✅ 完全支持 |
| **MySQL** | `REGEXP` | `WHERE field REGEXP 'pattern'` | ✅ 完全支持 |
| **MongoDB** | `$regex` | `{ field: { $regex: 'pattern' } }` | ✅ 完全支持 |
| **SQLite** | - | - | ❌ 不支持 |

**使用示例**：

```rust
use rat_quickdb::*;

// 正则表达式查询：匹配包含 "_wang" 或 "_chen" 的用户名
let conditions = vec![QueryCondition {
    field: "username".to_string(),
    operator: QueryOperator::Regex,
    value: DataValue::String(".*_wang|_chen.*".to_string()),
}];

let users = ModelManager::<User>::find(conditions, None).await?;
```

**运行正则表达式查询示例**：

```bash
# PostgreSQL 正则表达式查询
cargo run --example string_fuzzy_search_pgsql --features postgres-support

# MySQL 正则表达式查询
cargo run --example string_fuzzy_search_mysql --features mysql-support

# MongoDB 正则表达式查询
cargo run --example string_fuzzy_search_mongodb --features mongodb-support

# SQLite（不支持正则表达式）
cargo run --example string_fuzzy_search_sqlite --features sqlite-support
```

**注意事项**：
- ❌ **SQLite 不支持 REGEXP**：SQLite 不支持正则表达式查询
- ✅ **PostgreSQL 使用 `~` 操作符**：框架已自动适配 PostgreSQL 的正则表达式语法
- ✅ **MySQL 使用 `REGEXP` 函数**：框架已自动适配 MySQL 的正则表达式语法
- ✅ **MongoDB 使用 `$regex` 操作符**：框架已自动适配 MongoDB 的正则表达式语法

**⚠️ 跨数据库兼容性建议**：
> **重要**：如果您的应用需要跨数据库支持（特别是需要支持 SQLite），**强烈建议不要使用正则表达式查询**。请改用以下替代方案：
> - **Contains 操作符**：用于简单的包含匹配（`LIKE '%value%'`）
> - **StartsWith 操作符**：用于前缀匹配（`LIKE 'value%'`）
> - **EndsWith 操作符**：用于后缀匹配（`LIKE '%value'`）
> - **多个 OR 条件**：用于复杂的模式匹配（例如：`field = 'value1' OR field = 'value2'`）
>
> 这些替代方案在所有数据库中都能正常工作，确保应用的跨数据库兼容性。

**性能建议**：
- 正则表达式查询比精确匹配和模糊匹配（LIKE）慢
- 建议为常用正则查询字段创建索引
- 复杂的正则表达式可能影响查询性能，请谨慎使用

#### 按需启用特性

**仅使用SQLite**:
```toml
[dependencies]
rat_quickdb = { version = "0.5.1", features = ["sqlite-support"] }
```

**使用PostgreSQL**:
```toml
[dependencies]
rat_quickdb = { version = "0.5.1", features = ["postgres-support"] }
```

**使用所有数据库**:
```toml
[dependencies]
rat_quickdb = { version = "0.5.1", features = ["full"] }
```

**L2缓存配置注意事项**:
- L2缓存功能已内置在 `rat_memcache` 中，无需额外特性
- L2缓存需要磁盘空间用于缓存持久化
- 配置示例见下面的"缓存配置"部分

#### 运行示例

不同的示例需要不同的特性支持：

```bash
# 基础模型定义示例
cargo run --example model_definition --features sqlite-support

# 复杂查询示例
cargo run --example complex_query_demo --features sqlite-support

# 分页查询示例
cargo run --example model_pagination_demo --features sqlite-support

# 特殊类型测试示例
cargo run --example special_types_test --features sqlite-support

# ID生成策略示例
cargo run --example id_strategy_test --features sqlite-support

# 手动表管理示例
cargo run --example manual_table_management --features sqlite-support

# 其他数据库示例
cargo run --example model_definition_mysql --features mysql-support
cargo run --example model_definition_pgsql --features postgres-support
cargo run --example model_definition_mongodb --features mongodb-support

# 缓存绕过性能测试示例
cargo run --example cache_bypass_comparison_mysql --features mysql-support
cargo run --example cache_bypass_comparison_pgsql --features postgres-support
cargo run --example cache_bypass_comparison_sqlite --features sqlite-support
cargo run --example cache_bypass_comparison_mongodb --features mongodb-support
```

## ⚠️ 重要架构说明

### ODM层使用要求 (v0.3.0+)

**从v0.3.0版本开始，强制使用define_model!宏定义模型，不再允许使用普通结构体进行数据库操作。**

所有数据库操作必须通过以下方式：

1. **推荐：使用模型API**
```rust
use rat_quickdb::*;
use rat_quickdb::ModelOperations;

// 定义模型
define_model! {
    struct User {
        id: String,
        username: String,
        email: String,
    }
    // ... 字段定义
}

// 创建和保存
let user = User {
    id: String::new(), // 框架自动生成ID
    username: "张三".to_string(),
    email: "zhangsan@example.com".to_string(),
};
let user_id = user.save().await?;

// 查询
let found_user = ModelManager::<User>::find_by_id(&user_id).await?;
```

2. **备选：使用ODM API**
```rust
use rat_quickdb::*;

// 通过add_database添加数据库配置
let config = DatabaseConfig::builder()
    .db_type(DatabaseType::SQLite)
    .connection(ConnectionConfig::SQLite {
        path: "test.db".to_string(),
        create_if_missing: true,
    })
    .alias("main".to_string())
    .build()?;
add_database(config).await?;

// 使用ODM操作数据库
let mut user_data = HashMap::new();
user_data.insert("username".to_string(), DataValue::String("张三".to_string()));
create("users", user_data, Some("main")).await?;
```

3. **禁止的用法**
```rust
// ❌ 错误：不再允许直接访问连接池管理器
// let pool_manager = get_global_pool_manager();
// let pool = pool_manager.get_connection_pools().get("main");
```

这种设计确保了：
- **架构完整性**: 统一的数据访问层
- **安全性**: 防止直接操作底层连接池导致的资源泄漏
- **一致性**: 所有操作都经过相同的ODM层处理
- **维护性**: 统一的错误处理和日志记录

## 📋 从旧版本升级

### 从 v0.2.x 升级到 v0.3.0

v0.3.0 是一个重大变更版本，包含破坏性更改。请查看详细的[迁移指南](MIGRATION_GUIDE_0_3_0.md)。

**主要变更**：
- ✅ 强制使用 `define_model!` 宏定义模型
- ✅ 消除动态表结构推断的"保姆设置"问题
- ✅ 提供更明确的类型安全和字段定义
- ✅ 修复重大架构Bug

### 从 v0.3.1 升级到 v0.3.2+

**🚨 破坏性变更：便捷函数必须显式指定ID策略**

从v0.3.2版本开始，所有数据库配置的便捷函数（`sqlite_config`、`postgres_config`、`mysql_config`、`mongodb_config`）现在要求必须显式传入`id_strategy`参数。

**变更原因**：
- 消除硬编码的"保姆设置"，确保用户完全控制ID生成策略
- 所有数据库统一默认使用`AutoIncrement`策略
- 避免不同数据库有不同默认策略导致的混淆

**API变更**：
```rust
// v0.3.1及之前（已移除）
let config = sqlite_config("sqlite_db", "./test.db", pool_config)?;

// v0.3.2+（新API）
let config = sqlite_config(
    "sqlite_db",
    "./test.db",
    pool_config,
    Some(IdStrategy::AutoIncrement)  // 必须显式指定
)?;
```

**迁移指南**：
1. **推荐方式**：迁移到构建器模式，获得更好的类型安全性和一致性
```rust
// 推荐使用构建器模式替代便捷函数：
let config = DatabaseConfig::builder()
    .db_type(DatabaseType::SQLite)
    .connection(ConnectionConfig::SQLite {
        path: "./test.db".to_string(),
        create_if_missing: true,
    })
    .pool_config(pool_config)
    .alias("sqlite_db".to_string())
    .id_strategy(IdStrategy::AutoIncrement)
    .build()?;

// PostgreSQL使用UUID（PostgreSQL推荐）
let config = DatabaseConfig::builder()
    .db_type(DatabaseType::PostgreSQL)
    .connection(ConnectionConfig::PostgreSQL {
        host: "localhost".to_string(),
        port: 5432,
        database: "mydatabase".to_string(),
        username: "username".to_string(),
        password: "password".to_string(),
    })
    .pool_config(pool_config)
    .alias("postgres_db".to_string())
    .id_strategy(IdStrategy::Uuid)
    .build()?;
```

2. **临时兼容**：如果必须暂时维护现有代码，请添加必需的`IdStrategy`参数，但尽快规划迁移到构建器模式

**影响范围**：
- 所有使用便捷函数配置数据库的代码
- 使用`mongodb_config_with_builder`的代码（已移除重复函数）
- 依赖特定数据库默认ID策略的应用

这个变更确保了配置的一致性和用户控制权，符合"不做保姆设置"的设计原则。

## 🚀 快速开始

### 基本使用

查看 `examples/model_definition.rs` 了解完整的模型定义和使用方法。

### ID生成策略示例

查看 `examples/id_strategy_test.rs` 了解不同ID生成策略的使用方法。

### 数据库适配器示例

- **SQLite**: `examples/model_definition.rs` (运行时使用 `--features sqlite-support`)
- **PostgreSQL**: `examples/model_definition_pgsql.rs`
- **MySQL**: `examples/model_definition_mysql.rs`
- **MongoDB**: `examples/model_definition_mongodb.rs`

### 模型定义（推荐方式）

查看 `examples/model_definition.rs` 了解完整的模型定义、CRUD操作和复杂查询示例。

### 字段类型和验证

查看 `examples/model_definition.rs` 中包含的字段类型定义和验证示例。

### 索引管理

索引会根据模型定义自动创建，无需手动管理。参考 `examples/model_definition.rs` 了解索引定义方式。

## 🔒 SQLite布尔值兼容性

SQLite数据库将布尔值存储为整数（0和1），这可能导致serde反序列化错误。rat_quickdb提供了多种解决方案：

### 方案1: sqlite_bool_field() - 推荐（零配置）

```rust
use rat_quickdb::*;

rat_quickdb::define_model! {
    struct User {
        id: Option<i32>,
        username: String,
        is_active: bool,        // 自动SQLite兼容
        is_pinned: bool,        // 自动SQLite兼容
        is_verified: bool,      // 自动SQLite兼容
    }

    collection = "users",
    fields = {
        id: integer_field(None, None),
        username: string_field(Some(50), Some(3), None).required(),
        // 使用sqlite_bool_field() - 自动处理SQLite布尔值兼容性
        is_active: sqlite_bool_field(),
        is_pinned: sqlite_bool_field(),
        is_verified: sqlite_bool_field_with_default(false),
    }
}
```

### 方案2: 手动serde属性 + 通用反序列化器

```rust
use rat_quickdb::*;
use serde::Deserialize;

#[derive(Debug, Serialize, Deserialize)]
struct User {
    id: Option<i32>,
    username: String,

    // 手动指定反序列化器
    #[serde(deserialize_with = "rat_quickdb::sqlite_bool::deserialize_bool_from_any")]
    is_active: bool,

    #[serde(deserialize_with = "rat_quickdb::sqlite_bool::deserialize_bool_from_int")]
    is_pinned: bool,
}

rat_quickdb::define_model! {
    struct User {
        id: Option<i32>,
        username: String,
        is_active: bool,
        is_pinned: bool,
    }

    collection = "users",
    fields = {
        id: integer_field(None, None),
        username: string_field(Some(50), Some(3), None).required(),
        // 使用传统boolean_field() - 配合手动serde属性
        is_active: boolean_field(),
        is_pinned: boolean_field(),
    }
}
```

### 方案3: 传统方式（需要手动处理）

```rust
// 对于已有代码，可以使用传统boolean_field()
// 但需要确保数据源中的布尔值格式正确
rat_quickdb::define_model! {
    struct User {
        id: Option<i32>,
        username: String,
        is_active: bool,        // 需要手动处理兼容性
    }

    collection = "users",
    fields = {
        id: integer_field(None, None),
        username: string_field(Some(50), Some(3), None).required(),
        is_active: boolean_field(),  // 传统方式
    }
}
```

### 反序列化器选择指南

- `deserialize_bool_from_any()`: 支持整数、布尔值、字符串 "true"/"false"
- `deserialize_bool_from_int()`: 支持整数和布尔值
- `sqlite_bool_field()`: 自动选择最佳反序列化器

### 迁移指南

从传统`boolean_field()`迁移到`sqlite_bool_field()`：

```rust
// 之前（可能有兼容性问题）
is_active: boolean_field(),

// 之后（完全兼容）
is_active: sqlite_bool_field(),
```

## 🆔 ID生成策略

rat_quickdb支持多种ID生成策略，满足不同场景的需求：

### AutoIncrement（自增ID）- 默认推荐
```rust
DatabaseConfig::builder()
    .id_strategy(IdStrategy::AutoIncrement)
    .build()?

// 便捷函数使用
let config = sqlite_config(
    "sqlite_db",
    "./test.db",
    pool_config,
    Some(IdStrategy::AutoIncrement)
)?;
```

### UUID（通用唯一标识符）- PostgreSQL推荐
```rust
DatabaseConfig::builder()
    .id_strategy(IdStrategy::Uuid)
    .build()?

// 便捷函数使用
let config = postgres_config(
    "postgres_db",
    "localhost",
    5432,
    "mydatabase",
    "username",
    "password",
    pool_config,
    Some(IdStrategy::Uuid)
)?;
```

#### ⚠️ PostgreSQL UUID策略特殊要求

**重要提醒**：PostgreSQL对类型一致性有严格要求，如果使用UUID策略：

1. **主键表**：ID字段将为UUID类型
2. **关联表**：所有外键字段也必须为UUID类型
3. **类型匹配**：不允许UUID类型与其他类型进行关联

**示例**：
```rust
// 用户表使用UUID ID
define_model! {
    struct User {
        id: String,  // 将映射为PostgreSQL UUID类型
        username: String,
    }
    collection = "users",
    fields = {
        id: uuid_field(),
        username: string_field(Some(50), Some(3), None).required(),
    }
}

// 订单表的外键也必须使用UUID类型
define_model! {
    struct Order {
        id: String,
        user_id: String,  // 必须为UUID类型以匹配users.id
        amount: f64,
    }
    collection = "orders",
    fields = {
        id: uuid_field(),
        user_id: uuid_field().required(),  // 外键必须使用相同类型
        amount: float_field(None, None),
    }
}
```

**解决方案**：
- 对于新项目：PostgreSQL推荐全面使用UUID策略
- 对于现有项目：可以使用`IdStrategy::Custom`手动生成UUID字符串作为兼容方案
- 混合策略：主表使用UUID，关联表也必须使用UUID，保持类型一致性

#### ✨ PostgreSQL UUID自动转换功能

从v0.3.4版本开始，PostgreSQL适配器支持UUID字段的**自动转换**，让用户可以使用字符串UUID进行查询操作。

**功能特点**：
- **自动转换**：查询时传入字符串UUID，适配器自动转换为UUID类型
- **严格验证**：无效UUID格式直接报错，不做容错修复
- **用户友好**：保持API一致性，无需手动转换UUID类型
- **类型安全**：确保数据库层面的UUID类型一致性

**使用示例**：
```rust
// 用户模型定义（注意：结构体中用String，字段定义中用uuid_field）
define_model! {
    struct User {
        id: String,  // ⚠️ 结构体中必须使用String
        username: String,
    }
    collection = "users",
    fields = {
        id: uuid_field(),  // ⚠️ 字段定义中必须使用uuid_field
        username: string_field(Some(50), Some(3), None).required(),
    }
}

// 文章模型，author_id为UUID外键
define_model! {
    struct Article {
        id: String,
        title: String,
        author_id: String,  // ⚠️ 结构体中必须使用String
    }
    collection = "articles",
    fields = {
        id: uuid_field(),
        title: string_field(Some(200), Some(1), None).required(),
        author_id: uuid_field().required(),  // ⚠️ 字段定义中必须使用uuid_field
    }
}

// 查询：直接使用字符串UUID，自动转换！
let conditions = vec![
    QueryCondition {
        field: "author_id".to_string(),
        operator: QueryOperator::Eq,
        value: DataValue::String("550e8400-e29b-41d4-a716-446655440000".to_string()),
    }
];

let articles = ModelManager::<Article>::find(conditions, None).await?;
// PostgreSQL适配器自动将字符串转换为UUID类型进行查询
```

#### ⚠️ 反直觉的设计要求（重要！）

**当前限制**：使用UUID策略时，模型定义存在一个**反直觉**的设计要求：

```rust
define_model! {
    struct User {
        id: String,           // ⚠️ 结构体中必须使用String类型
        // 不能写成：id: uuid::Uuid
    }
    fields = {
        id: uuid_field(),     // ⚠️ 但字段定义中必须使用uuid_field()
        // 不能写成：id: string_field(...)
    }
}
```

**为什么会这样？**
1. **Rust类型系统限制**：宏系统在生成模型时需要统一的基础类型
2. **数据库类型映射**：`uuid_field()`告诉适配器创建UUID数据库列
3. **查询转换**：运行时字符串UUID自动转换为UUID数据库类型

**正确用法**：
- ✅ **结构体字段**：始终使用`String`类型
- ✅ **字段定义**：UUID字段使用`uuid_field()`，其他字段使用对应函数
- ✅ **查询操作**：直接使用`DataValue::String("uuid-string")`，自动转换
- ✅ **类型安全**：PostgreSQL数据库层面保持UUID类型一致性

**错误用法**：
- ❌ 结构体中使用`uuid::Uuid`类型（编译错误）
- ❌ UUID字段使用`string_field()`定义（失去UUID类型支持）
- ❌ 混用不同数据库的UUID策略（类型不匹配）

**暂时无法解决的原因**：
- Rust宏系统的类型推导限制
- 需要保持与现有代码的向后兼容
- 跨数据库的统一API设计要求

**未来改进方向**：
- v0.4.0计划引入更直观的类型安全的UUID字段定义
- 考虑使用编译时类型推导减少这种不一致性
- 提供更清晰的编译时错误提示

#### 🔍 UUID 字段查询：跨数据库注意事项

**重要**：不同数据库中 `uuid_field()` 的查询方式不同

| 数据库 | 存储类型 | 查询时使用的 DataValue | 说明 |
|--------|----------|----------------------|------|
| **PostgreSQL** | 原生 UUID | `DataValue::String` | 框架自动转换为 UUID 类型 |
| **MongoDB** | String | `DataValue::String` | ⚠️ 必须使用字符串，**不可用** `DataValue::Uuid` |
| **MySQL** | String | `DataValue::String` | ⚠️ 必须使用字符串，**不可用** `DataValue::Uuid` |
| **SQLite** | String | `DataValue::String` | ⚠️ 必须使用字符串，**不可用** `DataValue::Uuid` |

**错误示例**：
```rust
// ❌ 错误：在 MongoDB/MySQL/SQLite 中使用 DataValue::Uuid
let conditions = vec![QueryCondition {
    field: "account_id".to_string(),
    operator: QueryOperator::Eq,
    value: DataValue::Uuid(uuid),  // 在 MongoDB/MySQL/SQLite 中查询不到结果！
}];
```

**正确示例**：
```rust
// ✅ 正确：统一使用 DataValue::String
let uuid_str = "550e8400-e29b-41d4-a716-446655440000";
let conditions = vec![QueryCondition {
    field: "account_id".to_string(),
    operator: QueryOperator::Eq,
    value: DataValue::String(uuid_str.to_string()),  // 所有数据库都支持
}];
```

**设计原因**：
- MongoDB/MySQL/SQLite 没有原生的 UUID 类型，`uuid_field()` 在这些数据库中存储为字符串
- 为保持 API 一致性，所有数据库的 UUID 字段查询统一使用 `DataValue::String`
- PostgreSQL 适配器会自动将字符串转换为其原生的 UUID 类型

### Snowflake（雪花算法）
```rust
DatabaseConfig::builder()
    .id_strategy(IdStrategy::Snowflake {
        machine_id: 1,
        datacenter_id: 1
    })
    .build()?
```

### ObjectId（MongoDB风格）
```rust
DatabaseConfig::builder()
    .id_strategy(IdStrategy::ObjectId)
    .build()?
```

### Custom（自定义前缀）
```rust
DatabaseConfig::builder()
    .id_strategy(IdStrategy::Custom("user_".to_string()))
    .build()?
```

## 🔄 ObjectId跨数据库处理

rat_quickdb为ObjectId策略提供了跨数据库的一致性处理，确保在不同数据库后端都能正常工作。

### 存储方式差异

**MongoDB**：
- 存储为原生`ObjectId`类型
- 查询时返回MongoDB原生ObjectId对象
- 性能最优，支持MongoDB所有ObjectId特性

**其他数据库（SQLite、PostgreSQL、MySQL）**：
- 存储为24位十六进制字符串（如：`507f1f77bcf86cd799439011`）
- 查询时返回字符串格式的ObjectId
- 保持与MongoDB ObjectId格式的兼容性

### 使用示例

```rust
// MongoDB - 原生ObjectId支持
let config = mongodb_config(
    "mongodb_db",
    "localhost",
    27017,
    "mydatabase",
    Some("username"),
    Some("password"),
    pool_config,
    Some(IdStrategy::ObjectId)
)?;

// SQLite/PostgreSQL/MySQL - 字符串格式ObjectId
let config = sqlite_config(
    "sqlite_db",
    "./test.db",
    pool_config,
    Some(IdStrategy::ObjectId)
)?;
```

### 模型定义

ObjectId策略在模型定义中统一使用`String`类型：

```rust
define_model! {
    struct Document {
        id: String,  // MongoDB为ObjectId，其他数据库为字符串
        title: String,
        content: String,
    }
    collection = "documents",
    fields = {
        id: string_field(None, None),  // 统一使用string_field
        title: string_field(Some(200), Some(1), None).required(),
        content: string_field(Some(10000), None, None),
    }
}
```

### 查询和操作

```rust
// 创建文档
let doc = Document {
    id: String::new(),  // 自动生成ObjectId
    title: "示例文档".to_string(),
    content: "文档内容".to_string(),
};
let doc_id = doc.save().await?;

// 查询文档
let found_doc = ModelManager::<Document>::find_by_id(&doc_id).await?;

// 注意：ObjectId为24位十六进制字符串格式
assert_eq!(doc_id.len(), 24);  // 其他数据库
// 在MongoDB中，这将是一个原生ObjectId对象
```

### 类型转换处理

rat_quickdb自动处理ObjectId在不同数据库中的类型转换：

1. **保存时**：自动生成ObjectId格式（字符串或原生对象）
2. **查询时**：保持原格式返回，框架内部处理转换
3. **迁移时**：数据格式在不同数据库间保持兼容

### 性能考虑

- **MongoDB**：原生ObjectId性能最优，支持索引优化
- **其他数据库**：字符串索引性能良好，长度固定（24字符）
- **跨数据库**：统一的字符串格式便于数据迁移和同步

这种设计确保了ObjectId策略在所有支持的数据库中都能一致工作，同时充分利用各数据库的原生特性。

## 🧠 缓存配置与缓存绕过

rat_quickdb提供了灵活的缓存管理功能，包括智能缓存和缓存绕过机制。

### 缓存绕过功能

在某些场景下（如金融交易、实时数据查询），您可能需要强制从数据库获取最新数据，绕过缓存。rat_quickdb提供了 `find_with_cache_control` 方法来满足这一需求：

```rust
use rat_quickdb::ModelOperations;

// 正常查询（使用缓存）
let cached_results = ModelManager::<User>::find(conditions, None).await?;

// 强制跳过缓存查询
let fresh_results = ModelManager::<User>::find_with_cache_control(
    conditions,
    None,
    true  // bypass_cache = true
).await?;
```

**适用场景**：
- 🏦 **金融交易**：确保获取最新的账户余额和交易记录
- 📊 **实时数据**：股票价格、实时监控数据等
- 🔍 **数据一致性**：在数据更新后立即验证结果
- 🧪 **测试场景**：需要绕过缓存进行基准测试

### 缓存绕过性能对比示例

rat_quickdb提供了完整的缓存绕过性能测试示例：

```bash
# MySQL 缓存绕过测试
cargo run --example cache_bypass_comparison_mysql --features mysql-support

# PostgreSQL 缓存绕过测试
cargo run --example cache_bypass_comparison_pgsql --features postgres-support

# SQLite 缓存绕过测试
cargo run --example cache_bypass_comparison_sqlite --features sqlite-support

# MongoDB 缓存绕过测试
cargo run --example cache_bypass_comparison_mongodb --features mongodb-support
```

**性能提升示例**（实际测试结果）：
- MySQL：16x 性能提升
- PostgreSQL：2.25x 性能提升
- MongoDB：1.88x 性能提升（重复查询），20x 提升（批量查询）

### 基本缓存配置（仅L1内存缓存）
```rust
use rat_quickdb::types::{CacheConfig, CacheStrategy, TtlConfig, L1CacheConfig};

let cache_config = CacheConfig {
    enabled: true,
    strategy: CacheStrategy::Lru,
    ttl_config: TtlConfig {
        default_ttl_secs: 300,  // 5分钟缓存
        max_ttl_secs: 3600,     // 最大1小时
        check_interval_secs: 60, // 检查间隔
    },
    l1_config: L1CacheConfig {
        max_capacity: 1000,     // 最多1000个条目
        max_memory_mb: 64,       // 64MB内存限制
        enable_stats: true,      // 启用统计
    },
    l2_config: None,           // 不使用L2磁盘缓存
    compression_config: CompressionConfig::default(),
    version: "1".to_string(),
};

DatabaseConfig::builder()
    .cache(cache_config)
    .build()?
```

### L1+L2缓存配置（内置L2缓存支持）
```rust
use rat_quickdb::types::{CacheConfig, CacheStrategy, TtlConfig, L1CacheConfig, L2CacheConfig};
use std::path::PathBuf;

let cache_config = CacheConfig {
    enabled: true,
    strategy: CacheStrategy::Lru,
    ttl_config: TtlConfig {
        default_ttl_secs: 1800, // 30分钟缓存
        max_ttl_secs: 7200,     // 最大2小时
        check_interval_secs: 120, // 检查间隔
    },
    l1_config: L1CacheConfig {
        max_capacity: 5000,     // 最多5000个条目
        max_memory_mb: 128,      // 128MB内存限制
        enable_stats: true,      // 启用统计
    },
    l2_config: Some(L2CacheConfig {
        max_size_mb: 1024,      // 1GB磁盘缓存
        cache_dir: PathBuf::from("./cache"), // 缓存目录
        enable_persistence: true, // 启用持久化
        enable_compression: true, // 启用压缩
        cleanup_interval_secs: 300, // 清理间隔
    }),
    compression_config: CompressionConfig::default(),
    version: "1".to_string(),
};

DatabaseConfig::builder()
    .cache(cache_config)
    .build()?
```

**L2缓存特性说明**:
- L2缓存功能已内置在 `rat_memcache` 中，无需额外特性
- 需要磁盘空间存储缓存数据
- 适合缓存大量数据或需要持久化的场景
- 只需在 `CacheConfig` 中配置 `l2_config` 即可启用L2缓存

### 缓存统计和管理
```rust
// 获取缓存统计信息
let stats = get_cache_stats("default").await?;
println!("缓存命中率: {:.2}%", stats.hit_rate * 100.0);
println!("缓存条目数: {}", stats.entries);

// 清理缓存
clear_cache("default").await?;
clear_all_caches().await?;
```

## 📝 日志控制

rat_quickdb现在完全由调用者控制日志初始化：

```rust
use rat_logger::{Logger, LoggerBuilder, LevelFilter};

// 调用者负责初始化日志系统
let logger = LoggerBuilder::new()
    .with_level(LevelFilter::Debug)
    .with_file("app.log")
    .build();

logger.init().expect("日志初始化失败");

// 然后初始化rat_quickdb（不再自动初始化日志）
rat_quickdb::init();
```

## 🔧 数据库配置

### 推荐方式：使用构建器模式

**推荐**：使用`DatabaseConfig::builder()`模式，提供完整的配置控制和类型安全：

```rust
use rat_quickdb::*;
use rat_quickdb::types::{DatabaseType, ConnectionConfig, PoolConfig, IdStrategy};

let pool_config = PoolConfig::builder()
    .max_connections(10)
    .min_connections(2)
    .connection_timeout(30)        // 秒
    .idle_timeout(300)             // 秒
    .max_lifetime(1800)            // 秒
    .max_retries(3)
    .retry_interval_ms(1000)
    .keepalive_interval_sec(60)
    .health_check_timeout_sec(10)
    .build()?;

// SQLite 配置
let sqlite_config = DatabaseConfig::builder()
    .db_type(DatabaseType::SQLite)
    .connection(ConnectionConfig::SQLite {
        path: "./test.db".to_string(),
        create_if_missing: true,
    })
    .pool_config(pool_config.clone())
    .alias("sqlite_db".to_string())
    .id_strategy(IdStrategy::AutoIncrement)  // 推荐策略
    .build()?;

// PostgreSQL 配置
let postgres_config = DatabaseConfig::builder()
    .db_type(DatabaseType::PostgreSQL)
    .connection(ConnectionConfig::PostgreSQL {
        host: "localhost".to_string(),
        port: 5432,
        database: "mydatabase".to_string(),
        username: "username".to_string(),
        password: "password".to_string(),
    })
    .pool_config(pool_config.clone())
    .alias("postgres_db".to_string())
    .id_strategy(IdStrategy::Uuid)  // PostgreSQL推荐UUID策略
    .build()?;

// MySQL 配置
let mysql_config = DatabaseConfig::builder()
    .db_type(DatabaseType::MySQL)
    .connection(ConnectionConfig::MySQL {
        host: "localhost".to_string(),
        port: 3306,
        database: "mydatabase".to_string(),
        username: "username".to_string(),
        password: "password".to_string(),
    })
    .pool_config(pool_config.clone())
    .alias("mysql_db".to_string())
    .id_strategy(IdStrategy::AutoIncrement)  // MySQL推荐自增策略
    .build()?;

// MongoDB 配置
let mongodb_config = DatabaseConfig::builder()
    .db_type(DatabaseType::MongoDB)
    .connection(ConnectionConfig::MongoDB(
        MongoDbConnectionBuilder::new("localhost", 27017, "mydatabase")
            .with_auth("username", "password")
            .build()
    ))
    .pool_config(pool_config)
    .alias("mongodb_db".to_string())
    .id_strategy(IdStrategy::ObjectId)  // MongoDB推荐ObjectId策略
    .build()?;

// 添加到连接池管理器
add_database(sqlite_config).await?;
add_database(postgres_config).await?;
add_database(mysql_config).await?;
add_database(mongodb_config).await?;
```

### 高级MongoDB配置

```rust
use rat_quickdb::*;
use rat_quickdb::types::{TlsConfig, ZstdConfig};

let tls_config = TlsConfig {
    enabled: true,
    verify_server_cert: false,
    verify_hostname: false,
    ..Default::default()
};

let zstd_config = ZstdConfig {
    enabled: true,
    compression_level: Some(3),
    compression_threshold: Some(1024),
};

let mongodb_builder = MongoDbConnectionBuilder::new("localhost", 27017, "mydatabase")
    .with_auth("username", "password")
    .with_auth_source("admin")
    .with_direct_connection(true)
    .with_tls_config(tls_config)
    .with_zstd_config(zstd_config);

let advanced_mongodb_config = DatabaseConfig::builder()
    .db_type(DatabaseType::MongoDB)
    .connection(ConnectionConfig::MongoDB(mongodb_builder))
    .pool_config(pool_config)
    .alias("advanced_mongodb".to_string())
    .id_strategy(IdStrategy::ObjectId)
    .build()?;

add_database(advanced_mongodb_config).await?;
```

### 🚨 即将废弃的便捷函数（不推荐使用）

> **重要警告**：以下便捷函数已标记为废弃，将在v0.4.0版本中移除。请使用上面推荐的构建器模式。

```rust
// 🚨 即将废弃 - 请勿在新项目中使用
// 这些函数存在API不一致性和硬编码问题

// 废弃的SQLite配置
let config = sqlite_config(  // 🚨 即将废弃
    "sqlite_db",
    "./test.db",
    pool_config,
    Some(IdStrategy::AutoIncrement)  // 必须显式指定
)?;

// 废弃的PostgreSQL配置
let config = postgres_config(  // 🚨 即将废弃
    "postgres_db",
    "localhost",
    5432,
    "mydatabase",
    "username",
    "password",
    pool_config,
    Some(IdStrategy::Uuid)
)?;

// 废弃的MySQL配置
let config = mysql_config(  // 🚨 即将废弃
    "mysql_db",
    "localhost",
    3306,
    "mydatabase",
    "username",
    "password",
    pool_config,
    Some(IdStrategy::AutoIncrement)
)?;

// 废弃的MongoDB配置
let config = mongodb_config(  // 🚨 即将废弃
    "mongodb_db",
    "localhost",
    27017,
    "mydatabase",
    Some("username"),
    Some("password"),
    pool_config,
    Some(IdStrategy::ObjectId)
)?;
```

**废弃原因**：
- ❌ API不一致性：不同数据库的便捷函数参数不统一
- ❌ 硬编码默认值：违背"不做保姆设置"的设计原则
- ❌ 功能限制：无法支持高级配置选项
- ❌ 维护困难：重复代码增加维护成本

**推荐替代方案**：
- ✅ **构建器模式**：类型安全、配置完整、API统一
- ✅ **完全控制**：用户完全控制所有配置选项
- ✅ **扩展性强**：支持所有数据库的高级特性
- ✅ **类型安全**：编译时检查配置正确性

### ID策略推荐

根据数据库特性选择最适合的ID策略：

| 数据库 | 推荐策略 | 备选策略 | 说明 |
|--------|----------|----------|------|
| **SQLite** | AutoIncrement | ObjectId | AutoIncrement原生支持，性能最佳 |
| **PostgreSQL** | UUID | AutoIncrement | UUID原生支持，类型安全 |
| **MySQL** | AutoIncrement | ObjectId | AutoIncrement原生支持，性能最佳 |
| **MongoDB** | ObjectId | AutoIncrement | ObjectId原生支持，MongoDB生态标准 |

**重要提醒**：PostgreSQL使用UUID策略时，所有关联表的外键字段也必须使用UUID类型以保持类型一致性。

## 🛠️ 核心API

### 数据库管理
- `init()` - 初始化库
- `add_database(config)` - 添加数据库配置
- `remove_database(alias)` - 移除数据库配置
- `get_aliases()` - 获取所有数据库别名
- `set_default_alias(alias)` - 设置默认数据库别名

### 模型操作（推荐）
```rust
// 保存记录
let user_id = user.save().await?;

// 查询记录
let found_user = ModelManager::<User>::find_by_id(&user_id).await?;
let users = ModelManager::<User>::find(conditions, options).await?;

// 缓存绕过查询（适用于实时数据场景）
let users = ModelManager::<User>::find_with_cache_control(
    conditions,
    options,
    true  // bypass_cache = true, 强制跳过缓存
).await?;

// 更新记录
let mut updates = HashMap::new();
updates.insert("username".to_string(), DataValue::String("新名字".to_string()));
let updated = user.update(updates).await?;

// 删除记录
let deleted = user.delete().await?;
```

#### ⚠️ save() 与 update() 方法区别

**重要**：这两个方法有明确的职责划分，请勿混用

| 方法 | 用途 | 返回值 | 注意事项 |
|------|------|--------|----------|
| `save()` | **仅插入新数据** | 返回新记录的 **ID 字符串**（不是完整对象） | • 如果 `id` 字段为空，自动生成新 ID<br>• 如果 `id` 字段有值，使用该 ID 插入（可能主键冲突）<br>• 无论何种情况都执行 INSERT 操作<br>• 如需完整对象，请用 `find_by_id(id)` 再次查询 |
| `update()` | **仅更新已存在的记录** | 返回 `bool`（成功/失败） | • 根据实例的 `id` 定位记录<br>• 只更新参数中指定的字段<br>• 如果记录不存在会返回错误<br>• 不是 UPSERT 操作 |

**常见错误**：
- ❌ 试图用 `save()` 更新已存在的记录 → 会造成主键冲突或重复插入
- ❌ 试图用 `update()` 插入新记录 → 会因记录不存在而失败

**正确用法**：
```rust
// ✅ 插入新记录
let new_user = User { /* ... */ };
let user_id = new_user.save().await?;  // 返回 ID 字符串
let complete_user = ModelManager::<User>::find_by_id(&user_id).await?.unwrap();

// ✅ 更新已存在的记录
let mut updates = HashMap::new();
updates.insert("username".to_string(), DataValue::String("新名字".to_string()));
complete_user.update(updates).await?;  // 返回 bool
```

#### ⚠️ DataValue 类型匹配要求

**重要**：更新和查询时，DataValue 类型必须与字段类型严格匹配

| 字段类型 | 必须使用的 DataValue | 错误用法 |
|---------|---------------------|----------|
| `integer_field()` | `DataValue::Int` | ❌ `DataValue::String` |
| `float_field()` | `DataValue::Float` | ❌ `DataValue::String` |
| `datetime_field()` | `DataValue::DateTimeUTC` | ❌ `DataValue::String` |
| `boolean_field()` | `DataValue::Bool` | ❌ `DataValue::Int` |

**常见错误**：将所有值都转换为字符串 `DataValue::String(value.to_string())`

**正确示例**：
```rust
let mut updates = HashMap::new();
updates.insert("count".to_string(), DataValue::Int(100));  // ✅ 整数用 Int
updates.insert("price".to_string(), DataValue::Float(99.99));  // ✅ 浮点用 Float
updates.insert("name".to_string(), DataValue::String("test".to_string()));  // ✅ 字符串用 String
updates.insert("active".to_string(), DataValue::Bool(true));  // ✅ 布尔用 Bool
updates.insert("updated_at".to_string(), DataValue::DateTimeUTC(chrono::Utc::now()));  // ✅ 时间用 DateTimeUTC
model.update(updates).await?;
```

📖 **详细说明**：[DataValue 类型使用指南](docs/datatype_guide.md)

### ODM操作（底层接口）
- `create(collection, data, alias)` - 创建记录
- `find_by_id(collection, id, alias)` - 根据ID查找
- `find(collection, conditions, options, alias)` - 查询记录
- `update(collection, id, data, alias)` - 更新记录
- `delete(collection, id, alias)` - 删除记录
- `count(collection, query, alias)` - 计数
- `exists(collection, query, alias)` - 检查是否存在

## 🏗️ 架构特点

rat_quickdb采用现代化架构设计：

1. **无锁队列架构**: 避免直接持有数据库连接的生命周期问题
2. **模型自动注册**: 首次使用时自动注册模型元数据
3. **自动索引管理**: 根据模型定义自动创建表和索引
4. **跨数据库适配**: 统一的接口支持多种数据库类型
5. **异步消息处理**: 基于Tokio的高效异步处理

## 🔄 工作流程

```
应用层 → 模型操作 → ODM层 → 消息队列 → 连接池 → 数据库
    ↑                                        ↓
    └────────────── 结果返回 ←────────────────┘
```

## 📊 性能特性

- **连接池管理**: 智能连接复用和管理
- **异步操作**: 非阻塞的数据库操作
- **批量处理**: 支持批量操作优化
- **缓存集成**: 内置缓存减少数据库访问
- **压缩支持**: MongoDB支持ZSTD压缩

## 🎯 支持的字段类型

- `integer_field` - 整数字段（支持范围和约束）
- `string_field` - 字符串字段（支持长度限制，可设置大长度作为长文本使用）
- `float_field` - 浮点数字段（支持范围和精度）
- `boolean_field` - 布尔字段
- `datetime_field` - 日期时间字段
- `uuid_field` - UUID字段
- `json_field` - JSON字段（支持任意JSON数据，包括对象和数组）
- `array_field` - 数组字段（支持同类型元素数组）
- `list_field` - 列表字段（array_field的别名）
- `dict_field` - ~~字典/对象字段（已废弃，请使用 json_field 替代）~~
- `reference_field` - 引用字段（外键，声明用途，实际存储为字符串）

#### 关联字段（外键）使用说明

`reference_field` 用于声明字段的外键引用关系，但在实际使用中需要配合具体的 ID 类型字段：

```rust
// ✅ 正确：使用 uuid_field 定义关联字段
define_model! {
    struct Employee {
        id: String,
        name: String,
        department_id: String,  // 关联字段
    }
    fields = {
        id: uuid_field().required().unique(),
        name: string_field(None, None, None).required(),
        department_id: uuid_field().required(),  // 使用 uuid_field，不是 reference_field
    }
}

// 查询关联字段
let conditions = vec![QueryCondition {
    field: "department_id".to_string(),
    operator: QueryOperator::Eq,
    value: DataValue::String(dept_id.to_string()),  // 所有数据库都使用 String
}];
```

**重要说明**：
- `reference_field()` 仅用于**声明**字段的引用关系，不指定存储类型
- 实际定义关联字段时，**必须使用具体的 ID 类型**（如 `uuid_field()`）
- MongoDB/MySQL/SQLite 中的 UUID 关联字段查询使用 `DataValue::String`
- PostgreSQL 中的 UUID 关联字段查询也使用 `DataValue::String`（框架自动转换）

**完整的关联查询示例**请参考：`examples/reference_query_mongodb.rs`

### ⚠️ 字段使用限制和最佳实践

为了保持系统的简洁性和性能，请遵循以下字段使用原则：

#### Array字段 - 只支持简单格式
```rust
// ✅ 推荐：使用Array字段存储简单值列表
tags: array_field(String::default()),          // ["tag1", "tag2", "tag3"]
scores: array_field(DataValue::Float(0.0)),    // [95.5, 88.0, 92.3]
user_ids: array_field(DataValue::String("")),  // ["user_123", "user_456"]

// ❌ 不推荐：存储复杂嵌套结构
complex_data: array_field(DataValue::Object(HashMap::new())), // 复杂对象
nested_arrays: array_field(DataValue::Array(vec![])),          // 嵌套数组
```

**限制说明**：
- Array字段设计用于存储简单的同类型值列表
- 不支持在Array字段内存储复杂嵌套结构（对象、嵌套数组等）
- 如需存储复杂数据，请使用专门的模型表或JSON字段
- 避免在Array字段内搜索复杂查询条件

#### JSON字段 - 支持但不推荐复杂嵌套
```rust
// ✅ 推荐：使用JSON字段存储配置信息
config: json_field(),  // {"theme": "dark", "language": "zh-CN"}
metadata: json_field(), // {"version": "1.0", "author": "张三"}

// ⚠️ 谨慎使用：深度嵌套的JSON结构
deep_nested: json_field(), // {"level1": {"level2": {"level3": {"data": "value"}}}}

// ❌ 不支持：搜索JSON字段内的数组内容
// 例如：查询 config.tags 中包含 "tag1" 的记录
```

**限制说明**：
- JSON字段支持存储复杂嵌套结构，但深度嵌套会影响查询性能
- JsonContains查询操作符**不支持搜索JSON字段内的数组内容**
- 如需数组查询功能，请使用专门的Array字段类型
- 建议JSON结构保持在3层嵌套以内

#### 设计原则
1. **简单优先**：能用简单字段就不要用复杂字段
2. **类型明确**：数组用Array字段，配置用JSON字段，对象用专门模型
3. **查询友好**：设计时考虑后续查询需求，避免无法查询的结构
4. **性能考虑**：复杂嵌套结构会显著影响查询和索引性能
5. **基础类型**：模型字段必须使用 Rust 基础类型（String、i32、f64、bool 等），禁止使用枚举类型

#### ⚠️ 禁止使用枚举类型作为模型字段
```rust
// ❌ 错误：使用枚举类型作为模型字段
#[derive(Debug, Clone, Serialize, Deserialize)]
enum UserRole {
    Admin,
    User,
    Guest,
}

define_model! {
    struct User {
        id: String,
        role: UserRole,  // 禁止使用枚举！
    }
    fields = {
        id: uuid_field().unique(),
        role: ???,  // 无法定义！
    }
}

// ✅ 正确：使用字符串类型
define_model! {
    struct User {
        id: String,
        role: String,
    }
    fields = {
        id: uuid_field().unique(),
        role: string_field(None, None, None),
    }
}
```

**原因**：
- rat_quickdb 只能处理基础数据类型（String、i32、f64、bool 等）
- 枚举类型会导致序列化/反序列化问题，查询时无法正确匹配
- 如需使用枚举，应在业务逻辑层进行转换，不要在模型定义中使用

#### 替代方案推荐
```rust
// 场景1：需要存储用户标签（使用Array字段）
define_model! {
    struct User {
        id: String,
        username: String,
        tags: Vec<String>,  // 使用Array字段，支持IN查询
    }
    fields = {
        id: string_field(None, None),
        username: string_field(Some(50), Some(3), None).required(),
        tags: array_field(DataValue::String("")),  // 简单值数组
    }
}

// 场景2：需要存储用户配置（使用JSON字段）
define_model! {
    struct User {
        id: String,
        username: String,
        config: serde_json::Value,  // 使用JSON字段，存储配置
    }
    fields = {
        id: string_field(None, None),
        username: string_field(Some(50), Some(3), None).required(),
        config: json_field(),  // 配置信息，支持JsonContains查询
    }
}

// 场景3：需要存储复杂数据关系（使用专门的模型）
define_model! {
    struct UserAddress {
        id: String,
        user_id: String,  // 外键关系
        street: String,
        city: String,
        country: String,
    }
    fields = {
        id: string_field(None, None),
        user_id: string_field(None, None).required(),
        street: string_field(Some(200), None, None).required(),
        city: string_field(Some(100), None, None).required(),
        country: string_field(Some(100), None, None).required(),
    }
}
```

遵循这些原则可以确保你的应用具有良好的性能、可维护性和查询能力。

## 🕐 时间字段处理

### UTC时间存储标准

rat_quickdb统一使用UTC时间存储所有datetime字段，确保跨时区的数据一致性。

#### 存储方式
- **所有数据库**: datetime字段统一存储为UTC时间
- **SQLite**: 时间戳格式（Unix timestamp）
- **PostgreSQL/MySQL/MongoDB**: 原生datetime类型（UTC）

#### 存储过程中的时间处理

**重要**: 存储过程返回的时间字段可能需要手动转换格式，特别是SQLite中的时间戳。

```rust
// 手动转换时间戳为可读格式
match datetime_value {
    DataValue::Int(timestamp) => {
        // SQLite: 时间戳转换为可读格式
        chrono::DateTime::from_timestamp(*timestamp, 0)
            .unwrap_or_default()
            .format("%Y-%m-%d %H:%M:%S UTC")
            .to_string()
    },
    DataValue::DateTime(dt) => {
        // 其他数据库: 直接格式化datetime
        dt.format("%Y-%m-%d %H:%M:%S UTC").to_string()
    },
    _ => datetime_value.to_string(),
}
```

#### 时间字段的3种结构方案

rat_quickdb 提供3种不同的时间字段定义方式，满足不同的时区处理需求：

##### 方案1：标准UTC时间字段
```rust
// 模型定义
created_at: chrono::DateTime<chrono::Utc>,

// 字段配置
created_at: datetime_field(),  // 默认UTC (+00:00)

// 数据创建
let now = chrono::Utc::now();
created_at: now,  // 直接传入UTC时间
```

##### 方案2：带时区的FixedOffset时间字段
```rust
// 模型定义
local_time_cst: chrono::DateTime<chrono::FixedOffset>,

// 字段配置
local_time_cst: datetime_with_tz_field("+08:00"),  // 北京时间

// 数据创建
let now = chrono::Utc::now();
local_time_cst: now.into(),  // 转换为FixedOffset
```

##### 方案3：时区字符串字段（RFC3339格式）
```rust
// 模型定义
local_time_est: String,

// 字段配置
local_time_est: datetime_with_tz_field("-05:00"),  // 美东时间

// 数据创建
let now = chrono::Utc::now();
local_time_est: now.to_rfc3339(),  // 传入RFC3339字符串
```

#### 核心原理

**关键特性**：开发者始终传入 `Utc::now()`，框架根据字段定义自动处理时区转换！

- ✅ **统一时间源**：所有字段都使用 `Utc::now()` 作为输入（**入库时必须是UTC now**）
- ✅ **自动转换**：读取时框架根据时区设置自动转换为对应时区（**输出会自动根据datetime_with_tz_field设置**）
- ✅ **存储一致性**：数据库中存储的是统一的UTC时间戳
- ✅ **显示多样性**：同一个UTC时间根据字段配置显示不同时区的本地时间

**重要理解**：
- **入库**：所有时间字段都是同一个 `Utc::now()` 时间戳
- **输出**：框架根据 `datetime_with_tz_field("+08:00")` 等设置自动转换显示

**示例说明**：
```rust
let now = Utc::now();  // 假设是 2024-06-15 12:00:00 UTC

// 入库：所有字段都是同一个时间戳
created_at_utc: now,        // 2024-06-15 12:00:00 UTC
local_time_cst: now.into(), // 2024-06-15 12:00:00 UTC (存储)
local_time_est: now.to_rfc3339(), // 2024-06-15 12:00:00 UTC (存储)

// 输出：框架自动转换显示
created_at_utc: 2024-06-15 12:00:00 UTC     // 保持UTC
local_time_cst: 2024-06-15 20:00:00 +08:00  // 转换为北京时间
local_time_est: 2024-06-15 07:00:00 -05:00  // 转换为美东时间
```

#### 时区格式规范

**支持的时区格式**：
- `+00:00` - UTC
- `+08:00` - 北京时间
- `-05:00` - 美东时间
- `+12:45` - 支持分钟偏移
- `-09:30` - 支持负分钟偏移

**无效格式示例**：
- ❌ `CST` - 时区缩写
- ❌ `UTC` - 时区缩写
- ❌ `+8:00` - 缺少前导零
- ❌ `+08:0` - 分钟格式错误
- ❌ `+25:00` - 超出有效范围

#### 完整示例

```rust
// 完整的3种时间字段模型
define_model! {
    struct TimeZoneTestModel {
        id: String,
        name: String,
        created_at_utc: chrono::DateTime<chrono::Utc>,        // 方案1
        local_time_cst: chrono::DateTime<chrono::FixedOffset>, // 方案2
        local_time_est: String,                                // 方案3
    }
    collection = "timezone_test",
    database = "main",
    fields = {
        id: string_field(None, None, None).required().unique(),
        name: string_field(None, None, None).required(),
        created_at_utc: datetime_field(),              // 默认UTC (+00:00)
        local_time_cst: datetime_with_tz_field("+08:00"), // 北京时间
        local_time_est: datetime_with_tz_field("-05:00"), // 美东时间
    }
}

// 数据创建 - 3种方案对比
let now = chrono::Utc::now();
let test_model = TimeZoneTestModel {
    id: String::new(),  // 框架自动生成UUID
    name: "时区测试".to_string(),
    created_at_utc: now,                    // 方案1：直接UTC时间
    local_time_cst: now.into(),             // 方案2：转换为FixedOffset
    local_time_est: now.to_rfc3339(),       // 方案3：RFC3339字符串
};
```

#### 最佳实践

1. **推荐使用方案1**（`datetime_field()`）作为主要时间字段
2. **需要本地时间时**使用方案2（`datetime_with_tz_field()`）
3. **需要字符串格式**时使用方案3（RFC3339）
4. **始终使用 `Utc::now()`** 作为时间源，让框架处理转换

#### 时区辅助工具

rat_quickdb 提供2个重要的时区处理工具，位于 `rat_quickdb::utils::timezone` 模块：

##### 1. `parse_timezone_offset_to_seconds()`
将时区偏移字符串转换为秒数
```rust
use rat_quickdb::utils::timezone::parse_timezone_offset_to_seconds;

let seconds = parse_timezone_offset_to_seconds("+08:00")?;  // 返回 28800
let seconds = parse_timezone_offset_to_seconds("-05:00")?;  // 返回 -18000
```

##### 2. `utc_to_timezone()` ⭐ **非常常用**
将UTC时间转换为指定时区的时间（如果要存储特定时区时间的话）
```rust
use rat_quickdb::utils::timezone::utc_to_timezone;

let utc_now = chrono::Utc::now();
let beijing_time = utc_to_timezone(utc_now, "+08:00")?;    // 北京时间
let ny_time = utc_to_timezone(utc_now, "-05:00")?;         // 美东时间
```

**工具使用示例**：
```rust
use rat_quickdb::utils::timezone::*;

// 手动时区转换（框架通常自动处理，但如果要存储特定时区时间很有用）
let now = Utc::now();
let beijing_dt = utc_to_timezone(now, "+08:00")?;  // 存储北京时间
let est_dt = utc_to_timezone(now, "-05:00")?;      // 存储美东时间

// 验证时区格式
let offset_seconds = parse_timezone_offset_to_seconds("+09:30")?;  // 34200
```

**完整示例参考**：
- `examples/test_datetime_with_tz_field.rs` - 3种时间字段完整演示

这种设计确保了：
- ✅ **时区一致性** - 避免时区混乱
- ✅ **跨数据库兼容** - 统一的UTC标准
- ✅ **性能优化** - 避免复杂的数据库时间转换
- ✅ **用户友好** - 灵活的显示格式控制

## 📝 索引支持

- **唯一索引**: `unique()` 约束
- **复合索引**: 多字段组合索引
- **普通索引**: 基础查询优化索引
- **自动创建**: 基于模型定义自动创建
- **跨数据库**: 支持所有数据库类型的索引

## 🌟 版本信息

**当前版本**: 0.5.1

**支持Rust版本**: 1.70+

**重要更新**:
- v0.5.1: 版本更新，升级 rat_memcache 至 0.2.8
- v0.4.7: 新增大小写不敏感查询支持
- v0.3.0: 强制使用define_model!宏定义模型，修复重大架构问题，提升类型安全性！

## 📄 许可证

本项目采用 [LGPL-v3](LICENSE) 许可证。

## 🤝 贡献

欢迎提交Issue和Pull Request来改进这个项目！

## 📚 技术文档

### 数据库限制说明

- **[MySQL 限制说明](docs/mysql_limitations.md)** - 必须遵守的索引长度限制
- **[PostgreSQL 限制说明](docs/postgresql_limitations.md)** - 必须遵守的UUID类型处理要求

### 功能指南

- **[DataValue 类型使用指南](docs/datatype_guide.md)** - ⚠️ **重要**：DataValue 类型匹配规则和常见错误
- **[字段版本管理](docs/field_versioning.md)** - 模型版本追踪、升级/回滚、DDL生成

### 其他文档

- **[迁移指南](MIGRATION_GUIDE_0_3_0.md)** - v0.3.0 迁移指南
- **[更新日志](CHANGELOG.md)** - 版本更新记录

## 🔧 疑难杂症

### 并发操作的网络延迟问题

在高并发操作中，特别是跨网络环境访问数据库时，可能会遇到数据同步问题：

#### 问题描述
在高并发写入后立即进行查询操作时，可能出现查询结果不一致的情况，这通常由以下原因造成：

1. **网络延迟**: 云数据库或跨地域访问的网络延迟
2. **数据库主从同步**: 主从复制架构下的同步延迟
3. **连接池缓冲**: 连接池中的操作队列缓冲

#### 解决方案

**方案1: 根据网络环境配置等待时间**

```rust
// 网络环境与建议等待时间
let wait_ms = match network_environment {
    NetworkEnv::Local => 0,        // 本地数据库
    NetworkEnv::LAN => 10,         // 局域网
    NetworkEnv::Cloud => 100,      // 云数据库
    NetworkEnv::CrossRegion => 200, // 跨地域
};

// 在写入操作后添加等待
tokio::time::sleep(tokio::time::Duration::from_millis(wait_ms)).await;
```

**方案2: 使用重试机制**

```rust
async fn safe_query_with_retry<T, F, Fut>(operation: F) -> Result<T>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T>>,
{
    let mut retries = 3;
    loop {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) if retries > 0 => {
                retries -= 1;
                tokio::time::sleep(Duration::from_millis(50)).await;
            },
            Err(e) => return Err(e),
        }
    }
}
```

**方案3: 智能延迟检测**

```rust
// 动态检测网络延迟并调整等待时间
async fn adaptive_network_delay() -> Duration {
    let start = Instant::now();
    let _ = health_check().await;
    let base_latency = start.elapsed();

    // 等待时间为基础延迟的3倍，最小10ms，最大200ms
    let wait_time = std::cmp::max(
        Duration::from_millis(10),
        std::cmp::min(base_latency * 3, Duration::from_millis(200))
    );

    wait_time
}
```

#### 最佳实践建议

- **本地开发**: 无需等待或等待5-10ms
- **局域网环境**: 等待10-50ms
- **云数据库**: 等待100-200ms或使用重试机制
- **生产环境**: 强烈建议使用重试机制代替固定等待
- **高并发场景**: 考虑使用批量操作减少网络往返

#### 架构说明

rat_quickdb采用单Worker架构来保证数据一致性：
- **单Worker**: 避免多连接并发写入导致的数据冲突
- **长连接**: Worker与数据库保持持久连接，减少连接开销
- **消息队列**: 通过异步消息队列处理请求，保证顺序性

这种设计在保证数据一致性的同时，仍能提供良好的并发性能。

## 📞 联系方式

如有问题或建议，请通过以下方式联系：
- 创建Issue: [GitHub Issues](https://github.com/your-repo/rat_quickdb/issues)
- 邮箱: oldmos@gmail.com