# i18n 多语言化 Checklist

> 本文档列出所有未使用 `rat_embed_lang` 进行多语言化处理的用户可见提示信息。
> 每项标注了文件路径和行号，用于制定分阶段修复方案。

---

## 当前状态

| 指标 | 数值 |
|------|------|
| 已注册翻译 key | 34 个 |
| 实际调用 i18n 的文件 | 1 个 (`src/pool/pool.rs`) |
| 未处理的文件数 | ~30 个 |
| 未处理的消息条数 | ~280 条 |
| 支持语言 | zh-CN / en-US / ja-JP |

---

## P0: `QuickDbError` 枚举 — error.rs

> **优先级最高**：thiserror 的 `#[error("...")]` 生成的 `Display` impl 永远输出硬编码中文，所有下游错误展示都受影响。

- [ ] `src/error.rs:11` — `ConnectionError`: `"数据库连接失败: {message}"`
- [ ] `src/error.rs:15` — `PoolError`: `"连接池操作失败: {message}"`
- [ ] `src/error.rs:19` — `QueryError`: `"查询执行失败: {message}"`
- [ ] `src/error.rs:23` — `SerializationError`: `"数据序列化失败: {message}"`
- [ ] `src/error.rs:27` — `ValidationError`: `"模型验证失败: {field} - {message}"`
- [ ] `src/error.rs:31` — `ConfigError`: `"配置错误: {message}"`
- [ ] `src/error.rs:35` — `AliasNotFound`: `"数据库别名 '{alias}' 未找到"`
- [ ] `src/error.rs:39` — `UnsupportedDatabase`: `"不支持的数据库类型: {db_type}"`
- [ ] `src/error.rs:43` — `TransactionError`: `"事务操作失败: {message}"`
- [ ] `src/error.rs:47` — `TaskExecutionError`: `"任务执行失败: {0}"`
- [ ] `src/error.rs:51` — `CacheError`: `"缓存操作失败: {message}"`
- [ ] `src/error.rs:55` — `IoError`: `"IO 操作失败: {0}"`
- [ ] `src/error.rs:59` — `JsonError`: `"JSON 处理失败: {0}"`
- [ ] `src/error.rs:63` — `Other`: `"操作失败: {0}"`
- [ ] `src/error.rs:67` — `TableNotExistError`: `"表或集合 '{table}' 不存在: {message}"`
- [ ] `src/error.rs:71` — `VersionError`: `"版本管理操作失败: {message}"`
- [ ] `src/error.rs:75` — `NotFound`: `"数据未找到: {message}"`

---

## P1: 字段验证 — model/field_types.rs

> 用户输入校验失败时直接展示，影响面广。

