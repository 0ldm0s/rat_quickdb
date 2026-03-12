# DataValue 类型使用指南

## ⚠️ 重要：数据库对数据类型敏感

**数据库对数据类型非常敏感**，使用错误的 `DataValue` 类型会导致：
- ❌ 查询无法找到数据（类型不匹配）
- ❌ 更新操作失败
- ❌ 数据无法正确存储
- ❌ 索引失效

**核心原则**：**字段类型必须与 DataValue 类型严格匹配**

---

## 📋 DataValue 类型对应表

| 字段定义 | 正确的 DataValue | 错误的 DataValue | 说明 |
|---------|-----------------|-----------------|------|
| `integer_field()` | `DataValue::Int` | `DataValue::String`, `DataValue::Float` | 整数必须用 Int |
| `string_field()` | `DataValue::String` | `DataValue::Int`, `DataValue::Float` | 字符串必须用 String |
| `float_field()` | `DataValue::Float` | `DataValue::String`, `DataValue::Int` | 浮点数必须用 Float |
| `boolean_field()` | `DataValue::Bool` | `DataValue::String`, `DataValue::Int` | 布尔值必须用 Bool |
| `datetime_field()` | `DataValue::DateTimeUTC` | `DataValue::String` | 时间必须用 DateTimeUTC |
| `uuid_field()` | `DataValue::String` | `DataValue::Uuid` ⚠️ | MongoDB/MySQL/SQLite 用 String |
| `array_field()` | `DataValue::Array` | `DataValue::String` | 数组必须用 Array |
| `json_field()` | `DataValue::Object` | `DataValue::String` | JSON 对象必须用 Object |

---

## 🔍 常见错误示例

### 错误 1：整数使用 String

```rust
// ❌ 错误：将整数转换为字符串
let mut updates = HashMap::new();
updates.insert("character_count".to_string(), DataValue::String(new_count.to_string()));
model.update(updates).await?;
```

**问题**：
- 数据库中 `character_count` 是整数类型
- 查询时使用整数条件 `QueryOperator::Eq` 比较 `DataValue::Int(5)`
- 但更新时存的是字符串 `"5"`
- 导致类型不匹配，查询失败

**正确做法**：
```rust
// ✅ 正确：使用 Int 类型
let mut updates = HashMap::new();
updates.insert("character_count".to_string(), DataValue::Int(new_count));
model.update(updates).await?;
```

---

### 错误 2：浮点数使用 String

```rust
// ❌ 错误：将浮点数转换为字符串
let mut updates = HashMap::new();
updates.insert("salary".to_string(), DataValue::String(salary.to_string()));
model.update(updates).await?;
```

**问题**：
- 浮点数字段应该存储为数据库的浮点类型
- 字符串无法参与数值比较和排序
- 数值范围查询会失败

**正确做法**：
```rust
// ✅ 正确：使用 Float 类型
let mut updates = HashMap::new();
updates.insert("salary".to_string(), DataValue::Float(salary));
model.update(updates).await?;
```

---

### 错误 3：时间字段使用 String

```rust
// ❌ 错误：将时间转换为字符串
fn current_time_value() -> DataValue {
    DataValue::String(chrono::Utc::now().to_rfc3339())
}
```

**问题**：
- `datetime_field()` 在数据库中是 datetime 类型
- 字符串格式的时间无法使用时间范围查询
- 时区转换会出问题

**正确做法**：
```rust
// ✅ 正确：使用 DateTimeUTC 类型
fn current_time_value() -> DataValue {
    DataValue::DateTimeUTC(chrono::Utc::now())
}
```

---

### 错误 4：JSON 转 DataValue 时类型错误

```rust
// ❌ 错误：所有数字都转成字符串
fn json_to_data_value(value: &Value) -> DataValue {
    match value {
        Value::Number(n) => DataValue::String(n.to_string()),  // 错误！
        Value::String(s) => DataValue::String(s.clone()),
        // ...
    }
}
```

**问题**：
- 整数和浮点数被错误地转换为字符串
- 导致数值查询和比较失败

**正确做法**：
```rust
// ✅ 正确：区分整数和浮点数
fn json_to_data_value(value: &Value) -> DataValue {
    match value {
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                DataValue::Int(i)  // 整数用 Int
            } else if let Some(f) = n.as_f64() {
                DataValue::Float(f)  // 浮点用 Float
            } else {
                DataValue::Float(n.as_f64().unwrap_or(0.0))
            }
        }
        Value::String(s) => DataValue::String(s.clone()),
        Value::Bool(b) => DataValue::Bool(*b),
        Value::Null => DataValue::Null,
        _ => DataValue::String(value.to_string()),  // 其他类型兜底
    }
}
```

---

