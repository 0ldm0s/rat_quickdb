# rat_quickdb 连接池文档

## 概述

连接池（`src/pool/`）管理数据库连接，提供高效的连接复用。支持多数据库连接管理。

## 核心组件

### MultiConnectionManager

位置：`src/pool/multi_connection_manager.rs`

多连接管理器，管理 MySQL、PostgreSQL、MongoDB 的连接池。

```rust
pub struct MultiConnectionManager {
    // 多数据库连接管理
}
```

**关键方法**：
- `new(config)` - 创建新的连接管理器
- `get_connection(alias)` - 获取指定数据库的连接
- `add_database(config)` - 添加数据库配置
- `remove_database(alias)` - 移除数据库配置
- `health_check()` - 健康检查

### SqliteWorker

位置：`src/pool/sqlite_worker.rs`

SQLite 专用工作器，使用单连接 + 无锁队列（WAL 模式）。

```rust
pub struct SqliteWorker {
    // SQLite 工作器
}
```

**特点**：
- 单连接多任务
- 无锁队列（`crossbeam_queue`）
- WAL 模式支持
- 自动重连

### ConnectionPool

位置：`src/pool/pool.rs`

连接池抽象。

```rust
pub struct ConnectionPool {
    // 连接池实现
}
```

### DatabaseConnection

位置：`src/pool/types.rs`

数据库连接枚举。

```rust
pub enum DatabaseConnection {
    SQLite(SqliteConnection),
    PostgreSQL(PostgresConnection),
    MySQL(MysqlConnection),
    MongoDB(MongoConnection),
}
```

### ConnectionWorker

位置：`src/pool/types.rs`

连接工作器。

```rust
pub struct ConnectionWorker {
    // 连接工作器实现
}
```

### DatabaseOperation

位置：`src/pool/types.rs`

数据库操作枚举。

```rust
pub enum DatabaseOperation {
    Query(String),
    Execute(String),
    // ... 其他操作
}
```

### PooledConnection

位置：`src/pool/types.rs`

池化连接。

```rust
pub struct PooledConnection {
    // 池化连接实现
}
```

## 配置

### PoolConfig

位置：`src/pool/config.rs`

连接池配置。

```rust
pub struct PoolConfig {
    pub max_connections: u32,
    pub min_connections: u32,
    pub connection_timeout: Duration,
    pub idle_timeout: Duration,
    pub max_lifetime: Duration,
}
```

### ExtendedPoolConfig

位置：`src/pool/types.rs`

扩展连接池配置。

```rust
pub struct ExtendedPoolConfig {
    pub pool_config: PoolConfig,
    pub retry_count: u32,
    pub retry_delay: Duration,
    pub health_check_interval: Duration,
}
```

## 连接管理

### SQLite 连接

- 单连接 + 无锁队列
- WAL 模式支持
- 自动重连
- 线程安全

### MySQL/PostgreSQL/MongoDB 连接

- 多连接池
- 连接保活
- 自动重连
- 负载均衡

## 连接池策略

### 连接获取

```rust
// 获取连接
let connection = pool.get_connection("my_database").await?;

// 使用连接
connection.execute("SELECT * FROM users").await?;
```

### 连接释放

连接在离开作用域时自动释放回池。

### 健康检查

```rust
// 健康检查
let is_healthy = pool.health_check().await?;
```

## 并发模型

- 基于 Tokio 异步运行时
- 使用 `Arc` 共享不可变数据
- 使用 `Mutex` / `RwLock` 保护可变状态
- SQLite 使用无锁队列（`crossbeam_queue`）

## 错误处理

连接池错误会被转换为 `QuickDbError`：

- `ConnectionError` - 连接错误
- `PoolExhaustedError` - 连接池耗尽
- `ConnectionTimeoutError` - 连接超时