# MySQL 限制说明

## 必须遵守的限制

### 索引长度限制

MySQL索引长度限制为3072字节，复合索引包含长字符串字段时会超限。

**错误信息**：
```
Specified key was too long; max key length is 3072 bytes
```

**必须设置字符串字段长度限制**：
```rust
department: string_field(Some(100), Some(1), None).required(),
position: string_field(Some(100), Some(1), None).required(),
```

**避免在复合索引中使用过多长字符串字段**：
```rust
// ❌ 错误
indexes = [
    { fields: ["department", "position", "name"], name: "idx_dept_pos_name" },
]
```
