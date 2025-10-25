//! # 日志配置构建器模块
//!
//! 提供日志配置的构建器实现，支持链式调用和严格验证

use crate::config::core::{LoggingConfig, LogLevel};
use crate::error::QuickDbError;
use std::path::PathBuf;
use rat_logger::info;

/// 日志配置构建器
#[derive(Debug)]
pub struct LoggingConfigBuilder {
    level: Option<LogLevel>,
    console: Option<bool>,
    file_path: Option<PathBuf>,
    max_file_size: Option<u64>,
    max_files: Option<u32>,
    structured: Option<bool>,
}
impl LoggingConfigBuilder {
    /// 创建新的构建器
    pub fn new() -> Self {
        Self {
            level: None,
            console: None,
            file_path: None,
            max_file_size: None,
            max_files: None,
            structured: None,
        }
    }

    /// 设置日志级别
    /// 
    /// # 参数
    /// 
    /// * `level` - 日志级别
    pub fn level(mut self, level: LogLevel) -> Self {
        self.level = Some(level);
        self
    }

    /// 设置是否输出到控制台
    /// 
    /// # 参数
    /// 
    /// * `console` - 是否输出到控制台
    pub fn console(mut self, console: bool) -> Self {
        self.console = Some(console);
        self
    }

    /// 设置日志文件路径
    /// 
    /// # 参数
    /// 
    /// * `file_path` - 日志文件路径
    pub fn file_path<P: Into<PathBuf>>(mut self, file_path: Option<P>) -> Self {
        self.file_path = file_path.map(|p| p.into());
        self
    }

    /// 设置日志文件最大大小
    /// 
    /// # 参数
    /// 
    /// * `max_file_size` - 日志文件最大大小（字节）
    pub fn max_file_size(mut self, max_file_size: u64) -> Self {
        self.max_file_size = Some(max_file_size);
        self
    }

    /// 设置保留的日志文件数量
    /// 
    /// # 参数
    /// 
    /// * `max_files` - 保留的日志文件数量
    pub fn max_files(mut self, max_files: u32) -> Self {
        self.max_files = Some(max_files);
        self
    }

    /// 设置是否启用结构化日志
    /// 
    /// # 参数
    /// 
    /// * `structured` - 是否启用结构化日志
    pub fn structured(mut self, structured: bool) -> Self {
        self.structured = Some(structured);
        self
    }

    /// 构建日志配置
    /// 
    /// # 错误
    /// 
    /// 如果任何必需的配置项未设置，将返回错误
    pub fn build(self) -> Result<LoggingConfig, QuickDbError> {
        let level = self.level.ok_or_else(|| {
            crate::quick_error!(config, "日志级别必须设置")
        })?;
        
        let console = self.console.ok_or_else(|| {
            crate::quick_error!(config, "控制台输出选项必须设置")
        })?;
        
        let max_file_size = self.max_file_size.ok_or_else(|| {
            crate::quick_error!(config, "日志文件最大大小必须设置")
        })?;
        
        let max_files = self.max_files.ok_or_else(|| {
            crate::quick_error!(config, "保留日志文件数量必须设置")
        })?;
        
        let structured = self.structured.ok_or_else(|| {
            crate::quick_error!(config, "结构化日志选项必须设置")
        })?;

        if max_file_size == 0 {
            return Err(crate::quick_error!(config, "日志文件最大大小不能为零"));
        }

        if max_files == 0 {
            return Err(crate::quick_error!(config, "保留日志文件数量不能为零"));
        }

        info!("创建日志配置: 级别={:?}, 控制台={}, 结构化={}", level, console, structured);

        Ok(LoggingConfig {
            level,
            console,
            file_path: self.file_path,
            max_file_size,
            max_files,
            structured,
        })
    }
}
impl Default for LoggingConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}
