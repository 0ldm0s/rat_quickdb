//! SQLite时区和范围查询演示示例
//!
//! 展示如何使用时区感知的DateTime字段进行范围查询

use rat_quickdb::*;
use rat_quickdb::types::{QueryCondition, QueryOperator, DataValue, QueryOptions};
use rat_quickdb::manager::shutdown;
use rat_quickdb::{ModelOperations, string_field, integer_field, datetime_field};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

// 定义用户活动日志模型（使用北京时间）
define_model! {
    /// 用户活动日志模型
    struct UserActivity {
        id: String,
        user_id: String,
        username: String,
        activity_type: String,
        description: String,
        beijing_time: String,  // 现在使用String类型保持输入输出一致性
        duration_minutes: i32,
        ip_address: String,
        created_at: String,   // 同样改为String类型
    }
    collection = "user_activities",
    database = "default",
    fields = {
        id: string_field(None, None, None).required().unique(),
        user_id: string_field(None, None, None).required(),
        username: string_field(Some(100), Some(1), None).required(),
        activity_type: string_field(Some(50), Some(1), None).required(),
        description: string_field(Some(500), Some(1), None).required(),
        beijing_time: datetime_field(Some("+08:00".to_string())).required(),  // 北京时间字段
        duration_minutes: integer_field(Some(0), Some(1440)).required(),  // 活动持续时间（分钟）
        ip_address: string_field(Some(45), Some(1), None).required(),
        created_at: datetime_field(Some("+00:00".to_string())).required(),  // UTC时间
    }
    indexes = [
        { fields: ["user_id"], unique: false, name: "idx_user_id" },
        { fields: ["beijing_time"], unique: false, name: "idx_beijing_time" },
        { fields: ["activity_type"], unique: false, name: "idx_activity_type" },
        { fields: ["user_id", "beijing_time"], unique: false, name: "idx_user_time" },
    ],
}

// 定义系统事件模型（使用UTC时间）
define_model! {
    /// 系统事件模型
    struct SystemEvent {
        id: String,
        event_type: String,
        component: String,
        severity: String,
        message: String,
        utc_timestamp: String,  // 现在使用String类型保持输入输出一致性
        affected_users: i32,
        resolved: bool,
        resolved_at: Option<String>,  // 同样改为String类型
    }
    collection = "system_events",
    database = "default",
    fields = {
        id: string_field(None, None, None).required().unique(),
        event_type: string_field(Some(50), Some(1), None).required(),
        component: string_field(Some(100), Some(1), None).required(),
        severity: string_field(Some(20), Some(1), None).required(),
        message: string_field(Some(1000), Some(1), None).required(),
        utc_timestamp: datetime_field(Some("+00:00".to_string())).required(),  // UTC时间戳
        affected_users: integer_field(Some(0), None).required(),
        resolved: boolean_field().required(),
        resolved_at: datetime_field(Some("+00:00".to_string())),
    }
    indexes = [
        { fields: ["event_type"], unique: false, name: "idx_event_type" },
        { fields: ["severity"], unique: false, name: "idx_severity" },
        { fields: ["utc_timestamp"], unique: false, name: "idx_utc_timestamp" },
        { fields: ["component", "utc_timestamp"], unique: false, name: "idx_component_time" },
    ],
}

