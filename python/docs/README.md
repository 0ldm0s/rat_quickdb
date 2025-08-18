# RatQuickDB Python 文档中心

欢迎使用 RatQuickDB Python 绑定！这里是完整的文档索引，帮助您快速找到所需的信息。

## 📚 文档导航

### 🚀 新手入门

| 文档 | 描述 | 适合人群 |
|------|------|----------|
| [快速入门指南](getting_started.md) | 5分钟上手 RatQuickDB，包含完整示例 | 初学者 |
| [API 参考文档](api_reference.md) | 完整的 API 接口说明和使用方法 | 所有用户 |
| [查询操作符指南](query_operators_guide.md) | 详细的查询操作符使用说明 | 需要复杂查询的用户 |

### 📖 详细文档

#### 1. [快速入门指南](getting_started.md)

**适合**: 第一次使用 RatQuickDB 的用户

**内容包括**:
- ✅ 安装和环境配置
- ✅ 5分钟快速体验
- ✅ 基础 CRUD 操作
- ✅ 缓存配置入门
- ✅ 完整可运行示例
- ✅ 常见问题解答

**预计阅读时间**: 15分钟

#### 2. [API 参考文档](api_reference.md)

**适合**: 需要详细了解 API 接口的开发者

**内容包括**:
- 🔧 完整的 API 接口列表
- 🔧 数据库管理 (SQLite/MySQL/PostgreSQL/MongoDB)
- 🔧 CRUD 操作详解
- 🔧 批量操作和聚合查询
- 🔧 缓存配置 (L1/L2/TTL/压缩)
- 🔧 错误处理和最佳实践
- 🔧 性能优化建议

**预计阅读时间**: 30分钟

#### 3. [查询操作符指南](query_operators_guide.md)

**适合**: 需要执行复杂查询的用户

**内容包括**:
- 🔍 15种查询操作符详解
- 🔍 三种查询条件格式
- 🔍 数据类型支持说明
- 🔍 性能优化技巧
- 🔍 实际使用示例
- 🔍 错误处理方法

**预计阅读时间**: 20分钟

## 🎯 按使用场景选择文档

### 场景 1: 我是新手，想快速上手

**推荐路径**: 
1. [快速入门指南](getting_started.md) - 了解基础概念和操作
2. [查询操作符指南](query_operators_guide.md) - 学习查询语法
3. [API 参考文档](api_reference.md) - 深入了解高级功能

### 场景 2: 我需要执行复杂查询

**推荐路径**:
1. [查询操作符指南](query_operators_guide.md) - 重点学习
2. [API 参考文档](api_reference.md) - 查看聚合操作
3. [快速入门指南](getting_started.md) - 参考完整示例

### 场景 3: 我要配置生产环境

**推荐路径**:
1. [API 参考文档](api_reference.md) - 重点关注缓存配置和最佳实践
2. [快速入门指南](getting_started.md) - 参考错误处理
3. [查询操作符指南](query_operators_guide.md) - 了解性能优化

### 场景 4: 我要迁移现有项目

**推荐路径**:
1. [API 参考文档](api_reference.md) - 了解完整 API
2. [快速入门指南](getting_started.md) - 参考数据库配置
3. [查询操作符指南](query_operators_guide.md) - 转换查询语法

## 🔧 支持的功能特性

### 数据库支持

| 数据库 | 支持状态 | 推荐场景 |
|--------|----------|----------|
| SQLite | ✅ 完全支持 | 开发测试、小型应用 |
| MySQL | ✅ 完全支持 | Web 应用、中型项目 |
| PostgreSQL | ✅ 完全支持 | 复杂查询、大型应用 |
| MongoDB | ✅ 完全支持 | 文档存储、快速迭代 |

### 查询操作符

| 类别 | 操作符 | 数量 |
|------|--------|------|
| 比较操作 | `eq`, `ne`, `gt`, `gte`, `lt`, `lte` | 6个 |
| 字符串操作 | `contains`, `starts_with`, `ends_with`, `regex` | 4个 |
| 列表操作 | `in_list`, `not_in` | 2个 |
| 空值操作 | `exists`, `is_null`, `is_not_null` | 3个 |
| **总计** | | **15个** |

### 缓存系统

