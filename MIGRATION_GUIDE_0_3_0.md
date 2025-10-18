# 迁移指南：从 v0.2.x 到 v0.3.0

## 🚨 重大变更通知

**版本 0.3.0 是一个重大变更版本**，包含破坏性更改。这些更改是为了修复架构中的重大问题并提升库的可靠性。

## 主要变更内容

### 1. 强制使用 `define_model!` 宏定义模型

**变更原因：**
- 消除动态表结构推断导致的"保姆设置"问题
- 避免不可预期的推断结果
- 提供更明确的类型安全和字段定义
- 修复重大架构Bug

**变更详情：**
- 之前：允许使用普通结构体进行数据库操作，库会动态推断表结构
- 现在：**必须**使用 `define_model!` 宏预定义模型，明确指定字段类型

### 2. 自动表创建行为变更

**变更原因：**
- 确保数据一致性和类型安全
- 防止意外的字段类型推断错误

**变更详情：**
- 之前：如果表不存在，会根据数据自动推断并创建表结构
- 现在：如果表不存在且没有预定义模型，**会抛出错误**
- 必须先使用 `define_model!` 宏定义模型，然后才能创建表

## 迁移步骤

### 步骤 1：识别需要迁移的代码

找到所有直接使用结构体进行数据库操作的地方：

```rust
// ❌ 旧代码 - 不再支持
struct User {
    id: String,
    name: String,
    email: String,
}

// 直接使用普通结构体
let user = User { /* ... */ };
odm.create("users", user.to_data_map(), None).await?;
```

### 步骤 2：使用 `define_model!` 宏重定义模型

```rust
// ✅ 新代码 - 必须使用 define_model! 宏
use rat_quickdb::{define_model, string_field, integer_field, datetime_field};

define_model! {
    struct User {
        id: String,
        name: String,
        email: String,
        created_at: chrono::DateTime<chrono::Utc>,
    }
    collection = "users",
    fields = {
        id: string_field(None, None, None).required().unique(),
        name: string_field(None, None, None).required(),
        email: string_field(None, None, None).required(),
        created_at: datetime_field().required(),
    }
    indexes = [
        { fields: ["email"], unique: true, name: "idx_email" },
    ],
}
```

### 步骤 3：更新数据库操作代码

```rust
// ✅ 新代码 - 使用模型方法
let user = User {
    id: String::new(), // 框架会自动生成ID
    name: "张三".to_string(),
    email: "zhangsan@example.com".to_string(),
    created_at: chrono::Utc::now(),
};

// 使用模型的 save() 方法
let created_id = user.save().await?;

// 或者使用 ModelManager
let found_user = ModelManager::<User>::find_by_id(&created_id).await?;
```

## 字段类型映射

### 常用字段类型定义

| Rust 类型 | define_model! 宏定义 | 说明 |
|----------|---------------------|------|
| `String` | `string_field(max_length, min_length, regex)` | 字符串字段 |
| `i32/i64` | `integer_field(min_value, max_value)` | 整数字段 |
| `f64` | `float_field(min_value, max_value)` | 浮点数字段 |
| `bool` | `boolean_field()` | 布尔字段 |
| `chrono::DateTime<Utc>` | `datetime_field()` | 日期时间字段 |
| `Vec<T>` | `array_field(element_types, max_length, min_length)` | 数组字段 |
| `HashMap<String, Value>` | `object_field(max_properties, required_properties)` | 对象字段 |

### 字段约束链式调用

```rust
string_field(None, None, None)           // 基本字符串字段
    .required()                          // 必填
    .unique()                            // 唯一
    .default("默认值".to_string())        // 默认值
    .max_length(100)                     // 最大长度
    .regex(r"^[a-zA-Z0-9]+$".to_string()) // 正则表达式验证
```

## 完整迁移示例

### 旧代码 (v0.2.x)