#[tokio::main]
async fn main() -> QuickDbResult<()> {
    println!("=== SQLite时区和范围查询演示 ===");

    // 初始化系统
    rat_quickdb::init();

    // 清理旧的测试数据
    match std::fs::remove_file("./timezone_range_demo.db") {
        Ok(_) => println!("✅ 成功清理旧的数据库文件"),
        Err(e) => println!("⚠️ 清理旧数据库文件失败（可能文件不存在）: {}", e),
    }

    // 创建数据库配置
    let config = DatabaseConfig::builder()
        .db_type(DatabaseType::SQLite)
        .connection(ConnectionConfig::SQLite {
            path: "./timezone_range_demo.db".to_string(),
            create_if_missing: true,
        })
        .pool(PoolConfig::builder()
            .min_connections(2)
            .max_connections(10)
            .connection_timeout(30)
            .idle_timeout(300)
            .max_lifetime(3600)
            .max_retries(3)
            .retry_interval_ms(1000)
            .keepalive_interval_sec(60)
            .health_check_timeout_sec(10)
            .build()?)
        .alias("default".to_string())
        .id_strategy(IdStrategy::Uuid)
        .build()?;

    // 初始化数据库
    add_database(config).await?;

    println!("数据库初始化完成\n");

    // 创建测试数据
    create_test_data().await?;

    println!("\n=== 开始时区和范围查询测试 ===\n");

    // 测试1：北京时间范围查询
    test_beijing_time_range_query().await?;

    // 测试2：UTC时间范围查询
    test_utc_time_range_query().await?;

    // 测试3：数字范围查询（持续时间）
    test_duration_range_query().await?;

    // 测试4：复合范围查询
    test_complex_range_query().await?;

    // 测试5：时区转换验证
    test_timezone_conversion().await?;

    println!("\n=== 时区和范围查询演示完成 ===");

    // 关闭连接池
    shutdown().await?;

    Ok(())
}

/// 创建测试数据
async fn create_test_data() -> QuickDbResult<()> {
    println!("创建测试数据...");

    // 创建用户活动数据（使用北京时间）
    let activities = vec![
        create_user_activity(
            "user_001",
            "张三",
            "login",
            "用户登录系统",
            "2024-01-15T09:30:00+08:00",  // 北京时间上午9:30
            15,
            "192.168.1.100"
        ),
        create_user_activity(
            "user_002",
            "李四",
            "file_upload",
            "上传文件report.pdf",
            "2024-01-15T14:20:00+08:00",  // 北京时间下午2:20
            45,
            "192.168.1.101"
        ),
        create_user_activity(
            "user_003",
            "王五",
            "data_export",
            "导出年度报表数据",
            "2024-01-15T16:45:00+08:00",  // 北京时间下午4:45
            120,
            "192.168.1.102"
        ),
        create_user_activity(
            "user_001",
            "张三",
            "meeting",
            "参加团队会议",
            "2024-01-15T20:00:00+08:00",  // 北京时间晚上8:00
            90,
            "192.168.1.100"
        ),
        create_user_activity(
            "user_004",
            "赵六",
            "system_backup",
            "执行系统备份",
            "2024-01-16T01:30:00+08:00",  // 北京时间次日凌晨1:30
            60,
            "192.168.1.103"
        ),
    ];

    for activity in activities {
        match activity.save().await {
            Ok(id) => println!("✅ 创建用户活动: {} - {}", id, activity.username),
            Err(e) => println!("❌ 创建用户活动失败: {}", e),
        }
    }

    // 创建系统事件数据（使用UTC时间）
    let events = vec![
        create_system_event(
            "server_restart",
            "web_server",
            "warning",
            "Web服务器重启",
            "2024-01-15T01:30:00Z",  // UTC时间1:30
            150
        ),
        create_system_event(
            "database_maintenance",
            "database",
            "info",
            "数据库例行维护",
            "2024-01-15T06:00:00Z",  // UTC时间6:00
            0
        ),
        create_system_event(
            "security_alert",
            "firewall",
            "critical",
            "检测到异常登录尝试",
            "2024-01-15T12:45:00Z",  // UTC时间12:45
            3
        ),
        create_system_event(
            "performance_issue",
            "api_server",
            "error",
            "API响应时间异常",
            "2024-01-15T15:30:00Z",  // UTC时间15:30
            500
        ),
        create_system_event(
            "network_outage",
            "load_balancer",
            "critical",
            "负载均衡器故障",
            "2024-01-15T18:15:00Z",  // UTC时间18:15
            1200
        ),
    ];

    for event in events {
        match event.save().await {
            Ok(id) => println!("✅ 创建系统事件: {} - {}", id, event.event_type),
            Err(e) => println!("❌ 创建系统事件失败: {}", e),
        }
    }

    println!("✅ 测试数据创建完成");
    Ok(())
}

