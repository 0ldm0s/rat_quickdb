# PostgreSQL 限制说明

## 必须遵守的限制

### UUID类型处理要求

使用UUID策略时必须遵循反直觉的设计：

**结构体中必须使用String类型**：
```rust
struct User {
    id: String,  // 必须用String
    // ...
}
```

**字段定义中必须使用uuid_field**：
```rust
fields = {
    id: uuid_field().required().unique(),
    // ...
}
```