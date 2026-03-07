# 字段版本管理

## 概述

字段版本管理是 rat_quickdb v0.5.3+ 新增的功能，用于追踪模型字段变更、支持版本升级/回滚，并自动生成数据库 DDL 语句。

## 功能特点

- **版本追踪**：记录模型的版本历史和变更记录
- **升级检测**：自动检测字段变更（新增、删除、修改）
- **DDL 生成**：根据数据库类型自动生成升级和降级 DDL
- **回滚支持**：支持回滚到上一版本并生成对应的降级 DDL
- **多数据库支持**：支持 SQLite、PostgreSQL、MySQL、MongoDB

## 核心类型

```rust
use rat_quickdb::field_versioning::{
    FieldVersionManager,     // 版本管理器
    ModelVersionMeta,        // 模型版本元数据
    VersionChange,           // 版本变更记录
    VersionChangeType,       // 变更类型（Upgrade/Downgrade）
    VersionUpgradeResult,    // 升级/降级结果
};
```

### ModelVersionMeta

模型版本元数据，包含：
- `model_name`: 模型名称
- `current_version`: 当前版本号
- `last_upgrade_time`: 上次升级时间
- `last_downgrade_time`: 上次降级时间

### VersionChange

版本变更记录，包含：
- `model_name`: 模型名称
- `from_version`: 源版本
- `to_version`: 目标版本
- `change_type`: 变更类型（Upgrade/Downgrade）
- `timestamp`: 变更时间

### VersionUpgradeResult

升级/降级结果，包含：
- `old_version`: 旧版本号
- `new_version`: 新版本号
- `upgrade_ddl`: 升级 DDL 语句
- `downgrade_ddl`: 降级 DDL 语句（可用于回滚）

## 基本使用

### 1. 创建版本管理器

```rust
use rat_quickdb::field_versioning::FieldVersionManager;
use rat_quickdb::types::DatabaseType;
use std::path::PathBuf;

// 指定存储路径
let storage_path = PathBuf::from("./version_storage");

// 创建版本管理器
let manager = FieldVersionManager::new(
    storage_path,
    DatabaseType::SQLite  // 指定数据库类型
)?;

// 或使用默认路径（~/.rat_quickdb/<alias>）
let default_path = FieldVersionManager::default_storage_path("main");
```

### 2. 注册模型

```rust
use rat_quickdb::model::field_types::{FieldDefinition, FieldType, ModelMeta};
use std::collections::HashMap;

// 创建模型定义
let mut fields = HashMap::new();
fields.insert("id".to_string(), FieldDefinition {
    field_type: FieldType::String {
        max_length: Some(36),
        min_length: None,
        regex: None,
    },
    required: true,
    unique: true,
    default: None,
    indexed: false,
    description: Some("用户ID".to_string()),
    validator: None,
    sqlite_compatibility: false,
});

let model = ModelMeta {
    collection_name: "users".to_string(),
    database_alias: Some("main".to_string()),
    fields,
    indexes: vec![],
    description: Some("用户表".to_string()),
    version: Some(1),  // 初始版本
};

// 注册模型
manager.register_model(&model)?;

// 生成初始 DDL 文件
let ddl = manager.read_ddl("users", true)?;
println!("初始 DDL:\n{}", ddl);
```

### 3. 升级模型

```rust
// 创建新版本模型（添加新字段）
let mut new_model = model.clone();
new_model.version = Some(2);

new_model.fields.insert("email".to_string(), FieldDefinition {
    field_type: FieldType::String {
        max_length: Some(100),
        min_length: None,
        regex: None,
    },
    required: true,
    unique: true,
    default: None,
    indexed: false,
    description: Some("邮箱地址".to_string()),
    validator: None,
    sqlite_compatibility: false,
});

// 执行升级
let result = manager.upgrade_model("users", &new_model)?;

println!("升级: v{} -> v{}", result.old_version, result.new_version);
println!("升级 DDL:\n{}", result.upgrade_ddl);
println!("降级 DDL:\n{}", result.downgrade_ddl);
```

### 4. 回滚模型

```rust
// 回滚到上一版本
let result = manager.rollback_model("users")?;

println!("回滚: v{} -> v{}", result.old_version, result.new_version);
println!("降级 DDL:\n{}", result.downgrade_ddl);
```

### 5. 查询版本信息

```rust
// 获取当前版本
let version = manager.get_version("users")?;
println!("当前版本: {:?}", version);

// 获取变更历史
let changes = manager.get_changes("users")?;
for change in changes {
    println!(
        "v{} -> v{} ({:?}) at {}",
        change.from_version,
        change.to_version,
        change.change_type,
        change.timestamp.format("%Y-%m-%d %H:%M:%S")
    );
}
```

### 6. DDL 文件管理

```rust
// 获取 DDL 文件路径
let upgrade_path = manager.get_ddl_path("users", true);   // 升级 DDL
let downgrade_path = manager.get_ddl_path("users", false); // 降级 DDL

// 读取 DDL 内容
let ddl = manager.read_ddl("users", true)?;
```

## API 参考

### FieldVersionManager