| 缓存层级 | 类型 | 特点 |
|----------|------|------|
| L1 缓存 | 内存缓存 | 速度最快，容量有限 |
| L2 缓存 | Redis 缓存 | 容量大，支持分布式 |
| TTL 配置 | 过期时间 | 灵活的缓存策略 |
| 压缩配置 | 数据压缩 | 节省存储空间 |

## 📋 快速参考

### 常用代码片段

#### 初始化

```python
from rat_quickdb_py import create_db_queue_bridge, PyCacheConfig, PyL1CacheConfig

cache_config = PyCacheConfig.builder() \
    .l1_cache(PyL1CacheConfig.builder() \
        .capacity(1000) \
        .memory_limit_mb(50) \
        .build()) \
    .build()

bridge = create_db_queue_bridge(cache_config)
```

#### 数据库连接

```python
# SQLite
db_config = {"type": "sqlite", "connection_string": "./app.db"}

# MySQL
db_config = {"type": "mysql", "connection_string": "mysql://user:pass@host:3306/db"}

# PostgreSQL
db_config = {"type": "postgresql", "connection_string": "postgresql://user:pass@host:5432/db"}

# MongoDB
db_config = {"type": "mongodb", "connection_string": "mongodb://host:27017/db"}

bridge.add_database("my_db", json.dumps(db_config))
```

#### 基础操作

```python
# 创建
result = bridge.create("users", json.dumps(data), "my_db")

# 查询
result = bridge.find("users", json.dumps(query), "my_db")

# 更新
result = bridge.update("users", json.dumps(conditions), json.dumps(update_data), "my_db")

# 删除
result = bridge.delete("users", json.dumps(conditions), "my_db")

# 统计
result = bridge.count("users", json.dumps(conditions), "my_db")
```

#### 查询操作符

```python
# 简单查询
query = {"name": "张三"}

# 条件查询
query = [
    {"field": "age", "operator": "Gt", "value": 25},
    {"field": "department", "operator": "Eq", "value": "技术部"}
]

# 复杂查询
query = [
    {"field": "salary", "operator": "Gte", "value": 8000},
    {"field": "name", "operator": "Contains", "value": "李"},
    {"field": "email", "operator": "EndsWith", "value": "@company.com"}
]
```

## 🆘 获取帮助

### 文档内查找

1. **快速查找**: 使用浏览器的查找功能 (Ctrl+F / Cmd+F)
2. **按场景查找**: 参考上面的「按使用场景选择文档」
3. **按功能查找**: 查看「支持的功能特性」表格

### 常见问题

| 问题类型 | 查看文档 | 章节 |
|----------|----------|------|
| 安装问题 | [快速入门指南](getting_started.md) | 安装部分 |
| 连接数据库 | [快速入门指南](getting_started.md) | 连接数据库 |
| 查询语法 | [查询操作符指南](query_operators_guide.md) | 全部内容 |
| 性能优化 | [API 参考文档](api_reference.md) | 最佳实践 |
| 错误处理 | [快速入门指南](getting_started.md) | 错误处理最佳实践 |
| 缓存配置 | [API 参考文档](api_reference.md) | 缓存配置 |

### 示例代码

所有文档都包含可运行的示例代码，您可以：

1. 直接复制粘贴运行
2. 根据需要修改参数
3. 组合不同的代码片段

### 社区支持

- 📧 **问题反馈**: [GitHub Issues]
- 💬 **社区讨论**: [GitHub Discussions]
- 📖 **源码查看**: [GitHub Repository]
- 🐛 **Bug 报告**: [GitHub Issues]

## 📈 文档更新日志

| 版本 | 日期 | 更新内容 |
|------|------|----------|
| v1.0.0 | 2024-01-XX | 初始版本，包含完整的文档体系 |
| | | - 快速入门指南 |
| | | - API 参考文档 |
| | | - 查询操作符指南 |
| | | - 文档索引 |

## 🎉 开始使用

准备好开始使用 RatQuickDB 了吗？

👉 **新手用户**: 从 [快速入门指南](getting_started.md) 开始  
👉 **有经验用户**: 直接查看 [API 参考文档](api_reference.md)  
👉 **需要复杂查询**: 重点学习 [查询操作符指南](query_operators_guide.md)  

---

**RatQuickDB** - 高性能、多数据库、统一 API 的数据库抽象层

*让数据库操作变得简单而强大* 🚀