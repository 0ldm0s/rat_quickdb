# RAT QuickDB Python 绑定架构设计

## 概述

RAT QuickDB Python 绑定采用完全解耦的队列架构，确保 Python 端与 Rust 底层实现彻底分离，通过 JSON 字符串进行所有数据交互。

## 架构图

```
Python 调用层 → Python 请求队列 → python_to_rust 通道 → Rust 胶水层(ODM) → rust_to_python 通道 → Python 响应处理 → Python 调用层
```

## 核心组件

### 1. Python 端组件

#### PyDbQueueBridge
- **职责**: Python 接口类，提供所有数据库操作的 Python API
- **功能**:
  - 构造 JSON 请求消息
  - 发送请求到 Python 请求队列
  - 管理响应通道映射
  - 等待并处理响应

#### Python 请求队列系统 (未来版本)
- **职责**: 本地排队处理 Python 调用层的请求
- **功能**:
  - 接收多个并发的 Python 调用
  - 按序处理请求
  - 管理请求优先级
  - 处理请求超时和重试

### 2. 通信通道

#### python_to_rust 通道
- **类型**: `mpsc::UnboundedSender<PyDbRequest>`
- **流向**: Python → Rust
- **内容**: 所有请求消息，包括：
  - 数据库操作 (CRUD)
  - 数据库配置 (添加/删除数据库)
  - 表管理 (创建/删除表)
  - 模型注册

#### rust_to_python 通道
- **类型**: `oneshot::Sender<PyDbResponse>` (每个请求独立)
- **流向**: Rust → Python
- **内容**: 处理结果，包括：
  - 成功的数据结果
  - 错误信息
  - 操作状态

### 3. Rust 胶水层

#### PyDbQueueBridgeAsync (守护线程)
- **职责**: 后台守护线程，持有 ODM 管理器
- **功能**:
  - 启动时创建 ODM 管理器实例
  - 从 `python_to_rust` 通道接收请求
  - 解析 JSON 请求消息
  - 使用 ODM 管理器处理所有操作
  - 通过 `rust_to_python` 通道返回结果

#### ODM 管理器
- **职责**: 所有数据库操作的统一接口
- **功能**:
  - 管理数据库连接池
  - 处理 CRUD 操作
  - 处理数据库配置
  - 处理表管理
  - 缓存管理

## 数据交互格式

### 请求消息格式 (PyDbRequest)
```rust
struct PyDbRequest {
    request_id: String,           // 唯一请求ID
    operation: String,            // 操作类型
    collection: String,           // 集合/表名
    data: Option<HashMap<String, DataValue>>,  // 数据内容
    conditions: Option<Vec<QueryCondition>>,    // 查询条件
    database_config: Option<DatabaseConfig>,     // 数据库配置
    fields: Option<HashMap<String, FieldType>>,  // 字段定义
    alias: Option<String>,         // 数据库别名
    response_sender: oneshot::Sender<PyDbResponse>, // 响应通道
}
```

### 响应消息格式 (PyDbResponse)
```rust
struct PyDbResponse {
    request_id: String,    // 对应的请求ID
    success: bool,         // 操作是否成功
    data: String,          // JSON 格式的结果数据
    error: Option<String>, // 错误信息
}
```

## 操作类型

### 数据库操作
- `"create"`: 创建记录
- `"find"`: 查询记录
- `"find_by_id"`: 根据ID查询
- `"find_with_groups"`: 复杂条件查询
- `"update"`: 更新记录
- `"update_by_id"`: 根据ID更新
- `"delete"`: 删除记录
- `"delete_by_id"`: 根据ID删除
- `"count"`: 统计记录数量

### 数据库配置
- `"add_database"`: 添加数据库
- `"remove_database"`: 移除数据库
- `"set_default_alias"`: 设置默认别名

### 表管理
- `"create_table"`: 创建表
- `"drop_table"`: 删除表

### 模型管理
- `"register_model"`: 注册ODM模型

## JSON 数据格式示例