/// 测试北京时间范围查询
async fn test_beijing_time_range_query() -> QuickDbResult<()> {
    println!("1. 北京时间范围查询测试");
    println!("====================");

    // 查询2024年1月15日北京时间9:00-18:00的用户活动
    let start_time_str = "2024-01-15T09:00:00+08:00";
    let end_time_str = "2024-01-15T18:00:00+08:00";

    println!("查询时间范围: {} - {}", start_time_str, end_time_str);

    let conditions = vec![
        QueryCondition {
            field: "beijing_time".to_string(),
            operator: QueryOperator::Gte,
            value: DataValue::String(start_time_str.to_string()),
        },
        QueryCondition {
            field: "beijing_time".to_string(),
            operator: QueryOperator::Lte,
            value: DataValue::String(end_time_str.to_string()),
        },
    ];

    match ModelManager::<UserActivity>::find(conditions, None).await {
        Ok(activities) => {
            println!("✅ 找到 {} 个用户活动:", activities.len());
            for activity in activities {
                // 现在框架支持输入输出一致性：
                // 输入：String("2024-01-15T09:00:00+08:00")
                // 输出：String("2024-01-15T09:00:00+08:00")
                println!("   - {} ({}) - {} - 持续{}分钟 - {}",
                    activity.username,
                    activity.activity_type,
                    activity.beijing_time,  // 现在是String类型，自动保持时区格式
                    activity.duration_minutes,
                    activity.description
                );
            }
        },
        Err(e) => println!("❌ 查询失败: {}", e),
    }

    println!();
    Ok(())
}

/// 测试UTC时间范围查询
async fn test_utc_time_range_query() -> QuickDbResult<()> {
    println!("2. UTC时间范围查询测试");
    println!("===================");

    // 查询2024年1月15日UTC时间12:00-20:00的系统事件
    let start_time_str = "2024-01-15T12:00:00+00:00";
    let end_time_str = "2024-01-15T20:00:00+00:00";

    println!("查询时间范围: {} - {}", start_time_str, end_time_str);

    let conditions = vec![
        QueryCondition {
            field: "utc_timestamp".to_string(),
            operator: QueryOperator::Gte,
            value: DataValue::String(start_time_str.to_string()),
        },
        QueryCondition {
            field: "utc_timestamp".to_string(),
            operator: QueryOperator::Lte,
            value: DataValue::String(end_time_str.to_string()),
        },
    ];

    match ModelManager::<SystemEvent>::find(conditions, None).await {
        Ok(events) => {
            println!("✅ 找到 {} 个系统事件:", events.len());
            for event in events {
                println!("   - {} ({}) - 影响用户:{} - {}",
                    event.event_type,
                    event.severity,
                    event.affected_users,
                    event.utc_timestamp  // 现在是String类型，自动保持时区格式
                );
                println!("     消息: {}", event.message);
            }
        },
        Err(e) => println!("❌ 查询失败: {}", e),
    }

    println!();
    Ok(())
}

/// 测试数字范围查询（持续时间）
async fn test_duration_range_query() -> QuickDbResult<()> {
    println!("3. 数字范围查询测试（活动持续时间）");
    println!("=================================");

    // 查询持续时间在30分钟到2小时之间的用户活动
    let conditions = vec![
        QueryCondition {
            field: "duration_minutes".to_string(),
            operator: QueryOperator::Gte,
            value: DataValue::Int(30),
        },
        QueryCondition {
            field: "duration_minutes".to_string(),
            operator: QueryOperator::Lte,
            value: DataValue::Int(120),  // 2小时 = 120分钟
        },
    ];

    println!("查询条件: 活动持续时间 >= 30分钟 且 <= 120分钟");

    match ModelManager::<UserActivity>::find(conditions, None).await {
        Ok(activities) => {
            println!("✅ 找到 {} 个长时间活动:", activities.len());
            for activity in activities {
                println!("   - {} ({}) - {}分钟 - {}",
                    activity.username,
                    activity.activity_type,
                    activity.duration_minutes,
                    activity.description
                );
            }
        },
        Err(e) => println!("❌ 查询失败: {}", e),
    }

    println!();
    Ok(())
}

