//! 测试新的带时区的DateTime字段 (MongoDB版本)
//!
//! 验证 datetime_with_tz_field 函数的功能

use rat_quickdb::*;
use rat_quickdb::types::{DatabaseType, ConnectionConfig, PoolConfig};
use rat_quickdb::manager::health_check;
use rat_quickdb::{ModelManager, ModelOperations, datetime_with_tz_field};
use rat_logger::{LoggerBuilder, LevelFilter, handler::term::TermConfig};
use chrono::{Utc, DateTime};

// 数据库别名常量
const DATABASE_ALIAS: &str = "main";

// 定义测试模型
define_model! {
    /// 带时区的测试模型
    struct TimeZoneTestModel {
        id: String,
        name: String,
        created_at_utc: chrono::DateTime<chrono::Utc>,
        local_time_cst: chrono::DateTime<chrono::FixedOffset>,
        local_time_est: String,
    }
    collection = "timezone_test",
    database = DATABASE_ALIAS,
    fields = {
        id: string_field(None, None, None).required().unique(),
        name: string_field(None, None, None).required(),
        created_at_utc: datetime_field(),  // 默认UTC (+00:00)
        local_time_cst: datetime_with_tz_field("+08:00"),  // 北京时间
        local_time_est: datetime_with_tz_field("-05:00"),  // 美东时间
    }
}

