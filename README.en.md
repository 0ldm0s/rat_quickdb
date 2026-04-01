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
- **📋 Stored Procedures**: Cross-database unified stored procedure API with multi-table JOIN and aggregation queries

## 🔄 Version Changes

### v0.5.1 - Version Update

**New Features:**
- 🎯 **Case-Insensitive Query**: All database adapters now support case-insensitive string queries
- 🔄 **Dual Type System**: Provides simplified and full query condition types to meet different usage scenarios
- 📊 **Cross-Database Support**: MongoDB, MySQL, PostgreSQL, SQLite all supported
- 🔄 **Automatic Type Conversion**: Simplified version automatically converted to full version, no manual handling required

**Type Description:**

This version provides two query condition types:

1. **`QueryCondition` (Simplified)**: Suitable for most scenarios
   - Does not contain `case_insensitive` field
   - Default case-sensitive
   - Simpler to use, clearer code

2. **`QueryConditionWithConfig` (Full)**: Suitable for scenarios requiring configuration
   - Contains `case_insensitive` field to control case sensitivity
   - Supports more configuration options for the future
   - Used for queries requiring advanced features like case-insensitive matching

**Usage Examples:**

```rust
use rat_quickdb::*;

// ===== Simplified: Default case-sensitive query (recommended for daily use) =====
let results = ModelManager::<User>::find(
    vec![QueryCondition {
        field: "username".to_string(),
        operator: QueryOperator::Eq,
        value: DataValue::String("admin".to_string()),
        // No case_insensitive field, default case-sensitive
    }],
    None
).await?;

// ===== Full: Case-insensitive query =====
let insensitive_results = ModelManager::<User>::find_with_config(
    vec![QueryConditionWithConfig {
        field: "username".to_string(),
        operator: QueryOperator::Eq,
        value: DataValue::String("admin".to_string()),
        case_insensitive: true,  // Enable case-insensitive
    }],
    None
).await?;

// ===== Full: Case-sensitive query (explicit) =====
let sensitive_results = ModelManager::<User>::find_with_config(
    vec![QueryConditionWithConfig {
        field: "username".to_string(),
        operator: QueryOperator::Eq,
        value: DataValue::String("ADMIN".to_string()),
        case_insensitive: false,  // Explicitly disable case-insensitive
    }],
    None
).await?;
```

**Method Correspondence:**

| Simplified Method | Full Method | Description |
|------------------|-------------|-------------|
| `find(conditions)` | `find_with_config(conditions)` | Find records |
| `count(conditions)` | `count_with_config(conditions)` | Count records |
| `delete_many(conditions)` | `delete_many_with_config(conditions)` | Batch delete |
| `find_with_cache_control(conditions, options, bypass)` | (internal method) | Cache control |

**Automatic Conversion Mechanism:**

All simplified methods internally convert `QueryCondition` to `QueryConditionWithConfig` (`case_insensitive` defaults to `false`), no manual handling required.

**Implementation:**
- **MongoDB**: Uses regex `$regex: "^value$", $options: "i"`
- **MySQL**: Uses `LOWER(field) = LOWER(value)`
- **PostgreSQL**: Uses `LOWER(field) = LOWER(value)`
- **SQLite**: Uses `LOWER(field) = LOWER(value)`

**Use Cases:**
- 📧 Username/email query (users may input any case)
- 🔍 Product name search (case-insensitive)
- 🏷️ Tag and category query (improved query friendliness)
- 🌍 Multi-language text search (adapts to different language case rules)

**Performance Notes:**
- Enabling case-insensitive queries on string fields slightly reduces query performance
- Recommended for fuzzy match fields, keep default case-sensitive for exact match fields
- Can optimize performance by creating functional indexes (e.g., `LOWER(field)`)

**Testing:**
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

### v0.4.5 - Unified Table Not Exist Error Handling

**New Features:**
- 🎯 **Unified TableNotExistError**: All database adapters now provide consistent table-not-exist error recognition
- 🔄 **MongoDB Special Handling**: Pragmatic strategy for MongoDB's collection auto-creation feature
- 📊 **Unified Interface**: Callers don't need to distinguish database types, get consistent error handling experience
- 🎛️ **Business-Friendly**: Clear error expectations, easy for business logic handling