- [ ] `src/model/field_types.rs:168` — `"必填字段不能为空"`
- [ ] `src/model/field_types.rs:189` — `"字符串长度不能超过{}"`
- [ ] `src/model/field_types.rs:197` — `"字符串长度不能少于{}"`
- [ ] `src/model/field_types.rs:205` — `"正则表达式无效: {}"`
- [ ] `src/model/field_types.rs:211` — `"字符串不匹配正则表达式"`
- [ ] `src/model/field_types.rs:218` — `"字段类型不匹配，期望字符串类型"`
- [ ] `src/model/field_types.rs:231` — `"整数值不能小于{}"`
- [ ] `src/model/field_types.rs:239` — `"整数值不能大于{}"`
- [ ] `src/model/field_types.rs:246` — `"字段类型不匹配，期望整数类型"`
- [ ] `src/model/field_types.rs:259` — `"浮点数值不能小于{}"`
- [ ] `src/model/field_types.rs:267` — `"浮点数值不能大于{}"`
- [ ] `src/model/field_types.rs:274` — `"字段类型不匹配，期望浮点数类型"`
- [ ] `src/model/field_types.rs:282` — `"字段类型不匹配，期望布尔类型"`
- [ ] `src/model/field_types.rs:290` — `"字段类型不匹配，期望日期时间类型"`
- [ ] `src/model/field_types.rs:323` — `"无效的RFC3339日期时间格式: '{}' (字段: {})"`
- [ ] `src/model/field_types.rs:341` — `"无效的日期时间格式，期望RFC3339或YYYY-MM-DD HH:MM:SS格式: '{}' (字段: {})"`
- [ ] `src/model/field_types.rs:360` — `"字段类型不匹配，期望日期时间类型或字符串或整数 (字段: {})"`
- [ ] `src/model/field_types.rs:372` — `"无效的时区偏移格式: '{}', 期望格式: +00:00, +08:00, -05:00"`
- [ ] `src/model/field_types.rs:400` — `"无效的UUID格式: '{}' (字段: {})"`
- [ ] `src/model/field_types.rs:423` — `"字段类型不匹配，期望UUID字符串或UUID类型，实际收到: {:?} (字段: {})"`
- [ ] `src/model/field_types.rs:446` — `"数组元素数量不能超过{}"`
- [ ] `src/model/field_types.rs:454` — `"数组元素数量不能少于{}"`
- [ ] `src/model/field_types.rs:473` — `"数组元素数量不能超过{}"`
- [ ] `src/model/field_types.rs:481` — `"数组元素数量不能少于{}"`
- [ ] `src/model/field_types.rs:494` — `"JSON字符串不是有效的数组格式"`
- [ ] `src/model/field_types.rs:500` — `"无法解析JSON字符串"`
- [ ] `src/model/field_types.rs:507` — `"字段类型不匹配，期望数组类型或JSON字符串"`
- [ ] `src/model/field_types.rs:522` — `"字段类型不匹配，期望对象类型"`
- [ ] `src/model/field_types.rs:533` — `"引用字段必须是字符串ID"`
- [ ] `src/model/field_types.rs:541` — `"字段类型不匹配，期望大整数类型"`
- [ ] `src/model/field_types.rs:549` — `"字段类型不匹配，期望双精度浮点数类型"`
- [ ] `src/model/field_types.rs:557` — `"字段类型不匹配，期望文本类型"`
- [ ] `src/model/field_types.rs:565` — `"字段类型不匹配，期望日期类型"`
- [ ] `src/model/field_types.rs:573` — `"字段类型不匹配，期望时间类型"`
- [ ] `src/model/field_types.rs:581` — `"字段类型不匹配，期望二进制数据（Base64字符串）"`
- [ ] `src/model/field_types.rs:592` — `"字段类型不匹配，期望十进制数类型"`

---

## P2: 安全验证 — security.rs

> 字段名/表名校验失败时展示。

- [ ] `src/security.rs:35` — `"字段名不能为空"`
- [ ] `src/security.rs:43` — `"字段名长度不能超过64个字符"`
- [ ] `src/security.rs:71` — `"表名不能为空"`
- [ ] `src/security.rs:79` — `"表名长度不能超过64个字符"`
- [ ] `src/security.rs:146` — `"SQL字段名不能以数字开头"`
- [ ] `src/security.rs:155` — `"SQL字段名包含非法字符 '{}' 在位置 {}"`
- [ ] `src/security.rs:222` — `"字段名不能使用SQL关键字: {}"`
- [ ] `src/security.rs:239` — `"NoSQL字段名不能以$开头"`
- [ ] `src/security.rs:247` — `"NoSQL字段名不能包含点号"`
- [ ] `src/security.rs:273` — `"字段名不能使用MongoDB保留字: {}"`
- [ ] `src/security.rs:286` — `"SQL表名不能以数字开头"`
- [ ] `src/security.rs:295` — `"SQL表名包含非法字符 '{}' 在位置 {}"`
- [ ] `src/security.rs:337` — `"表名不能使用SQL关键字: {}"`
- [ ] `src/security.rs:352` — `"集合名不能以$开头"`
- [ ] `src/security.rs:360` — `"集合名不能包含空字符"`
- [ ] `src/security.rs:368` — `"集合名不能以system.开头"`

---

## P3: 配置构建器 — config/

> 配置错误在应用启动时展示，开发者高频接触。

### config/builders/database_builder.rs
- [ ] `src/config/builders/database_builder.rs:139` — `"数据库类型必须设置"`
- [ ] `src/config/builders/database_builder.rs:143` — `"连接配置必须设置"`
- [ ] `src/config/builders/database_builder.rs:147` — `"连接池配置必须设置"`
- [ ] `src/config/builders/database_builder.rs:151` — `"数据库别名必须设置"`
- [ ] `src/config/builders/database_builder.rs:155` — `"ID生成策略必须设置"`
- [ ] `src/config/builders/database_builder.rs:186` — `"数据库类型 {:?} 与连接配置不匹配"`

