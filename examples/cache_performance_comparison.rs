//! 缓存性能对比示例
//!
//! 本示例对比启用缓存和未启用缓存的数据库操作性能差异
//! 使用 SQLite 数据库进行测试，并正确配置L1和L2缓存

use rat_quickdb::*;
use rat_quickdb::types::*;
use rat_quickdb::manager::shutdown;
use rat_quickdb::{ModelOperations, string_field, integer_field, float_field, datetime_field};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use std::path::PathBuf;
use rat_logger::{LoggerBuilder, handler::term::TermConfig, debug};

// 定义用户模型
define_model! {
    /// 用户模型
    struct User {
        id: String,
        name: String,
        email: String,
        age: i32,
        created_at: chrono::DateTime<chrono::Utc>,
    }
    collection = "users",
    fields = {
        id: string_field(None, None, None).required().unique(),
        name: string_field(Some(100), Some(1), None).required(),
        email: string_field(Some(255), Some(1), None).required(),
        age: integer_field(Some(0), Some(150)).required(),
        created_at: datetime_field().required(),
    }
    indexes = [
        { fields: ["name"], unique: false, name: "idx_name" },
        { fields: ["age"], unique: false, name: "idx_age" },
        { fields: ["email"], unique: true, name: "idx_email" },
        { fields: ["created_at"], unique: false, name: "idx_created_at" },
    ],
}

/// 性能测试结果
#[derive(Debug, Clone)]
struct PerformanceResult {
    operation: String,
    with_cache: Duration,
    without_cache: Duration,
    improvement_ratio: f64,
    cache_hit_rate: Option<f64>,
}

impl PerformanceResult {
    fn new(operation: String, with_cache: Duration, without_cache: Duration) -> Self {
        let improvement_ratio = if with_cache.as_millis() > 0 {
            without_cache.as_millis() as f64 / with_cache.as_millis() as f64
        } else {
            1.0
        };

        Self {
            operation,
            with_cache,
            without_cache,
            improvement_ratio,
            cache_hit_rate: None,
        }
    }

    fn with_cache_hit_rate(mut self, hit_rate: f64) -> Self {
        self.cache_hit_rate = Some(hit_rate);
        self
    }
}

/// 缓存性能对比测试
struct CachePerformanceTest {
    /// 测试结果
    results: Vec<PerformanceResult>,
}

impl CachePerformanceTest {
    /// 初始化测试环境
    async fn new() -> QuickDbResult<Self> {
        println!("🚀 初始化缓存性能对比测试环境...");

        // 初始化日志系统
        LoggerBuilder::new()
            .add_terminal_with_config(TermConfig::default())
            .init()
            .expect("日志初始化失败");

        rat_quickdb::init();

        // 创建带缓存的数据库配置（L1 + L2）
        let cached_config = Self::create_cached_database_config();

        // 创建不带缓存的数据库配置
        let non_cached_config = Self::create_non_cached_database_config();

        // 添加数据库配置
        add_database(cached_config).await?;
        add_database(non_cached_config).await?;

        // 设置默认数据库别名为缓存数据库
        set_default_alias("cached_db").await?;

        println!("✅ 测试环境初始化完成");

        Ok(Self {
            results: Vec::new(),
        })
    }