| 方法 | 说明 |
|------|------|
| `new(path, db_type)` | 创建版本管理器 |
| `default_storage_path(alias)` | 获取默认存储路径 |
| `register_model(model)` | 注册模型初始版本 |
| `upgrade_model(name, model)` | 升级模型到新版本 |
| `rollback_model(name)` | 回滚到上一版本 |
| `get_version(name)` | 获取当前版本号 |
| `get_version_meta(name)` | 获取版本元数据 |
| `get_changes(name)` | 获取变更历史 |
| `get_ddl_path(name, is_upgrade)` | 获取 DDL 文件路径 |
| `read_ddl(name, is_upgrade)` | 读取 DDL 内容 |

## DDL 生成示例

### SQLite

```sql
-- 升级 DDL
-- 新增字段: email
ALTER TABLE users ADD COLUMN email VARCHAR(100);

-- 降级 DDL
-- 删除字段: email
ALTER TABLE users DROP COLUMN email;
```

### PostgreSQL

```sql
-- 升级 DDL
-- 新增字段: email
ALTER TABLE users ADD COLUMN email VARCHAR(100);

-- 降级 DDL
-- 删除字段: email
ALTER TABLE users DROP COLUMN email;
```

### MySQL

```sql
-- 升级 DDL
-- 新增字段: email
ALTER TABLE users ADD COLUMN email VARCHAR(100);

-- 降级 DDL
-- 删除字段: email
ALTER TABLE users DROP COLUMN email;
```

### MongoDB

```javascript
// MongoDB 使用灵活的 schema，不需要预定义结构
// 以下是建议的索引定义：

db.users.createIndex({ username: 1, unique: true });
```

## 最佳实践

### 1. 版本号管理

- 使用递增的整数版本号（1, 2, 3...）
- 每次模型变更都应该增加版本号
- 版本号应该在模型定义中明确指定

### 2. 存储路径

- 使用独立的目录存储版本数据
- 建议将版本数据纳入版本控制系统
- 生产环境应使用持久化存储路径

### 3. DDL 执行

- 生成的 DDL 应先在测试环境验证
- 升级前备份数据库
- 保存降级 DDL 以备回滚需要

### 4. 变更追踪

- 定期查看变更历史
- 记录每次变更的业务原因
- 重大变更应进行团队评审

## 注意事项

### 1. 存储路径别名隔离（重要）

版本管理器的存储路径使用别名进行隔离。**强烈建议使用模型的 `database_alias` 字段作为存储路径别名，而不是硬编码字符串。**

```rust
// 正确做法：从模型获取别名
let alias = model.database_alias.as_ref().unwrap_or(&"default".to_string());
let storage_path = FieldVersionManager::default_storage_path(alias);

// 错误做法：硬编码别名
let storage_path = FieldVersionManager::default_storage_path("main");  // 危险！
```

**硬编码别名的严重风险**：

| 场景 | 风险描述 | 后果 |
|------|----------|------|
| 多环境部署 | 开发、测试、生产环境使用相同别名 | 版本数据互相覆盖，导致版本追踪混乱 |
| 多数据库连接 | 应用连接多个数据库（用户库、订单库等） | 不同数据库的版本数据混在一起 |
| 多实例部署 | 同一服务器运行多个应用实例 | 存储路径冲突，数据损坏 |
| CI/CD 流水线 | 自动化测试环境与本地环境冲突 | 测试结果不可预测 |

**推荐做法**：

```rust
// 方案1：使用模型的 database_alias
let alias = model.database_alias.clone().unwrap_or_else(|| "default".to_string());
let manager = FieldVersionManager::new(
    FieldVersionManager::default_storage_path(&alias),
    db_type
)?;

// 方案2：使用配置文件管理别名
let alias = config.get_string("database.alias")?;
let manager = FieldVersionManager::new(
    FieldVersionManager::default_storage_path(&alias),
    db_type
)?;

// 方案3：使用环境变量区分环境
let env = std::env::var("APP_ENV").unwrap_or_else(|_| "development".to_string());
let alias = format!("{}_{}", env, database_name);
let manager = FieldVersionManager::new(
    FieldVersionManager::default_storage_path(&alias),
    db_type
)?;
```

### 2. 数据库兼容性

- SQLite 不支持删除列（DROP COLUMN），降级 DDL 可能无法执行
- MySQL 对索引长度有限制（3072字节）
- PostgreSQL UUID 类型要求所有关联字段使用相同类型

### 3. 数据迁移

- 版本管理器只生成 DDL，不自动执行
- 新增非空字段需要设置默认值或手动填充数据
- 删除字段前应确保数据已备份或不再需要

### 4. 并发安全

- 版本管理器内部使用读写锁保证线程安全
- 多进程环境需要确保存储路径不冲突
- 建议在应用启动时进行模型注册

### 5. 存储依赖

- 版本管理器使用 sled 嵌入式数据库存储元数据
- 存储路径需要读写权限
- 定期清理不需要的版本历史

## 完整示例

参见 `examples/field_versioning_sqlite.rs`，演示了完整的版本管理流程：

```bash
cargo run --example field_versioning_sqlite --features sqlite-support
```
