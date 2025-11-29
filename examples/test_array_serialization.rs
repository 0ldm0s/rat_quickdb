// 测试 Array 字段序列化和查询模式

use serde_json;

fn main() {
    println!("=== Array 字段序列化测试 ===\n");

    // 测试不同类型的数组
    let string_array = vec!["apple".to_string(), "banana".to_string()];
    let int_array = vec![1, 2, 3];
    let float_array = vec![1.5, 2.5, 3.5];
    let bool_array = vec![true, false];
    let mixed_array: Vec<serde_json::Value> = vec![
    serde_json::Value::String("apple".to_string()),
    serde_json::Value::Number(serde_json::Number::from(42)),
    serde_json::Value::Bool(true)
];

    println!("字符串数组: {}", serde_json::to_string(&string_array).unwrap());
    println!("整数数组: {}", serde_json::to_string(&int_array).unwrap());
    println!("浮点数组: {}", serde_json::to_string(&float_array).unwrap());
    println!("布尔数组: {}", serde_json::to_string(&bool_array).unwrap());
    println!("混合数组: {}", serde_json::to_string(&mixed_array).unwrap());

    println!("\n=== SQLite LIKE 查询模式测试 ===");
    println!("查找 'apple': LIKE '%\"apple\"%'");
    println!("查找 1: LIKE '%\"1\"%'");
    println!("查找 true: LIKE '%\"true\"%'");

    println!("\n=== 当前存储方案的问题 ===");
    println!("1. 数字和布尔值没有引号包围");
    println!("2. LIKE 查询可能会误匹配（比如查找1会匹配到12）");
    println!("3. 需要统一转为字符串格式解决这些问题");
}