**Core Improvements:**
```rust
// Unified table not exist error handling
match ModelManager::<User>::find_by_id("non-existent-id").await {
    Err(QuickDbError::TableNotExistError { table, message }) => {
        println!("Table does not exist: {}", table);
        // Caller knows explicitly that data needs to be initialized
    }
    // ... other error handling
}
```

**MongoDB Special Strategy:**
- Querying non-existent collection or empty collection returns `TableNotExistError`
- After caller receives error, inserting data will automatically create collection
- Provides unified error interface, hiding MongoDB semantic differences

### v0.4.2 - Cache Bypass Feature

**New Features:**
- 🎯 **Cache Bypass Support**: New `find_with_cache_control` method, supports forced cache skip query
- 🔄 **Backward Compatibility**: Original `find` method remains unchanged, acts as wrapper for new method
- 📊 **Performance Comparison**: Provides cache bypass performance test examples, showing actual performance differences
- 🎛️ **Flexible Control**: Can choose to use cache or force database query based on business needs

**Usage Examples:**
```rust
// Force skip cache query (suitable for real-time data scenarios like finance)
let results = ModelManager::<User>::find_with_cache_control(
    conditions,
    None,
    true  // bypass_cache = true
).await?;

// Normal cache query (default behavior)
let results = ModelManager::<User>::find(conditions, None).await?;
```

**Performance Test Examples:**
```bash
# Run cache bypass performance tests
cargo run --example cache_bypass_comparison_mysql --features mysql-support
cargo run --example cache_bypass_comparison_pgsql --features postgres-support
cargo run --example cache_bypass_comparison_sqlite --features sqlite-support
cargo run --example cache_bypass_comparison_mongodb --features mongodb-support
```

### v0.3.6 - Stored Procedure Virtual Table System

⚠️ **Important Change: Connection Pool Configuration Parameter Unit Changes**

**v0.3.6** introduces major improvements to connection pool configuration. **All timeout parameters now use seconds as units**:

```rust
// v0.3.6 new syntax (recommended)
let pool_config = PoolConfig::builder()
    .connection_timeout(30)        // 30 seconds (previously 5000 milliseconds)
    .idle_timeout(300)             // 300 seconds (previously 300000 milliseconds)
    .max_lifetime(1800)            // 1800 seconds (previously 1800000 milliseconds)
    .max_retries(3)                // New: Maximum retry count
    .retry_interval_ms(1000)       // New: Retry interval (milliseconds)
    .keepalive_interval_sec(60)    // New: Keepalive interval (seconds)
    .health_check_timeout_sec(10)  // New: Health check timeout (seconds)
    .build()?;
```

⚠️ **Important Change: Mandatory Builder Mode**

**v0.5.3+** requires using `PoolConfig::builder()` to create connection pool configuration, **direct struct construction is prohibited**:

```rust
// ✅ Correct: Use builder mode
let pool_config = PoolConfig::builder()
    .max_connections(10)
    .min_connections(2)
    .connection_timeout(30)
    .build()?;

// ❌ Error: Direct struct construction will cause compilation error
// let pool_config = PoolConfig {
//     max_connections: 10,
//     min_connections: 2,
//     ...
// };
```

**Reason**: Builder mode ensures all configuration parameters are validated, avoiding invalid configuration values.

**New Features:**
- 🎯 **Stored Procedure Virtual Table System**: Cross-four-database unified stored procedure API
- 🔗 **Multi-table JOIN Support**: Automatic JOIN statement and aggregation pipeline generation
- 📊 **Aggregation Query Optimization**: Automatic GROUP BY clause generation (SQL databases)
- 🧠 **Type-safe Stored Procedures**: Compile-time validation and type checking

## 📦 Installation

Add dependency in `Cargo.toml`:

```toml
[dependencies]
rat_quickdb = "0.5.1"
```

### 🔧 Feature Control

rat_quickdb uses Cargo features to control different database support and functionality. By default, only core functionality is included. You need to enable features based on the database types you use:

