# rat_quickdb 架构设计文档

## 整体架构

rat_quickdb 采用分层架构设计，从上到下分为四层：

```
用户代码
    │
    ▼
┌─────────────────┐
│   ODM 层 (odm/)  │  ← 高级操作接口
│                  │     - AsyncOdmManager
│                  │     - OdmOperations trait
└────────┬────────┘
         │
┌────────▼────────┐
│  模型层 (model/) │  ← 模型定义和管理
│                  │     - Model trait
│                  │     - define_model! 宏
│                  │     - ModelManager<T>
└────────┬────────┘
         │
┌────────▼────────┐
│ 适配器层 (adapter)│  ← 数据库适配
│                  │     - DatabaseAdapter trait
│                  │     - 各数据库实现
│                  │     - CachedDatabaseAdapter
└────────┬────────┘
         │
┌────────▼────────┐
│  连接池 (pool/)   │  ← 连接管理
│                  │     - MultiConnectionManager
│                  │     - SqliteWorker
└─────────────────┘
```

## 模块职责

### ODM 层 (`src/odm/`)

**职责**：提供高级的数据库操作接口，封装底层适配器细节。

**核心组件**：
- `AsyncOdmManager` - 异步 ODM 管理器，管理数据库连接和缓存
- `OdmOperations` trait - 定义 CRUD 操作接口
- `handlers/` - 各操作的具体实现（create, read, update, delete, upsert, stored_procedure）
- `global.rs` - 全局 ODM 管理器和便捷函数

**设计特点**：
- 基于 Handler 模式分离操作逻辑
- 支持全局和实例两种使用方式
- 自动处理缓存策略

### 模型层 (`src/model/`)

**职责**：定义数据模型结构，提供模型操作接口。

**核心组件**：
- `Model` trait - 模型核心 trait，定义元数据和数据转换
- `ModelOperations` trait - 模型 CRUD 操作
- `ModelManager<T>` - 泛型模型管理器
- `define_model!` 宏 - 自动生成样板代码
- `FieldDefinition` / `FieldType` - 字段定义

**设计特点**：
- 宏驱动的代码生成，减少样板代码
- 强类型模型定义
- 自动数据转换（DataMap ↔ 模型）

### 适配器层 (`src/adapter/`)

**职责**：封装各数据库的差异，提供统一接口。

**核心组件**：
- `DatabaseAdapter` trait - 统一适配器接口
- 各数据库实现（sqlite/, postgres/, mysql/, mongodb/）
- `CachedDatabaseAdapter` - 缓存装饰器

**设计特点**：
- Feature 门控：按需启用数据库支持
- 装饰器模式：缓存层可选叠加
- 查询构建器：各数据库独立实现 SQL/NoSQL 生成

### 连接池 (`src/pool/`)

**职责**：管理数据库连接，提供高效的连接复用。

**核心组件**：
- `MultiConnectionManager` - 多连接管理器（MySQL/PostgreSQL/MongoDB）
- `SqliteWorker` - SQLite 专用工作器（单连接多任务）
- `PoolConfig` - 连接池配置

**设计特点**：
- SQLite 使用单连接 + 无锁队列（WAL 模式）
- 其他数据库使用多连接 + 工作器模式
- 支持连接保活和自动重连

## 数据流

### 写入流程

```
用户调用 Model::save()
    │
    ▼
ModelManager::save()
    │
    ▼
AsyncOdmManager::create()
    │
    ▼
OdmOperations::create()
    │
    ▼
CreateHandler::handle()
    │
    ├──→ 数据验证
    ├──→ 生成 ID（如果需要）
    ├──→ 转换为 DataMap
    │
    ▼
DatabaseAdapter::create()
    │
    ├──→ 构建 SQL/NoSQL
    ├──→ 执行查询
    ├──→ 返回结果
    │
    ▼
更新缓存（如果启用）
    │
    ▼
返回给用户
```

### 读取流程

```
用户调用 Model::find() 或 Model::find_by_id()
    │
    ▼
ModelManager::find()
    │
    ▼
AsyncOdmManager::find()
    │
    ├──→ 检查缓存（如果启用）
    │       ├──→ 缓存命中：直接返回
    │       └──→ 缓存未命中：继续
    │
    ▼
OdmOperations::find()
    │
    ▼
ReadHandler::handle()
    │
    ├──→ 构建查询条件
    │
    ▼
DatabaseAdapter::find()
    │
    ├──→ 执行查询
    ├──→ 转换结果
    │
    ▼
更新缓存（如果启用）
    │
    ▼
返回给用户
```

## 缓存架构

### L1/L2 双层缓存

基于 `rat_memcache` 实现：

- **L1（内存缓存）**：高速访问，容量有限
- **L2（磁盘缓存）**：持久化存储，容量较大

### 缓存策略

- **TTL 过期**：支持配置缓存生存时间
- **缓存绕过**：`find_with_cache_control()` 支持强制跳过缓存
- **查询缓存**：按查询条件缓存结果
- **记录缓存**：按 ID 缓存单条记录

### 缓存键生成

`key_generator` 模块负责生成唯一的缓存键，格式：
- 查询缓存：`query:{database}:{table}:{conditions_hash}`
- 记录缓存：`record:{database}:{table}:{id}`

## 错误处理

### 统一错误类型

`QuickDbError` 枚举定义了所有可能的错误：

- `TableNotExistError` - 表不存在
- `ConnectionError` - 连接错误
- `QueryError` - 查询错误
- `ValidationError` - 验证错误
- `CacheError` - 缓存错误
- 等等

### 错误传播

使用 `QuickDbResult<T>`（即 `Result<T, QuickDbError>`）统一错误传播。

## 线程安全

### 并发模型

- 基于 Tokio 异步运行时
- 使用 `Arc` 共享不可变数据
- 使用 `Mutex` / `RwLock` 保护可变状态
- SQLite 使用无锁队列（`crossbeam_queue`）

### 全局状态

- `GLOBAL_OPERATION_LOCK` - 原子布尔值，控制全局操作锁定
- `once_cell` - 延迟初始化全局管理器

## 扩展点

### 添加新的数据库支持

1. 在 `src/adapter/` 下创建新目录
2. 实现 `DatabaseAdapter` trait
3. 在 `Cargo.toml` 中添加 feature 门控
4. 在 `adapter/mod.rs` 中注册工厂函数

### 自定义 ID 生成

实现 `IdGenerator` trait 或使用 `IdStrategy::Custom` 前缀策略。

### 自定义缓存策略

实现 `CacheStrategy` trait 或扩展现有缓存管理器。
