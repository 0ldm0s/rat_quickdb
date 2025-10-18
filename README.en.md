# rat_quickdb

[![Crates.io](https://img.shields.io/crates/v/rat_quickdb.svg)](https://crates.io/crates/rat_quickdb)
[![Documentation](https://docs.rs/rat_quickdb/badge.svg)](https://docs.rs/rat_quickdb)
[![License: LGPL-3.0](https://img.shields.io/badge/License-LGPL--3.0-blue.svg)](https://www.gnu.org/licenses/lgpl-3.0)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://rust-lang.org)
[![Downloads](https://img.shields.io/crates/d/rat_quickdb.svg)](https://crates.io/crates/rat_quickdb)

🚀 Powerful cross-database ODM library with unified interface for SQLite, PostgreSQL, MySQL, MongoDB

**🌐 Language Versions**: [中文](README.md) | [English](README.en.md) | [日本語](README.ja.md)

## ✨ Core Features

- **🎯 Auto Index Creation**: Automatically create tables and indexes based on model definitions, no manual intervention required
- **🗄️ Multi-Database Support**: SQLite, PostgreSQL, MySQL, MongoDB
- **🔗 Unified API**: Consistent interface for different databases
- **🔒 SQLite Boolean Compatibility**: Automatically handles SQLite boolean value storage differences, zero configuration compatibility
- **🏊 Connection Pool Management**: Efficient connection pool and lock-free queue architecture
- **⚡ Async Support**: Based on Tokio async runtime
- **🧠 Smart Caching**: Built-in caching support (based on rat_memcache), with TTL expiration and fallback mechanism
- **🆔 Multiple ID Generation Strategies**: AutoIncrement, UUID, Snowflake, ObjectId, Custom prefix
- **📝 Logging Control**: Complete logging initialization control by caller, avoiding library auto-initialization conflicts
- **🐍 Python Bindings**: Optional Python API support
- **📋 Task Queue**: Built-in async task queue system
- **🔍 Type Safety**: Strong type model definitions and validation

## 📦 Installation

Add dependency in `Cargo.toml`:

```toml
[dependencies]
rat_quickdb = "0.3.2"
```

### 🔧 Feature Control

rat_quickdb uses Cargo features to control different database support and functionality. By default, only core functionality is included. You need to enable features based on the database types you use:

```toml
[dependencies]
rat_quickdb = { version = "0.3.2", features = [
    "sqlite-support",    # Support SQLite database
    "postgres-support",  # Support PostgreSQL database
    "mysql-support",     # Support MySQL database
    "mongodb-support",   # Support MongoDB database
] }
```

#### Available Features

| Feature Name | Description | Default |
|-------------|-------------|---------|
| `sqlite-support` | SQLite database support | ❌ |
| `postgres-support` | PostgreSQL database support | ❌ |
| `mysql-support` | MySQL database support | ❌ |
| `mongodb-support` | MongoDB database support | ❌ |
| `melange-storage` | Deprecated: L2 cache functionality has been built into rat_memcache | ❌ |
| `python-bindings` | Python API bindings | ❌ |
| `full` | Enable all database support | ❌ |

#### Enable Features by Need

**SQLite only**:
```toml
[dependencies]
rat_quickdb = { version = "0.3.2", features = ["sqlite-support"] }
```

**PostgreSQL**:
```toml
[dependencies]
rat_quickdb = { version = "0.3.2", features = ["postgres-support"] }
```

**All databases**:
```toml
[dependencies]
rat_quickdb = { version = "0.3.2", features = ["full"] }
```

**L2 Cache Configuration Notes**:
- L2 cache functionality is built-in to `rat_memcache`, no additional features needed
- L2 cache requires disk space for cache persistence
- See "Cache Configuration" section below for configuration examples

#### Running Examples

Different examples require different feature support:

```bash
# Basic model definition example
cargo run --example model_definition --features sqlite-support

# Complex query example
cargo run --example complex_query_demo --features sqlite-support

# Pagination query example
cargo run --example model_pagination_demo --features sqlite-support

# Special types test example
cargo run --example special_types_test --features sqlite-support

# ID generation strategy example
cargo run --example id_strategy_test --features sqlite-support

# Manual table management example
cargo run --example manual_table_management --features sqlite-support

# Other database examples
cargo run --example model_definition_mysql --features mysql-support
cargo run --example model_definition_pgsql --features postgres-support
cargo run --example model_definition_mongodb --features mongodb-support

```

## ⚠️ Important Architecture Notice

### ODM Layer Usage Requirement (v0.3.0+)

**Starting from v0.3.0, you must use the define_model! macro to define models. Using plain structs for database operations is no longer allowed.**

All database operations must go through the following methods:

1. **Recommended: Use Model API**
```rust
use rat_quickdb::*;
use rat_quickdb::ModelOperations;

// Define model
define_model! {
    struct User {
        id: String,
        username: String,
        email: String,
    }
    // ... field definitions
}

// Create and save
let user = User {
    id: String::new(), // Framework auto-generates ID
    username: "张三".to_string(),
    email: "zhangsan@example.com".to_string(),
};
let user_id = user.save().await?;

// Query
let found_user = ModelManager::<User>::find_by_id(&user_id).await?;
```

2. **Alternative: Use ODM API**
```rust
use rat_quickdb::*;

// Add database configuration via add_database
let config = DatabaseConfig::builder()
    .db_type(DatabaseType::SQLite)
    .connection(ConnectionConfig::SQLite {
        path: "test.db".to_string(),
        create_if_missing: true,
    })
    .alias("main".to_string())
    .build()?;
add_database(config).await?;

// Use ODM to operate database
let mut user_data = HashMap::new();
user_data.insert("username".to_string(), DataValue::String("张三".to_string()));
create("users", user_data, Some("main")).await?;
```

3. **Prohibited Usage**
```rust
// ❌ Error: Direct access to connection pool manager is no longer allowed
// let pool_manager = get_global_pool_manager();
// let pool = pool_manager.get_connection_pools().get("main");
```

This design ensures:
- **Architecture Integrity**: Unified data access layer
- **Security**: Prevents resource leaks from direct low-level connection pool operations
- **Consistency**: All operations go through the same ODM layer processing
- **Maintainability**: Unified error handling and logging

## 📋 Upgrading from Previous Versions

### Upgrading from v0.2.x to v0.3.0

v0.3.0 is a major version with breaking changes. Please refer to the detailed [Migration Guide](MIGRATION_GUIDE_0_3_0.md).

**Key Changes**:
- ✅ Enforces `define_model!` macro for model definitions
- ✅ Eliminates "nanny settings" issues with dynamic table structure inference
- ✅ Provides clearer type safety and field definitions
- ✅ Fixes major architecture bugs

### Upgrading from v0.3.1 to v0.3.2+

**🚨 Breaking Change: Convenience Functions Require Explicit ID Strategy**

Starting from v0.3.2, all database configuration convenience functions (`sqlite_config`, `postgres_config`, `mysql_config`, `mongodb_config`) now require explicitly passing the `id_strategy` parameter.

**Reason for Change**:
- Eliminates hardcoded "nanny settings", ensuring users have complete control over ID generation strategy
- All databases now unified to use `AutoIncrement` strategy by default
- Avoids confusion from different databases having different default strategies

**API Changes**:
```rust
// v0.3.1 and earlier (removed)
let config = sqlite_config("sqlite_db", "./test.db", pool_config)?;

// v0.3.2+ (new API)
let config = sqlite_config(
    "sqlite_db",
    "./test.db",
    pool_config,
    Some(IdStrategy::AutoIncrement)  // Must explicitly specify
)?;
```

**Migration Guide**:
1. **Recommended**: Migrate to the builder pattern for better type safety and consistency
```rust
// Instead of deprecated convenience functions, use builder pattern:
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

// PostgreSQL with UUID (recommended for PostgreSQL)
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

2. **Temporary Compatibility**: If you must temporarily maintain existing code, add the required `IdStrategy` parameter, but plan migration to builder pattern ASAP

**Impact Scope**:
- All code using convenience functions for database configuration
- Code using `mongodb_config_with_builder` (duplicate function removed)
- Applications relying on specific database default ID strategies

This change ensures configuration consistency and user control, adhering to the "no nanny settings" design principle.

## 🚀 Quick Start

### Basic Usage

See `examples/model_definition.rs` for complete model definition and usage examples.

### ID Generation Strategy Examples

See `examples/id_strategy_test.rs` for different ID generation strategy usage.

### Database Adapter Examples

- **SQLite**: `examples/model_definition.rs` (run with `--features sqlite-support`)
- **PostgreSQL**: `examples/model_definition_pgsql.rs`
- **MySQL**: `examples/model_definition_mysql.rs`
- **MongoDB**: `examples/model_definition_mongodb.rs`

### Model Definition (Recommended)

See `examples/model_definition.rs` for complete model definition, CRUD operations, and complex query examples.

### Field Types and Validation

See `examples/model_definition.rs` for field type definitions and validation examples.

### Index Management

Indexes are automatically created based on model definitions, no manual management needed. Refer to `examples/model_definition.rs` for index definition methods.

## 🔒 SQLite Boolean Compatibility

SQLite databases store boolean values as integers (0 and 1), which can cause serde deserialization errors. rat_quickdb provides multiple solutions:

### Solution 1: sqlite_bool_field() - Recommended (Zero Configuration)

```rust
use rat_quickdb::*;

rat_quickdb::define_model! {
    struct User {
        id: Option<i32>,
        username: String,
        is_active: bool,        // Auto SQLite compatible
        is_pinned: bool,        // Auto SQLite compatible
        is_verified: bool,      // Auto SQLite compatible
    }

    collection = "users",
    fields = {
        id: integer_field(None, None),
        username: string_field(Some(50), Some(3), None).required(),
        // Use sqlite_bool_field() - Automatically handles SQLite boolean compatibility
        is_active: sqlite_bool_field(),
        is_pinned: sqlite_bool_field(),
        is_verified: sqlite_bool_field_with_default(false),
    }
}
```

### Solution 2: Manual serde Attributes + Universal Deserializer

```rust
use rat_quickdb::*;
use serde::Deserialize;

#[derive(Debug, Serialize, Deserialize)]
struct User {
    id: Option<i32>,
    username: String,

    // Manually specify deserializer
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
        // Use traditional boolean_field() - with manual serde attributes
        is_active: boolean_field(),
        is_pinned: boolean_field(),
    }
}
```

### Solution 3: Traditional Method (Requires Manual Handling)

```rust
// For existing code, you can use traditional boolean_field()
// But need to ensure boolean value format is correct in data source
rat_quickdb::define_model! {
    struct User {
        id: Option<i32>,
        username: String,
        is_active: bool,        // Requires manual compatibility handling
    }

    collection = "users",
    fields = {
        id: integer_field(None, None),
        username: string_field(Some(50), Some(3), None).required(),
        is_active: boolean_field(),  // Traditional method
    }
}
```

### Deserializer Selection Guide

- `deserialize_bool_from_any()`: Supports integers, booleans, strings "true"/"false"
- `deserialize_bool_from_int()`: Supports integers and booleans
- `sqlite_bool_field()`: Automatically selects the best deserializer

### Migration Guide

Migrating from traditional `boolean_field()` to `sqlite_bool_field()`:

```rust
// Before (may have compatibility issues)
is_active: boolean_field(),

// After (fully compatible)
is_active: sqlite_bool_field(),
```

## 🆔 ID Generation Strategies

rat_quickdb supports multiple ID generation strategies for different use cases:

### AutoIncrement (Auto-increment ID)
```rust
DatabaseConfig::builder()
    .id_strategy(IdStrategy::AutoIncrement)
    .build()?
```

### UUID (Universally Unique Identifier)
```rust
DatabaseConfig::builder()
    .id_strategy(IdStrategy::Uuid)
    .build()?
```

### Snowflake (Snowflake Algorithm)
```rust
DatabaseConfig::builder()
    .id_strategy(IdStrategy::Snowflake {
        machine_id: 1,
        datacenter_id: 1
    })
    .build()?
```

### ObjectId (MongoDB Style)
```rust
DatabaseConfig::builder()
    .id_strategy(IdStrategy::ObjectId)
    .build()?
```

### Custom (Custom Prefix)
```rust
DatabaseConfig::builder()
    .id_strategy(IdStrategy::Custom("user_".to_string()))
    .build()?
```

## 🧠 Cache Configuration

### Basic Cache Configuration (L1 Memory Cache Only)
```rust
use rat_quickdb::types::{CacheConfig, CacheStrategy, TtlConfig, L1CacheConfig};

let cache_config = CacheConfig {
    enabled: true,
    strategy: CacheStrategy::Lru,
    ttl_config: TtlConfig {
        default_ttl_secs: 300,  // 5 minutes cache
        max_ttl_secs: 3600,     // maximum 1 hour
        check_interval_secs: 60, // check interval
    },
    l1_config: L1CacheConfig {
        max_capacity: 1000,     // maximum 1000 entries
        max_memory_mb: 64,       // 64MB memory limit
        enable_stats: true,      // enable statistics
    },
    l2_config: None,           // no L2 disk cache
    compression_config: CompressionConfig::default(),
    version: "1".to_string(),
};

DatabaseConfig::builder()
    .cache(cache_config)
    .build()?
```

### L1+L2 Cache Configuration (Built-in L2 Cache Support)
```rust
use rat_quickdb::types::{CacheConfig, CacheStrategy, TtlConfig, L1CacheConfig, L2CacheConfig};
use std::path::PathBuf;

let cache_config = CacheConfig {
    enabled: true,
    strategy: CacheStrategy::Lru,
    ttl_config: TtlConfig {
        default_ttl_secs: 1800, // 30 minutes cache
        max_ttl_secs: 7200,     // maximum 2 hours
        check_interval_secs: 120, // check interval
    },
    l1_config: L1CacheConfig {
        max_capacity: 5000,     // maximum 5000 entries
        max_memory_mb: 128,      // 128MB memory limit
        enable_stats: true,      // enable statistics
    },
    l2_config: Some(L2CacheConfig {
        max_size_mb: 1024,      // 1GB disk cache
        cache_dir: PathBuf::from("./cache"), // cache directory
        enable_persistence: true, // enable persistence
        enable_compression: true, // enable compression
        cleanup_interval_secs: 300, // cleanup interval
    }),
    compression_config: CompressionConfig::default(),
    version: "1".to_string(),
};

DatabaseConfig::builder()
    .cache(cache_config)
    .build()?
```

**L2 Cache Feature Notes**:
- L2 cache functionality is built-in to `rat_memcache`, no additional features needed
- Requires disk space for cache data storage
- Suitable for caching large amounts of data or scenarios requiring persistence
- Simply configure `l2_config` in `CacheConfig` to enable L2 cache

### Cache Statistics and Management
```rust
// Get cache statistics
let stats = get_cache_stats("default").await?;
println!("Cache hit rate: {:.2}%", stats.hit_rate * 100.0);
println!("Cache entries: {}", stats.entries);

// Clear cache
clear_cache("default").await?;
clear_all_caches().await?;
```

## 📝 Logging Control

rat_quickdb now gives complete logging initialization control to the caller:

```rust
use rat_logger::{Logger, LoggerBuilder, LevelFilter};

// Caller is responsible for initializing the logging system
let logger = LoggerBuilder::new()
    .with_level(LevelFilter::Debug)
    .with_file("app.log")
    .build();

logger.init().expect("Failed to initialize logging");

// Then initialize rat_quickdb (no longer auto-initializes logging)
rat_quickdb::init();
```

## 🔧 Database Configuration

### Recommended: Builder Pattern

**Recommended**: Use `DatabaseConfig::builder()` pattern for complete configuration control and type safety:

```rust
use rat_quickdb::*;
use rat_quickdb::types::{DatabaseType, ConnectionConfig, PoolConfig, IdStrategy};

let pool_config = PoolConfig::builder()
    .max_connections(10)
    .min_connections(2)
    .connection_timeout(5000)
    .idle_timeout(300000)
    .max_lifetime(1800000)
    .build()?;

// SQLite Configuration
let sqlite_config = DatabaseConfig::builder()
    .db_type(DatabaseType::SQLite)
    .connection(ConnectionConfig::SQLite {
        path: "./test.db".to_string(),
        create_if_missing: true,
    })
    .pool_config(pool_config.clone())
    .alias("sqlite_db".to_string())
    .id_strategy(IdStrategy::AutoIncrement)  // Recommended strategy
    .build()?;

// PostgreSQL Configuration
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
    .id_strategy(IdStrategy::Uuid)  // PostgreSQL recommends UUID
    .build()?;

// MySQL Configuration
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
    .id_strategy(IdStrategy::AutoIncrement)  // MySQL recommends auto-increment
    .build()?;

// MongoDB Configuration
let mongodb_config = DatabaseConfig::builder()
    .db_type(DatabaseType::MongoDB)
    .connection(ConnectionConfig::MongoDB(
        MongoDbConnectionBuilder::new("localhost", 27017, "mydatabase")
            .with_auth("username", "password")
            .build()
    ))
    .pool_config(pool_config)
    .alias("mongodb_db".to_string())
    .id_strategy(IdStrategy::ObjectId)  // MongoDB recommends ObjectId
    .build()?;

// Add to connection pool manager
add_database(sqlite_config).await?;
add_database(postgres_config).await?;
add_database(mysql_config).await?;
add_database(mongodb_config).await?;
```

### Advanced MongoDB Configuration

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

### 🚨 Deprecated Convenience Functions (Not Recommended)

> **Important Warning**: The following convenience functions are marked as deprecated and will be removed in v0.4.0. Please use the recommended builder pattern above.

```rust
// 🚨 Deprecated - Do not use in new projects
// These functions have API inconsistencies and hardcoded defaults

// Deprecated SQLite configuration
let config = sqlite_config(  // 🚨 Deprecated
    "sqlite_db",
    "./test.db",
    pool_config,
    Some(IdStrategy::AutoIncrement)  // Must explicitly specify
)?;

// Deprecated PostgreSQL configuration
let config = postgres_config(  // 🚨 Deprecated
    "postgres_db",
    "localhost",
    5432,
    "mydatabase",
    "username",
    "password",
    pool_config,
    Some(IdStrategy::Uuid)
)?;

// Deprecated MySQL configuration
let config = mysql_config(  // 🚨 Deprecated
    "mysql_db",
    "localhost",
    3306,
    "mydatabase",
    "username",
    "password",
    pool_config,
    Some(IdStrategy::AutoIncrement)
)?;

// Deprecated MongoDB configuration
let config = mongodb_config(  // 🚨 Deprecated
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

**Deprecation Reasons**:
- ❌ API inconsistencies: Different databases have different convenience function parameters
- ❌ Hardcoded defaults: Violates the "no nanny settings" design principle
- ❌ Limited functionality: Cannot support advanced configuration options
- ❌ Maintenance difficulty: Duplicate code increases maintenance costs

**Recommended Alternatives**:
- ✅ **Builder Pattern**: Type-safe, complete configuration, consistent API
- ✅ **Full Control**: Users have complete control over all configuration options
- ✅ **Extensible**: Supports all advanced database features
- ✅ **Type Safety**: Compile-time configuration validation

### ID Strategy Recommendations

Choose the most suitable ID strategy based on database characteristics:

| Database | Recommended | Alternative | Description |
|----------|-------------|-------------|-------------|
| **SQLite** | AutoIncrement | ObjectId | AutoIncrement has native support and best performance |
| **PostgreSQL** | UUID | AutoIncrement | UUID has native support and type safety |
| **MySQL** | AutoIncrement | ObjectId | AutoIncrement has native support and best performance |
| **MongoDB** | ObjectId | AutoIncrement | ObjectId has native support, MongoDB ecosystem standard |

**Important Note**: When using UUID strategy with PostgreSQL, all foreign key fields in related tables must also use UUID type to maintain type consistency.

## 🛠️ Core API

### Database Management
- `init()` - Initialize library
- `add_database(config)` - Add database configuration
- `remove_database(alias)` - Remove database configuration
- `get_aliases()` - Get all database aliases
- `set_default_alias(alias)` - Set default database alias

### Model Operations (Recommended)
```rust
// Save record
let user_id = user.save().await?;

// Query record
let found_user = ModelManager::<User>::find_by_id(&user_id).await?;
let users = ModelManager::<User>::find(conditions, options).await?;

// Update record
let mut updates = HashMap::new();
updates.insert("username".to_string(), DataValue::String("new_name".to_string()));
let updated = user.update(updates).await?;

// Delete record
let deleted = user.delete().await?;
```

### ODM Operations (Low-level)
- `create(collection, data, alias)` - Create record
- `find_by_id(collection, id, alias)` - Find by ID
- `find(collection, conditions, options, alias)` - Query records
- `update(collection, id, data, alias)` - Update record
- `delete(collection, id, alias)` - Delete record
- `count(collection, query, alias)` - Count records
- `exists(collection, query, alias)` - Check existence

## 🏗️ Architecture Features

rat_quickdb adopts modern architecture design:

1. **Lock-free Queue Architecture**: Avoids direct database connection lifecycle issues
2. **Model Auto-registration**: Automatically registers model metadata on first use
3. **Auto Index Management**: Automatically creates tables and indexes based on model definitions
4. **Cross-database Adapter**: Unified interface supporting multiple database types
5. **Async Message Processing**: Efficient async processing based on Tokio

## 🔄 Workflow

```
Application Layer → Model Operations → ODM Layer → Message Queue → Connection Pool → Database
    ↑                                                             ↓
    └───────────────── Result Return ←───────────────────────────┘
```

## 📊 Performance Features

- **Connection Pool Management**: Intelligent connection reuse and management
- **Async Operations**: Non-blocking database operations
- **Batch Processing**: Supports batch operation optimization
- **Cache Integration**: Built-in caching reduces database access
- **Compression Support**: MongoDB supports ZSTD compression

## 🎯 Supported Field Types

- `integer_field` - Integer fields (with range and constraints)
- `string_field` - String fields (with length limits, can use large length as text replacement)
- `float_field` - Floating-point number fields (with range and precision)
- `boolean_field` - Boolean fields
- `datetime_field` - Date-time fields
- `uuid_field` - UUID fields
- `json_field` - JSON fields
- `array_field` - Array fields
- `list_field` - List fields (array_field alias)
- `dict_field` - Dictionary/Object fields (based on Object type)
- `reference_field` - Reference fields (foreign keys)

## 📝 Index Support

- **Unique Indexes**: `unique()` constraints
- **Composite Indexes**: Multi-field combination indexes
- **Regular Indexes**: Basic query optimization indexes
- **Auto Creation**: Automatically created based on model definitions
- **Cross-database**: Supports all database index types

## 🌟 Version Information

**Current Version**: 0.3.2

**Supported Rust Version**: 1.70+

**Important Update**: v0.3.0 enforces define_model! macro for model definitions, fixing major architecture issues and improving type safety!

## 📄 License

This project is licensed under the [LGPL-v3](LICENSE) license.

## 🤝 Contributing

Welcome to submit Issues and Pull Requests to improve this project!

## 📞 Contact

For questions or suggestions, please contact:
- Create Issue: [GitHub Issues](https://github.com/your-repo/rat_quickdb/issues)
- Email: oldmos@gmail.com