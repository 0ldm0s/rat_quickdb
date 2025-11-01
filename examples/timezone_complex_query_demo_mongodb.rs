//! MongoDB 时区复杂查询示例
//!
//! 展示如何使用MongoDB的datetime字段进行时区相关的复杂查询

use rat_quickdb::*;
use rat_quickdb::types::{QueryCondition, QueryConditionGroup, LogicalOperator, QueryOperator, DataValue, QueryOptions, SortConfig, SortDirection, PaginationConfig};
use rat_quickdb::manager::shutdown;
use rat_quickdb::{ModelOperations, string_field, integer_field, float_field, datetime_field, boolean_field};
use std::collections::HashMap;
use chrono::{Utc, DateTime, Duration, TimeZone};
use rand::Rng;
use rat_logger::{LoggerBuilder, handler::term::TermConfig, debug};

#[tokio::main]
async fn main() -> QuickDbResult<()> {
    // 初始化日志系统
    LoggerBuilder::new()
        .add_terminal_with_config(TermConfig::default())
        .init()
        .expect("日志初始化失败");

    rat_quickdb::init();
    println!("=== MongoDB 时区复杂查询示例 ===");

    // 创建数据库配置 - MongoDB配置（从model_definition_mongodb.rs复制）
    let config = DatabaseConfig {
        alias: "mongodb_default".to_string(),
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
        pool: PoolConfig::default(),
        id_strategy: IdStrategy::Uuid,
        cache: None,
    };

    // 初始化数据库
    add_database(config).await?;

    // 设置默认数据库别名
    rat_quickdb::set_default_alias("mongodb_default").await?;

    // 清理旧的测试集合（确保每次都是干净的状态）
    println!("清理旧的测试集合...");
    match drop_table("mongodb_default", "timezone_events").await {
        Ok(_) => println!("✅ 已清理旧的timezone_events集合"),
        Err(e) => println!("   注意：清理集合失败（可能集合不存在）: {}", e),
    }

    // 创建测试集合
    create_test_table().await?;

    // 插入测试数据
    insert_test_data().await?;

    println!("\n=== 开始时区复杂查询测试 ===\n");

    // 示例1: 基础时区查询 - 查找特定时区的事件
    println!("1. 基础时区查询: 查找UTC时区的事件");

    let utc_condition = QueryConditionGroup::Group {
        operator: LogicalOperator::And,
        conditions: vec![
            QueryConditionGroup::Single(QueryCondition {
                field: "timezone".to_string(),
                operator: QueryOperator::Eq,
                value: DataValue::String("UTC".to_string()),
            }),
        ],
    };

    let utc_result = ModelManager::<TimeZoneEvent>::find_with_groups(
        vec![utc_condition],
        None,
    ).await;

    match utc_result {
        Ok(events) => {
            if events.is_empty() {
                println!("   ❌ 查询结果为空：预期应该找到UTC时区事件（全球技术峰会），但查询返回0个结果");
            } else {
                println!("   ✅ 找到 {} 个UTC时区事件", events.len());
                for event in &events {
                    println!("   - {} ({}) - {} ({})",
                        event.event_name,
                        event.event_type,
                        event.location,
                        event.timezone);
                }
            }
        },
        Err(e) => println!("   ❌ 查询失败: {}", e),
    }

    println!();

    // 示例2: 时间范围查询 + 时区过滤 - 查找亚洲时区在特定时间范围内的事件
    println!("2. 时间范围查询 + 时区过滤: 查找亚洲时区且高优先级的事件");

    let current_time = Utc::now();
    let asian_timezones_condition = QueryConditionGroup::Group {
        operator: LogicalOperator::And,
        conditions: vec![
            // 时区条件组 (OR): 查找亚洲时区
            QueryConditionGroup::Group {
                operator: LogicalOperator::Or,
                conditions: vec![
                    QueryConditionGroup::Single(QueryCondition {
                        field: "timezone".to_string(),
                        operator: QueryOperator::Eq,
                        value: DataValue::String("Asia/Shanghai".to_string()),
                    }),
                    QueryConditionGroup::Single(QueryCondition {
                        field: "timezone".to_string(),
                        operator: QueryOperator::Eq,
                        value: DataValue::String("Asia/Tokyo".to_string()),
                    }),
                    QueryConditionGroup::Single(QueryCondition {
                        field: "timezone".to_string(),
                        operator: QueryOperator::Eq,
                        value: DataValue::String("Asia/Dubai".to_string()),
                    }),
                    QueryConditionGroup::Single(QueryCondition {
                        field: "timezone".to_string(),
                        operator: QueryOperator::Eq,
                        value: DataValue::String("Asia/Kolkata".to_string()),
                    }),
                ],
            },
            // 高优先级条件
            QueryConditionGroup::Single(QueryCondition {
                field: "priority".to_string(),
                operator: QueryOperator::Gte,
                value: DataValue::Int(7),
            }),
            // 活跃状态条件
            QueryConditionGroup::Single(QueryCondition {
                field: "is_active".to_string(),
                operator: QueryOperator::Eq,
                value: DataValue::Bool(true),
            }),
        ],
    };

    let asian_result = ModelManager::<TimeZoneEvent>::find_with_groups(
        vec![asian_timezones_condition],
        None,
    ).await;

    match asian_result {
        Ok(events) => {
            if events.is_empty() {
                println!("   ❌ 查询结果为空：预期应该找到亚洲高优先级活跃事件（亚洲开发者聚会、印度市场活动），但查询返回0个结果");
            } else {
                println!("   ✅ 找到 {} 个亚洲高优先级活跃事件", events.len());
                for event in &events {
                    println!("   - {} ({}) - {} (优先级: {}, 时区: {})",
                        event.event_name,
                        event.event_type,
                        event.location,
                        event.priority,
                        event.timezone);
                }
            }
        },
        Err(e) => println!("   ❌ 查询失败: {}", e),
    }

    println!();

    // 示例3: 复杂时间窗口查询 - 查找正在进行或即将开始的事件
    println!("3. 复杂时间窗口查询: 查找正在进行或即将开始的事件");

    let time_window_condition = QueryConditionGroup::Group {
        operator: LogicalOperator::And,
        conditions: vec![
            // 活跃状态
            QueryConditionGroup::Single(QueryCondition {
                field: "is_active".to_string(),
                operator: QueryOperator::Eq,
                value: DataValue::Bool(true),
            }),
            // 时间条件组: (事件时间在过去3小时内 OR 事件在未来2小时内)
            QueryConditionGroup::Group {
                operator: LogicalOperator::Or,
                conditions: vec![
                    // 事件在过去3小时内开始
                    QueryConditionGroup::Group {
                        operator: LogicalOperator::And,
                        conditions: vec![
                            QueryConditionGroup::Single(QueryCondition {
                                field: "start_time".to_string(),
                                operator: QueryOperator::Gte,
                                value: DataValue::DateTime(current_time - Duration::hours(3)),
                            }),
                            QueryConditionGroup::Single(QueryCondition {
                                field: "start_time".to_string(),
                                operator: QueryOperator::Lte,
                                value: DataValue::DateTime(current_time),
                            }),
                        ],
                    },
                    // 事件在未来2小时内开始
                    QueryConditionGroup::Group {
                        operator: LogicalOperator::And,
                        conditions: vec![
                            QueryConditionGroup::Single(QueryCondition {
                                field: "start_time".to_string(),
                                operator: QueryOperator::Gt,
                                value: DataValue::DateTime(current_time),
                            }),
                            QueryConditionGroup::Single(QueryCondition {
                                field: "start_time".to_string(),
                                operator: QueryOperator::Lte,
                                value: DataValue::DateTime(current_time + Duration::hours(2)),
                            }),
                        ],
                    },
                ],
            },
        ],
    };

    let time_window_result = ModelManager::<TimeZoneEvent>::find_with_groups(
        vec![time_window_condition],
        None,
    ).await;

    match time_window_result {
        Ok(events) => {
            if events.is_empty() {
                println!("   ✅ 查询成功，但没有找到正在进行或即将开始的事件（符合预期：测试数据设置的时间窗口不匹配当前时间）");
            } else {
                println!("   找到 {} 个正在进行或即将开始的事件", events.len());
                for event in &events {
                    let time_until_start = if event.start_time > current_time {
                        format!("将在 {} 分钟后开始",
                            (event.start_time - current_time).num_minutes())
                    } else {
                        format!("已开始 {} 分钟",
                            (current_time - event.start_time).num_minutes())
                    };
                    println!("   - {} - {} ({})",
                        event.event_name,
                        event.location,
                        time_until_start);
                }
            }
        },
        Err(e) => println!("   ❌ 查询失败: {}", e),
    }

    println!();

    // 示例4: 参与者规模和时区组合查询
    println!("4. 参与者规模和时区组合查询: 查找大型跨时区活动");

    let large_event_condition = QueryConditionGroup::Group {
        operator: LogicalOperator::And,
        conditions: vec![
            // 大型活动条件组
            QueryConditionGroup::Group {
                operator: LogicalOperator::Or,
                conditions: vec![
                    QueryConditionGroup::Single(QueryCondition {
                        field: "participant_count".to_string(),
                        operator: QueryOperator::Gte,
                        value: DataValue::Int(100),
                    }),
                    QueryConditionGroup::Single(QueryCondition {
                        field: "priority".to_string(),
                        operator: QueryOperator::Gte,
                        value: DataValue::Int(8),
                    }),
                ],
            },
            // 排除已结束的活动
            QueryConditionGroup::Single(QueryCondition {
                field: "is_active".to_string(),
                operator: QueryOperator::Eq,
                value: DataValue::Bool(true),
            }),
            // 非UTC时区（跨时区活动）
            QueryConditionGroup::Group {
                operator: LogicalOperator::And,
                conditions: vec![
                    QueryConditionGroup::Single(QueryCondition {
                        field: "timezone".to_string(),
                        operator: QueryOperator::Ne,
                        value: DataValue::String("UTC".to_string()),
                    }),
                ],
            },
        ],
    };

    let large_event_options = QueryOptions {
        conditions: vec![],
        sort: vec![
            SortConfig {
                field: "participant_count".to_string(),
                direction: SortDirection::Desc,
            },
            SortConfig {
                field: "priority".to_string(),
                direction: SortDirection::Desc,
            },
        ],
        pagination: Some(PaginationConfig {
            limit: 5,
            skip: 0,
        }),
        fields: vec![],
    };

    let large_event_result = ModelManager::<TimeZoneEvent>::find_with_groups(
        vec![large_event_condition],
        Some(large_event_options),
    ).await;

    match large_event_result {
        Ok(events) => {
            if events.is_empty() {
                println!("   ❌ 查询结果为空：预期应该找到跨时区大型活动（美洲用户大会、欧洲产品发布会等），但查询返回0个结果");
            } else {
                println!("   ✅ 找到 {} 个大型跨时区活动", events.len());
                for (i, event) in events.iter().enumerate() {
                    println!("   {}. {} ({} 人参与, 优先级: {}, 时区: {})",
                        i + 1,
                        event.event_name,
                        event.participant_count,
                        event.priority,
                        event.timezone);
                }
            }
        },
        Err(e) => println!("   ❌ 查询失败: {}", e),
    }

    println!();

    // 示例5: 多层嵌套复杂查询 - 综合条件查询
    println!("5. 多层嵌套复杂查询: 查找技术类高优先级事件或在欧美时区的大型活动");

    let complex_nested_condition = QueryConditionGroup::Group {
        operator: LogicalOperator::Or,
        conditions: vec![
            // 第一个条件组: 技术类高优先级事件
            QueryConditionGroup::Group {
                operator: LogicalOperator::And,
                conditions: vec![
                    QueryConditionGroup::Group {
                        operator: LogicalOperator::Or,
                        conditions: vec![
                            QueryConditionGroup::Single(QueryCondition {
                                field: "event_type".to_string(),
                                operator: QueryOperator::Contains,
                                value: DataValue::String("技术".to_string()),
                            }),
                            QueryConditionGroup::Single(QueryCondition {
                                field: "event_type".to_string(),
                                operator: QueryOperator::Contains,
                                value: DataValue::String("开发".to_string()),
                            }),
                        ],
                    },
                    QueryConditionGroup::Single(QueryCondition {
                        field: "priority".to_string(),
                        operator: QueryOperator::Gte,
                        value: DataValue::Int(7),
                    }),
                    QueryConditionGroup::Single(QueryCondition {
                        field: "is_active".to_string(),
                        operator: QueryOperator::Eq,
                        value: DataValue::Bool(true),
                    }),
                ],
            },
            // 第二个条件组: 欧美时区的大型活动
            QueryConditionGroup::Group {
                operator: LogicalOperator::And,
                conditions: vec![
                    QueryConditionGroup::Group {
                        operator: LogicalOperator::Or,
                        conditions: vec![
                            QueryConditionGroup::Single(QueryCondition {
                                field: "timezone".to_string(),
                                operator: QueryOperator::Eq,
                                value: DataValue::String("Europe/London".to_string()),
                            }),
                            QueryConditionGroup::Single(QueryCondition {
                                field: "timezone".to_string(),
                                operator: QueryOperator::Eq,
                                value: DataValue::String("America/New_York".to_string()),
                            }),
                        ],
                    },
                    QueryConditionGroup::Single(QueryCondition {
                        field: "participant_count".to_string(),
                        operator: QueryOperator::Gt,
                        value: DataValue::Int(150),
                    }),
                    QueryConditionGroup::Single(QueryCondition {
                        field: "is_active".to_string(),
                        operator: QueryOperator::Eq,
                        value: DataValue::Bool(true),
                    }),
                ],
            },
        ],
    };

    let complex_nested_result = ModelManager::<TimeZoneEvent>::find_with_groups(
        vec![complex_nested_condition],
        None,
    ).await;

    match complex_nested_result {
        Ok(events) => {
            if events.is_empty() {
                println!("   ❌ 查询结果为空：预期应该找到技术类高优先级事件或欧美时区大型活动，但查询返回0个结果");
            } else {
                println!("   ✅ 找到 {} 个匹配条件的事件", events.len());
                for event in &events {
                    let category = if event.event_type.contains("技术") || event.event_type.contains("开发") {
                        "技术类高优先级"
                    } else {
                        "欧美时区大型活动"
                    };
                    println!("   - [{}] {} - {} ({}, {} 人)",
                        category,
                        event.event_name,
                        event.location,
                        event.timezone,
                        event.participant_count);
                }
            }
        },
        Err(e) => println!("   ❌ 查询失败: {}", e),
    }

    println!();

    // 示例6: 选填时间字段查询 - 查找有设置结束时间的事件（排除end_time为null的记录）
    println!("6. 选填时间字段查询: 查找有明确结束时间的事件（排除end_time为null的记录）");

    let has_end_time_condition = QueryConditionGroup::Group {
        operator: LogicalOperator::And,
        conditions: vec![
            // 结束时间不为null的条件
            QueryConditionGroup::Single(QueryCondition {
                field: "end_time".to_string(),
                operator: QueryOperator::IsNotNull,
                value: DataValue::Null, // IsNotNull操作符不需要值，但框架要求提供一个值
            }),
            // 活跃状态条件
            QueryConditionGroup::Single(QueryCondition {
                field: "is_active".to_string(),
                operator: QueryOperator::Eq,
                value: DataValue::Bool(true),
            }),
            // 持续时间至少1小时的事件（结束时间 > 开始时间 + 1小时）
            QueryConditionGroup::Group {
                operator: LogicalOperator::And,
                conditions: vec![
                    QueryConditionGroup::Single(QueryCondition {
                        field: "end_time".to_string(),
                        operator: QueryOperator::Gt,
                        value: DataValue::DateTime(Utc::now() - Duration::days(1)), // 确保结束时间有意义
                    }),
                ],
            },
        ],
    };

    let has_end_time_result = ModelManager::<TimeZoneEvent>::find_with_groups(
        vec![has_end_time_condition],
        None,
    ).await;

    match has_end_time_result {
        Ok(events) => {
            if events.is_empty() {
                println!("   ❌ 查询结果为空：预期应该找到有明确结束时间的活跃事件，但查询返回0个结果");
            } else {
                println!("   ✅ 找到 {} 个有明确结束时间的活跃事件", events.len());
                for event in &events {
                    if let Some(end_time) = event.end_time {
                        let duration = end_time - event.start_time;
                        let duration_hours = duration.num_hours();
                        let duration_minutes = duration.num_minutes() % 60;

                        println!("   - {} ({}) - {} (持续时间: {}小时{}分钟)",
                            event.event_name,
                            event.event_type,
                            event.location,
                            duration_hours,
                            duration_minutes);

                        // 显示具体的时间信息
                        println!("     开始时间: {} UTC", event.start_time.format("%Y-%m-%d %H:%M:%S"));
                        println!("     结束时间: {} UTC", end_time.format("%Y-%m-%d %H:%M:%S"));
                    }
                }
            }
        },
        Err(e) => println!("   ❌ 查询失败: {}", e),
    }

    println!();

    // 额外演示：对比查询 - 查找没有设置结束时间的事件
    println!("6b. 对比查询: 查找没有设置结束时间的事件（end_time为null）");

    let no_end_time_condition = QueryConditionGroup::Group {
        operator: LogicalOperator::And,
        conditions: vec![
            // 结束时间为null的条件
            QueryConditionGroup::Single(QueryCondition {
                field: "end_time".to_string(),
                operator: QueryOperator::IsNull,
                value: DataValue::Null, // IsNull操作符不需要值，但框架要求提供一个值
            }),
            // 活跃状态条件
            QueryConditionGroup::Single(QueryCondition {
                field: "is_active".to_string(),
                operator: QueryOperator::Eq,
                value: DataValue::Bool(true),
            }),
        ],
    };

    let no_end_time_result = ModelManager::<TimeZoneEvent>::find_with_groups(
        vec![no_end_time_condition],
        None,
    ).await;

    match no_end_time_result {
        Ok(events) => {
            if events.is_empty() {
                println!("   ✅ 查询成功，但没有找到没有结束时间的活跃事件（说明所有活跃事件都设置了结束时间）");
            } else {
                println!("   ✅ 找到 {} 个没有设置结束时间的活跃事件", events.len());
                for event in &events {
                    println!("   - {} ({}) - {} (开始时间: {} UTC, 结束时间: 未设置)",
                        event.event_name,
                        event.event_type,
                        event.location,
                        event.start_time.format("%Y-%m-%d %H:%M:%S"));
                }
            }
        },
        Err(e) => println!("   ❌ 查询失败: {}", e),
    }

    println!("\n=== 时区复杂查询示例完成 ===");

    // 关闭连接池
    shutdown().await?;

    Ok(())
}

