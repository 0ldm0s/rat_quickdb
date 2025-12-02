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
- **🧠 智能缓存**: 内置缓存支持（基于rat_memcache），支持TTL过期和回退机制
- **🆔 多种ID生成策略**: AutoIncrement、UUID、Snowflake、ObjectId、Custom前缀
- **📝 日志控制**: 由调用者完全控制日志初始化，避免库自动初始化冲突
- **🐍 Python绑定**: 可选Python API支持
- **📋 任务队列**: 内置异步任务队列系统
- **🔍 类型安全**: 强类型模型定义和验证
- **📋 存储过程**: 跨数据库的统一存储过程API，支持多表JOIN和聚合查询

## 🔄 版本变更说明

### v0.3.6 (当前版本) - 存储过程虚拟表系统

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

**新功能：**
- 🎯 **存储过程虚拟表系统**：跨四种数据库的统一存储过程API
- 🔗 **多表JOIN支持**：自动生成JOIN语句和聚合管道
- 📊 **聚合查询优化**：自动GROUP BY子句生成（SQL数据库）
- 🧠 **类型安全存储过程**：编译时验证和类型检查

## 📦 安装

在`Cargo.toml`中添加依赖：

```toml
[dependencies]
rat_quickdb = "0.3.6"
```

### 🔧 特性控制

rat_quickdb 使用 Cargo 特性来控制不同数据库的支持和功能。默认情况下只包含核心功能，你需要根据使用的数据库类型启用相应的特性：

```toml
[dependencies]
rat_quickdb = { version = "0.3.6", features = [
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

**重要**：不同数据库对JSON操作的支持版本要求不同：

| 数据库 | 最低版本要求 | JSON支持 | Contains操作符实现 | JsonContains操作符实现 |
|--------|-------------|----------|-------------------|-------------------------|
| **MySQL** | 5.7+ / MariaDB 10.2+ | ✅ 完整支持 | 字符串字段使用LIKE，JSON字段使用JSON_CONTAINS() | ❌ 暂时不支持 |
| **PostgreSQL** | 9.2+ | ✅ 完整支持 | 字符串字段使用LIKE，JSON字段使用@>操作符 | ✅ 完全支持 |
| **SQLite** | 3.38.0+ | ✅ 基础支持 | 仅字符串字段支持LIKE操作 | ❌ 不支持 |
| **MongoDB** | 7.0+ | ✅ 原生支持 | 原生$regex操作符 | ✅ 完全支持 |

⚠️ **版本兼容性注意事项**：
- MySQL 5.6及以下版本不支持JSON_CONTAINS函数，会导致运行时错误
- PostgreSQL早期版本可能需要启用JSON扩展
- SQLite JSON功能是可选的，需要在编译时启用

#### 按需启用特性

**仅使用SQLite**:
```toml
[dependencies]
rat_quickdb = { version = "0.3.6", features = ["sqlite-support"] }
```

**使用PostgreSQL**:
```toml
[dependencies]
rat_quickdb = { version = "0.3.6", features = ["postgres-support"] }
```

**使用所有数据库**:
```toml
[dependencies]
rat_quickdb = { version = "0.3.6", features = ["full"] }
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

## 🧠 缓存配置

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

// 更新记录
let mut updates = HashMap::new();
updates.insert("username".to_string(), DataValue::String("新名字".to_string()));
let updated = user.update(updates).await?;

// 删除记录
let deleted = user.delete().await?;
```

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
- `json_field` - JSON字段
- `array_field` - 数组字段
- `list_field` - 列表字段（array_field的别名）
- `dict_field` - 字典/对象字段（基于Object类型）
- `reference_field` - 引用字段（外键）

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

#### 最佳实践

1. **存储时**: 始终使用UTC时间
```rust
let now = chrono::Utc::now();  // 获取当前UTC时间
```

2. **显示时**: 根据用户需求转换时区和格式
```rust
// 转换为本地时间显示
let local_time = utc_time.with_timezone(&chrono::Local);
```

3. **存储过程中**: 在应用层处理时间格式转换，避免在SQL中增加复杂度

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

**当前版本**: 0.3.4

**支持Rust版本**: 1.70+

**重要更新**: v0.3.0 强制使用define_model!宏定义模型，修复重大架构问题，提升类型安全性！

## 📄 许可证

本项目采用 [LGPL-v3](LICENSE) 许可证。

## 🤝 贡献

欢迎提交Issue和Pull Request来改进这个项目！

## 📚 技术文档

### 数据库限制说明

- **[MySQL 限制说明](docs/mysql_limitations.md)** - 必须遵守的索引长度限制
- **[PostgreSQL 限制说明](docs/postgresql_limitations.md)** - 必须遵守的UUID类型处理要求

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