### config/builders/global_builder.rs
- [ ] `src/config/builders/global_builder.rs:81` — `"至少需要配置一个数据库"`
- [ ] `src/config/builders/global_builder.rs:86` — `"应用配置必须设置"`
- [ ] `src/config/builders/global_builder.rs:90` — `"日志配置必须设置"`
- [ ] `src/config/builders/global_builder.rs:97` — `"默认数据库 '{}' 不存在于数据库配置中"`

### config/builders/pool_builder.rs
- [ ] `src/config/builders/pool_builder.rs:146` — PoolConfig 各字段 "必须设置" 校验 (~9 条)
- [ ] `src/config/builders/pool_builder.rs:182` — PoolConfig 范围校验 (~4 条)

### config/builders/app_builder.rs
- [ ] `src/config/builders/app_builder.rs:89-105` — AppConfig 各字段 "必须设置" 校验 (~5 条)

### config/builders/logging_builder.rs
- [ ] `src/config/builders/logging_builder.rs:101-117` — LoggingConfig 各字段 "必须设置" 校验 (~5 条)
- [ ] `src/config/builders/logging_builder.rs:120-124` — LoggingConfig 范围校验 (~2 条)

### config/core.rs
- [ ] `src/config/core.rs:107` — `"解析TOML配置文件失败: {}"`
- [ ] `src/config/core.rs:110` — `"解析JSON配置文件失败: {}"`
- [ ] `src/config/core.rs:128` — `"序列化TOML配置失败: {}"`
- [ ] `src/config/core.rs:131` — `"序列化JSON配置失败: {}"`
- [ ] `src/config/core.rs:145` — `"未设置默认数据库"`
- [ ] `src/config/core.rs:149` — `"找不到默认数据库配置: {}"`
- [ ] `src/config/core.rs:160` — `"找不到数据库配置: {}"`

---

## P4: ODM 层 — odm/

> ODM 是用户最常用的操作接口，错误消息高频出现。

### odm/operations/odm_operations_impl.rs
- [ ] `"ODM后台任务已停止"` — 出现约 14 次
- [ ] `"ODM请求处理失败"` — 出现约 14 次

### odm/manager_core.rs
- [ ] `src/odm/manager_core.rs:250,313` — `"连接池操作通道已关闭"`
- [ ] `src/odm/manager_core.rs:257,320` — `"等待连接池响应超时"`

### odm/handlers/create_handler.rs
- [ ] `src/odm/handlers/create_handler.rs:135` — `"连接池操作通道已关闭"`
- [ ] `src/odm/handlers/create_handler.rs:142` — `"等待连接池响应超时"`
- [ ] `src/odm/handlers/create_handler.rs:155` — `"创建操作返回的数据中缺少id字段"`

### odm/handlers/read_handler.rs
- [ ] `src/odm/handlers/read_handler.rs` — `"连接池操作通道已关闭"` x2
- [ ] `src/odm/handlers/read_handler.rs` — `"等待连接池响应超时"` x2

### odm/handlers/update_handler.rs
- [ ] `src/odm/handlers/update_handler.rs` — `"连接池操作通道已关闭"` x2
- [ ] `src/odm/handlers/update_handler.rs` — `"等待连接池响应超时"` x2

### odm/handlers/delete_handler.rs
- [ ] `src/odm/handlers/delete_handler.rs` — `"连接池操作通道已关闭"` x3
- [ ] `src/odm/handlers/delete_handler.rs` — `"等待连接池响应超时"` x3
- [ ] `src/odm/handlers/delete_handler.rs` — `"等待数据库操作结果超时"` x2

---

## P5: 连接池层 — pool/

### pool/pool.rs（已部分使用 i18n）
- [ ] `src/pool/pool.rs:85` — `"不支持的数据库类型（可能需要启用相应的feature）"`
- [ ] `src/pool/pool.rs` — `"操作发送未实现"` / `"发送操作失败"` / `"接收响应失败"` (~25 条，在 match 分支中重复)