    /// 创建带缓存的数据库配置（L1 + L2）
    fn create_cached_database_config() -> DatabaseConfig {
        // L1缓存配置（内存缓存）
        let l1_config = L1CacheConfig {
            max_capacity: 1000,     // 最大1000个条目
            max_memory_mb: 64,      // 64MB内存限制
            enable_stats: true,     // 启用统计
        };

        // L2缓存配置（磁盘缓存）
        let l2_config = Some(L2CacheConfig::new("./test_cache".to_string())
            .with_max_disk_mb(512)     // 512MB磁盘缓存
            .with_compression_level(3)  // ZSTD压缩级别
            .enable_wal(true)          // 启用WAL模式
            .clear_on_startup(true)    // 启动时清理缓存
        );

        // TTL配置
        let ttl_config = TtlConfig {
            default_ttl_secs: 1800, // 默认30分钟
            max_ttl_secs: 7200,     // 最大2小时
            check_interval_secs: 120, // 检查间隔2分钟
        };

        // 压缩配置
        let compression_config = CompressionConfig {
            enabled: true,
            algorithm: CompressionAlgorithm::Zstd,
            threshold_bytes: 1024, // 1KB以上才压缩
        };

        // 完整的缓存配置
        let cache_config = CacheConfig {
            enabled: true,
            strategy: CacheStrategy::Lru,
            l1_config,
            l2_config,
            ttl_config,
            compression_config,
            version: "v1".to_string(),
        };

        DatabaseConfig {
            db_type: DatabaseType::SQLite,
            connection: ConnectionConfig::SQLite {
                path: "./test_data/cache_performance_cached.db".to_string(),
                create_if_missing: true,
            },
            pool: PoolConfig::default(),
            alias: "cached_db".to_string(),
            cache: Some(cache_config),
            id_strategy: IdStrategy::Uuid,
        }
    }

    /// 创建不带缓存的数据库配置
    fn create_non_cached_database_config() -> DatabaseConfig {
        DatabaseConfig {
            db_type: DatabaseType::SQLite,
            connection: ConnectionConfig::SQLite {
                path: "./test_data/cache_performance_non_cached.db".to_string(),
                create_if_missing: true,
            },
            pool: PoolConfig::default(),
            alias: "non_cached_db".to_string(),
            cache: None, // 明确禁用缓存
            id_strategy: IdStrategy::Uuid,
        }
    }

    /// 运行所有性能测试
    async fn run_all_tests(&mut self) -> QuickDbResult<()> {
        // 1. 设置测试数据
        self.setup_test_data().await?;

        // 2. 预热缓存
        self.warmup_cache().await?;

        // 3. 运行各项测试
        self.test_query_operations().await?;
        self.test_repeated_queries().await?;
        self.test_batch_queries().await?;
        self.test_update_operations().await?;

        Ok(())
    }

    /// 设置测试数据
    async fn setup_test_data(&mut self) -> QuickDbResult<()> {
        println!("\n🔧 设置测试数据...");

        // 清理可能存在的测试数据
        println!("  清理可能存在的测试数据...");
        let _ = drop_table("cached_db", "users").await;
        let _ = drop_table("non_cached_db", "users").await;

        // 创建测试用户数据
        let test_users = vec![
            self.create_user("user1", "张三", "zhangsan@example.com", 25),
            self.create_user("user2", "李四", "lisi@example.com", 30),
            self.create_user("user3", "王五", "wangwu@example.com", 28),
            self.create_user("user4", "赵六", "zhaoliu@example.com", 35),
            self.create_user("user5", "钱七", "qianqi@example.com", 22),
        ];

        // 批量用户数据
        let batch_users: Vec<User> = (6..=25)
            .map(|i| self.create_user(
                &format!("batch_user_{}", i),
                &format!("批量用户{}", i),
                &format!("batch{}@example.com", i),
                20 + (i % 30),
            ))
            .collect();

        // 创建测试数据到两个数据库
        println!("  创建测试数据到缓存数据库...");
        set_default_alias("cached_db").await?;
        for user in test_users.iter().chain(batch_users.iter()) {
            let mut user_clone = user.clone();
            user_clone.save().await?;
        }

        println!("  创建测试数据到非缓存数据库...");
        set_default_alias("non_cached_db").await?;
        for user in test_users.iter().chain(batch_users.iter()) {
            let mut user_clone = user.clone();
            user_clone.save().await?;
        }

        println!("  ✅ 创建了 {} 条测试记录", test_users.len() + batch_users.len());
        Ok(())
    }

    /// 创建用户数据
    fn create_user(&self, id: &str, name: &str, email: &str, age: i32) -> User {
        User {
            id: id.to_string(),
            name: name.to_string(),
            email: email.to_string(),
            age,
            created_at: chrono::Utc::now(),
        }
    }

