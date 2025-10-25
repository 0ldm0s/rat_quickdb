use serde::{Deserialize, Serialize};

/// ID 生成策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IdStrategy {
    /// 数据库自增 ID（数字）
    AutoIncrement,
    /// UUID v4（字符串）
    Uuid,
    /// 雪花算法 ID（字符串）
    Snowflake {
        /// 机器 ID（0-1023）
        machine_id: u16,
        /// 数据中心 ID（0-31）
        datacenter_id: u8,
    },
    /// MongoDB ObjectId（字符串）
    ObjectId,
    /// 自定义 ID 生成器
    Custom(String),
}

/// ID 类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IdType {
    /// 数字 ID
    Number(i64),
    /// 字符串 ID
    String(String),
}

impl std::fmt::Display for IdType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IdType::Number(n) => write!(f, "{}", n),
            IdType::String(s) => write!(f, "{}", s),
        }
    }
}

impl Default for IdStrategy {
    fn default() -> Self {
        Self::AutoIncrement
    }
}

impl IdStrategy {
    /// 创建 UUID 策略
    pub fn uuid() -> Self {
        Self::Uuid
    }

    /// 创建雪花算法策略
    pub fn snowflake(machine_id: u16, datacenter_id: u8) -> Self {
        Self::Snowflake {
            machine_id,
            datacenter_id,
        }
    }

    /// 创建 ObjectId 策略
    pub fn object_id() -> Self {
        Self::ObjectId
    }

    /// 创建自定义策略
    pub fn custom(generator: String) -> Self {
        Self::Custom(generator)
    }
}

impl From<i64> for IdType {
    fn from(value: i64) -> Self {
        Self::Number(value)
    }
}

impl From<String> for IdType {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<&str> for IdType {
    fn from(value: &str) -> Self {
        Self::String(value.to_string())
    }
}