// 定义时区事件模型
define_model! {
    /// 时区事件模型 - 用于演示timestamptz字段和时区相关查询
    struct TimeZoneEvent {
        id: String,
        event_name: String,
        event_type: String,
        location: String,
        timezone: String,  // 时区标识符
        event_time: chrono::DateTime<chrono::Utc>,  // 主事件时间 (timestamptz)
        start_time: chrono::DateTime<chrono::Utc>,  // 开始时间 (timestamptz)
        end_time: Option<chrono::DateTime<chrono::Utc>>,  // 结束时间 (timestamptz)
        participant_count: i32,
        is_active: bool,
        priority: i32,  // 优先级 1-10
        created_at: chrono::DateTime<chrono::Utc>,
        updated_at: Option<chrono::DateTime<chrono::Utc>>,
    }
    collection = "timezone_events",
    fields = {
        id: string_field(None, None, None).required().unique(),
        event_name: string_field(Some(200), Some(1), None).required(),
        event_type: string_field(Some(50), Some(1), None).required(),
        location: string_field(Some(100), Some(1), None).required(),
        timezone: string_field(Some(50), Some(1), None).required(),
        event_time: datetime_field().required(),
        start_time: datetime_field().required(),
        end_time: datetime_field(),
        participant_count: integer_field(Some(0), Some(10000)).required(),
        is_active: boolean_field().required(),
        priority: integer_field(Some(1), Some(10)).required(),
        created_at: datetime_field().required(),
        updated_at: datetime_field(),
    }
    indexes = [
        { fields: ["event_type"], unique: false, name: "idx_event_type" },
        { fields: ["timezone"], unique: false, name: "idx_timezone" },
        { fields: ["event_time"], unique: false, name: "idx_event_time" },
        { fields: ["start_time"], unique: false, name: "idx_start_time" },
        { fields: ["end_time"], unique: false, name: "idx_end_time" },
        { fields: ["is_active"], unique: false, name: "idx_is_active" },
        { fields: ["priority"], unique: false, name: "idx_priority" },
        { fields: ["event_type", "timezone"], unique: false, name: "idx_type_timezone" },
        { fields: ["event_time", "timezone"], unique: false, name: "idx_time_timezone" },
        { fields: ["is_active", "event_time"], unique: false, name: "idx_active_time" },
        { fields: ["event_type", "is_active", "event_time"], unique: false, name: "idx_type_active_time" },
    ],
}