### 添加数据库请求
```json
{
  "operation": "add_database",
  "database_config": {
    "db_type": "SQLite",
    "connection": {
      "path": "/path/to/database.db",
      "create_if_missing": true
    },
    "pool": {
      "max_connections": 10,
      "min_connections": 1
    },
    "alias": "main",
    "id_strategy": "AutoIncrement"
  }
}
```

### 创建记录请求
```json
{
  "operation": "create",
  "collection": "users",
  "data": {
    "name": "张三",
    "email": "zhangsan@example.com",
    "age": 30
  },
  "alias": "main"
}
```

### 查询请求
```json
{
  "operation": "find",
  "collection": "users",
  "conditions": [
    {
      "field": "age",
      "operator": "gt",
      "value": 25
    }
  ],
  "alias": "main"
}
```

### 创建表请求
```json
{
  "operation": "create_table",
  "collection": "users",
  "fields": {
    "id": "integer",
    "name": "string",
    "email": "string",
    "age": "integer",
    "created_at": "datetime"
  },
  "alias": "main"
}
```

## 设计原则

### 1. 完全解耦
- Python 端不直接持有任何 Rust 对象
- Python 端不直接调用任何 Rust 函数
- 所有交互通过队列和 JSON 字符串进行

### 2. 统一接口
- 所有操作都通过相同的请求/响应格式
- 配置和数据操作使用相同的通信机制
- 错误处理统一

### 3. 可扩展性
- 新增操作类型只需扩展操作枚举
- 新增数据库类型只需扩展配置解析
- 支持未来版本的功能扩展

### 4. 性能优化
- 异步处理，不阻塞 Python 主线程
- 连接池复用，避免频繁连接
- 支持 JSON 字符串批量操作

## 实现状态

### 当前版本 (v0.2.x)
- ✅ 基础队列通信框架
- ✅ CRUD 操作支持
- ✅ 基本数据库配置
- ✅ 表管理功能
- ❌ 本地请求队列系统 (待实现)
- ❌ 批量操作优化 (待实现)

### 未来版本规划

#### v0.3.x
- [ ] 实现 Python 本地请求队列系统
- [ ] 添加请求优先级管理
- [ ] 实现请求超时和重试机制
- [ ] 支持批量操作优化

#### v0.4.x
- [ ] 添加连接池状态监控
- [ ] 实现分布式锁支持
- [ ] 添加事务支持
- [ ] 支持读写分离

## 错误处理

### Python 端错误
- JSON 解析错误
- 请求格式错误
- 响应超时错误
- 队列通信错误

### Rust 胶水层错误
- 数据库连接错误
- ODM 操作错误
- 配置验证错误
- 资源不足错误

### 错误传播
所有错误都通过响应消息的 `error` 字段传递，包含详细的错误信息和错误代码。

## 性能考虑

### 1. 连接复用
- 使用连接池避免频繁创建连接
- 支持长连接保持
- 自动清理空闲连接

### 2. 异步处理
- 所有操作都是异步的
- 不阻塞 Python 主线程
- 支持并发操作

### 3. 缓存机制
- 查询结果缓存
- 连接状态缓存
- 配置信息缓存

### 4. 内存管理
- 大结果集流式处理
- 及时释放不需要的资源
- 避免内存泄漏

## 安全考虑

### 1. 输入验证
- 严格的 JSON 格式验证
- SQL 注入防护
- 参数类型检查

### 2. 权限控制
- 数据库访问权限验证
- 操作权限检查
- 敏感信息保护

### 3. 连接安全
- 支持 TLS/SSL 加密
- 认证信息保护
- 安全的连接配置

## 测试策略

### 1. 单元测试
- Python 端接口测试
- Rust 胶水层功能测试
- 错误处理测试

### 2. 集成测试
- 端到端操作测试
- 多数据库兼容性测试
- 性能压力测试

### 3. 兼容性测试
- 不同 Python 版本测试
- 不同操作系统测试
- 不同数据库版本测试

## 维护和监控

### 1. 日志记录
- 操作日志记录
- 错误日志记录
- 性能指标记录

### 2. 监控指标
- 连接池状态监控
- 操作性能监控
- 错误率监控

### 3. 调试支持
- 详细的错误信息
- 操作追踪支持
- 调试模式开关