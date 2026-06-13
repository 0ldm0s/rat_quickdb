# rat_quickdb 缓存系统文档

## 概述

缓存系统（`src/cache/`）基于 `rat_memcache` 实现，提供 L1 内存缓存和 L2 磁盘缓存的双层缓存架构。

## 核心组件

### CacheManager

位置：`src/cache/cache_manager.rs`

缓存管理器，负责缓存的读写和管理。

```rust
pub struct CacheManager {
    // 基于 rat_memcache 的实现
}
```

**关键方法**：
- `get(key)` - 获取缓存值
- `set(key, value, ttl)` - 设置缓存值
- `delete(key)` - 删除缓存
- `clear()` - 清空缓存
- `get_stats()` - 获取缓存统计

### CacheStats

位置：`src/cache/stats.rs`

缓存统计信息。

```rust
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub size: usize,
}
```

### CachePerformanceStats

位置：`src/cache/stats.rs`

缓存性能统计。

```rust
pub struct CachePerformanceStats {
    pub l1_stats: CacheStats,
    pub l2_stats: CacheStats,
    pub total_hits: u64,
    pub total_misses: u64,
}
```

## 缓存键生成

### key_generator

位置：`src/cache/key_generator.rs`

缓存键生成器，负责生成唯一的缓存键。

**键格式**：
- 查询缓存：`query:{database}:{table}:{conditions_hash}`
- 记录缓存：`record:{database}:{table}:{id}`

**关键方法**：
- `generate_query_key(database, table, conditions)` - 生成查询缓存键
- `generate_record_key(database, table, id)` - 生成记录缓存键
- `hash_conditions(conditions)` - 哈希查询条件

## 查询缓存

位置：`src/cache/query_cache.rs`

按查询条件缓存结果。

**功能**：
- 自动缓存查询结果
- 支持 TTL 过期
- 支持缓存绕过
- 自动失效相关缓存

## 记录缓存

位置：`src/cache/record_cache.rs`

按 ID 缓存单条记录。

**功能**：
- 自动缓存单条记录
- 支持 TTL 过期
- 写入时自动更新缓存
- 删除时自动失效缓存

## 缓存操作

位置：`src/cache/operations.rs`

缓存操作的具体实现。

**功能**：
- 缓存读写操作
- 缓存失效策略
- 缓存统计收集
- 缓存性能监控

## 缓存策略

### L1/L2 双层缓存

基于 `rat_memcache` 实现：

- **L1（内存缓存）**：高速访问，容量有限
- **L2（磁盘缓存）**：持久化存储，容量较大

### 缓存配置

```rust
pub struct CacheConfig {
    pub enabled: bool,
    pub strategy: CacheStrategy,
    pub ttl_seconds: Option<u64>,
    pub max_size: Option<usize>,
}
```

### CacheStrategy 枚举

```rust
pub enum CacheStrategy {
    None,           // 不缓存
    Memory,         // 仅内存缓存（L1）
    Disk,           // 仅磁盘缓存（L2）
    MemoryAndDisk,  // 双层缓存（L1 + L2）
}
```

## 缓存绕过

`find_with_cache_control()` 方法支持强制跳过缓存查询：

```rust
// 强制跳过缓存查询（适用于金融等实时数据场景）
let results = ModelManager::<User>::find_with_cache_control(
    conditions,
    None,
    true  // bypass_cache = true
).await?;
```

## 缓存集成

ODM 层自动处理缓存：
- 读取时检查缓存
- 写入时更新/失效缓存
- 支持缓存绕过

## 缓存统计

通过 `get_cache_stats()` 函数获取缓存统计：

```rust
pub use manager::{get_cache_stats, get_cache_manager};

let stats = get_cache_stats()?;
println!("缓存命中率: {}", stats.hit_rate());
```