/// 测试复合范围查询
async fn test_complex_range_query() -> QuickDbResult<()> {
    println!("4. 复合范围查询测试");
    println!("==================");

    // 查询特定时间范围内且影响用户数超过100的系统事件
    let start_time_str = "2024-01-15T06:00:00+00:00";
    let end_time_str = "2024-01-15T19:00:00+00:00";

    let conditions = vec![
        QueryCondition {
            field: "utc_timestamp".to_string(),
            operator: QueryOperator::Gte,
            value: DataValue::String(start_time_str.to_string()),
        },
        QueryCondition {
            field: "utc_timestamp".to_string(),
            operator: QueryOperator::Lte,
            value: DataValue::String(end_time_str.to_string()),
        },
        QueryCondition {
            field: "affected_users".to_string(),
            operator: QueryOperator::Gt,
            value: DataValue::Int(100),
        },
    ];

    println!("查询条件:");
    println!("  时间范围: {} - {}", start_time_str, end_time_str);
    println!("  影响用户数: > 100");

    match ModelManager::<SystemEvent>::find(conditions, None).await {
        Ok(events) => {
            println!("✅ 找到 {} 个高影响系统事件:", events.len());
            for event in events {
                println!("   - {} ({}) - 影响用户:{} - {}",
                    event.event_type,
                    event.severity,
                    event.affected_users,
                    event.utc_timestamp  // 现在是String类型，自动保持时区格式
                );
                println!("     消息: {}", event.message);
            }
        },
        Err(e) => println!("❌ 查询失败: {}", e),
    }

    println!();
    Ok(())
}

/// 测试时区转换验证
async fn test_timezone_conversion() -> QuickDbResult<()> {
    println!("5. 时区转换验证测试");
    println!("==================");

    // 查询所有用户活动，验证时区转换
    match ModelManager::<UserActivity>::find(vec![], None).await {
        Ok(activities) => {
            println!("✅ 验证 {} 个用户活动的时区转换:", activities.len());
            for activity in activities {
                println!("   用户: {} - 活动: {}", activity.username, activity.activity_type);
                println!("     北京时间: {} (输入输出一致)", activity.beijing_time);
                println!("     描述: {}", activity.description);
                println!();
            }
        },
        Err(e) => println!("❌ 查询失败: {}", e),
    }

    Ok(())
}

// 辅助函数

/// 创建用户活动数据
fn create_user_activity(
    user_id: &str,
    username: &str,
    activity_type: &str,
    description: &str,
    beijing_time_str: &str,  // 北京时间字符串
    duration_minutes: i32,
    ip_address: &str,
) -> UserActivity {
    UserActivity {
        id: String::new(),  // 框架会自动生成ID
        user_id: user_id.to_string(),
        username: username.to_string(),
        activity_type: activity_type.to_string(),
        description: description.to_string(),
        beijing_time: beijing_time_str.to_string(),  // 直接使用字符串格式
        duration_minutes,
        ip_address: ip_address.to_string(),
        created_at: Utc::now().to_rfc3339(),  // 转换为字符串格式
    }
}

/// 创建系统事件数据
fn create_system_event(
    event_type: &str,
    component: &str,
    severity: &str,
    message: &str,
    utc_timestamp_str: &str,  // UTC时间字符串
    affected_users: i32,
) -> SystemEvent {
    SystemEvent {
        id: String::new(),  // 框架会自动生成ID
        event_type: event_type.to_string(),
        component: component.to_string(),
        severity: severity.to_string(),
        message: message.to_string(),
        utc_timestamp: utc_timestamp_str.to_string(),  // 直接使用字符串格式
        affected_users,
        resolved: false,
        resolved_at: None,
    }
}

