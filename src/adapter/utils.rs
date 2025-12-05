//! 数据库适配器通用工具模块

/// 获取字段的类型定义
///
/// # 参数
/// * `table_name` - 表名
/// * `alias` - 数据库别名
/// * `field_name` - 字段名
///
/// # 返回值
/// * `Some(FieldType)` - 字段的类型定义
/// * `None` - 无法确定字段类型（表不存在或字段不存在）
pub fn get_field_type(
    table_name: &str,
    alias: &str,
    field_name: &str,
) -> Option<crate::model::FieldType> {
    // 通过全局管理器使用别名获取模型元数据
    if let Some(model_meta) = crate::manager::get_model_with_alias(table_name, alias) {
        model_meta
            .fields
            .get(field_name)
            .map(|field_def| field_def.field_type.clone())
    } else {
        None
    }
}
