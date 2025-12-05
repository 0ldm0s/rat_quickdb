//! 多语言错误消息系统测试示例

use rat_quickdb::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化rat_quickdb，包括多语言错误消息系统
    rat_quickdb::init();

    println!("=== 多语言错误消息系统测试 ===\n");

    // 测试中文错误消息
    println!("1. 中文错误消息测试：");
    unsafe {
        std::env::set_var("RAT_LANG", "zh-CN");
    }
    rat_quickdb::i18n::set_language("zh-CN");

    let error_msg = rat_quickdb::i18n::tf(
        "error.sqlite_connection",
        &[("message", "无法连接到数据库文件")],
    );
    println!("   SQLite连接错误: {}", error_msg);

    let error_msg = rat_quickdb::i18n::tf(
        "error.validation",
        &[("field", "username"), ("message", "用户名不能为空")],
    );
    println!("   验证错误: {}", error_msg);

    let error_msg = rat_quickdb::i18n::tf("error.connection", &[("message", "网络超时")]);
    println!("   连接错误: {}", error_msg);

    println!();

    // 测试英文错误消息
    println!("2. 英文错误消息测试：");
    rat_quickdb::i18n::set_language("en-US");

    let error_msg = rat_quickdb::i18n::tf(
        "error.sqlite_connection",
        &[("message", "Cannot connect to database file")],
    );
    println!("   SQLite connection error: {}", error_msg);

    let error_msg = rat_quickdb::i18n::tf(
        "error.validation",
        &[
            ("field", "username"),
            ("message", "Username cannot be empty"),
        ],
    );
    println!("   Validation error: {}", error_msg);

    let error_msg = rat_quickdb::i18n::tf("error.connection", &[("message", "Network timeout")]);
    println!("   Connection error: {}", error_msg);

    println!();

    // 测试日文错误消息
    println!("3. 日文错误消息测试：");
    rat_quickdb::i18n::set_language("ja-JP");

    let error_msg = rat_quickdb::i18n::tf(
        "error.sqlite_connection",
        &[("message", "データベースファイルに接続できません")],
    );
    println!("   SQLite接続エラー: {}", error_msg);

    let error_msg = rat_quickdb::i18n::tf(
        "error.validation",
        &[
            ("field", "username"),
            ("message", "ユーザー名は空にできません"),
        ],
    );
    println!("   検証エラー: {}", error_msg);

    let error_msg = rat_quickdb::i18n::tf(
        "error.connection",
        &[("message", "ネットワークタイムアウト")],
    );
    println!("   接続エラー: {}", error_msg);

    println!();

    // 测试fallback机制（不存在的语言会fallback到英语）
    println!("4. Fallback机制测试（德语 -> 英语）：");
    rat_quickdb::i18n::set_language("de-DE");

    let error_msg = rat_quickdb::i18n::tf(
        "error.sqlite_connection",
        &[("message", "Cannot connect to database file")],
    );
    println!("   SQLite connection error: {}", error_msg);

    println!();

    // 显示当前语言
    println!("5. 当前语言设置：");
    println!("   当前语言: {}", rat_quickdb::i18n::current_language());

    println!();

    // 测试一些简单的错误消息（无参数）
    println!("6. 简单错误消息测试：");
    rat_quickdb::i18n::set_language("zh-CN");
    println!(
        "   PostgreSQL配置不匹配: {}",
        rat_quickdb::i18n::t("error.postgres_config_mismatch")
    );

    rat_quickdb::i18n::set_language("en-US");
    println!(
        "   PostgreSQL config mismatch: {}",
        rat_quickdb::i18n::t("error.postgres_config_mismatch")
    );

    rat_quickdb::i18n::set_language("ja-JP");
    println!(
        "   PostgreSQL設定不一致: {}",
        rat_quickdb::i18n::t("error.postgres_config_mismatch")
    );

    println!("\n=== 测试完成 ===");

    Ok(())
}