```rust
use rat_quickdb::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
struct User {
    id: String,
    name: String,
    email: String,
    age: i32,
}

impl User {
    fn to_data_map(&self) -> HashMap<String, DataValue> {
        let mut map = HashMap::new();
        map.insert("id".to_string(), DataValue::String(self.id.clone()));
        map.insert("name".to_string(), DataValue::String(self.name.clone()));
        map.insert("email".to_string(), DataValue::String(self.email.clone()));
        map.insert("age".to_string(), DataValue::Int(self.age as i64));
        map
    }
}

#[tokio::main]
async fn main() -> QuickDbResult<()> {
    // 配置数据库
    let config = DatabaseConfig {
        db_type: DatabaseType::SQLite,
        connection: ConnectionConfig::SQLite {
            path: "./test.db".to_string(),
            create_if_missing: true,
        },
        pool: PoolConfig::default(),
        alias: "main".to_string(),
        cache: None,
        id_strategy: IdStrategy::Uuid,
    };

    add_database(config).await?;

    // 创建用户 - 库会自动推断表结构
    let user = User {
        id: uuid::Uuid::new_v4().to_string(),
        name: "张三".to_string(),
        email: "zhangsan@example.com".to_string(),
        age: 25,
    };

    let created_id = odm.create("users", user.to_data_map(), None).await?;
    println!("用户创建成功，ID: {}", created_id);

    Ok(())
}
```

### 新代码 (v0.3.0)

```rust
use rat_quickdb::*;
use rat_quickdb::{define_model, string_field, integer_field};
use chrono::Utc;

define_model! {
    struct User {
        id: String,
        name: String,
        email: String,
        age: i32,
        created_at: chrono::DateTime<chrono::Utc>,
    }
    collection = "users",
    fields = {
        id: string_field(None, None, None).required().unique(),
        name: string_field(None, None, None).required(),
        email: string_field(None, None, None).required().unique(),
        age: integer_field(None, None).required(),
        created_at: datetime_field().required(),
    }
    indexes = [
        { fields: ["email"], unique: true, name: "idx_email" },
        { fields: ["name"], unique: false, name: "idx_name" },
    ],
}

impl User {
    fn new(name: &str, email: &str, age: i32) -> Self {
        Self {
            id: String::new(), // 框架会自动生成ID
            name: name.to_string(),
            email: email.to_string(),
            age,
            created_at: Utc::now(),
        }
    }
}

#[tokio::main]
async fn main() -> QuickDbResult<()> {
    // 配置数据库
    let config = DatabaseConfig {
        db_type: DatabaseType::SQLite,
        connection: ConnectionConfig::SQLite {
            path: "./test.db".to_string(),
            create_if_missing: true,
        },
        pool: PoolConfig::default(),
        alias: "main".to_string(),
        cache: None,
        id_strategy: IdStrategy::Uuid,
    };

    add_database(config).await?;

    // 创建用户 - 使用预定义模型
    let user = User::new("张三", "zhangsan@example.com", 25);

    let created_id = user.save().await?;
    println!("用户创建成功，ID: {}", created_id);

    // 查询用户
    if let Some(found_user) = ModelManager::<User>::find_by_id(&created_id).await? {
        println!("找到用户: {}", found_user.name);
    }

    Ok(())
}
```

## 常见问题解答

### Q: 为什么要做这个变更？
A: 动态推断表结构会导致不可预期的结果和"保姆设置"问题，这是为了修复架构重大bug必须做出的改进。

### Q: 我的现有代码必须立即迁移吗？
A: 是的，v0.3.0 不再支持旧的动态推断方式。建议立即迁移以确保功能正常。

### Q: 迁移复杂吗？
A: 迁移相对简单，主要是将结构体定义改为 `define_model!` 宏定义。大多数情况下，迁移后的代码会更加简洁和类型安全。

### Q: 如果我只是想测试功能，不想定义完整的模型怎么办？
A: 即使是测试，也必须使用 `define_model!` 宏。但可以定义最简单的模型：

```rust
define_model! {
    struct TestModel {
        id: String,
        data: String,
    }
    collection = "test_table",
    fields = {
        id: string_field(None, None, None).required().unique(),
        data: string_field(None, None, None).required(),
    }
}
```

### Q: 过期的示例文件怎么办？
A: 所有使用旧方式的示例已重命名为 `.deprecated.rs`。建议查看使用 `define_model!` 宏的新示例。

## 需要帮助？

如果遇到迁移问题，请：

1. 查看新示例文件（非 `.deprecated.rs` 结尾的文件）
2. 参考 `model_definition.rs` 示例了解完整的模型定义
3. 查看 `id_strategy_test.rs` 了解基础CRUD操作
4. 提交 Issue 寻求帮助

---

**重要提醒：** v0.3.0 是一个重大改进，虽然需要迁移工作，但会带来更好的类型安全、性能和可维护性。