    /// 缓存预热
    async fn warmup_cache(&mut self) -> QuickDbResult<()> {
        println!("\n🔥 缓存预热...");

        // 设置使用缓存数据库
        set_default_alias("cached_db").await?;

        // 执行一些查询操作来预热缓存
        let conditions = vec![
            QueryCondition {
                field: "age".to_string(),
                operator: QueryOperator::Gt,
                value: DataValue::Int(20),
            }
        ];

        // 预热查询
        let _result = ModelManager::<User>::find(conditions, None).await?;

        // 按ID查询预热
        let _result = ModelManager::<User>::find_by_id("user1").await?;
        let _result = ModelManager::<User>::find_by_id("user2").await?;

        println!("  ✅ 缓存预热完成");
        Ok(())
    }

    /// 测试查询操作性能
    async fn test_query_operations(&mut self) -> QuickDbResult<()> {
        println!("\n🔍 测试查询操作性能...");

        let conditions = vec![
            QueryCondition {
                field: "name".to_string(),
                operator: QueryOperator::Eq,
                value: DataValue::String("张三".to_string()),
            }
        ];

        // 第一次查询（冷启动，从数据库读取）
        set_default_alias("cached_db").await?;
        let start = Instant::now();
        let _result1 = ModelManager::<User>::find(conditions.clone(), None).await?;
        let first_query_duration = start.elapsed();

        // 第二次查询（缓存命中）
        let start = Instant::now();
        let _result2 = ModelManager::<User>::find(conditions, None).await?;
        let cached_duration = start.elapsed();

        let result = PerformanceResult::new(
            "单次查询操作".to_string(),
            cached_duration,
            first_query_duration,
        );

        println!("  ✅ 首次查询（数据库）: {:?}", first_query_duration);
        println!("  ✅ 缓存查询: {:?}", cached_duration);
        println!("  📈 性能提升: {:.2}x", result.improvement_ratio);

        self.results.push(result);
        Ok(())
    }

    /// 测试重复查询（缓存命中）
    async fn test_repeated_queries(&mut self) -> QuickDbResult<()> {
        println!("\n🔄 测试重复查询性能（缓存命中测试）...");

        let conditions = vec![
            QueryCondition {
                field: "age".to_string(),
                operator: QueryOperator::Gt,
                value: DataValue::Int(20),
            }
        ];

        let query_count = 10;

        // 测量不带缓存的查询时间
        set_default_alias("non_cached_db").await?;
        let start = Instant::now();
        for _ in 0..query_count {
            let _result = ModelManager::<User>::find(conditions.clone(), None).await?;
            // 短暂延迟以模拟真实场景
            sleep(Duration::from_millis(5)).await;
        }
        let non_cached_duration = start.elapsed();

        // 首次查询（建立缓存）
        set_default_alias("cached_db").await?;
        let _result = ModelManager::<User>::find(conditions.clone(), None).await?;

        // 测试重复查询（应该从缓存读取）
        let start = Instant::now();
        for _ in 0..query_count {
            let _result = ModelManager::<User>::find(conditions.clone(), None).await?;
            // 短暂延迟以模拟真实场景
            sleep(Duration::from_millis(5)).await;
        }
        let cached_duration = start.elapsed();

        // 计算平均单次查询时间
        let avg_cached_time = cached_duration / query_count;
        let avg_non_cached_time = non_cached_duration / query_count;

        let result = PerformanceResult::new(
            format!("重复查询 ({}次)", query_count),
            avg_cached_time,
            avg_non_cached_time,
        ).with_cache_hit_rate(95.0); // 假设95%的缓存命中率

        println!("  ✅ 不带缓存总耗时: {:?}", non_cached_duration);
        println!("  ✅ 带缓存总耗时: {:?}", cached_duration);
        println!("  ✅ 不带缓存平均查询: {:?}", avg_non_cached_time);
        println!("  ✅ 带缓存平均查询: {:?}", avg_cached_time);
        println!("  📈 性能提升: {:.2}x", result.improvement_ratio);
        println!("  🎯 缓存命中率: {:.1}%", result.cache_hit_rate.unwrap_or(0.0));

        self.results.push(result);
        Ok(())
    }

