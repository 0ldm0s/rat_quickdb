# rat_quickdb 配置系统文档

## 概述

配置系统（`src/config/`）提供灵活的配置构建器，支持多种数据库配置和全局设置。

## 核心组件

### AppConfig

位置：`src/config/core.rs`

应用配置结构体。

```rust
pub struct AppConfig {
    pub database: DatabaseConfig,
    pub pool: PoolConfig,
    pub cache: CacheConfig,
    pub logging: LoggingConfig,
}
```

### AppConfigBuilder

位置：`src/config/core.rs`

应用配置构建器。

```rust
pub struct AppConfigBuilder {
    // 配置构建器
}
```

**关键方法**：
- `database(config)` - 设置数据库配置
- `pool(config)` - 设置连接池配置
- `cache(config)` - 设置缓存配置
- `logging(config)` - 设置日志配置
- `build()` - 构建配置

### DatabaseConfigBuilder

位置：`src/config/database_builder.rs`

数据库配置构建器。

```rust
pub struct DatabaseConfigBuilder {
    // 数据库配置构建器
}
```

**工厂函数**：
- `sqlite_config(path)` - SQLite 配置
- `postgres_config(host, port, db, user, pass)` - PostgreSQL 配置
- `mysql_config(host, port, db, user, pass)` - MySQL 配置
- `mongodb_config(uri, db)` - MongoDB 配置

### GlobalConfigBuilder

位置：`src/config/global_builder.rs`

全局配置构建器。

```rust
pub struct GlobalConfigBuilder {
    // 全局配置构建器
}
```

### LoggingConfigBuilder

位置：`src/config/logging_builder.rs`

日志配置构建器。

```rust
pub struct LoggingConfigBuilder {
    // 日志配置构建器
}
```

### PoolConfigBuilder

位置：`src/config/pool_builder.rs`

连接池配置构建器。

```rust
pub struct PoolConfigBuilder {
    // 连接池配置构建器
}
```

## 快捷配置

位置：`src/config/convenience.rs`

提供便捷的配置函数：

```rust
// SQLite 配置
pub fn sqlite_config(path: &str) -> DatabaseConfig

// PostgreSQL 配置
pub fn postgres_config(
    host: &str,
    port: u16,
    database: &str,
    username: &str,
    password: &str,
) -> DatabaseConfig

// MySQL 配置
pub fn mysql_config(
    host: &str,
    port: u16,
    database: &str,
    username: &str,
    password: &str,
) -> DatabaseConfig

// MongoDB 配置
pub fn mongodb_config(uri: &str, database: &str) -> DatabaseConfig
```

## 使用示例

### 基本配置

```rust
use rat_quickdb::config::*;

let config = AppConfig::builder()
    .database(sqlite_config("my_db.sqlite"))
    .pool(PoolConfig::builder().max_connections(10).build())
    .cache(CacheConfig::builder().enabled(true).build())
    .build()?;
```

### 多数据库配置

```rust
let config = AppConfig::builder()
    .database(
        DatabaseConfig::builder()
            .alias("primary")
            .sqlite_config("primary.db")
            .build()
    )
    .database(
        DatabaseConfig::builder()
            .alias("secondary")
            .postgres_config("localhost", 5432, "mydb", "user", "pass")
            .build()
    )
    .build()?;
```

## 环境配置

位置：`src/config/environment.rs`

支持环境变量配置：

```rust
pub enum Environment {
    Development,
    Testing,
    Staging,
    Production,
}
```

## 全局配置

位置：`src/config/global.rs`

全局配置管理：

```rust
pub struct GlobalConfig {
    // 全局配置
}
```

**关键方法**：
- `instance()` - 获取全局配置实例
- `set(config)` - 设置全局配置
- `get()` - 获取全局配置

## 日志配置

位置：`src/config/logging.rs`

日志配置：

```rust
pub struct LoggingConfig {
    pub level: LogLevel,
    pub format: LogFormat,
    pub output: LogOutput,
}
```

### LogLevel 枚举

```rust
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}
```

## 配置验证

配置在构建时会进行验证：
- 数据库连接参数验证
- 连接池参数验证
- 缓存参数验证
- 日志参数验证

## 配置序列化

配置支持序列化和反序列化：

```rust
// 从文件加载配置
let config = AppConfig::from_file("config.toml")?;

// 保存配置到文件
config.to_file("config.toml")?;
```