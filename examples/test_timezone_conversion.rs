//! 简单的时区转换测试POC

use chrono::{DateTime, Utc};

fn main() {
    println!("=== 时区转换测试 ===");

    // 测试输入
    let dt_str = "2024-01-15T09:00:00+08:00";

    println!("原始字符串: {}", dt_str);

    // 解析RFC3339格式
    let parsed_dt = DateTime::parse_from_rfc3339(dt_str).unwrap();
    println!("解析后的DateTime: {}", parsed_dt);

    // 转换为UTC
    let utc_dt = parsed_dt.with_timezone(&Utc);
    println!("UTC时间: {}", utc_dt);

    // 获取UTC时间的时区偏移
    let utc_offset = utc_dt.format("%:z").to_string();
    println!("UTC时区格式: {}", utc_offset);

    // 获取原始时间的时区偏移
    let original_offset = parsed_dt.format("%:z").to_string();
    println!("原始时区格式: {}", original_offset);

    // 测试错误信息中显示的时区
    println!("\n问题分析:");
    println!("- 字符串包含时区: {}", original_offset);
    println!("- 字段定义时区: +08:00");
    println!("- 是否匹配: {}", original_offset == "+08:00");
}
