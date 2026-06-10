# rat_quickdb ODM 层文档

## 概述

ODM 层（`src/odm/`）提供高级的数据库操作接口，封装底层适配器细节，支持全局和实例两种使用方式。

## 核心组件

### AsyncOdmManager

位置：`src/odm/manager_core.rs`

异步 ODM 管理器，管理数据库连接和缓存。

```rust
pub struct AsyncOdmManager {
    // 数据库配置
    // 连接池
    // 缓存管理器
}
```

**关键方法**：
- `create()` - 创建记录
- `find_by_id()` - 按 ID 查询
- `find()` - 条件查询
- `update()` - 更新记录
- `delete()` - 删除记录
- `count()` - 统计数量
- `create_stored_procedure()` - 创建存储过程

### OdmOperations trait

位置：`src/odm/traits.rs`

定义 ODM 操作接口。

```rust
#[async_trait]
pub trait OdmOperations {
    async fn create(&self, request: OdmRequest) -> QuickDbResult<DataMap>;
    async fn find_by_id(&self, request: OdmRequest) -> QuickDbResult<Option<DataMap>>;
    async fn find(&self, request: OdmRequest) -> QuickDbResult<Vec<DataMap>>;
    async fn update(&self, request: OdmRequest) -> QuickDbResult<()>;
    async fn delete(&self, request: OdmRequest) -> QuickDbResult<()>;
    async fn count(&self, request: OdmRequest) -> QuickDbResult<u64>;
    // ... 其他方法
}
```

### OdmRequest 枚举

位置：`src/odm/types.rs`

```rust
pub enum OdmRequest {
    Create {
        table: String,
        data: DataMap,
    },
    FindById {
        table: String,
        id: DataValue,
    },
    Find {
        table: String,
        conditions: Vec<QueryConditionWithConfig>,
        options: Option<QueryOptions>,
    },
    Update {
        table: String,
        id: DataValue,
        operations: Vec<UpdateOperation>,
    },
    Delete {
        table: String,
        id: DataValue,
    },
    Count {
        table: String,
        conditions: Vec<QueryConditionWithConfig>,
    },
    // ... 其他变体
}
```

## Handler 模式

位置：`src/odm/handlers/`

每个操作有独立的 Handler 实现：

### CreateHandler

位置：`src/odm/handlers/create_handler.rs`

处理创建操作：
1. 数据验证
2. 生成 ID（如果需要）
3. 转换为 DataMap
4. 调用适配器创建
5. 更新缓存

### ReadHandler

位置：`src/odm/handlers/read_handler.rs`

处理读取操作：
1. 检查缓存
2. 构建查询条件
3. 调用适配器查询
4. 转换结果
5. 更新缓存

### UpdateHandler

位置：`src/odm/handlers/update_handler.rs`

处理更新操作：
1. 数据验证
2. 构建更新操作
3. 调用适配器更新
4. 失效相关缓存

### DeleteHandler

位置：`src/odm/handlers/delete_handler.rs`

处理删除操作：
1. 调用适配器删除
2. 失效相关缓存

### UpsertHandler

位置：`src/odm/handlers/upsert_handler.rs`

处理 Upsert 操作（插入或更新）：
1. 尝试查找记录
2. 存在则更新，不存在则创建

### StoredProcedureHandler

位置：`src/odm/handlers/stored_procedure_handler.rs`

处理存储过程操作：
1. 构建 JOIN 关系
2. 生成 SQL/聚合管道
3. 执行查询
4. 转换结果

## 全局函数

位置：`src/odm/global.rs`

提供便捷的全局函数：

```rust
pub fn get_odm_manager() -> &'static AsyncOdmManager
pub async fn find<T: Model>(conditions: Vec<QueryCondition>, options: Option<QueryOptions>) -> QuickDbResult<Vec<T>>
pub async fn create<T: Model>(model: &T) -> QuickDbResult<()>
pub async fn update<T: Model>(id: &DataValue, operations: Vec<UpdateOperation>) -> QuickDbResult<()>
pub async fn delete<T: Model>(id: &DataValue) -> QuickDbResult<()>
pub async fn count<T: Model>(conditions: Vec<QueryCondition>) -> QuickDbResult<u64>
```

## 操作实现

位置：`src/odm/operations/odm_operations_impl.rs`

`OdmOperations` trait 的完整实现（约 14,000 行），包含：
- 各操作的具体逻辑
- 缓存策略处理
- 错误处理
- 日志记录

## 使用方式

### 实例方式

```rust
let manager = AsyncOdmManager::new(config).await?;
let result = manager.find(request).await?;
```

### 全局方式

```rust
use rat_quickdb::odm::*;

// 初始化后自动使用全局管理器
let results = find::<User>(conditions, options).await?;
```

## 缓存集成

ODM 层自动处理缓存：
- 读取时检查缓存
- 写入时更新/失效缓存
- 支持缓存绕过

## 错误处理

所有操作返回 `QuickDbResult<T>`，错误会被统一处理和传播。

## 日志记录

使用 `rat_logger` 记录操作日志：
- `debug!` - 调试信息
- `info!` - 操作信息
- `warn!` - 警告信息
- `error!` - 错误信息