/// 创建测试集合（现在由模型自动处理）
async fn create_test_table() -> QuickDbResult<()> {
    // 模型会自动创建集合和索引，无需手动操作
    println!("✅ 时区事件集合定义完成（通过模型自动创建）");
    Ok(())
}

/// 插入测试数据
async fn insert_test_data() -> QuickDbResult<()> {
    println!("插入时区事件测试数据...");

    let base_time = Utc::now();
    let test_events = vec![
        create_timezone_event(
            "全球技术峰会",
            "技术会议",
            "线上",
            "UTC",
            base_time + Duration::hours(8),
            base_time + Duration::hours(8),
            Duration::hours(4), // 4小时基础持续时间
            500,
            true,
            8,
        ),
        create_timezone_event(
            "亚洲开发者聚会",
            "技术聚会",
            "上海",
            "Asia/Shanghai",
            base_time + Duration::hours(10),
            base_time + Duration::hours(10),
            Duration::hours(4), // 4小时基础持续时间
            150,
            true,
            7,
        ),
        create_timezone_event(
            "欧洲产品发布会",
            "产品发布",
            "伦敦",
            "Europe/London",
            base_time + Duration::hours(14),
            base_time + Duration::hours(14),
            Duration::hours(2), // 2小时基础持续时间
            200,
            true,
            9,
        ),
        create_timezone_event(
            "美洲用户大会",
            "用户会议",
            "纽约",
            "America/New_York",
            base_time + Duration::hours(18),
            base_time + Duration::hours(18),
            Duration::hours(4), // 4小时基础持续时间
            300,
            true,
            10,
        ),
        create_timezone_event(
            "太平洋区域研讨会",
            "研讨会",
            "东京",
            "Asia/Tokyo",
            base_time + Duration::hours(6),
            base_time + Duration::hours(6),
            Duration::hours(3), // 3小时基础持续时间
            80,
            true,
            6,
        ),
        create_timezone_event(
            "澳洲技术交流会",
            "技术交流",
            "悉尼",
            "Australia/Sydney",
            base_time + Duration::hours(12),
            base_time + Duration::hours(12),
            Duration::hours(3), // 3小时基础持续时间
            60,
            false,  // 已结束的活动
            5,
        ),
        create_timezone_event(
            "中东地区培训",
            "培训",
            "迪拜",
            "Asia/Dubai",
            base_time + Duration::hours(16),
            base_time + Duration::hours(16),
            Duration::hours(3), // 3小时基础持续时间
            40,
            true,
            4,
        ),
        create_timezone_event(
            "印度市场活动",
            "市场活动",
            "班加罗尔",
            "Asia/Kolkata",
            base_time + Duration::hours(11),
            base_time + Duration::hours(11),
            Duration::hours(2), // 2小时基础持续时间
            120,
            true,
            7,
        ),
    ];

    for (i, event_data) in test_events.iter().enumerate() {
        // 从HashMap数据创建TimeZoneEvent结构体实例
        let event = TimeZoneEvent {
            id: String::new(), // 框架会自动生成ID
            event_name: if let Some(DataValue::String(name)) = event_data.get("event_name") {
                name.clone()
            } else {
                "".to_string()
            },
            event_type: if let Some(DataValue::String(event_type)) = event_data.get("event_type") {
                event_type.clone()
            } else {
                "".to_string()
            },
            location: if let Some(DataValue::String(location)) = event_data.get("location") {
                location.clone()
            } else {
                "".to_string()
            },
            timezone: if let Some(DataValue::String(timezone)) = event_data.get("timezone") {
                timezone.clone()
            } else {
                "".to_string()
            },
            event_time: if let Some(DataValue::DateTime(dt)) = event_data.get("event_time") {
                *dt
            } else {
                Utc::now()
            },
            start_time: if let Some(DataValue::DateTime(dt)) = event_data.get("start_time") {
                *dt
            } else {
                Utc::now()
            },
            end_time: if let Some(end_time_data) = event_data.get("end_time") {
                match end_time_data {
                    DataValue::DateTime(dt) => Some(*dt),
                    _ => None,
                }
            } else {
                None
            },
            participant_count: if let Some(DataValue::Int(count)) = event_data.get("participant_count") {
                *count as i32
            } else {
                0
            },
            is_active: if let Some(DataValue::Bool(active)) = event_data.get("is_active") {
                *active
            } else {
                false
            },
            priority: if let Some(DataValue::Int(priority)) = event_data.get("priority") {
                *priority as i32
            } else {
                1
            },
            created_at: if let Some(DataValue::DateTime(dt)) = event_data.get("created_at") {
                *dt
            } else {
                Utc::now()
            },
            updated_at: Some(Utc::now()),
        };

        let result = event.save().await?;
        println!("   创建事件 {}: {} ({})", i + 1, result,
            if let Some(DataValue::String(tz)) = event_data.get("timezone") {
                tz
            } else {
                "Unknown"
            });
    }

    println!("✅ 时区事件测试数据插入完成");
    Ok(())
}

