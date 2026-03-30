# 历史遗留问题清单

> 本文档记录在 i18n 重构过程中发现的历史遗留问题，与 i18n 无关，需要单独修复。

---

## 1. 示例文件缺少新增字段

**文件**: `examples/cache_performance_comparison.rs:183, 209`
**严重程度**: 编译错误
**影响**: `DatabaseConfig` 后续新增了 `version_storage_path` 和 `enable_versioning` 两个字段，但该示例未同步更新，导致示例无法编译。

**修复**: 在两处 `DatabaseConfig` 构造中添加：
```rust
version_storage_path: None,
enable_versioning: None,
```