```toml
[dependencies]
rat_quickdb = { version = "0.5.1", features = [
    "sqlite-support",    # Support SQLite database
    "postgres-support",  # Support PostgreSQL database
    "mysql-support",     # Support MySQL database
    "mongodb-support",   # Support MongoDB database
] }
```

#### Available Features List

| Feature Name | Description | Default Enabled |
|-------------|-------------|-----------------|
| `sqlite-support` | SQLite database support | ❌ |
| `postgres-support` | PostgreSQL database support | ❌ |
| `mysql-support` | MySQL database support | ❌ |
| `mongodb-support` | MongoDB database support | ❌ |
| `melange-storage` | Deprecated: L2 cache functionality is built into rat_memcache | ❌ |
| `python-bindings` | Python API bindings | ❌ |
| `full` | Enable all database support | ❌ |

#### Database Version Requirements

**Important**: Different databases have different version requirements for JSON operations and regex support:

| Database | Minimum Version | JSON Support | Contains Operator | JsonContains Operator | Regex Operator |
|----------|-----------------|---------------|-------------------|----------------------|----------------|
| **MySQL** | 5.7+ / MariaDB 10.2+ | ✅ Full support | String fields use LIKE, JSON fields use JSON_CONTAINS() | ❌ Not supported | ✅ REGEXP operator |
| **PostgreSQL** | 9.2+ | ✅ Full support | String fields use LIKE, JSON fields use @> operator | ✅ Full support | ✅ `~` operator |
| **SQLite** | 3.38.0+ | ✅ Basic support | Only string fields support LIKE | ❌ Not supported | ❌ Not supported |
| **MongoDB** | 7.0+ | ✅ Native support | Native $regex operator | ✅ Full support | ✅ Native $regex operator |

⚠️ **Version Compatibility Notes**:
- MySQL 5.6 and below does not support JSON_CONTAINS function, will cause runtime errors
- Early PostgreSQL versions may need to enable JSON extension
- SQLite JSON functionality is optional, needs to be enabled at compile time

#### 🔍 Regular Expression Query (Regex)

rat_quickdb supports cross-database regular expression queries using `QueryOperator::Regex` operator:

**Database-specific Implementation**:

| Database | Operator | Syntax Example | Status |
|----------|----------|----------------|--------|
| **PostgreSQL** | `~` | `WHERE field ~ 'pattern'` | ✅ Full support |
| **MySQL** | `REGEXP` | `WHERE field REGEXP 'pattern'` | ✅ Full support |
| **MongoDB** | `$regex` | `{ field: { $regex: 'pattern' } }` | ✅ Full support |
| **SQLite** | - | - | ❌ Not supported |

**Usage Example**:

```rust
use rat_quickdb::*;

// Regex query: match usernames containing "_wang" or "_chen"
let conditions = vec![QueryCondition {
    field: "username".to_string(),
    operator: QueryOperator::Regex,
    value: DataValue::String(".*_wang|_chen.*".to_string()),
}];

let users = ModelManager::<User>::find(conditions, None).await?;
```

**Running Regex Query Examples**:

```bash
# PostgreSQL regex query
cargo run --example string_fuzzy_search_pgsql --features postgres-support

# MySQL regex query
cargo run --example string_fuzzy_search_mysql --features mysql-support

# MongoDB regex query
cargo run --example string_fuzzy_search_mongodb --features mongodb-support

# SQLite (does not support regex)
cargo run --example string_fuzzy_search_sqlite --features sqlite-support
```

**Notes**:
- ❌ **SQLite does not support REGEXP**: SQLite does not support regular expression queries
- ✅ **PostgreSQL uses `~` operator**: The framework automatically adapts to PostgreSQL's regex syntax
- ✅ **MySQL uses `REGEXP` function**: The framework automatically adapts to MySQL's regex syntax
- ✅ **MongoDB uses `$regex` operator**: The framework automatically adapts to MongoDB's regex syntax