#[tokio::main]
async fn main() -> QuickDbResult<()> {
    // 初始化日志
    LoggerBuilder::new()
        .add_terminal_with_config(TermConfig::default())
        .init()
        .expect("日志初始化失败");

    println!("🚀 测试带时区的DateTime字段 (MongoDB版本)");
    println!("========================\n");

    // 1. 配置数据库
    println!("1. 配置MongoDB数据库...");
    let db_config = DatabaseConfig {
        alias: DATABASE_ALIAS.to_string(),
        db_type: DatabaseType::MongoDB,
        connection: ConnectionConfig::MongoDB {
            host: "db0.0ldm0s.net".to_string(),
            port: 27017,
            database: "testdb".to_string(),
            username: Some("testdb".to_string()),
            password: Some("testdb123456".to_string()),
            auth_source: Some("testdb".to_string()),
            direct_connection: true,
            tls_config: Some(rat_quickdb::types::TlsConfig {
                enabled: true,
                ca_cert_path: None,
                client_cert_path: None,
                client_key_path: None,
                verify_server_cert: false,
                verify_hostname: false,
                min_tls_version: None,
                cipher_suites: None,
            }),
            zstd_config: Some(rat_quickdb::types::ZstdConfig {
                enabled: true,
                compression_level: Some(3),
                compression_threshold: Some(1024),
            }),
            options: {
                let mut opts = std::collections::HashMap::new();
                opts.insert("retryWrites".to_string(), "true".to_string());
                opts.insert("w".to_string(), "majority".to_string());
                Some(opts)
            },
        },
        pool: PoolConfig::builder()
                .max_connections(10)
                .min_connections(1)
                .connection_timeout(10)
                .idle_timeout(300)
                .max_lifetime(1800)
                .max_retries(3)
                .retry_interval_ms(1000)
                .keepalive_interval_sec(60)
                .health_check_timeout_sec(10)
                .build()
                .unwrap(),
        id_strategy: IdStrategy::Uuid,
        cache: None,
    };

    add_database(db_config).await?;
    println!("✅ 数据库配置完成\n");

    // 2. 健康检查
    println!("2. 数据库健康检查...");
    let health_results = health_check().await;
    if let Some(&is_healthy) = health_results.get(DATABASE_ALIAS) {
        if is_healthy {
            println!("✅ 数据库连接正常");
        } else {
            println!("❌ 数据库连接异常");
            return Err(QuickDbError::ConnectionError {
                message: "数据库连接异常".to_string(),
            });
        }
    } else {
        println!("❌ 未找到main数据库配置");
        return Err(QuickDbError::ConnectionError {
            message: "未找到main数据库配置".to_string(),
        });
    }
    println!();

    // 清理之前的表数据
    println!("3. 清理之前的测试表...");
    match rat_quickdb::drop_table(DATABASE_ALIAS, "timezone_test").await {
        Ok(_) => println!("✅ 表清理完成"),
        Err(e) => println!("⚠️  表清理失败（可能表不存在）: {}", e),
    }
    println!();

    // 4. 测试字段定义
    println!("3. 🔧 测试字段定义");
    println!("==================");

    // 测试新的字段类型
    let utc_field = datetime_field();  // 默认UTC
    let cst_field = datetime_with_tz_field("+08:00");  // 北京时间
    let est_field = datetime_with_tz_field("-05:00");  // 美东时间

    println!("✅ datetime_field() - 默认UTC时区: {:?}", utc_field.field_type);
    println!("✅ datetime_with_tz_field(+08:00) - 北京时间: {:?}", cst_field.field_type);
    println!("✅ datetime_with_tz_field(-05:00) - 美东时间: {:?}", est_field.field_type);
    println!();

    // 4. 测试字段验证
    println!("4. ✅ 测试字段验证");
    println!("==================");

    // 测试有效的时区格式
    let valid_timezones = vec!["+00:00", "+08:00", "-05:00", "+12:45", "-09:30"];
    for tz in valid_timezones {
        let field = datetime_with_tz_field(tz);
        match field.validate(&DataValue::String("2024-06-15 12:00:00".to_string())) {
            Ok(_) => println!("✅ 时区 {}: 验证通过", tz),
            Err(e) => println!("❌ 时区 {}: 验证失败 - {}", tz, e),
        }
    }

    // 测试无效的时区格式
    let invalid_timezones = vec!["CST", "UTC", "+8:00", "+08:0", "25:00", "+24:00"];
    for tz in invalid_timezones {
        let field = datetime_with_tz_field(tz);
        match field.validate(&DataValue::String("2024-06-15 12:00:00".to_string())) {
            Ok(_) => println!("❌ 时区 {}: 应该失败但通过了", tz),
            Err(_) => println!("✅ 时区 {}: 正确拒绝了无效格式", tz),
        }
    }
    println!();

    // 5. 创建测试数据
    println!("5. 📝 创建测试数据");
    println!("==================");

    let now = Utc::now();

    // 开发者始终传入UTC时间，框架根据字段定义自动处理时区
    let test_model = TimeZoneTestModel {
        id: String::new(), // 框架会自动生成UUID
        name: "时区测试".to_string(),
        created_at_utc: now,
        local_time_cst: now.into(),  // 转换为FixedOffset
        local_time_est: now.to_rfc3339(),  // 传入RFC3339字符串，框架应该根据-05:00时区设置处理
    };

    match test_model.save().await {
        Ok(id) => {
            println!("✅ 成功创建测试模型，ID: {}", id);
        },
        Err(e) => {
            println!("❌ 创建测试模型失败: {}", e);
            return Err(e);
        }
    }
    println!();

    // 6. 查询测试数据
    println!("6. 🔍 查询测试数据");
    println!("==================");

    match ModelManager::<TimeZoneTestModel>::find(vec![], None).await {
        Ok(models) => {
            println!("✅ 查询到 {} 条记录", models.len());
            for (index, model) in models.iter().enumerate() {
                println!("📋 第{}条记录:", index + 1);

                // 动态判断字段类型
                println!("  name: {} (实际类型: {})", model.name, std::any::type_name_of_val(&model.name));
                println!("  created_at_utc: {} (实际类型: {})", model.created_at_utc, std::any::type_name_of_val(&model.created_at_utc));
                println!("  local_time_cst: {} (实际类型: {})", model.local_time_cst, std::any::type_name_of_val(&model.local_time_cst));
                println!("  local_time_est: {} (实际类型: {})", model.local_time_est, std::any::type_name_of_val(&model.local_time_est));

                println!("  ---");

                // 尝试调用format方法（如果是DateTime类型才会成功）
                if std::any::type_name_of_val(&model.created_at_utc).contains("DateTime") {
                    println!("  created_at_utc.format(): {}", model.created_at_utc.format("%Y-%m-%d %H:%M:%S UTC"));
                } else {
                    println!("  created_at_utc (直接输出): {}", model.created_at_utc);
                }

                if std::any::type_name_of_val(&model.local_time_cst).contains("DateTime") {
                    println!("  local_time_cst.format(): {}", model.local_time_cst.format("%Y-%m-%d %H:%M:%S"));
                } else {
                    println!("  local_time_cst (直接输出): {}", model.local_time_cst);
                }

                // local_time_est现在是String类型，直接输出
                println!("  local_time_est (String类型): {}", model.local_time_est);

                println!();
            }
        },
        Err(e) => println!("❌ 查询失败: {}", e),
    }
    println!();

    // 7. 检查数据库存储（跳过删除，保留数据供调试）
    println!("7. 💾 检查数据库存储");
    println!("==================");
    println!("⏸️  跳过数据删除，保留数据库文件供调试分析");

    println!("\n🎉 带时区DateTime字段测试完成 (MongoDB版本)！");
    println!("📋 总结:");
    println!("  - 新增 datetime_with_tz_field() 函数支持时区偏移");
    println!("  - 时区格式：+00:00, +08:00, -05:00");
    println!("  - 自动验证时区格式有效性");
    println!("  - 向后兼容：datetime_field() 等同于 datetime_with_tz_field(+00:00)");
    Ok(())
}