use serde::{Deserialize, Serialize};

/// 缓存配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// 是否启用缓存
    pub enabled: bool,
    /// 缓存策略
    pub strategy: CacheStrategy,
    /// L1 缓存配置
    pub l1_config: L1CacheConfig,
    /// L2 缓存配置（可选）
    pub l2_config: Option<L2CacheConfig>,
    /// TTL 配置
    pub ttl_config: TtlConfig,
    /// 压缩配置
    pub compression_config: CompressionConfig,
    /// 缓存版本标识，变更此值可清理所有缓存
    #[serde(default = "default_cache_version")]
    pub version: String,
}

/// 默认缓存版本
fn default_cache_version() -> String {
    "v1".to_string()
}

/// 缓存策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CacheStrategy {
    /// LRU（最近最少使用）
    Lru,
    /// LFU（最少使用频率）
    Lfu,
    /// FIFO（先进先出）
    Fifo,
    /// 自定义策略
    Custom(String),
}

/// L1 缓存配置（内存缓存）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L1CacheConfig {
    /// 最大容量（条目数）
    pub max_capacity: usize,
    /// 最大内存使用（字节）
    pub max_memory_mb: usize,
    /// 是否启用统计
    pub enable_stats: bool,
}

/// L2 缓存配置（磁盘缓存）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L2CacheConfig {
    /// 存储路径
    pub storage_path: String,
    /// 最大磁盘使用（MB）
    pub max_disk_mb: usize,
    /// 压缩级别（0-22）
    pub compression_level: i32,
    /// 是否启用 WAL
    pub enable_wal: bool,
    /// 启动时清空缓存目录
    pub clear_on_startup: bool,
}

/// TTL 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtlConfig {
    /// 默认 TTL（秒）
    pub default_ttl_secs: u64,
    /// 最大 TTL（秒）
    pub max_ttl_secs: u64,
    /// TTL 检查间隔（秒）
    pub check_interval_secs: u64,
}

/// 压缩配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionConfig {
    /// 是否启用压缩
    pub enabled: bool,
    /// 压缩算法
    pub algorithm: CompressionAlgorithm,
    /// 压缩阈值（字节）
    pub threshold_bytes: usize,
}

/// 压缩算法
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompressionAlgorithm {
    /// ZSTD 压缩
    Zstd,
    /// LZ4 压缩
    Lz4,
    /// Gzip 压缩
    Gzip,
}