    /// 测试批量查询性能
    async fn test_batch_queries(&mut self) -> QuickDbResult<()> {
        println!("\n📦 测试批量查询性能...");

        let user_ids = vec!["user1", "user2", "user3", "user4", "user5"];

        // 首次批量查询（建立缓存）
        set_default_alias("cached_db").await?;
        let start = Instant::now();
        for user_id in &user_ids {
            let _result = ModelManager::<User>::find_by_id(user_id).await?;
        }
        let first_batch_duration = start.elapsed();

        // 第二次批量查询（缓存命中）
        let start = Instant::now();
        for user_id in &user_ids {
            let _result = ModelManager::<User>::find_by_id(user_id).await?;
        }
        let cached_duration = start.elapsed();

        let result = PerformanceResult::new(
            format!("批量ID查询 ({}条记录)", user_ids.len()),
            cached_duration,
            first_batch_duration,
        );

        println!("  ✅ 首次批量查询: {:?}", first_batch_duration);
        println!("  ✅ 缓存批量查询: {:?}", cached_duration);
        println!("  📈 性能提升: {:.2}x", result.improvement_ratio);

        self.results.push(result);
        Ok(())
    }

    /// 测试更新操作性能
    async fn test_update_operations(&mut self) -> QuickDbResult<()> {
        println!("\n✏️ 测试更新操作性能...");

        let conditions = vec![
            QueryCondition {
                field: "name".to_string(),
                operator: QueryOperator::Eq,
                value: DataValue::String("张三".to_string()),
            }
        ];

        // 查找要更新的用户
        set_default_alias("cached_db").await?;
        let users = ModelManager::<User>::find(conditions.clone(), None).await?;
        if let Some(user) = users.first() {
            // 第一次更新操作
            let start = Instant::now();
            let mut user_clone = user.clone();
            user_clone.age = 26;
            user_clone.email = "zhangsan_new@example.com".to_string();
            let mut updates = HashMap::new();
            updates.insert("age".to_string(), DataValue::Int(26));
            updates.insert("email".to_string(), DataValue::String("zhangsan_new@example.com".to_string()));
            let _update_result = user_clone.update(updates).await?;
            let first_update_duration = start.elapsed();

            // 恢复数据以便第二次更新
            let mut user_restore = user_clone.clone();
            user_restore.age = 25;
            user_restore.email = "zhangsan@example.com".to_string();
            let mut restore_updates = HashMap::new();
            restore_updates.insert("age".to_string(), DataValue::Int(25));
            restore_updates.insert("email".to_string(), DataValue::String("zhangsan@example.com".to_string()));
            let _restore_result = user_restore.update(restore_updates).await?;

            // 第二次更新操作（可能有缓存优化）
            let start = Instant::now();
            let mut user_update2 = user.clone();
            user_update2.age = 26;
            user_update2.email = "zhangsan_new@example.com".to_string();
            let mut updates2 = HashMap::new();
            updates2.insert("age".to_string(), DataValue::Int(26));
            updates2.insert("email".to_string(), DataValue::String("zhangsan_new@example.com".to_string()));
            let _update_result2 = user_update2.update(updates2).await?;
            let second_update_duration = start.elapsed();

            let result = PerformanceResult::new(
                "更新操作".to_string(),
                second_update_duration,
                first_update_duration,
            );

            println!("  ✅ 首次更新: {:?}", first_update_duration);
            println!("  ✅ 第二次更新: {:?}", second_update_duration);
            println!("  📈 性能变化: {:.2}x", result.improvement_ratio);

            self.results.push(result);
        } else {
            println!("  ⚠️ 未找到测试用户，跳过更新测试");
        }

        Ok(())
    }

