#[cfg(test)]
mod tests {
    use serde_json;

    /// 基础的JSON解析测试
    #[test]
    fn test_basic_json_parsing() {
        println!("🔍 测试基础JSON解析");

        // 模拟Python侧传递的JSON数据
        let python_json = r#"
        {
            "id": "",
            "username": "test_user",
            "email": "test@example.com",
            "created_at": "2025-10-20T13:54:23.695487+00:00",
            "updated_at": "2025-10-20T13:54:23.695487+00:00",
            "last_login": null,
            "age": 25,
            "is_active": true
        }
        "#;

        // 解析JSON
        let json_value: serde_json::Value = serde_json::from_str(python_json)
            .expect("JSON解析失败");

        println!("🔍 原始JSON: {}", serde_json::to_string_pretty(&json_value).unwrap());

        // 检查所有字段
        if let serde_json::Value::Object(obj) = json_value {
            println!("🔍 字段分析:");
            for (field_name, field_value) in &obj {
                println!("  {}: {:?}", field_name, field_value);

                // 特别检查datetime字段
                if field_name.ends_with("_at") {
                    match field_value {
                        serde_json::Value::String(s) => {
                            println!("    (datetime字段检测): {}", s);

                            // 简单的格式检测
                            if s.contains('T') && s.contains('+') {
                                println!("    ✅ ISO格式带时区");
                            } else if s.contains('T') {
                                println!("    ✅ ISO格式无时区");
                            } else if s.len() == 19 {
                                let dash_count = s.chars().filter(|&c| c == '-').count();
                                let colon_count = s.chars().filter(|&c| c == ':').count();
                                if dash_count == 2 && colon_count == 2 {
                                    println!("    ✅ MySQL格式");
                                }
                            } else {
                                println!("    ❓ 未知格式");
                            }
                        },
                        serde_json::Value::Null => {
                            println!("    (datetime字段): null");
                        },
                        _ => {
                            println!("    (datetime字段): 非字符串类型");
                        }
                    }
                }
            }
        }

        println!("✅ 基础JSON测试完成");
    }

    /// 测试字符串处理
    #[test]
    fn test_string_handling() {
        println!("🔍 测试字符串处理");

        let test_strings = vec![
            ("ISO带时区", "2025-10-20T13:54:23.695487+00:00"),
            ("ISO无时区", "2025-10-20T13:54:23"),
            ("MySQL格式", "2025-10-20 13:54:23"),
            ("空字符串", ""),
            ("用户数据", "这是用户数据"),
        ];

        for (name, test_str) in test_strings {
            println!("🔍 {}: {}", name, test_str);

            if test_str.is_empty() {
                println!("    → 空字符串");
            } else if test_str.contains('T') {
                println!("    → 包含'T'，可能是datetime");
            } else if test_str.contains('-') {
                println!("    → 包含'-'，可能是日期");
            } else if test_str.chars().all(|c| c.is_ascii_digit()) {
                println!("    → 纯数字，可能是时间戳");
            } else {
                println!("    → 普通字符串");
            }
        }

        println!("✅ 字符串处理测试完成");
    }
}