**⚠️ Cross-Database Compatibility Recommendation**:
> **Important**: If your application needs cross-database support (especially needing to support SQLite), **strongly recommend NOT using regular expression queries**. Please use these alternatives instead:
> - **Contains Operator**: For simple contains matching (`LIKE '%value%'`)
> - **StartsWith Operator**: For prefix matching (`LIKE 'value%'`)
> - **EndsWith Operator**: For suffix matching (`LIKE '%value'`)
> - **Multiple OR conditions**: For complex pattern matching (e.g., `field = 'value1' OR field = 'value2'`)
>
> These alternatives work properly across all databases, ensuring your application's cross-database compatibility.

**Performance Recommendations**:
- Regular expression queries are slower than exact matching and fuzzy matching (LIKE)
- Recommended to create indexes for commonly queried regex fields
- Complex regular expressions may affect query performance, please use with caution

#### Enable Features as Needed

**SQLite only**:
```toml
[dependencies]
rat_quickdb = { version = "0.5.1", features = ["sqlite-support"] }
```

**PostgreSQL**:
```toml
[dependencies]
rat_quickdb = { version = "0.5.1", features = ["postgres-support"] }
```

**All databases**:
```toml
[dependencies]
rat_quickdb = { version = "0.5.1", features = ["full"] }
```

**L2 Cache Configuration Notes**:
- L2 cache functionality is built-in to `rat_memcache`, no additional features needed
- L2 cache requires disk space for cache persistence
- Configuration examples are in the "Cache Configuration" section below

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
    id: String::new(), // Framework automatically generates ID
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
1. **Recommended**: Migrate to builder pattern for better type safety and consistency
```rust
// Recommended to use builder pattern instead of convenience functions:
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

// PostgreSQL using UUID (PostgreSQL recommended)
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

2. **Temporary Compatibility**: If you must temporarily maintain existing code, please add the required `IdStrategy` parameter, but plan migration to builder pattern as soon as possible

**Affected Scope**:
- All code using convenience functions for database configuration
- Code using `mongodb_config_with_builder` (duplicate function removed)
- Applications relying on specific database default ID strategies

This change ensures configuration consistency and user control, aligning with the "no nanny settings" design principle.

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

See `examples/model_definition.rs` for complete model definitions, CRUD operations, and complex query examples.

### Field Types and Validation

See field type definitions and validation examples in `examples/model_definition.rs`.

### Index Management

Indexes are automatically created based on model definitions, no manual management needed. Refer to `examples/model_definition.rs` for index definition methods.

## 🔒 SQLite Boolean Compatibility

SQLite database stores boolean values as integers (0 and 1), which may cause serde deserialization errors. rat_quickdb provides multiple solutions:

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
        // Use sqlite_bool_field() - automatically handles SQLite boolean compatibility
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
        // Use traditional boolean_field() - combined with manual serde attributes
        is_active: boolean_field(),
        is_pinned: boolean_field(),
    }
}
```

### Solution 3: Traditional Method (Requires Manual Handling)

```rust
// For existing code, you can use traditional boolean_field()
// But need to ensure boolean value format in data source is correct
rat_quickdb::define_model! {
    struct User {
        id: Option<i32>,
        username: String,
        is_active: bool,        // Need to manually handle compatibility
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
- `sqlite_bool_field()`: Automatically selects best deserializer

### Migration Guide

Migrating from traditional `boolean_field()` to `sqlite_bool_field()`:

```rust
// Before (may have compatibility issues)
is_active: boolean_field(),

// After (fully compatible)
is_active: sqlite_bool_field(),
```

## 🆔 ID Generation Strategies

rat_quickdb supports multiple ID generation strategies to meet different scenario needs:

### AutoIncrement (Auto-increment ID) - Default Recommended
```rust
DatabaseConfig::builder()
    .id_strategy(IdStrategy::AutoIncrement)
    .build()?

// Convenience function usage
let config = sqlite_config(
    "sqlite_db",
    "./test.db",
    pool_config,
    Some(IdStrategy::AutoIncrement)
)?;
```

### UUID (Universal Unique Identifier) - PostgreSQL Recommended
```rust
DatabaseConfig::builder()
    .id_strategy(IdStrategy::Uuid)
    .build()?

// Convenience function usage
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

#### ⚠️ PostgreSQL UUID Strategy Special Requirements

