use crate::types::database_config::{ConnectionConfig, TlsConfig, ZstdConfig};
use std::collections::HashMap;

/// MongoDB 连接构建器
pub struct MongoDbConnectionBuilder {
    host: String,
    port: u16,
    database: String,
    username: Option<String>,
    password: Option<String>,
    auth_source: Option<String>,
    direct_connection: bool,
    tls_config: Option<TlsConfig>,
    zstd_config: Option<ZstdConfig>,
    options: HashMap<String, String>,
}

impl MongoDbConnectionBuilder {
    /// 创建新的MongoDB连接构建器
    pub fn new<H: Into<String>, D: Into<String>>(host: H, port: u16, database: D) -> Self {
        Self {
            host: host.into(),
            port,
            database: database.into(),
            username: None,
            password: None,
            auth_source: None,
            direct_connection: false,
            tls_config: None,
            zstd_config: None,
            options: HashMap::new(),
        }
    }

    /// 设置用户名和密码
    pub fn with_auth<U: Into<String>, P: Into<String>>(mut self, username: U, password: P) -> Self {
        self.username = Some(username.into());
        self.password = Some(password.into());
        self
    }

    /// 设置认证数据库
    pub fn with_auth_source<A: Into<String>>(mut self, auth_source: A) -> Self {
        self.auth_source = Some(auth_source.into());
        self
    }

    /// 启用直接连接
    pub fn with_direct_connection(mut self, direct: bool) -> Self {
        self.direct_connection = direct;
        self
    }

    /// 设置TLS配置
    pub fn with_tls_config(mut self, tls_config: TlsConfig) -> Self {
        self.tls_config = Some(tls_config);
        self
    }

    /// 设置ZSTD压缩配置
    pub fn with_zstd_config(mut self, zstd_config: ZstdConfig) -> Self {
        self.zstd_config = Some(zstd_config);
        self
    }

    /// 添加自定义选项
    pub fn with_option<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.options.insert(key.into(), value.into());
        self
    }

    /// 构建ConnectionConfig::MongoDB
    pub fn build(self) -> ConnectionConfig {
        ConnectionConfig::MongoDB {
            host: self.host,
            port: self.port,
            database: self.database,
            username: self.username,
            password: self.password,
            auth_source: self.auth_source,
            direct_connection: self.direct_connection,
            tls_config: self.tls_config,
            zstd_config: self.zstd_config,
            options: if self.options.is_empty() {
                None
            } else {
                Some(self.options)
            },
        }
    }

    /// 生成MongoDB连接URI（用于内部使用）
    #[doc(hidden)]
    pub fn build_uri(&self) -> String {
        let mut uri = String::from("mongodb://");

        // 添加认证信息
        if let (Some(username), Some(password)) = (&self.username, &self.password) {
            uri.push_str(&urlencoding::encode(username));
            uri.push(':');
            uri.push_str(&urlencoding::encode(password));
            uri.push('@');
        }

        // 添加主机和端口
        uri.push_str(&self.host);
        uri.push(':');
        uri.push_str(&self.port.to_string());

        // 添加数据库
        uri.push('/');
        uri.push_str(&self.database);

        // 构建查询参数
        let mut params = Vec::new();

        if let Some(auth_source) = &self.auth_source {
            params.push(format!("authSource={}", urlencoding::encode(auth_source)));
        }

        if self.direct_connection {
            params.push("directConnection=true".to_string());
        }

        if let Some(tls_config) = &self.tls_config {
            if tls_config.enabled {
                params.push("tls=true".to_string());
            }
        }

        if let Some(zstd_config) = &self.zstd_config {
            if zstd_config.enabled {
                params.push("compressors=zstd".to_string());
            }
        }

        // 添加自定义选项
        for (key, value) in &self.options {
            params.push(format!(
                "{}={}",
                urlencoding::encode(key),
                urlencoding::encode(value)
            ));
        }

        if !params.is_empty() {
            uri.push('?');
            uri.push_str(&params.join("&"));
        }

        uri
    }
}
