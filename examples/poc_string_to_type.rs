// POC: 字符串转类型并调用的不同方案验证

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

// 定义一些示例结构体
struct User {
    name: &'static str,
}

struct Order {
    id: u32,
}

struct Product {
    price: f64,
}

// 方案1: 定义一个创建trait
trait CreateTable {
    async fn create_table() -> Result<(), String>;
}

impl CreateTable for User {
    async fn create_table() -> Result<(), String> {
        println!("正在创建User表...");
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        println!("User表创建完成！");
        Ok(())
    }
}

impl CreateTable for Order {
    async fn create_table() -> Result<(), String> {
        println!("正在创建Order表...");
        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
        println!("Order表创建完成！");
        Ok(())
    }
}

impl CreateTable for Product {
    async fn create_table() -> Result<(), String> {
        println!("正在创建Product表...");
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        println!("Product表创建完成！");
        Ok(())
    }
}

// 方案1: 手动匹配分发
async fn create_table_by_name_match(model_name: &str) -> Result<(), String> {
    match model_name {
        "User" => User::create_table().await,
        "Order" => Order::create_table().await,
        "Product" => Product::create_table().await,
        _ => Err(format!("未知的模型: {}", model_name)),
    }
}

// 方案2: 简化版HashMap（避免trait object复杂性）
async fn create_table_by_name_hashmap(model_name: &str) -> Result<(), String> {
    match model_name {
        "User" => User::create_table().await,
        "Order" => Order::create_table().await,
        "Product" => Product::create_table().await,
        _ => Err(format!("未知的模型: {}", model_name)),
    }
}

// 方案3: 使用宏生成分发表
macro_rules! generate_model_registry {
    ($($model:ident),*) => {
        async fn create_table_by_name_macro(model_name: &str) -> Result<(), String> {
            match model_name {
                $(
                    stringify!($model) => $model::create_table().await,
                )*
                _ => Err(format!("未知的模型: {}", model_name)),
            }
        }

        fn get_all_models() -> Vec<&'static str> {
            vec![$(stringify!($model)),*]
        }
    };
}

// 使用宏生成注册表
generate_model_registry!(User, Order, Product);

// 测试函数
async fn test_scheme_1() {
    println!("\n=== 方案1: 手动匹配分发 ===");
    let models = vec!["User", "Order", "Product", "Unknown"];

    for model in models {
        match create_table_by_name_match(model).await {
            Ok(_) => println!("✅ {} 创建成功", model),
            Err(e) => println!("❌ {}", e),
        }
    }
}

async fn test_scheme_2() {
    println!("\n=== 方案2: HashMap工厂函数 ===");
    let models = vec!["User", "Order", "Product", "Unknown"];

    for model in models {
        match create_table_by_name_hashmap(model).await {
            Ok(_) => println!("✅ {} 创建成功", model),
            Err(e) => println!("❌ {}", e),
        }
    }
}

async fn test_scheme_3() {
    println!("\n=== 方案3: 宏生成分发表 ===");

    println!("所有已注册的模型: {:?}", get_all_models());

    let models = vec!["User", "Order", "Product", "Unknown"];

    for model in models {
        match create_table_by_name_macro(model).await {
            Ok(_) => println!("✅ {} 创建成功", model),
            Err(e) => println!("❌ {}", e),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("POC: 字符串转类型并调用的不同方案验证");
    println!("=========================================");

    test_scheme_1().await;
    test_scheme_2().await;
    test_scheme_3().await;

    println!("\n=== 总结 ===");
    println!("方案1: 手动匹配 - 简单直接，但需要手动维护");
    println!("方案2: 简化版匹配 - 实际与方案1相同");
    println!("方案3: 宏生成 - 类型安全，性能好，编译时确定");

    println!("\n推荐: 方案3（宏生成）最适合静态已知的类型列表");

    Ok(())
}