/// 创建时区事件数据的辅助函数
fn create_timezone_event(
    name: &str,
    event_type: &str,
    location: &str,
    timezone: &str,
    event_time: DateTime<Utc>,
    start_time: DateTime<Utc>,
    base_duration: Duration, // 基础持续时间
    participant_count: i32,
    is_active: bool,
    priority: i32,
) -> HashMap<String, DataValue> {
    // 随机决定是否设置结束时间（70%概率设置结束时间，30%概率不设置）
    let mut rng = rand::thread_rng();
    let has_end_time = rng.gen_bool(0.7);

    // 计算结束时间（如果设置的话）
    let end_time = if has_end_time {
        // 随机调整持续时间：基础时间 ± 1小时
        let duration_variation = rng.gen_range(-60..=60); // 分钟变化
        let actual_duration = base_duration + Duration::minutes(duration_variation);
        Some(start_time + actual_duration)
    } else {
        None
    };

    let mut event_data = HashMap::new();
    event_data.insert("event_name".to_string(), DataValue::String(name.to_string()));
    event_data.insert("event_type".to_string(), DataValue::String(event_type.to_string()));
    event_data.insert("location".to_string(), DataValue::String(location.to_string()));
    event_data.insert("timezone".to_string(), DataValue::String(timezone.to_string()));
    event_data.insert("event_time".to_string(), DataValue::DateTime(event_time));
    event_data.insert("start_time".to_string(), DataValue::DateTime(start_time));

    // 只有在有结束时间时才设置end_time字段
    if let Some(end_time) = end_time {
        event_data.insert("end_time".to_string(), DataValue::DateTime(end_time));
        debug!("事件 '{}' 设置了结束时间: {}", name, end_time.format("%Y-%m-%d %H:%M:%S UTC"));
    } else {
        debug!("事件 '{}' 没有设置结束时间", name);
    }

    event_data.insert("participant_count".to_string(), DataValue::Int(participant_count as i64));
    event_data.insert("is_active".to_string(), DataValue::Bool(is_active));
    event_data.insert("priority".to_string(), DataValue::Int(priority as i64));
    event_data.insert("created_at".to_string(), DataValue::DateTime(Utc::now()));
    event_data
}