**Important Reminder**: PostgreSQL has strict requirements for type consistency. If using UUID strategy:

1. **Primary Key Table**: ID field will be UUID type
2. **Related Tables**: All foreign key fields must also be UUID type
3. **Type Matching**: UUID types cannot be associated with other types

**Example**:
```rust
// User table using UUID ID
define_model! {
    struct User {
        id: String,  // Will be mapped to PostgreSQL UUID type
        username: String,
    }
    collection = "users",
    fields = {
        id: uuid_field(),
        username: string_field(Some(50), Some(3), None).required(),
    }
}

// Order table's foreign key must also use UUID type
define_model! {
    struct Order {
        id: String,
        user_id: String,  // Must be UUID type to match users.id
        amount: f64,
    }
    collection = "orders",
    fields = {
        id: uuid_field(),
        user_id: uuid_field().required(),  // Foreign key must use same type
        amount: float_field(None, None),
    }
}
```

**Solutions**:
- For new projects: PostgreSQL recommends comprehensive use of UUID strategy
- For existing projects: Can use `IdStrategy::Custom` to manually generate UUID strings as compatibility solution
- Mixed strategy: Primary table uses UUID, related tables must also use UUID, maintaining type consistency

#### ✨ PostgreSQL UUID Auto-conversion Feature

Starting from v0.3.4 version, PostgreSQL adapter supports **auto-conversion** for UUID fields, allowing users to use string UUIDs for query operations.

**Feature Highlights**:
- **Auto-conversion**: Pass string UUID during query, adapter automatically converts to UUID type
- **Strict validation**: Invalid UUID formats directly error, no fault-tolerant fixes
- **User-friendly**: Maintains API consistency, no need to manually convert UUID types
- **Type safety**: Ensures UUID type consistency at database level

**Usage Example**:
```rust
// User model definition (Note: use String in struct, uuid_field in field definition)
define_model! {
    struct User {
        id: String,  // ⚠️ Must use String in struct
        username: String,
    }
    collection = "users",
    fields = {
        id: uuid_field(),  // ⚠️ Must use uuid_field in field definition
        username: string_field(Some(50), Some(3), None).required(),
    }
}

// Article model, author_id is UUID foreign key
define_model! {
    struct Article {
        id: String,
        title: String,
        author_id: String,  // ⚠️ Must use String in struct
    }
    collection = "articles",
    fields = {
        id: uuid_field(),
        title: string_field(Some(200), Some(1), None).required(),
        author_id: uuid_field().required(),  // ⚠️ Must use uuid_field in field definition
    }
}

// Query: directly use string UUID, auto-convert!
let conditions = vec![
    QueryCondition {
        field: "author_id".to_string(),
        operator: QueryOperator::Eq,
        value: DataValue::String("550e8400-e29b-41d4-a716-446655440000".to_string()),
    }
];

let articles = ModelManager::<Article>::find(conditions, None).await?;
// PostgreSQL adapter automatically converts string to UUID type for query
```

#### ⚠️ Counter-intuitive Design Requirements (Important!)

**Current Limitation**: When using UUID strategy, model definition has a **counter-intuitive** design requirement:

```rust
define_model! {
    struct User {
        id: String,           // ⚠️ Must use String type in struct
        // Cannot write: id: uuid::Uuid
    }
    fields = {
        id: uuid_field(),     // ⚠️ But must use uuid_field() in field definition
        // Cannot write: id: string_field(...)
    }
}
```

**Why is this like this?**
1. **Rust type system limitations**: Macro system needs unified base types when generating models
2. **Database type mapping**: `uuid_field()` tells adapter to create UUID database column
3. **Query conversion**: Runtime string UUID automatically converted to UUID database type

**Correct Usage**:
- ✅ **Struct fields**: Always use `String` type
- ✅ **Field definitions**: UUID fields use `uuid_field()`, other fields use corresponding functions
- ✅ **Query operations**: Directly use `DataValue::String("uuid-string")`, auto-convert
- ✅ **Type safety**: PostgreSQL database level maintains UUID type consistency

