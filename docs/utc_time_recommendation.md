# UTC时间使用建议

## 🌍 时区处理最佳实践

为了避免时区转换问题和确保跨数据库的一致性，强烈建议在RatQuickDB的Python集成中使用UTC时间。

## 📝 Python集成推荐做法

### ✅ 推荐方式：使用UTC时间
```python
from datetime import datetime, timezone

# 创建UTC时间
now = datetime.now(timezone.utc)

user_data = {
    "created_at": now.isoformat(),  # "2025-10-20T13:54:23.695487+00:00"
    "updated_at": now.isoformat(),
    "published_at": now.isoformat(),
    "last_login": now.isoformat() if last_login else None,
}
```

### ❌ 避免方式：使用本地时间
```python
# 不要这样做，可能导致时区问题
from datetime import datetime

now = datetime.now()  # 本地时间，有歧义

user_data = {
    "created_at": now.strftime("%Y-%m-%d %H:%M:%S"),  # 缺少时区信息
}
```

## 🔧 主库示例更新建议

### ✅ Rust代码推荐
```rust
use chrono::Utc;

let now = Utc::now();  // 推荐：使用UTC

// 在模型中使用
user.created_at = Utc::now();
user.updated_at = Some(Utc::now());
```

### ❌ 避免的写法
```rust
use chrono::Local;

let now = Local::now();  // 避免：本地时间，除非特殊需求
```

## 📋 优势

1. **避免时区转换问题** - UTC是全球标准，无歧义
2. **数据库一致性** - 所有数据库都正确支持UTC
3. **跨平台兼容** - 不受服务器时区设置影响
4. **简化调试** - 时间值明确，便于调试和测试
5. **国际化友好** - 适合全球应用

## ⚠️ 特殊情况

只有在以下特殊情况下才考虑使用本地时间：
- 应用明确需要本地时间显示
- 法律法规要求使用本地时间
- 用户界面需要显示本地时间（在显示层转换）

## 🔄 显示层转换

如果需要向用户显示本地时间，应该在显示层进行转换：

```python
# 存储时使用UTC
created_at = datetime.now(timezone.utc).isoformat()

# 显示时转换为本地时间
from datetime import datetime, timezone
import pytz

def format_local_time(utc_str):
    utc_dt = datetime.fromisoformat(utc_str)
    local_tz = pytz.timezone('Asia/Shanghai')  # 用户时区
    local_dt = utc_dt.astimezone(local_tz)
    return local_dt.strftime("%Y-%m-%d %H:%M:%S")
```

## 📚 相关文档

- [Python datetime文档](https://docs.python.org/3/library/datetime.html)
- [chrono时区处理](https://docs.rs/chrono/0.4/chrono/offset/trait.TimeZone.html)
- [PostgreSQL时区最佳实践](https://www.postgresql.org/docs/current/datatype-datetime.html)