## 🎯 查询条件中的类型匹配

查询条件的类型也必须与字段类型匹配：

```rust
// ✅ 正确：整数字段使用 Int
let conditions = vec![QueryCondition {
    field: "age".to_string(),
    operator: QueryOperator::Gte,
    value: DataValue::Int(18),  // age 是 integer_field
}];

// ❌ 错误：整数字段使用 String
let conditions = vec![QueryCondition {
    field: "age".to_string(),
    operator: QueryOperator::Gte,
    value: DataValue::String("18".to_string()),  // 类型不匹配，查询失败
}];
```

---

## 📊 各数据库的类型要求

### PostgreSQL
- **严格类型检查**：PostgreSQL 对类型要求最严格
- UUID 字段查询使用 `DataValue::String`（框架自动转换）
- 其他字段必须严格匹配类型

### MongoDB
- `uuid_field()` 存储为 **String**
- 查询时使用 `DataValue::String`，**不可使用** `DataValue::Uuid`
- 数值字段必须使用 `DataValue::Int`/`DataValue::Float`

### MySQL
- `uuid_field()` 存储为 **String**
- 查询时使用 `DataValue::String`
- 数值字段必须使用 `DataValue::Int`/`DataValue::Float`

### SQLite
- `uuid_field()` 存储为 **String**
- 查询时使用 `DataValue::String`
- 数值字段必须使用 `DataValue::Int`/`DataValue::Float`
- 布尔值存储为 0/1，框架自动处理

---

## ✅ 最佳实践

### 1. 在业务层正确转换类型

```rust
// 从 JSON/HTTP 请求转换为 DataValue 时，保持类型正确
pub fn json_to_data_value(value: &serde_json::Value) -> DataValue {
    match value {
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                DataValue::Int(i)
            } else if let Some(f) = n.as_f64() {
                DataValue::Float(f)
            } else {
                DataValue::Float(n.as_f64().unwrap_or(0.0))
            }
        }
        serde_json::Value::String(s) => DataValue::String(s.clone()),
        serde_json::Value::Bool(b) => DataValue::Bool(*b),
        serde_json::Value::Null => DataValue::Null,
        _ => panic!("不支持的 JSON 类型"),
    }
}
```

### 2. 更新操作时使用正确的 DataValue

```rust
// ✅ 正确示例
let mut updates = HashMap::new();
updates.insert("count".to_string(), DataValue::Int(100));  // 整数
updates.insert("price".to_string(), DataValue::Float(99.99));  // 浮点
updates.insert("name".to_string(), DataValue::String("test".to_string()));  // 字符串
updates.insert("active".to_string(), DataValue::Bool(true));  // 布尔
updates.insert("updated_at".to_string(), DataValue::DateTimeUTC(chrono::Utc::now()));  // 时间
model.update(updates).await?;
```

### 3. 查询时使用正确的 DataValue

```rust
// ✅ 正确示例
let conditions = vec![
    QueryCondition {
        field: "age".to_string(),
        operator: QueryOperator::Gte,
        value: DataValue::Int(18),  // 整数字段用 Int
    },
    QueryCondition {
        field: "username".to_string(),
        operator: QueryOperator::Eq,
        value: DataValue::String("admin".to_string()),  // 字符串字段用 String
    },
    QueryCondition {
        field: "price".to_string(),
        operator: QueryOperator::Lt,
        value: DataValue::Float(100.0),  // 浮点字段用 Float
    },
];
```

---

## 🔧 调试技巧

如果查询总是找不到数据，检查类型是否匹配：

```rust
// 1. 查看数据库中实际存储的类型
let results = ModelManager::<User>::find(vec![], None).await?;
for user in results {
    println!("age: {:?}, type: {:?}", user.age, std::any::type_name::<i32>());
}

// 2. 检查查询条件的类型
let condition = QueryCondition {
    field: "age".to_string(),
    operator: QueryOperator::Eq,
    value: DataValue::Int(18),  // 确保这里是 Int，不是 String
};
println!("Query value: {:?}", condition.value);
```

---

## 📚 总结

**记住这个简单规则**：

| 如果字段是... | 更新时使用... | 查询时使用... |
|-------------|--------------|--------------|
| 整数 | `DataValue::Int` | `DataValue::Int` |
| 浮点 | `DataValue::Float` | `DataValue::Float` |
| 字符串 | `DataValue::String` | `DataValue::String` |
| 布尔 | `DataValue::Bool` | `DataValue::Bool` |
| 时间 | `DataValue::DateTimeUTC` | `DataValue::DateTimeUTC` |
| UUID（所有数据库） | `DataValue::String` | `DataValue::String` |

**不要把所有东西都转成字符串！** 这是导致查询失败的最常见原因。