**Incorrect Usage**:
- ❌ Use `uuid::Uuid` type in struct (compilation error)
- ❌ Define UUID fields with `string_field()` (lose UUID type support)
- ❌ Mix UUID strategies across different databases (type mismatch)

**Reasons it cannot be resolved temporarily**:
- Rust macro system type inference limitations
- Need to maintain backward compatibility with existing code
- Cross-database unified API design requirements

**Future improvement directions**:
- v0.4.0 plans to introduce more intuitive type-safe UUID field definitions
- Consider using compile-time type inference to reduce this inconsistency
- Provide clearer compile-time error messages

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

## 🔄 ObjectId Cross-database Handling

rat_quickdb provides consistent handling for ObjectId strategy across databases, ensuring normal operation under different database backends.

### Storage Method Differences

**MongoDB**:
- Stored as native `ObjectId` type
- Returns MongoDB native ObjectId object when querying
- Best performance, supports all MongoDB ObjectId features

**Other databases (SQLite, PostgreSQL, MySQL)**:
- Stored as 24-digit hexadecimal string (e.g., `507f1f77bcf86cd799439011`)
- Returns string format ObjectId when querying
- Maintains compatibility with MongoDB ObjectId format

### Usage Example

```rust
// MongoDB - Native ObjectId support
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

// SQLite/PostgreSQL/MySQL - String format ObjectId
let config = sqlite_config(
    "sqlite_db",
    "./test.db",
    pool_config,
    Some(IdStrategy::ObjectId)
)?;
```

### Model Definition

ObjectId strategy uniformly uses `String` type in model definitions:

```rust
define_model! {
    struct Document {
        id: String,  // MongoDB is ObjectId, other databases are string
        title: String,
        content: String,
    }
    collection = "documents",
    fields = {
        id: string_field(None, None),  // Uniformly use string_field
        title: string_field(Some(200), Some(1), None).required(),
        content: string_field(Some(10000), None, None),
    }
}
```

### Query and Operations

```rust
// Create document
let doc = Document {
    id: String::new(),  // Auto-generate ObjectId
    title: "Example Document".to_string(),
    content: "Document content".to_string(),
};
let doc_id = doc.save().await?;

// Query document
let found_doc = ModelManager::<Document>::find_by_id(&doc_id).await?;

// Note: ObjectId is 24-digit hexadecimal string format
assert_eq!(doc_id.len(), 24);  // Other databases
// In MongoDB, this will be a native ObjectId object
```

### Type Conversion Handling

rat_quickdb automatically handles ObjectId type conversion in different databases:

1. **When saving**: Auto-generate ObjectId format (string or native object)
2. **When querying**: Return original format, framework handles conversion internally
3. **When migrating**: Data format remains compatible across different databases

### Performance Considerations

- **MongoDB**: Native ObjectId has best performance, supports index optimization
- **Other databases**: String index performance is good, fixed length (24 characters)
- **Cross-database**: Unified string format facilitates data migration and synchronization

This design ensures ObjectId strategy works consistently across all supported databases while fully utilizing each database's native features.

## 🧠 Cache Configuration

### Basic Cache Configuration (L1 Memory Cache Only)
```rust
use rat_quickdb::types::{CacheConfig, CacheStrategy, TtlConfig, L1CacheConfig};

let cache_config = CacheConfig {
    enabled: true,
    strategy: CacheStrategy::Lru,
    ttl_config: TtlConfig {
        default_ttl_secs: 300,  // 5 minutes cache
        max_ttl_secs: 3600,     // Maximum 1 hour
        check_interval_secs: 60, // Check interval
    },
    l1_config: L1CacheConfig {
        max_capacity: 1000,     // Maximum 1000 entries
        max_memory_mb: 64,       // 64MB memory limit
        enable_stats: true,      // Enable statistics
    },
    l2_config: None,           // Do not use L2 disk cache
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
        max_ttl_secs: 7200,     // Maximum 2 hours
        check_interval_secs: 120, // Check interval
    },
    l1_config: L1CacheConfig {
        max_capacity: 5000,     // Maximum 5000 entries
        max_memory_mb: 128,      // 128MB memory limit
        enable_stats: true,      // Enable statistics
    },
    l2_config: Some(L2CacheConfig {
        max_size_mb: 1024,      // 1GB disk cache
        cache_dir: PathBuf::from("./cache"), // Cache directory
        enable_persistence: true, // Enable persistence
        enable_compression: true, // Enable compression
        cleanup_interval_secs: 300, // Cleanup interval
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
- Requires disk space to store cache data
- Suitable for caching large amounts of data or scenarios needing persistence
- Just configure `l2_config` in `CacheConfig` to enable L2 cache

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

rat_quickdb is now completely controlled by the caller for logging initialization:

```rust
use rat_logger::{Logger, LoggerBuilder, LevelFilter};

