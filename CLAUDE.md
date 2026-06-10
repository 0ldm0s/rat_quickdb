# CLAUDE.md

本文件为 eva (0ldm0s.net/eva-cli) 在此仓库中工作时提供指导。

## 项目概述

`rat_quickdb` 是一个 Rust 编写的跨数据库 ODM（Object-Document Mapper）库，支持 SQLite、PostgreSQL、MySQL、MongoDB 的统一接口操作。

**当前版本**：0.5.4

## 常用命令

```bash
# 构建（核心功能，无需数据库）
just build

# 使用特定数据库支持构建
cargo build --features sqlite-support
cargo build --features postgres-support
cargo build --features mysql-support
cargo build --features mongodb-support
cargo build --features full

# 代码检查
cargo clippy --all-features

# 运行所有测试
just test

# 运行单个测试
cargo test test_name --features sqlite-support

# 运行示例
just run --example model_operations_sqlite --features sqlite-support
just run --example query_operations_mongodb --features mongodb-support

# 生成文档
cargo doc --all-features
```

## 架构设计

```
用户代码
    │
    ▼
┌─────────────────┐
│   ODM 层 (odm/)  │  ← find, create, update, delete 操作
│                  │     - handlers/ : 请求处理器
│                  │     - operations/ : 操作实现
└────────┬────────┘
         │
┌────────▼────────┐
│  模型层 (model/) │  ← Model, ModelManager trait 定义
│                  │     - macros/ : 模型宏
│                  │     - field_types/ : 字段类型
└────────┬────────┘
         │
┌────────▼────────┐
│ 适配器层 (adapter)│  ← SQLite | PostgreSQL | MySQL | MongoDB
│                  │     - cached/ : 缓存装饰器
└────────┬────────┘
         │
┌────────▼────────┐
│  连接池 (pool/)   │
│                  │     - multi_connection_manager.rs
│                  │     - sqlite_worker.rs
└─────────────────┘
```

核心模块：
- `adapter/` - 数据库适配器实现，各数据库独立子目录（sqlite/, postgres/, mysql/, mongodb/）+ cached 缓存装饰器
- `model/` - 数据模型定义和 trait，包含字段类型（`field_types.rs`）和宏（`macros.rs`）、数据转换（`conversion/`）
- `odm/` - ODM 操作层，按职责分离：
  - `handlers/` - 请求处理器（create_handler, read_handler, update_handler, delete_handler, upsert_handler, stored_procedure_handler）
  - `operations/` - 操作实现（odm_operations_impl.rs）
  - `manager_core.rs` - AsyncOdmManager 核心
  - `global.rs` - 全局 ODM 管理器
- `manager/` - 数据库/模型/缓存管理器（manager.rs, database_ops.rs, model_ops.rs, cache_ops.rs, maintenance.rs）
- `table/` - 表管理（schema 定义、版本管理）
- `pool/` - 连接池管理，支持多数据库
- `cache/` - 缓存系统（基于 rat_memcache）：cache_manager, key_generator, operations, query_cache, record_cache, stats
- `types/` - 类型定义模块：
  - `query/` - 查询条件类型
  - `data_value/` - DataValue 枚举
  - `id_types/` - ID 生成策略
  - `update_operations/` - 更新操作
  - `cache_config/` - 缓存配置类型
  - `database_config/` - 数据库配置类型
  - `mongo_builder.rs` - MongoDB 查询构建器
  - `serde_helpers.rs` - Serde 序列化辅助
- `stored_procedure/` - 跨数据库存储过程 API
- `field_versioning/` - 字段版本管理，支持模型版本追踪、升级/回滚、DDL 生成
- `config/` - 配置构建器（builders/ 子目录 + convenience.rs 快捷配置）
- `python_api/` - Python 语言绑定（database_processors/, json_queue_bridge, simple_queue_bridge）
- `i18n/` - 多语言错误消息系统

## 特性说明

- 异步支持：基于 Tokio
- 缓存：内置智能缓存，支持 L1 内存缓存和 L2 磁盘缓存，支持 TTL 过期和缓存绕过
- ID 生成：AutoIncrement、UUID、Snowflake、ObjectId、Custom 前缀
- 自动索引：根据模型定义自动创建表和索引
- 存储过程：跨数据库的统一存储过程 API，支持多表 JOIN 和聚合查询
- 字段版本管理：支持模型字段版本控制，可追踪变更、生成升级/回滚 DDL
- 日志控制：由调用者完全控制日志初始化，避免库自动初始化冲突
- SQLite 布尔值兼容：自动处理 SQLite 布尔值存储差异
- i18n 支持：多语言错误消息系统（基于 rat_embed_lang）

## 测试与示例

```bash
# 运行测试（需要启用对应的数据库特性）
cargo test --features sqlite-support

# 运行集成测试
cargo test --test basic_test --features sqlite-support

# 列出所有示例
ls examples/

# 运行特定示例
just run --example query_operations_sqlite --features sqlite-support
just run --example model_operations_mongodb --features mongodb-support
just run --example field_versioning_sqlite --features sqlite-support
```

**测试位置**：
- `tests/` - 集成测试
- `examples/` - 包含 70+ 个示例文件，涵盖各种使用场景

## 重要设计模式

### 全局操作锁机制
- 首次查询操作后，全局操作锁会锁定，禁止添加新的数据库配置
- 这是设计上的约束，防止在运行时动态修改数据库配置
- 锁定逻辑在 `lib.rs` 中的 `lock_global_operations()` 和 `is_global_operations_locked()`

### 双类型查询系统
- **`QueryCondition`**（简化版）：默认大小写敏感，适合大多数场景
- **`QueryConditionWithConfig`**（完整版）：支持大小写不敏感等高级配置
- 简化版可通过 `.into()` 自动转换为完整版

### 字段版本管理
- 支持模型字段的版本追踪和变更管理
- 可生成字段升级/回滚的 DDL 语句
- 适用于数据库迁移和模型演进场景
- 详细文档参考 `docs/field_versioning.md`

## Feature 门控说明

所有数据库支持都是可选特性，按需启用：

| Feature | 说明 |
|---------|------|
| `sqlite-support` | 启用 SQLite 支持 |
| `postgres-support` | 启用 PostgreSQL 支持 |
| `mysql-support` | 启用 MySQL 支持 |
| `mongodb-support` | 启用 MongoDB 支持 |
| `melange-storage` | 内部标识符，L2 缓存已通过 rat_memcache 内置，用户无需手动启用 |
| `full` | 启用所有数据库支持 + melange-storage |

**注意**：尝试使用未启用特性的数据库会导致编译错误。

## 开发约定

### 代码风格
- 所有注释和文档使用中文
- 使用相对路径而非绝对路径
- 代码修改前必须先使用 Read 工具阅读相关文件

### 数据处理后处理
- `lib.rs` 中的 `process_data_fields_from_metadata()` 函数处理数据库返回的原始数据
- 自动处理：
  - JSON 字符串 → DataValue 转换
  - SQLite 布尔值（0/1）→ Bool 转换
  - DateTimeWithTz 字段的时区转换

### 多语言支持
- 错误消息支持多语言（通过 `rat_embed_lang`）
- 初始化时调用 `rat_quickdb::init()` 加载 i18n 系统

@.eva/docs/已有能力.md
@.eva/docs/architecture.md
@.eva/docs/types.md
@.eva/docs/adapter.md
@.eva/docs/odm.md

refine-tags
type: code
domain: 跨数据库 ODM 库
keywords: ODM, 数据库, SQLite, PostgreSQL, MySQL, MongoDB, 异步, 缓存, 连接池, 存储过程