### pool/sqlite_worker.rs
- [ ] `src/pool/sqlite_worker.rs:154` — `"SQLite连接配置类型不匹配"`
- [ ] `src/pool/sqlite_worker.rs:164` — `"SQLite内存数据库连接失败: {}"`
- [ ] `src/pool/sqlite_worker.rs:176` — `"SQLite数据库文件不存在且未启用自动创建: {}"`
- [ ] `src/pool/sqlite_worker.rs:185` — `"创建SQLite数据库目录失败: {}"`
- [ ] `src/pool/sqlite_worker.rs:194` — `"创建SQLite数据库文件失败: {}"`
- [ ] `src/pool/sqlite_worker.rs:202` — `"SQLite连接失败: {}"`

---

## P6: 适配器层 — adapter/

### adapter/mongodb/query.rs
- [ ] `src/adapter/mongodb/query.rs:53` — `"转换ID为BSON失败: {}"`
- [ ] `src/adapter/mongodb/query.rs:70,89` — `"MongoDB集合 '{}' 不存在"` / `"不存在或为空"`
- [ ] `src/adapter/mongodb/query.rs:74` — `"MongoDB查询失败: {}"`
- [ ] `src/adapter/mongodb/query.rs:176,248` — `"MongoDB条件组合查询失败: {}"`
- [ ] `src/adapter/mongodb/query.rs:184,256` — `"MongoDB游标遍历失败: {}"`
- [ ] `src/adapter/mongodb/query.rs:190,262` — `"MongoDB文档反序列化失败: {}"`
- [ ] `src/adapter/mongodb/query.rs:295,299,333,337` — MongoDB 集合/计数错误

### adapter/mongodb/adapter.rs
- [ ] `src/adapter/mongodb/adapter.rs:187,503` — `"序列化MongoDB聚合管道失败: {}"`
- [ ] `src/adapter/mongodb/adapter.rs:532` — `"聚合管道序列化失败: {}"`
- [ ] `src/adapter/mongodb/adapter.rs:546` — `"MongoDB聚合查询失败: {}"`
- [ ] `src/adapter/mongodb/adapter.rs:554` — `"MongoDB聚合游标遍历失败: {}"`
- [ ] `src/adapter/mongodb/adapter.rs:559` — `"MongoDB聚合文档反序列化失败: {}"`

### adapter/mongodb/operations.rs
- [ ] `src/adapter/mongodb/operations.rs:107` — `"使用{:?}策略时必须提供ID字段"`
- [ ] `src/adapter/mongodb/operations.rs:145,157` — `"转换DataValue为BSON失败: {}"`
- [ ] `src/adapter/mongodb/operations.rs:171,307,540` — `"MongoDB插入/更新失败: {}"`
- [ ] `src/adapter/mongodb/operations.rs:363-503` — 各类型转 BSON 失败 (~7 条)
- [ ] `src/adapter/mongodb/operations.rs:569` — `"MongoDB集合 '{}' 不存在"`
- [ ] `src/adapter/mongodb/operations.rs:573` — `"MongoDB删除失败: {}"`
- [ ] `src/adapter/mongodb/operations.rs:677` — `"存储过程配置验证失败: {}"`
- [ ] `src/adapter/mongodb/operations.rs:738` — `"存储过程 '{}' 不存在"`
- [ ] `src/adapter/mongodb/operations.rs:753` — `"解析聚合管道模板失败: {}"`

### adapter/mongodb/utils.rs
- [ ] `src/adapter/mongodb/utils.rs:107` — `"Json字段类型接收到非对象/数组数据: {:?}，这是内部错误..."`
- [ ] `src/adapter/mongodb/utils.rs:282` — `"转换更新数据为BSON失败: {}"`

### adapter/mongodb/query_builder.rs
- [ ] `src/adapter/mongodb/query_builder.rs:305` — `"无效的JSON格式: {}"`
- [ ] `src/adapter/mongodb/query_builder.rs:359,395,446` — 查询构建器错误 (~3 条)

---

## P7: 序列化 — serializer.rs ✅