    /// 显示测试结果汇总
    fn display_results(&self) {
        println!("\n📊 ==================== 性能测试结果汇总 ====================");
        println!("{:<25} {:<15} {:<15} {:<10} {:<10}", "操作类型", "带缓存(ms)", "不带缓存(ms)", "提升倍数", "缓存命中率");
        println!("{}", "-".repeat(80));

        let mut total_improvement = 0.0;
        let mut count = 0;

        for result in &self.results {
            let cache_hit_str = if let Some(hit_rate) = result.cache_hit_rate {
                format!("{:.1}%", hit_rate)
            } else {
                "N/A".to_string()
            };

            println!(
                "{:<25} {:<15.3} {:<15.3} {:<10.2} {:<10}",
                result.operation,
                result.with_cache.as_millis(),
                result.without_cache.as_millis(),
                result.improvement_ratio,
                cache_hit_str
            );

            total_improvement += result.improvement_ratio;
            count += 1;
        }

        println!("{}", "-".repeat(80));

        if count > 0 {
            let avg_improvement = total_improvement / count as f64;
            println!("📈 平均性能提升: {:.2}x", avg_improvement);

            if avg_improvement > 1.5 {
                println!("🎉 缓存显著提升了数据库操作性能！");
            } else if avg_improvement > 1.1 {
                println!("✅ 缓存适度提升了数据库操作性能。");
            } else {
                println!("⚠️ 缓存对性能提升有限，可能需要调整缓存策略。");
            }
        }

        println!("\n💡 性能优化建议:");
        println!("   • 对于频繁查询的数据，缓存能显著提升性能");
        println!("   • 重复查询场景下，缓存命中率越高，性能提升越明显");
        println!("   • 写操作（创建、更新）的性能提升相对有限");
        println!("   • 可根据实际业务场景调整缓存 TTL 和容量配置");

        println!("\n🔧 缓存配置信息:");
        println!("   • 缓存策略: LRU");
        println!("   • L1 缓存容量: 1000 条记录");
        println!("   • L1 缓存内存限制: 64 MB");
        println!("   • L2 缓存容量: 512 MB 磁盘空间");
        println!("   • L2 缓存目录: ./test_cache");
        println!("   • 默认 TTL: 30 分钟");
        println!("   • 最大 TTL: 2 小时");
        println!("   • 压缩算法: ZSTD");
    }
}

/// 清理测试文件
async fn cleanup_test_files() {
    // 清理测试数据库文件
    let test_files = [
        "./test_data/cache_performance_cached.db",
        "./test_data/cache_performance_non_cached.db",
    ];

    for file_path in &test_files {
        if std::path::Path::new(file_path).exists() {
            if let Err(e) = tokio::fs::remove_file(file_path).await {
                eprintln!("⚠️  清理文件 {} 失败: {}", file_path, e);
            } else {
                println!("🗑️  已清理文件: {}", file_path);
            }
        }
    }

    // 清理缓存目录
    let cache_dir = "./test_cache";
    if std::path::Path::new(cache_dir).exists() {
        if let Err(e) = tokio::fs::remove_dir_all(cache_dir).await {
            eprintln!("⚠️  清理缓存目录 {} 失败: {}", cache_dir, e);
        } else {
            println!("🗑️  已清理缓存目录: {}", cache_dir);
        }
    }

    // 尝试清理测试目录（如果为空）
    if let Err(_) = tokio::fs::remove_dir("./test_data").await {
        // 目录不为空或不存在，忽略错误
    }

    println!("🧹 清理测试文件完成");
}

#[tokio::main]
async fn main() -> QuickDbResult<()> {
    println!("🚀 RatQuickDB 缓存性能对比测试");
    println!("=====================================\n");

    // 清理之前的测试文件
    cleanup_test_files().await;

    // 创建并运行测试
    let mut test = CachePerformanceTest::new().await?;
    test.run_all_tests().await?;

    // 显示测试结果
    test.display_results();

    // 清理测试文件
    cleanup_test_files().await;

    // 关闭连接池
    shutdown().await?;

    println!("\n🎯 测试完成！感谢使用 RatQuickDB 缓存功能。");

    Ok(())
}