""
"""
rat_quickdb_py - RAT QuickDB Python Bindings

跨数据库ODM库的Python绑定，支持SQLite、PostgreSQL、MySQL、MongoDB的统一接口

Version: 0.3.2
"""

__version__ = "0.3.2"

# 从Rust编译的模块中导入主要类
# 这些类由maturin在构建时自动注册
try:
    from .rat_quickdb_py import (
        # 基础函数
        DbQueueBridge, create_db_queue_bridge,
        init_logging, init_logging_with_level, init_logging_advanced,
        is_logging_initialized,
        log_info, log_error, log_warn, log_debug, log_trace,
        get_version, get_name, get_info,

        # 配置类
        PyCacheConfig, PyL1CacheConfig, PyL2CacheConfig, PyTtlConfig,
        PyCompressionConfig, PyTlsConfig, PyZstdConfig,

        # ODM模型系统类
        FieldType, FieldDefinition, IndexDefinition, ModelMeta,

        # 字段创建函数
        string_field, integer_field, boolean_field, datetime_field,
        uuid_field, reference_field, array_field, json_field,
        list_field, float_field, dict_field,

        # 模型管理函数
        register_model
    )
    __all__ = [
        # 基础函数
        "DbQueueBridge", "create_db_queue_bridge",
        "init_logging", "init_logging_with_level", "init_logging_advanced",
        "is_logging_initialized",
        "log_info", "log_error", "log_warn", "log_debug", "log_trace",
        "get_version", "get_name", "get_info",

        # 配置类
        "PyCacheConfig", "PyL1CacheConfig", "PyL2CacheConfig", "PyTtlConfig",
        "PyCompressionConfig", "PyTlsConfig", "PyZstdConfig",

        # ODM模型系统类
        "FieldType", "FieldDefinition", "IndexDefinition", "ModelMeta",

        # 字段创建函数
        "string_field", "integer_field", "boolean_field", "datetime_field",
        "uuid_field", "reference_field", "array_field", "json_field",
        "list_field", "float_field", "dict_field",

        # 模型管理函数
        "register_model"
    ]
except ImportError:
    # 如果Rust模块不可用（例如在开发环境中），提供友好的错误信息
    __all__ = []
    import warnings
    warnings.warn(
        "RAT QuickDB Rust扩展未正确加载。请确保已运行 'maturin develop' 或 'pip install -e .'",
        ImportWarning
    )

# NativeDataBridge类定义（在成功导入时定义）
if 'DbQueueBridge' in locals():
    class NativeDataBridge:
        """原生数据桥接器，自动处理DataValue到Python类型的转换"""

        def __init__(self, bridge):
            self.bridge = bridge

        def _convert_response(self, response_str):
            """转换响应中的DataValue格式为Python原生类型"""
            import json
            from .utils import convert_datavalue_to_python
            response = json.loads(response_str)

            if response.get("success") and "data" in response:
                response["data"] = convert_datavalue_to_python(response["data"])

            return response

        def add_postgresql_database(self, alias, host, port, database, username, password,
                                  max_connections=None, min_connections=None, connection_timeout=None,
                                  idle_timeout=None, max_lifetime=None, cache_config=None, id_strategy=None):
            """添加PostgreSQL数据库（返回dict格式）"""
            response_str = self.bridge.add_postgresql_database(alias, host, port, database,
                                                             username, password, max_connections,
                                                             min_connections, connection_timeout,
                                                             idle_timeout, max_lifetime, cache_config,
                                                             id_strategy)
            return self._convert_response(response_str)

        def drop_table(self, table, alias=None):
            """删除数据表（返回dict格式）"""
            response_str = self.bridge.drop_table(table, alias)
            return self._convert_response(response_str)

        def create(self, table, data, alias=None):
            """创建记录（返回Python原生格式）"""
            import json
            data_json = json.dumps(data)
            response_str = self.bridge.create(table, data_json, alias)
            return self._convert_response(response_str)

        def find_by_id(self, table, id, alias=None):
            """根据ID查找记录（返回Python原生格式）"""
            response_str = self.bridge.find_by_id(table, id, alias)
            return self._convert_response(response_str)

        def find(self, table, conditions=None, sort=None, limit=None, offset=None, alias=None):
            """查询记录（返回Python原生格式）"""
            # 构建查询对象
            query = {}
            if conditions is not None:
                query["conditions"] = conditions
            if sort is not None:
                query["sort"] = sort
            if limit is not None:
                query["limit"] = limit
            if offset is not None:
                query["offset"] = offset

            import json
            query_json = json.dumps(query) if query else "{}"

            response_str = self.bridge.find(table, query_json, alias)
            return self._convert_response(response_str)

        def update(self, table, conditions, data, alias=None):
            """更新记录（返回Python原生格式）"""
            response_str = self.bridge.update(table, conditions, data, alias)
            return self._convert_response(response_str)

        def delete(self, table, conditions, alias=None):
            """删除记录（返回Python原生格式）"""
            response_str = self.bridge.delete(table, conditions, alias)
            return self._convert_response(response_str)

        def count(self, table, conditions=None, alias=None):
            """计数记录（返回Python原生格式）"""
            response_str = self.bridge.count(table, conditions, alias)
            return self._convert_response(response_str)
else:
    NativeDataBridge = None

# 便捷的别名 (仅在成功导入时定义)
try:
    DatabaseBridge = DbQueueBridge
except NameError:
    DatabaseBridge = None
