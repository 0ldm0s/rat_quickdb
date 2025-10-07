//! 数据库服务器版本查询示例
//!
//! 本示例展示如何通过正确的ODM层API获取不同数据库服务器的版本信息
//! 采用渐进式添加，先测试SQLite，后续可扩展到其他数据库
//! 注意：始终通过ODM层操作，不直接访问连接池，保持架构完整性

use rat_quickdb::*;
use rat_logger::{info, warn, error, debug};

/// 服务器版本查询测试器
struct ServerVersionTester {
    /// 测试结果收集
    test_results: Vec<DatabaseTestResult>,
}

/// 数据库测试结果
#[derive(Debug)]
struct DatabaseTestResult {
    /// 数据库类型
    db_type: String,
    /// 别名
    alias: String,
    /// 版本信息
    version: String,
    /// 测试是否成功
    success: bool,
    /// 错误信息（如果有）
    error: Option<String>,
}

impl ServerVersionTester {
    /// 创建新的测试器
    fn new() -> Self {
        Self {
            test_results: Vec::new(),
        }
    }

    /// 运行所有测试
    async fn run_all_tests(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // 初始化日志系统
        rat_logger::LoggerBuilder::new()
            .add_terminal_with_config(rat_logger::handler::term::TermConfig::default())
            .init()?;

        info!("=== 数据库服务器版本查询测试 ===");
        info!("注意：本示例通过正确的ODM层API操作，保持架构完整性");

        // 一次性初始化rat_quickdb
        rat_quickdb::init();
        info!("✅ rat_quickdb库初始化完成");

        // ========== 测试SQLite ==========
        self.test_sqlite().await?;

        // TODO: 后续可以添加其他数据库测试
        // self.test_postgresql().await?;
        // self.test_mysql().await?;
        // self.test_mongodb().await?;

        // 打印测试结果汇总
        self.print_summary();

        Ok(())
    }

    /// 测试SQLite数据库
    async fn test_sqlite(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("\n📄 开始测试SQLite数据库...");

        // 创建SQLite数据库配置
        let sqlite_config = DatabaseConfig::builder()
            .db_type(DatabaseType::SQLite)
            .connection(ConnectionConfig::SQLite {
                path: "/tmp/test_server_version.db".to_string(),
                create_if_missing: true,
            })
            .pool(PoolConfig::builder()
                .min_connections(1)
                .max_connections(5)
                .connection_timeout(30)
                .idle_timeout(300)
                .max_lifetime(1800)
                .build()?)
            .alias("sqlite_test".to_string())
            .disable_cache() // 禁用缓存
            .id_strategy(IdStrategy::AutoIncrement)
            .build()?;

        // 通过正确的ODM API添加数据库配置
        add_database(sqlite_config).await?;
        info!("✅ SQLite数据库配置添加成功");

        // 通过ODM层测试版本查询（这是唯一正确的方式）
        match rat_quickdb::get_server_version(Some("sqlite_test")).await {
            Ok(version) => {
                info!("✅ SQLite版本查询成功: {}", version);

                // 记录测试结果
                self.test_results.push(DatabaseTestResult {
                    db_type: "SQLite".to_string(),
                    alias: "sqlite_test".to_string(),
                    version,
                    success: true,
                    error: None,
                });
            },
            Err(e) => {
                error!("❌ SQLite版本查询失败: {}", e);

                // 记录测试结果
                self.test_results.push(DatabaseTestResult {
                    db_type: "SQLite".to_string(),
                    alias: "sqlite_test".to_string(),
                    version: "未知".to_string(),
                    success: false,
                    error: Some(e.to_string()),
                });
            }
        }

        Ok(())
    }

    /// 打印测试结果汇总
    fn print_summary(&self) {
        info!("\n=== 测试结果汇总 ===");

        let mut success_count = 0;
        let total_count = self.test_results.len();

        for result in &self.test_results {
            if result.success {
                success_count += 1;
                info!("✅ {} ({}) - 版本: {}",
                    result.db_type, result.alias, result.version);
            } else {
                error!("❌ {} ({}) - 错误: {}",
                    result.db_type, result.alias,
                    result.error.as_deref().unwrap_or("未知错误"));
            }
        }

        info!("\n📊 总体结果: {}/{} 个数据库测试成功", success_count, total_count);

        if success_count == total_count {
            info!("🎉 所有数据库版本查询测试通过！");
        } else {
            warn!("⚠️  部分数据库测试失败，请检查配置和连接");
        }

        info!("\n💡 提示：");
        info!("- 本示例展示了正确使用rat_quickdb ODM层API的方式");
        info!("- 使用add_database()配置数据库，避免直接操作连接池");
        info!("- 通过get_server_version()查询服务器信息，保持ODM封装性");
        info!("- 使用别名系统管理多个数据库连接");
        info!("- 后续可以添加PostgreSQL、MySQL、MongoDB等数据库测试");

        info!("\n✅ 架构完整性：");
        info!("- rat_quickdb已修复内部API暴露问题，所有示例都使用正确的ODM层API");
        info!("- 保持了良好的封装性和架构安全性");
        info!("- 所有示例都遵循最佳实践，通过add_database()和ODM操作数据库");
    }
}

/// 主函数
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut tester = ServerVersionTester::new();
    tester.run_all_tests().await
}