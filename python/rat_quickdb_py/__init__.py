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
        "register_model",

        # Python框架层功能
        "convert_datavalue_to_python", "NativeDataBridge", "create_native_db_queue_bridge"
    ]
except ImportError:
    # 如果Rust模块不可用（例如在开发环境中），提供友好的错误信息
    __all__ = []
    import warnings
    warnings.warn(
        "RAT QuickDB Rust扩展未正确加载。请确保已运行 'maturin develop' 或 'pip install -e .'",
        ImportWarning
    )

# Python框架层工具
from .utils import convert_datavalue_to_python

# 自动转换DataValue为Python原生类型的包装器
class NativeDataBridge:
    """
    Python框架层：自动转换DataValue格式的桥接器包装器

    这个类包装了原始的DbQueueBridge，自动将Rust返回的DataValue格式
    转换为Python开发者期望的原生类型。
    """

    def __init__(self, bridge):
        """
        初始化原生数据桥接器

        Args:
            bridge: 原始的DbQueueBridge实例
        """
        self.bridge = bridge

    def _convert_response(self, response_str):
        """
        转换响应中的DataValue格式为Python原生类型

        Args:
            response_str: JSON格式的响应字符串

        Returns:
            转换后的响应字典
        """
        import json
        response = json.loads(response_str)

        if response.get("success") and "data" in response:
            response["data"] = convert_datavalue_to_python(response["data"])

        return response

    def find_by_id(self, table, id, alias=None):
        """
        根据ID查找记录（返回Python原生格式）

        Args:
            table: 表名
            id: 记录ID
            alias: 数据库别名

        Returns:
            Python原生格式的记录数据
        """
        response_str = self.bridge.find_by_id(table, id, alias)
        return self._convert_response(response_str)

    def find(self, table, query_json, alias=None):
        """
        查找记录（返回Python原生格式）

        Args:
            table: 表名
            query_json: 查询条件JSON字符串
            alias: 数据库别名

        Returns:
            Python原生格式的记录列表
        """
        response_str = self.bridge.find(table, query_json, alias)
        return self._convert_response(response_str)

    def create(self, table, data_json, alias=None):
        """
        创建记录（返回Python原生格式）

        Args:
            table: 表名
            data_json: 数据JSON字符串
            alias: 数据库别名

        Returns:
            Python原生格式的创建结果
        """
        response_str = self.bridge.create(table, data_json, alias)
        return self._convert_response(response_str)

    def update(self, table, conditions_json, updates_json, alias=None):
        """
        更新记录（返回Python原生格式）

        Args:
            table: 表名
            conditions_json: 条件JSON字符串
            updates_json: 更新数据JSON字符串
            alias: 数据库别名

        Returns:
            Python原生格式的更新结果
        """
        response_str = self.bridge.update(table, conditions_json, updates_json, alias)
        return self._convert_response(response_str)

    def delete(self, table, conditions_json, alias=None):
        """
        删除记录（返回Python原生格式）

        Args:
            table: 表名
            conditions_json: 条件JSON字符串
            alias: 数据库别名

        Returns:
            Python原生格式的删除结果
        """
        response_str = self.bridge.delete(table, conditions_json, alias)
        return self._convert_response(response_str)

    def count(self, table, conditions_json, alias=None):
        """
        统计记录数量（返回Python原生格式）

        Args:
            table: 表名
            conditions_json: 条件JSON字符串
            alias: 数据库别名

        Returns:
            Python原生格式的统计结果
        """
        response_str = self.bridge.count(table, conditions_json, alias)
        return self._convert_response(response_str)

    # 包装数据库配置方法，确保返回dict格式
    def add_sqlite_database(self, alias, path, create_if_missing=None, max_connections=None,
                           min_connections=None, connection_timeout=None, idle_timeout=None,
                           max_lifetime=None, cache_config=None, id_strategy=None):
        """添加SQLite数据库（返回dict格式）"""
        response_str = self.bridge.add_sqlite_database(alias, path, create_if_missing,
                                                     max_connections, min_connections,
                                                     connection_timeout, idle_timeout,
                                                     max_lifetime, cache_config, id_strategy)
        return self._convert_response(response_str)

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

    def add_mysql_database(self, alias, host, port, database, username, password,
                          max_connections=None, min_connections=None, connection_timeout=None,
                          idle_timeout=None, max_lifetime=None, cache_config=None, id_strategy=None):
        """添加MySQL数据库（返回dict格式）"""
        response_str = self.bridge.add_mysql_database(alias, host, port, database,
                                                     username, password, max_connections,
                                                     min_connections, connection_timeout,
                                                     idle_timeout, max_lifetime, cache_config,
                                                     id_strategy)
        return self._convert_response(response_str)

    def add_mongodb_database(self, alias, host, port, database, username=None, password=None,
                            max_connections=None, min_connections=None, connection_timeout=None,
                            idle_timeout=None, max_lifetime=None, cache_config=None,
                            id_strategy=None, tls_config=None, zstd_config=None):
        """添加MongoDB数据库（返回dict格式）"""
        response_str = self.bridge.add_mongodb_database(alias, host, port, database,
                                                       username, password, max_connections,
                                                       min_connections, connection_timeout,
                                                       idle_timeout, max_lifetime, cache_config,
                                                       id_strategy, tls_config, zstd_config)
        return self._convert_response(response_str)

    # 转发其他方法到原始桥接器
    def __getattr__(self, name):
        """转发未包装的方法到原始桥接器"""
        return getattr(self.bridge, name)


def create_native_db_queue_bridge():
    """
    创建自动转换DataValue格式的数据库桥接器

    Returns:
        NativeDataBridge实例
    """
    return NativeDataBridge(create_db_queue_bridge())


# 模型装饰器
from .model_decorator import rat_dbmodel, rat_dbmetaclass

# 便捷的别名 (仅在成功导入时定义)
try:
    DatabaseBridge = DbQueueBridge
except NameError:
    DatabaseBridge = None