- [x] `src/serializer.rs:145` — `"序列化为JSON字符串失败: {}"`
- [x] `src/serializer.rs:151` — `"序列化为JSON字符串失败: {}"`
- [x] `src/serializer.rs:162` — `"解析JSON字符串失败: {}"`
- [x] `src/serializer.rs:176` — `"解析JSON字符串失败: {}"`
- [x] `src/serializer.rs:230` — `"序列化失败: {}"`
- [x] `src/serializer.rs:270` — `"序列化失败: {}"`
- [x] `src/serializer.rs:329` — `"无法处理记录序列化结果"`
- [x] `src/serializer.rs:364` — `"序列化失败: {}"`
- [x] `src/serializer.rs:432` — `"JSON值不是对象类型"`

---

## P8: 表管理 — table/ ✅

### table/schema.rs
- [x] `src/table/schema.rs:289` — `"表必须至少有一个列"`
- [x] `src/table/schema.rs:296` — `"列名 '{}' 重复"`
- [x] `src/table/schema.rs:304` — `"索引名 '{}' 重复"`
- [x] `src/table/schema.rs:310` — `"索引 '{}' 引用的列 '{}' 不存在"`
- [x] `src/table/schema.rs:322` — `"约束名 '{}' 重复"`
- [x] `src/table/schema.rs:328` — `"约束 '{}' 引用的列 '{}' 不存在"`

### table/version.rs
- [x] `src/table/version.rs:202` — `"源版本 {} 不存在"`
- [x] `src/table/version.rs:209` — `"目标版本 {} 不存在"`
- [x] `src/table/version.rs:256` — `"迁移脚本 {} 不存在"`
- [x] `src/table/version.rs:264` — `"迁移脚本 {} 状态不正确: {:?}"`
- [x] `src/table/version.rs:328` — `"迁移脚本 {} 不存在"`
- [x] `src/table/version.rs:338` — `"迁移脚本 {} 没有回滚脚本"`
- [x] `src/table/version.rs:346` — `"迁移脚本 {} 状态不正确，无法回滚: {:?}"`
- [x] `src/table/version.rs:431` — `"表 {} 不存在"`
- [x] `src/table/version.rs:445` — `"版本 {} 缺少迁移脚本"`
- [x] `src/table/version.rs:451` — `"版本 {} 不存在"`
- [x] `src/table/version.rs:465` — `"版本 {} 缺少回滚脚本"`
- [x] `src/table/version.rs:471` — `"版本 {} 缺少迁移脚本"`
- [x] `src/table/version.rs:477` — `"版本 {} 不存在"`

### table/manager.rs
- [x] `src/table/manager.rs:175` — `"无法获取默认连接池"`
- [x] `src/table/manager.rs:270` — `"初始版本"` (description 字符串)
- [x] `src/table/manager.rs:287` — `"无法获取默认连接池"`
- [x] `src/table/manager.rs:458` — `"表 {} 没有注册版本"`

---

## P9: 模型其他 — model/

### model/traits.rs
- [x] `src/model/traits.rs:64` — `"序列化失败: {}"`
- [x] `src/model/traits.rs:70` — `"解析JSON失败: {}"`
- [x] `src/model/traits.rs:113` — `"序列化失败: {}"`
- [x] `src/model/traits.rs:120` — `"解析JSON失败: {}"`
- [x] `src/model/traits.rs:307` — `"字段 '{}' 未在模型元数据中定义，这在 v0.3.0 中是不允许的"`

### model/conversion/datetime_conversion.rs
- [x] `src/model/conversion/datetime_conversion.rs:40` — `"不支持的数据类型 {:?}，期望DateTime<Utc>或RFC3339格式的字符串"`
- [x] `src/model/conversion/datetime_conversion.rs:108` — `"时间 '{}' 在时区 '{}' 下存在歧义（夏令时等）"`
- [x] `src/model/conversion/datetime_conversion.rs:120` — `"无效的时区偏移: {}"`
- [x] `src/model/conversion/datetime_conversion.rs:129` — `"无法解析日期时间字符串 '{}'。支持的格式：..."`
- [x] `src/model/conversion/datetime_conversion.rs:155` — `"无效的时区偏移格式: '{}'。期望格式: [+/-]HH:MM..."`
- [x] `src/model/conversion/datetime_conversion.rs:171` — `"无效的小时数: {}"`
- [x] `src/model/conversion/datetime_conversion.rs:180` — `"无效的分钟数: {}"`
- [x] `src/model/conversion/datetime_conversion.rs:187` — `"时区偏移超出范围: {}{}:{}.小时范围: 0-23，分钟范围: 0-59"`

