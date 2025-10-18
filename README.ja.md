# rat_quickdb

[![Crates.io](https://img.shields.io/crates/v/rat_quickdb.svg)](https://crates.io/crates/rat_quickdb)
[![Documentation](https://docs.rs/rat_quickdb/badge.svg)](https://docs.rs/rat_quickdb)
[![License: LGPL-3.0](https://img.shields.io/badge/License-LGPL--3.0-blue.svg)](https://www.gnu.org/licenses/lgpl-3.0)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://rust-lang.org)
[![Downloads](https://img.shields.io/crates/d/rat_quickdb.svg)](https://crates.io/crates/rat_quickdb)

🚀 SQLite、PostgreSQL、MySQL、MongoDB対応の強力なクロスデータベースODMライブラリ

**🌐 言語バージョン**: [中文](README.md) | [English](README.en.md) | [日本語](README.ja.md)

## ✨ コア機能

- **🎯 自動インデックス作成**: モデル定義に基づいてテーブルとインデックスを自動作成、手動介入不要
- **🗄️ マルチデータベース対応**: SQLite、PostgreSQL、MySQL、MongoDB
- **🔗 統一API**: 異なるデータベースでも一貫したインターフェース
- **🔒 SQLiteブール値互換性**: SQLiteのブール値保存の違いを自動処理、ゼロ設定互換
- **🏊 コネクションプール管理**: 効率的なコネクションプールとロックフリーキューアーキテクチャ
- **⚡ 非同期サポート**: Tokioベースの非同期ランタイム
- **🧠 スマートキャッシュ**: 組み込みキャッシュサポート（rat_memcacheベース）、TTL期限切れとフォールバック機構対応
- **🆔 複数のID生成戦略**: AutoIncrement、UUID、Snowflake、ObjectId、カスタム接頭辞
- **📝 ログ制御**: 呼び出し元による完全なログ初期化制御、ライブラリの自動初期化競合を回避
- **🐍 Pythonバインディング**: オプションのPython APIサポート
- **📋 タスクキュー**: 組み込み非同期タスクキューシステム
- **🔍 型安全性**: 強力な型モデル定義と検証

## 📦 インストール

`Cargo.toml`に依存関係を追加：

```toml
[dependencies]
rat_quickdb = "0.3.2"
```

### 🔧 特性制御

rat_quickdbはCargo機能を使用して異なるデータベースサポートと機能を制御します。デフォルトではコア機能のみが含まれます。使用するデータベースタイプに基づいて機能を有効にする必要があります：

```toml
[dependencies]
rat_quickdb = { version = "0.3.2", features = [
    "sqlite-support",    # SQLiteデータベースサポート
    "postgres-support",  # PostgreSQLデータベースサポート
    "mysql-support",     # MySQLデータベースサポート
    "mongodb-support",   # MongoDBデータベースサポート
] }
```

#### 利用可能な機能

| 機能名 | 説明 | デフォルト |
|--------|------|-----------|
| `sqlite-support` | SQLiteデータベースサポート | ❌ |
| `postgres-support` | PostgreSQLデータベースサポート | ❌ |
| `mysql-support` | MySQLデータベースサポート | ❌ |
| `mongodb-support` | MongoDBデータベースサポート | ❌ |
| `melange-storage` | 非推奨：L2キャッシュ機能はrat_memcacheに組み込まれました | ❌ |
| `python-bindings` | Python APIバインディング | ❌ |
| `full` | すべてのデータベースサポートを有効化 | ❌ |

#### 必要に応じて機能を有効化

**SQLiteのみ**:
```toml
[dependencies]
rat_quickdb = { version = "0.3.2", features = ["sqlite-support"] }
```

**PostgreSQL**:
```toml
[dependencies]
rat_quickdb = { version = "0.3.2", features = ["postgres-support"] }
```

**すべてのデータベース**:
```toml
[dependencies]
rat_quickdb = { version = "0.3.2", features = ["full"] }
```

**L2キャッシュ設定に関する注意事項**:
- L2キャッシュ機能は`rat_memcache`に組み込まれており、追加の機能は不要です
- L2キャッシュにはキャッシュ永続化のためのディスク容量が必要です
- 設定例については以下の「キャッシュ設定」セクションを参照してください

#### サンプルの実行

異なるサンプルでは異なる機能サポートが必要です：

```bash
# 基本モデル定義サンプル
cargo run --example model_definition --features sqlite-support

# 複雑なクエリサンプル
cargo run --example complex_query_demo --features sqlite-support

# ページネーションクエリサンプル
cargo run --example model_pagination_demo --features sqlite-support

# 特殊な型テストサンプル
cargo run --example special_types_test --features sqlite-support

# ID生成戦略サンプル
cargo run --example id_strategy_test --features sqlite-support

# 手動テーブル管理サンプル
cargo run --example manual_table_management --features sqlite-support

# その他のデータベースサンプル
cargo run --example model_definition_mysql --features mysql-support
cargo run --example model_definition_pgsql --features postgres-support
cargo run --example model_definition_mongodb --features mongodb-support
```

## ⚠️ 重要なアーキテクチャに関する注意事項

### ODMレイヤ使用要件 (v0.3.0+)

**v0.3.0から、define_model!マクロを使用してモデルを定義することが必須となりました。普通の構造体を使用したデータベース操作はできなくなりました。**

すべてのデータベース操作は以下の方法を通じて行う必要があります：

1. **推奨：モデルAPIを使用**
```rust
use rat_quickdb::*;
use rat_quickdb::ModelOperations;

// モデルを定義
define_model! {
    struct User {
        id: String,
        username: String,
        email: String,
    }
    // ... フィールド定義
}

// 作成と保存
let user = User {
    id: String::new(), // フレームワークが自動でIDを生成
    username: "張三".to_string(),
    email: "zhangsan@example.com".to_string(),
};
let user_id = user.save().await?;

// クエリ
let found_user = ModelManager::<User>::find_by_id(&user_id).await?;
```

2. **代替案：ODM APIを使用**
```rust
use rat_quickdb::*;

// add_databaseでデータベース設定を追加
let config = DatabaseConfig::builder()
    .db_type(DatabaseType::SQLite)
    .connection(ConnectionConfig::SQLite {
        path: "test.db".to_string(),
        create_if_missing: true,
    })
    .alias("main".to_string())
    .build()?;
add_database(config).await?;

// ODMでデータベース操作
let mut user_data = HashMap::new();
user_data.insert("username".to_string(), DataValue::String("張三".to_string()));
create("users", user_data, Some("main")).await?;
```

3. **禁止されている使用方法**
```rust
// ❌ エラー：コネクションプールマネージャへの直接アクセスは不可
// let pool_manager = get_global_pool_manager();
// let pool = pool_manager.get_connection_pools().get("main");
```

この設計により以下が保証されます：
- **アーキテクチャの完全性**: 統一されたデータアクセス層
- **セキュリティ**: 低レベルのコネクションプール直接操作によるリソースリークを防止
- **一貫性**: すべての操作が同じODMレイヤ処理を通過
- **保守性**: 統一されたエラーハンドリングとログ記録

## 📋 以前のバージョンからのアップグレード

### v0.2.x から v0.3.0 へのアップグレード

v0.3.0は破壊的変更を含むメジャーバージョンです。詳細な[移行ガイド](MIGRATION_GUIDE_0_3_0.md)を参照してください。

**主な変更**：
- ✅ define_model!マクロによるモデル定義を強制
- ✅ 動的テーブル構造推論の「お世話設定」問題を解消
- ✅ より明確なタイプセーフティとフィールド定義を提供
- ✅ 主要なアーキテクチャバグを修正

### v0.3.1 から v0.3.2+ へのアップグレード

**🚨 破壊的変更：コンビニエンス関数は明示的なID戦略を必須化**

v0.3.2から、すべてのデータベース設定コンビニエンス関数（`sqlite_config`、`postgres_config`、`mysql_config`、`mongodb_config`）は、明示的に`id_strategy`パラメータを渡すことが必須となりました。

**変更理由**：
- ハードコードされた「お世話設定」を排除し、ユーザーがID生成戦略を完全に制御できるようにする
- すべてのデータベースが統一して`AutoIncrement`戦略をデフォルト使用
- 異なるデータベースが異なるデフォルト戦略を持つことによる混乱を回避

**API変更**：
```rust
// v0.3.1以前（削除済み）
let config = sqlite_config("sqlite_db", "./test.db", pool_config)?;

// v0.3.2+（新しいAPI）
let config = sqlite_config(
    "sqlite_db",
    "./test.db",
    pool_config,
    Some(IdStrategy::AutoIncrement)  // 明示的に指定必須
)?;
```

**移行ガイド**：
1. **推奨**：より良いタイプセーフティと一貫性のため、ビルダーパターンに移行
```rust
// コンビニエンス関数の代わりにビルダーパターンを使用：
let config = DatabaseConfig::builder()
    .db_type(DatabaseType::SQLite)
    .connection(ConnectionConfig::SQLite {
        path: "./test.db".to_string(),
        create_if_missing: true,
    })
    .pool_config(pool_config)
    .alias("sqlite_db".to_string())
    .id_strategy(IdStrategy::AutoIncrement)
    .build()?;

// PostgreSQLでUUID（PostgreSQL推奨）
let config = DatabaseConfig::builder()
    .db_type(DatabaseType::PostgreSQL)
    .connection(ConnectionConfig::PostgreSQL {
        host: "localhost".to_string(),
        port: 5432,
        database: "mydatabase".to_string(),
        username: "username".to_string(),
        password: "password".to_string(),
    })
    .pool_config(pool_config)
    .alias("postgres_db".to_string())
    .id_strategy(IdStrategy::Uuid)
    .build()?;
```

2. **一時的互換性**：既存コードを一時的に維持する必要がある場合、必須の`IdStrategy`パラメータを追加してくださいが、可能な限り早くビルダーパターンへの移行を計画してください

**影響範囲**：
- データベース設定にコンビニエンス関数を使用するすべてのコード
- `mongodb_config_with_builder`を使用するコード（重複関数を削除）
- 特定のデータベースデフォルトID戦略に依存するアプリケーション

この変更は「お世話設定なし」の設計原則に従い、設定の一貫性とユーザーコントロールを確保します。

## 🚀 クイックスタート

### 基本的な使用方法

`examples/model_definition.rs` を参照して完全なモデル定義と使用方法を確認してください。

### ID生成ストラテジーの例

`examples/id_strategy_test.rs` を参照して異なるID生成ストラテジーの使用方法を確認してください。

### データベースアダプターの例

- **SQLite**: `examples/model_definition.rs` （実行時に `--features sqlite-support` を使用）
- **PostgreSQL**: `examples/model_definition_pgsql.rs`
- **MySQL**: `examples/model_definition_mysql.rs`
- **MongoDB**: `examples/model_definition_mongodb.rs`

### モデル定義（推奨）

`examples/model_definition.rs` を参照して完全なモデル定義、CRUD操作、複雑なクエリの例を確認してください。

### フィールドタイプと検証

`examples/model_definition.rs` に含まれるフィールドタイプ定義と検証の例を参照してください。

### インデックス管理

インデックスはモデル定義に基づいて自動作成されるため、手動管理は不要です。インデックス定義方法については `examples/model_definition.rs` を参照してください。

## 🔒 SQLiteブール値互換性

SQLiteデータベースはブール値を整数（0と1）として保存しますが、これによりserdeの逆シリアル化エラーが発生する可能性があります。rat_quickdbは複数のソリューションを提供します：

### ソリューション1: sqlite_bool_field() - 推奨（ゼロ設定）

```rust
use rat_quickdb::*;

rat_quickdb::define_model! {
    struct User {
        id: Option<i32>,
        username: String,
        is_active: bool,        // 自動SQLite互換
        is_pinned: bool,        // 自動SQLite互換
        is_verified: bool,      // 自動SQLite互換
    }

    collection = "users",
    fields = {
        id: integer_field(None, None),
        username: string_field(Some(50), Some(3), None).required(),
        // sqlite_bool_field()を使用 - SQLiteブール値互換性を自動処理
        is_active: sqlite_bool_field(),
        is_pinned: sqlite_bool_field(),
        is_verified: sqlite_bool_field_with_default(false),
    }
}
```

### ソリューション2: 手動serde属性 + 汎用逆シリアライザー

```rust
use rat_quickdb::*;
use serde::Deserialize;

#[derive(Debug, Serialize, Deserialize)]
struct User {
    id: Option<i32>,
    username: String,

    // 手動で逆シリアライザーを指定
    #[serde(deserialize_with = "rat_quickdb::sqlite_bool::deserialize_bool_from_any")]
    is_active: bool,

    #[serde(deserialize_with = "rat_quickdb::sqlite_bool::deserialize_bool_from_int")]
    is_pinned: bool,
}

rat_quickdb::define_model! {
    struct User {
        id: Option<i32>,
        username: String,
        is_active: bool,
        is_pinned: bool,
    }

    collection = "users",
    fields = {
        id: integer_field(None, None),
        username: string_field(Some(50), Some(3), None).required(),
        // 従来のboolean_field()を使用 - 手動serde属性と組み合わせ
        is_active: boolean_field(),
        is_pinned: boolean_field(),
    }
}
```

### ソリューション3: 従来方式（手動処理が必要）

```rust
// 既存コードの場合、従来のboolean_field()を使用できます
// ただし、データソースのブール値フォーマットが正しいことを確認する必要があります
rat_quickdb::define_model! {
    struct User {
        id: Option<i32>,
        username: String,
        is_active: bool,        // 互換性を手動で処理する必要があります
    }

    collection = "users",
    fields = {
        id: integer_field(None, None),
        username: string_field(Some(50), Some(3), None).required(),
        is_active: boolean_field(),  // 従来方式
    }
}
```

### 逆シリアライザー選択ガイド

- `deserialize_bool_from_any()`: 整数、ブール値、文字列 "true"/"false" をサポート
- `deserialize_bool_from_int()`: 整数とブール値をサポート
- `sqlite_bool_field()`: 最適な逆シリアライザーを自動選択

### 移行ガイド

従来の`boolean_field()`から`sqlite_bool_field()`への移行：

```rust
// 以前（互換性の問題がある可能性）
is_active: boolean_field(),

// 移行後（完全互換）
is_active: sqlite_bool_field(),
```

## 🆔 ID生成戦略

rat_quickdbは複数のID生成戦略をサポートし、異なるシーンのニーズに対応します：

### AutoIncrement（自動増分ID）
```rust
DatabaseConfig::builder()
    .id_strategy(IdStrategy::AutoIncrement)
    .build()?
```

### UUID（ユニバーサル一意識別子）
```rust
DatabaseConfig::builder()
    .id_strategy(IdStrategy::Uuid)
    .build()?
```

### Snowflake（スノーフレークアルゴリズム）
```rust
DatabaseConfig::builder()
    .id_strategy(IdStrategy::Snowflake {
        machine_id: 1,
        datacenter_id: 1
    })
    .build()?
```

### ObjectId（MongoDBスタイル）
```rust
DatabaseConfig::builder()
    .id_strategy(IdStrategy::ObjectId)
    .build()?
```

### Custom（カスタム接頭辞）
```rust
DatabaseConfig::builder()
    .id_strategy(IdStrategy::Custom("user_".to_string()))
    .build()?
```

## 🧠 キャッシュ設定

### 基本キャッシュ設定（L1メモリキャッシュのみ）
```rust
use rat_quickdb::types::{CacheConfig, CacheStrategy, TtlConfig, L1CacheConfig};

let cache_config = CacheConfig {
    enabled: true,
    strategy: CacheStrategy::Lru,
    ttl_config: TtlConfig {
        default_ttl_secs: 300,  // 5分間キャッシュ
        max_ttl_secs: 3600,     // 最大1時間
        check_interval_secs: 60, // チェック間隔
    },
    l1_config: L1CacheConfig {
        max_capacity: 1000,     // 最大1000エントリ
        max_memory_mb: 64,       // 64MBメモリ制限
        enable_stats: true,      // 統計を有効化
    },
    l2_config: None,           // L2ディスクキャッシュなし
    compression_config: CompressionConfig::default(),
    version: "1".to_string(),
};

DatabaseConfig::builder()
    .cache(cache_config)
    .build()?
```

### L1+L2キャッシュ設定（内蔵L2キャッシュサポート）
```rust
use rat_quickdb::types::{CacheConfig, CacheStrategy, TtlConfig, L1CacheConfig, L2CacheConfig};
use std::path::PathBuf;

let cache_config = CacheConfig {
    enabled: true,
    strategy: CacheStrategy::Lru,
    ttl_config: TtlConfig {
        default_ttl_secs: 1800, // 30分間キャッシュ
        max_ttl_secs: 7200,     // 最大2時間
        check_interval_secs: 120, // チェック間隔
    },
    l1_config: L1CacheConfig {
        max_capacity: 5000,     // 最大5000エントリ
        max_memory_mb: 128,      // 128MBメモリ制限
        enable_stats: true,      // 統計を有効化
    },
    l2_config: Some(L2CacheConfig {
        max_size_mb: 1024,      // 1GBディスクキャッシュ
        cache_dir: PathBuf::from("./cache"), // キャッシュディレクトリ
        enable_persistence: true, // 永続化を有効化
        enable_compression: true, // 圧縮を有効化
        cleanup_interval_secs: 300, // クリーンアップ間隔
    }),
    compression_config: CompressionConfig::default(),
    version: "1".to_string(),
};

DatabaseConfig::builder()
    .cache(cache_config)
    .build()?
```

**L2キャッシュ機能に関する注意事項**:
- L2キャッシュ機能は`rat_memcache`に組み込まれており、追加の機能は不要です
- キャッシュデータ保存のためのディスク容量が必要です
- 大量のデータキャッシュや永続化が必要なシーンに適しています
- `CacheConfig`で`l2_config`を設定するだけでL2キャッシュを有効化できます

### キャッシュ統計と管理
```rust
// キャッシュ統計情報を取得
let stats = get_cache_stats("default").await?;
println!("キャッシュヒット率: {:.2}%", stats.hit_rate * 100.0);
println!("キャッシュエントリ数: {}", stats.entries);

// キャッシュをクリア
clear_cache("default").await?;
clear_all_caches().await?;
```

## 📝 ログ制御

rat_quickdbは呼び出し元による完全なログ初期化制御を提供します：

```rust
use rat_logger::{Logger, LoggerBuilder, LevelFilter};

// 呼び出し元がログシステムの初期化を担当
let logger = LoggerBuilder::new()
    .with_level(LevelFilter::Debug)
    .with_file("app.log")
    .build();

logger.init().expect("ログ初期化失敗");

// 次にrat_quickdbを初期化（もはやログを自動初期化しない）
rat_quickdb::init();
```

## 🔧 データベース設定

### 推奨方式：ビルダーパターンの使用

**推奨**：`DatabaseConfig::builder()`パターンを使用して、完全な設定制御とタイプ安全性を確保してください：

```rust
use rat_quickdb::*;
use rat_quickdb::types::{DatabaseType, ConnectionConfig, PoolConfig, IdStrategy};

let pool_config = PoolConfig::builder()
    .max_connections(10)
    .min_connections(2)
    .connection_timeout(5000)
    .idle_timeout(300000)
    .max_lifetime(1800000)
    .build()?;

// SQLite 設定
let sqlite_config = DatabaseConfig::builder()
    .db_type(DatabaseType::SQLite)
    .connection(ConnectionConfig::SQLite {
        path: "./test.db".to_string(),
        create_if_missing: true,
    })
    .pool_config(pool_config.clone())
    .alias("sqlite_db".to_string())
    .id_strategy(IdStrategy::AutoIncrement)  // 推奨戦略
    .build()?;

// PostgreSQL 設定
let postgres_config = DatabaseConfig::builder()
    .db_type(DatabaseType::PostgreSQL)
    .connection(ConnectionConfig::PostgreSQL {
        host: "localhost".to_string(),
        port: 5432,
        database: "mydatabase".to_string(),
        username: "username".to_string(),
        password: "password".to_string(),
    })
    .pool_config(pool_config.clone())
    .alias("postgres_db".to_string())
    .id_strategy(IdStrategy::Uuid)  // PostgreSQLはUUIDを推奨
    .build()?;

// MySQL 設定
let mysql_config = DatabaseConfig::builder()
    .db_type(DatabaseType::MySQL)
    .connection(ConnectionConfig::MySQL {
        host: "localhost".to_string(),
        port: 3306,
        database: "mydatabase".to_string(),
        username: "username".to_string(),
        password: "password".to_string(),
    })
    .pool_config(pool_config.clone())
    .alias("mysql_db".to_string())
    .id_strategy(IdStrategy::AutoIncrement)  // MySQLは自動増分を推奨
    .build()?;

// MongoDB 設定
let mongodb_config = DatabaseConfig::builder()
    .db_type(DatabaseType::MongoDB)
    .connection(ConnectionConfig::MongoDB(
        MongoDbConnectionBuilder::new("localhost", 27017, "mydatabase")
            .with_auth("username", "password")
            .build()
    ))
    .pool_config(pool_config)
    .alias("mongodb_db".to_string())
    .id_strategy(IdStrategy::ObjectId)  // MongoDBはObjectIdを推奨
    .build()?;

// 接続プールマネージャーに追加
add_database(sqlite_config).await?;
add_database(postgres_config).await?;
add_database(mysql_config).await?;
add_database(mongodb_config).await?;
```

### 高度なMongoDB設定

```rust
use rat_quickdb::*;
use rat_quickdb::types::{TlsConfig, ZstdConfig};

let tls_config = TlsConfig {
    enabled: true,
    verify_server_cert: false,
    verify_hostname: false,
    ..Default::default()
};

let zstd_config = ZstdConfig {
    enabled: true,
    compression_level: Some(3),
    compression_threshold: Some(1024),
};

let mongodb_builder = MongoDbConnectionBuilder::new("localhost", 27017, "mydatabase")
    .with_auth("username", "password")
    .with_auth_source("admin")
    .with_direct_connection(true)
    .with_tls_config(tls_config)
    .with_zstd_config(zstd_config);

let advanced_mongodb_config = DatabaseConfig::builder()
    .db_type(DatabaseType::MongoDB)
    .connection(ConnectionConfig::MongoDB(mongodb_builder))
    .pool_config(pool_config)
    .alias("advanced_mongodb".to_string())
    .id_strategy(IdStrategy::ObjectId)
    .build()?;

add_database(advanced_mongodb_config).await?;
```

### 🚨 非推奨：コンビニエンス関数（使用禁止）

> **重要警告**：以下のコンビニエンス関数は非推奨としてマークされており、v0.4.0で削除されます。上記の推奨ビルダーパターンを使用してください。

**非推奨の関数**：
- `sqlite_config()` - 非推奨
- `postgres_config()` - 非推奨
- `mysql_config()` - 非推奨
- `mongodb_config()` - 非推奨
- `mongodb_config_with_builder()` - 非推奨

**非推奨の理由**：
- ❌ APIの一貫性がない：データベースごとに異なる関数パラメータ
- ❌ ハードコードされたデフォルト値：「お世話設定」なしの設計原則に違反
- ❌ 機能制限：高度な設定オプションをサポートできない
- ❌ メンテナンスの困難さ：重複コードがメンテナンスコストを増加

**推奨される代替案**：
- ✅ **ビルダーパターン**：タイプセーフ、設定完了、API統一
- ✅ **完全制御**：ユーザーがすべての設定オプションを完全に制御
- ✅ **拡張性**：すべてのデータベースの高度な機能をサポート
- ✅ **タイプセーフティ**：コンパイル時設定検証

### ID戦略の推奨事項

データベースの特性に基づいて最適なID戦略を選択してください：

| データベース | 推奨 | 代替案 | 説明 |
|------------|------|--------|------|
| **SQLite** | AutoIncrement | ObjectId | AutoIncrementはネイティブサポートで最適なパフォーマンス |
| **PostgreSQL** | UUID | AutoIncrement | UUIDはネイティブサポートでタイプセーフティ |
| **MySQL** | AutoIncrement | ObjectId | AutoIncrementはネイティブサポートで最適なパフォーマンス |
| **MongoDB** | ObjectId | AutoIncrement | ObjectIdはネイティブサポート、MongoDBエコシステム標準 |

**重要注意**：PostgreSQLでUUID戦略を使用する場合、関連テーブルのすべての外部キーフィールドもタイプ一貫性を保つためにUUIDタイプである必要があります。

## 🛠️ コアAPI

### データベース管理
- `init()` - ライブラリを初期化
- `add_database(config)` - データベース設定を追加
- `remove_database(alias)` - データベース設定を削除
- `get_aliases()` - すべてのデータベースエイリアスを取得
- `set_default_alias(alias)` - デフォルトデータベースエイリアスを設定

### モデル操作（推奨）
```rust
// レコードを保存
let user_id = user.save().await?;

// レコードをクエリ
let found_user = ModelManager::<User>::find_by_id(&user_id).await?;
let users = ModelManager::<User>::find(conditions, options).await?;

// レコードを更新
let mut updates = HashMap::new();
updates.insert("username".to_string(), DataValue::String("新しい名前".to_string()));
let updated = user.update(updates).await?;

// レコードを削除
let deleted = user.delete().await?;
```

### ODM操作（低レベル）
- `create(collection, data, alias)` - レコードを作成
- `find_by_id(collection, id, alias)` - IDで検索
- `find(collection, conditions, options, alias)` - レコードをクエリ
- `update(collection, id, data, alias)` - レコードを更新
- `delete(collection, id, alias)` - レコードを削除
- `count(collection, query, alias)` - レコード数をカウント
- `exists(collection, query, alias)` - 存在チェック

## 🏗️ アーキテクチャ機能

rat_quickdbはモダンアーキテクチャ設計を採用：

1. **ロックフリーキューアーキテクチャ**: 直接のデータベース接続ライフサイクル問題を回避
2. **モデル自動登録**: 初回使用時にモデルメタデータを自動登録
3. **自動インデックス管理**: モデル定義に基づいてテーブルとインデックスを自動作成
4. **クロスデータベースアダプタ**: 複数のデータベースタイプをサポートする統一インターフェース
5. **非同期メッセージ処理**: Tokioベースの効率的な非同期処理

## 🔄 ワークフロー

```
アプリケーション層 → モデル操作 → ODM層 → メッセージキュー → コネクションプール → データベース
    ↑                                                       ↓
    └────────────────── 結果返却 ←────────────────────────────┘
```

## 📊 パフォーマンス機能

- **コネクションプール管理**: インテリジェントなコネクション再利用と管理
- **非同期操作**: ノンブロッキングデータベース操作
- **バッチ処理**: バッチ操作最適化をサポート
- **キャッシュ統合**: 組み込みキャッシュでデータベースアクセスを削減
- **圧縮サポート**: MongoDBはZSTD圧縮をサポート

## 🎯 サポートされるフィールドタイプ

- `integer_field` - 整数フィールド（範囲と制約付き）
- `string_field` - 文字列フィールド（長さ制限付き、長い長さを設定してテキストとして使用可能）
- `float_field` - 浮動小数点数フィールド（範囲と精度付き）
- `boolean_field` - ブールフィールド
- `datetime_field` - 日時フィールド
- `uuid_field` - UUIDフィールド
- `json_field` - JSONフィールド
- `array_field` - 配列フィールド
- `list_field` - リストフィールド（array_fieldのエイリアス）
- `dict_field` - 辞書/オブジェクトフィールド（Objectタイプベース）
- `reference_field` - 参照フィールド（外部キー）

## 📝 インデックスサポート

- **ユニークインデックス**: `unique()` 制約
- **複合インデックス**: マルチフィールド組み合わせインデックス
- **通常インデックス**: 基本クエリ最適化インデックス
- **自動作成**: モデル定義に基づいて自動作成
- **クロスデータベース**: すべてのデータベースインデックスタイプをサポート

## 🌟 バージョン情報

**現在のバージョン**: 0.3.2

**サポートRustバージョン**: 1.70+

**重要なアップデート**: v0.3.0はdefine_model!マクロによるモデル定義を強制し、主要なアーキテクチャ問題を修正し、タイプセーフティを向上させます！

## 📄 ライセンス

このプロジェクトは[LGPL-v3](LICENSE)ライセンスの下で提供されています。

## 🤝 コントリビューション

このプロジェクトを改善するためのIssueやPull Requestの提出を歓迎します！

## 📞 お問い合わせ

質問や提案については、以下の方法でお問い合わせください：
- Issue作成: [GitHub Issues](https://github.com/your-repo/rat_quickdb/issues)
- メール: oldmos@gmail.com