// Caller is responsible for initializing logging system
let logger = LoggerBuilder::new()
    .with_level(LevelFilter::Debug)
    .with_file("app.log")
    .build();

logger.init().expect("Logging initialization failed");

// Then initialize rat_quickdb (no longer auto-initializes logging)
rat_quickdb::init();
```

## 🔧 Database Configuration

### Recommended: Use Builder Pattern

**Recommended**: Use `DatabaseConfig::builder()` pattern to provide complete configuration control and type safety:

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

// SQLite configuration
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

// PostgreSQL configuration
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
    .id_strategy(IdStrategy::Uuid)  // PostgreSQL recommends UUID strategy
    .build()?;

// MySQL configuration
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
    .id_strategy(IdStrategy::AutoIncrement)  // MySQL recommends auto-increment strategy
    .build()?;

// MongoDB configuration
let mongodb_config = DatabaseConfig::builder()
    .db_type(DatabaseType::MongoDB)
    .connection(ConnectionConfig::MongoDB(
        MongoDbConnectionBuilder::new("localhost", 27017, "mydatabase")
            .with_auth("username", "password")
            .build()
    ))
    .pool_config(pool_config)
    .alias("mongodb_db".to_string())
    .id_strategy(IdStrategy::ObjectId)  // MongoDB recommends ObjectId strategy
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
// These functions have API inconsistencies and hardcoded issues

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
- ❌ API inconsistency: Different databases have inconsistent convenience function parameters
- ❌ Hardcoded default values: Violates "no nanny settings" design principle
- ❌ Feature limitations: Cannot support advanced configuration options
- ❌ Maintenance difficulty: Duplicate code increases maintenance costs

**Recommended Alternatives**:
- ✅ **Builder pattern**: Type safe, complete configuration, unified API
- ✅ **Complete control**: Users have complete control over all configuration options
- ✅ **Extensibility**: Supports advanced features for all databases
- ✅ **Type safety**: Compile-time configuration correctness checks

### ID Strategy Recommendations

Choose the most suitable ID strategy based on database characteristics:

| Database | Recommended | Alternative | Description |
|----------|-------------|-------------|-------------|
| **SQLite** | AutoIncrement | ObjectId | AutoIncrement has native support, best performance |
| **PostgreSQL** | UUID | AutoIncrement | UUID has native support, type safe |
| **MySQL** | AutoIncrement | ObjectId | AutoIncrement has native support, best performance |
| **MongoDB** | ObjectId | AutoIncrement | ObjectId has native support, MongoDB ecosystem standard |

**Important Reminder**: When PostgreSQL uses UUID strategy, all foreign key fields in related tables must also use UUID type to maintain type consistency.

## 🛠️ Core APIs

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

// Query records
let found_user = ModelManager::<User>::find_by_id(&user_id).await?;
let users = ModelManager::<User>::find(conditions, options).await?;

// Update record
let mut updates = HashMap::new();
updates.insert("username".to_string(), DataValue::String("New Name".to_string()));
let updated = user.update(updates).await?;

// Delete record
let deleted = user.delete().await?;
```

### ODM Operations (Low-level Interface)
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
4. **Cross-database Adaptation**: Unified interface supports multiple database types
5. **Async Message Processing**: Efficient async processing based on Tokio

## 🔄 Workflow

```
Application Layer → Model Operations → ODM Layer → Message Queue → Connection Pool → Database
    ↑                                               ↓
    └───────────────── Result Return ←────────────────┘
```

## 📊 Performance Features