### model/data_conversion.rs
- [x] `src/model/data_conversion.rs:22` — `"无法从DataValue映射创建模型实例: {}"`
- [x] `src/model/data_conversion.rs:155` — `"字段 '{}' 不存在"`
- [x] `src/model/data_conversion.rs:160` — `"字段访问越界"`
- [x] `src/model/data_conversion.rs:212` — `"数据值不存在"`
- [x] `src/model/data_conversion.rs:215` — `"键访问错误"`
- [x] `src/model/data_conversion.rs:218` — `"键访问错误"`

---

## P10: 管理器层 — manager/

- [ ] `src/manager/cache_ops.rs:27` — `"数据库 {} 没有配置缓存管理器"`
- [ ] `src/manager/database_ops.rs:146` — `"没有配置默认数据库别名"`
- [ ] `src/manager/database_ops.rs:221` — `"数据库 {} 没有配置ID生成器"`
- [ ] `src/manager/database_ops.rs:236` — `"数据库 {} 没有MongoDB自增ID生成器"`
- [ ] `src/manager/alias_type_map.rs:19` — `"数据库类型映射表锁被污染"`
- [ ] `src/manager/mod.rs:42` — `"全局操作已锁定，禁止添加数据库！系统已开始执行查询操作..."` (panic)
- [ ] `src/manager/maintenance.rs:94` — `"无法获取缓存统计信息"`
- [ ] `src/manager/model_ops.rs:174` — `"集合 {} 没有注册的模型元数据，跳过表和索引创建"`

---

## P11: 存储过程 — stored_procedure/

- [ ] `src/stored_procedure/config.rs:406` — `"存储过程名称不能为空"`
- [ ] `src/stored_procedure/config.rs:413` — `"数据库别名不能为空"`
- [ ] `src/stored_procedure/config.rs:424` — `"至少需要一个字段或聚合管道操作"`
- [ ] `src/stored_procedure/config.rs:433` — `"JOIN字段不能为空"`
- [ ] `src/stored_procedure/config.rs:450` — `"数据库别名 '{}' 不存在"`
- [ ] `src/stored_procedure/config.rs:466` — `"警告：MongoDB对复杂JOIN支持有限，建议使用聚合管道中的$lookup操作"`
- [ ] `src/stored_procedure/config.rs:478` — `"{} 不支持MongoDB聚合管道，请使用传统字段映射和JOIN配置"`
- [ ] `src/stored_procedure/config.rs:494` — `"{} 存储过程必须定义字段映射"`

---

## P12: 类型系统 — types/

### types/data_value/mod.rs
- [ ] `src/types/data_value/mod.rs:98` — `"DataValue 转换为 JSON 失败: {}"`
- [ ] `src/types/data_value/mod.rs:105` — `"JSON 解析为 DataValue 失败: {}"`
- [ ] `src/types/data_value/mod.rs:220` — `"DataValue 反序列化失败: {}"`
- [ ] `src/types/data_value/mod.rs` — 各类型转换错误 (~10 条)

### types/serde_helpers.rs
- [ ] `src/types/serde_helpers.rs:43` — `"无法解析JSON字符串: {}"`
- [ ] `src/types/serde_helpers.rs:45` — `"期望JSON对象或JSON字符串"`

---

## 附注

### 已注册但未使用的翻译 key

以下 key 已在 `src/i18n/mod.rs` 注册翻译，但源码中未调用 `t()` / `tf()`：

- `error.table_no_columns`
- `error.column_name_duplicate`
- `error.index_name_duplicate`
- `error.index_column_not_found`
- `error.constraint_name_duplicate`
- `error.constraint_column_not_found`
- `error.json_serialize`
- `error.json_parse`
- `error.serialize`

### 技术难点

1. **`thiserror` 的 `#[error("...")]` 属性**：Rust 过程宏不支持在属性中调用运行时函数。需要改为手动实现 `Display` trait 或使用 `#[error(transparent)]` + 内部错误类型。
2. **`format!()` 中的参数化消息**：需要全部替换为 `tf("key", &[("param", &value)])` 形式。
3. **重复消息**：ODM 层约 30 处重复的 "通道已关闭" / "超时" 消息，可提取为公共函数一次修改。