- **Connection Pool Management**: Intelligent connection reuse and management
- **Async Operations**: Non-blocking database operations
- **Batch Processing**: Supports batch operation optimization
- **Cache Integration**: Built-in cache reduces database access
- **Compression Support**: MongoDB supports ZSTD compression

## 🎯 Supported Field Types

- `integer_field` - Integer fields (with range and constraints)
- `string_field` - String fields (with length limits, can use large length as long text)
- `float_field` - Floating-point number fields (with range and precision)
- `boolean_field` - Boolean fields
- `datetime_field` - Date-time fields
- `uuid_field` - UUID fields
- `json_field` - JSON fields (supports any JSON data, including objects and arrays)
- `array_field` - Array fields (supports homogeneous element arrays)
- `list_field` - List fields (array_field alias)
- `dict_field` - ~~Dictionary/Object fields (deprecated, please use json_field instead)~~
- `reference_field` - Reference fields (foreign keys)

## 📝 Index Support

- **Unique Indexes**: `unique()` constraints
- **Composite Indexes**: Multi-field combination indexes
- **Regular Indexes**: Basic query optimization indexes
- **Auto Creation**: Automatically created based on model definitions
- **Cross-database**: Supports all database index types

## 🌟 Version Information

**Current Version**: 0.5.1

**Supported Rust Version**: 1.70+

**Important Update**: v0.3.0 enforces define_model! macro for model definitions, fixing major architecture issues and improving type safety!

## 📄 License

This project is licensed under [LGPL-v3](LICENSE) license.

## 🤝 Contributing

Welcome to submit Issues and Pull Requests to improve this project!

## 🔧 Troubleshooting

### Network Latency Issues in Concurrent Operations

In high-concurrency operations, especially when accessing databases across network environments, you may encounter data synchronization issues:

#### Problem Description
When performing queries immediately after high-concurrency writes, inconsistent query results may occur. This is typically caused by:

1. **Network Latency**: Latency from cloud databases or cross-region access
2. **Database Master-Slave Sync**: Synchronization delays in master-slave replication architectures
3. **Connection Pool Buffering**: Operation queue buffering in connection pools

#### Solutions

**Solution 1: Configure Wait Time Based on Network Environment**

```rust
// Network environment and recommended wait times
let wait_ms = match network_environment {
    NetworkEnv::Local => 0,        // Local database
    NetworkEnv::LAN => 10,         // Local area network
    NetworkEnv::Cloud => 100,      // Cloud database
    NetworkEnv::CrossRegion => 200, // Cross-region
};

// Add wait after write operations
tokio::time::sleep(tokio::time::Duration::from_millis(wait_ms)).await;
```

**Solution 2: Use Retry Mechanism**

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

**Solution 3: Smart Latency Detection**

```rust
// Dynamically detect network latency and adjust wait time
async fn adaptive_network_delay() -> Duration {
    let start = Instant::now();
    let _ = health_check().await;
    let base_latency = start.elapsed();

    // Wait time is 3x base latency, min 10ms, max 200ms
    let wait_time = std::cmp::max(
        Duration::from_millis(10),
        std::cmp::min(base_latency * 3, Duration::from_millis(200))
    );

    wait_time
}
```

#### Best Practices

- **Local Development**: No wait needed or 5-10ms wait
- **LAN Environment**: 10-50ms wait
- **Cloud Database**: 100-200ms wait or use retry mechanism
- **Production Environment**: Strongly recommend retry mechanism instead of fixed wait
- **High Concurrency Scenarios**: Consider batch operations to reduce network round trips

#### Architecture Notes

rat_quickdb uses a single-worker architecture to ensure data consistency:
- **Single Worker**: Avoids data conflicts from concurrent multi-connection writes
- **Persistent Connections**: Worker maintains persistent connections with database, reducing connection overhead
- **Message Queue**: Processes requests through async message queue, ensuring ordering

This design provides good concurrency performance while maintaining data consistency.

## 📞 Contact

For questions or suggestions, please contact:
- Create Issue: [GitHub Issues](https://github.com/your-repo/rat_quickdb/issues)
- Email: oldmos@gmail.com