#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
RAT QuickDB Python 模型装饰器框架

提供类装饰器和元类，支持模型定义方式
"""

import json
from typing import Dict, Any, Optional, Type
from . import register_model, ModelMeta


class RatQuickDB:
    """
    RAT QuickDB 应用类，类似Flask应用模式

    使用方式:
        app = RatQuickDB()

        @app.model
        class User:
            username = rq.string_field(True, True, None, None, "用户名")
            # ...

        # 添加数据库后自动注册所有模型
        app.add_sqlite_database(...)
    """

    def __init__(self):
        self.models = []  # 延迟注册的模型
        self.database_aliases = set()  # 已配置的数据库别名
        self.bridge = None

    def model(self, table_name: Optional[str] = None,
             database_alias: str = "default",
             description: str = "",
             enable_cache: bool = True,
             cache_ttl: int = 300):
        """
        模型装饰器，延迟注册直到数据库配置完成

        Args:
            table_name: 表名，默认为类名的小写
            database_alias: 数据库别名
            description: 模型描述
            enable_cache: 是否启用缓存
            cache_ttl: 缓存TTL

        Returns:
            装饰器函数
        """
        def decorator(cls: Type) -> Type:
            # 收集字段定义
            fields = {}

            # 获取所有类属性
            for name, value in cls.__dict__.items():
                if name.startswith('_'):  # 跳过私有属性
                    continue

                # 检查是否是字段定义
                if hasattr(value, '__class__') and value.__class__.__name__ == 'FieldDefinition':
                    fields[name] = value
                elif hasattr(value, '__class__') and value.__class__.__name__ == 'PyFieldDefinition':
                    fields[name] = value

            # 收集索引定义和处理Meta类配置
            indexes = []
            meta_class = getattr(cls, 'Meta', None)
            final_table_name = table_name or cls.__name__.lower()
            final_database_alias = database_alias
            final_description = description

            if meta_class:
                indexes = getattr(meta_class, 'indexes', [])

                # 从Meta类获取其他配置
                table_name_from_meta = getattr(meta_class, 'table_name', None)
                if table_name_from_meta:
                    final_table_name = table_name_from_meta

                database_alias_from_meta = getattr(meta_class, 'database_alias', None)
                if database_alias_from_meta:
                    final_database_alias = database_alias_from_meta

                description_from_meta = getattr(meta_class, 'description', None)
                if description_from_meta:
                    final_description = description_from_meta

            # 转换字段为正确格式
            fields_dict = {}
            for field_name, field_def in fields.items():
                if hasattr(field_def, '__class__') and field_def.__class__.__name__ == 'FieldDefinition':
                    fields_dict[field_name] = field_def
                else:
                    # 创建基本的字段定义
                    from . import string_field
                    basic_field = string_field(required=True)
                    fields_dict[field_name] = basic_field

            # 转换索引为正确格式
            indexes_list = []
            for index in indexes:
                if isinstance(index, dict):
                    # 需要创建IndexDefinition对象
                    from . import IndexDefinition
                    fields_list = index.get('fields', [])
                    unique = index.get('unique', False)
                    index_name = index.get('index_name', f"idx_{'_'.join(fields_list)}")
                    index_def = IndexDefinition(fields_list, unique, index_name)
                    indexes_list.append(index_def)
                elif hasattr(index, 'fields') and hasattr(index, 'unique') and hasattr(index, 'name'):
                    # 直接是IndexDefinition对象，直接添加
                    indexes_list.append(index)
                elif hasattr(index, 'to_dict'):
                    indexes_list.append(index)

            # 创建ModelMeta对象
            model_meta_obj = ModelMeta(
                collection_name=final_table_name,
                fields=fields_dict,
                indexes=indexes_list,
                database_alias=final_database_alias,
                description=final_description or f"{cls.__name__}模型",
            )

            # 为类添加有用的属性和方法
            cls._model_meta = model_meta_obj
            cls._fields = fields
            cls._table_name = final_table_name
            cls._database_alias = final_database_alias

            # 添加基础类方法
            @classmethod
            def get_table_name(cls):
                return cls._table_name

            @classmethod
            def get_fields(cls):
                return cls._fields

            @classmethod
            def get_model_meta(cls):
                return cls._model_meta

            cls.get_table_name = get_table_name
            cls.get_fields = get_fields
            cls.get_model_meta = get_model_meta

            # 添加类似主库的find()、create()等方法
            cls = add_model_find_methods(cls)

            # 将模型添加到延迟注册列表
            self.models.append((cls.__name__, model_meta_obj))

            print(f"📋 模型 {cls.__name__} 已准备就绪，等待数据库配置后注册")

            return cls

        return decorator

    def register_model(self, model_meta_obj, model_name: str = None):
        """注册单个模型"""
        try:
            # 调试：打印模型元数据
            if model_name:
                print(f"🔍 注册模型 {model_name} 的元数据:")
                print(f"   表名: {model_meta_obj.collection_name}")
                print(f"   字段数量: {len(model_meta_obj.fields)}")
                print(f"   索引数量: {len(model_meta_obj.indexes)}")
                for i, idx in enumerate(model_meta_obj.indexes):
                    print(f"   索引{i+1}: 字段={idx.fields}, 唯一={idx.unique}, 名称={idx.name}")

            response = register_model(model_meta_obj)
            result = json.loads(response)

            if not result.get("success"):
                print(f"❌ 模型 {model_name or 'Unknown'} 注册失败: {result.get('error')}")
                import sys
                sys.exit(1)
            else:
                print(f"✅ 模型 {model_name or 'Unknown'} 注册成功")
                return True

        except Exception as e:
            print(f"❌ 注册模型 {model_name or 'Unknown'} 时发生错误: {e}")
            import sys
            sys.exit(1)

    def register_all_models(self):
        """注册所有延迟的模型"""
        print(f"🔧 开始注册 {len(self.models)} 个模型...")

        for model_name, model_meta_obj in self.models:
            self.register_model(model_meta_obj, model_name)

        print("✅ 所有模型注册完成")

    def add_sqlite_database(self, *args, **kwargs):
        """添加SQLite数据库并注册模型"""
        from . import create_db_queue_bridge as create_native_db_queue_bridge

        if self.bridge is None:
            self.bridge = create_native_db_queue_bridge()

        result = self.bridge.add_sqlite_database(*args, **kwargs)

        if result.get("success"):
            alias = kwargs.get('alias', 'default')
            self.database_aliases.add(alias)
            print(f"✅ SQLite数据库 '{alias}' 配置成功")

            # 自动注册所有模型
            self.register_all_models()
        else:
            print(f"❌ SQLite数据库配置失败: {result.get('error')}")
            import sys
            sys.exit(1)

        return result

    def add_postgresql_database(self, *args, **kwargs):
        """添加PostgreSQL数据库并注册模型"""
        bridge = self.get_bridge()
        result = bridge.add_postgresql_database(*args, **kwargs)

        if result.get("success"):
            alias = kwargs.get('alias', 'default')
            self.database_aliases.add(alias)
            print(f"✅ PostgreSQL数据库 '{alias}' 配置成功")

            # 自动注册所有模型
            self.register_all_models()
        else:
            print(f"❌ PostgreSQL数据库配置失败: {result.get('error')}")
            import sys
            sys.exit(1)

        return result

    def add_mysql_database(self, *args, **kwargs):
        """添加MySQL数据库并注册模型"""
        from . import create_db_queue_bridge as create_native_db_queue_bridge

        if self.bridge is None:
            self.bridge = create_native_db_queue_bridge()

        result = self.bridge.add_mysql_database(*args, **kwargs)

        if result.get("success"):
            alias = kwargs.get('alias', 'default')
            self.database_aliases.add(alias)
            print(f"✅ MySQL数据库 '{alias}' 配置成功")

            # 自动注册所有模型
            self.register_all_models()
        else:
            print(f"❌ MySQL数据库配置失败: {result.get('error')}")
            import sys
            sys.exit(1)

        return result

    def add_mongodb_database(self, *args, **kwargs):
        """添加MongoDB数据库并注册模型"""
        from . import create_db_queue_bridge as create_native_db_queue_bridge

        if self.bridge is None:
            self.bridge = create_native_db_queue_bridge()

        result = self.bridge.add_mongodb_database(*args, **kwargs)

        if result.get("success"):
            alias = kwargs.get('alias', 'default')
            self.database_aliases.add(alias)
            print(f"✅ MongoDB数据库 '{alias}' 配置成功")

            # 自动注册所有模型
            self.register_all_models()
        else:
            print(f"❌ MongoDB数据库配置失败: {result.get('error')}")
            import sys
            sys.exit(1)

        return result

    def get_bridge(self):
        """获取数据库桥接器"""
        if self.bridge is None:
            from . import create_db_queue_bridge, NativeDataBridge
            raw_bridge = create_db_queue_bridge()
            self.bridge = NativeDataBridge(raw_bridge)
        return self.bridge

    def drop_table(self, table_name: str, alias: str = "default"):
        """删除数据表

        Args:
            table_name: 表名
            alias: 数据库别名，默认为"default"

        Returns:
            删除结果字典
        """
        bridge = self.get_bridge()
        try:
            result = bridge.drop_table(table_name, alias)
            return result
        except Exception as e:
            return {
                "success": False,
                "error": f"删除表失败: {str(e)}"
            }


# 全局应用实例
_app = None

def get_app():
    """获取全局应用实例"""
    global _app
    if _app is None:
        _app = RatQuickDB()
    return _app


# 自动创建全局桥接器并注册到__all__中
from . import create_db_queue_bridge as create_native_db_queue_bridge
from . import NativeDataBridge
_raw_bridge = create_native_db_queue_bridge()
_global_bridge = NativeDataBridge(_raw_bridge)


def add_model_find_methods(cls):
    """为模型类添加类似主库的find()、create()等方法"""

    def find(cls, conditions=None, alias=None):
        """查询记录"""
        import json
        if conditions is None:
            conditions = []
        if alias is None:
            alias = cls._database_alias

        try:
            # 直接传递conditions参数，让NativeDataBridge处理JSON序列化
            response = _global_bridge.find(cls._table_name, conditions, None, None, None, alias)
            return _global_bridge._convert_response(response) if isinstance(response, str) else response
        except RuntimeError as e:
            return {
                "success": False,
                "error": str(e),
                "data": []
            }

    def find_by_id(cls, id, alias=None):
        """根据ID查询记录"""
        if alias is None:
            alias = cls._database_alias

        try:
            response = _global_bridge.find_by_id(cls._table_name, id, alias)
            return _global_bridge._convert_response(response) if isinstance(response, str) else response
        except RuntimeError as e:
            return {
                "success": False,
                "error": str(e),
                "data": None
            }

    def create(cls, data, alias=None):
        """创建记录"""
        import json
        from .utils import convert_dict_to_datavalue

        if alias is None:
            alias = cls._database_alias

        if isinstance(data, dict):
            # 使用模型元数据将Python原生类型转换为带标签的DataValue格式
            model_meta = cls.get_model_meta()
            converted_data = convert_dict_to_datavalue(data, model_meta)
            print(f"🔍 Python端 - 转换前的数据: {data}")
            print(f"🔍 Python端 - 转换后的带标签数据: {converted_data}")
            data_str = json.dumps(converted_data)
            print(f"🔍 Python端 - 发送的JSON字符串: {data_str}")
        else:
            data_str = str(data)

        try:
            response = _global_bridge.create(cls._table_name, data_str, alias)
            return _global_bridge._convert_response(response) if isinstance(response, str) else response
        except (RuntimeError, ValueError) as e:
            return {
                "success": False,
                "error": str(e),
                "data": None
            }

    def update(cls, conditions, updates, alias=None):
        """更新记录"""
        import json
        from .utils import convert_dict_to_datavalue

        if alias is None:
            alias = cls._database_alias

        # 转换conditions和updates数据
        model_meta = cls.get_model_meta()
        converted_conditions = convert_dict_to_datavalue(conditions, model_meta) if isinstance(conditions, dict) else conditions
        converted_updates = convert_dict_to_datavalue(updates, model_meta) if isinstance(updates, dict) else updates

        conditions_str = json.dumps(converted_conditions)
        updates_str = json.dumps(converted_updates)

        try:
            response = _global_bridge.update(cls._table_name, conditions_str, updates_str, alias)
            return _global_bridge._convert_response(response) if isinstance(response, str) else response
        except (RuntimeError, ValueError) as e:
            return {
                "success": False,
                "error": str(e),
                "data": 0
            }

    def delete(cls, conditions, alias=None):
        """删除记录"""
        import json
        if alias is None:
            alias = cls._database_alias

        conditions_str = json.dumps(conditions)

        try:
            response = _global_bridge.delete(cls._table_name, conditions_str, alias)
            return _global_bridge._convert_response(response) if isinstance(response, str) else response
        except RuntimeError as e:
            return {
                "success": False,
                "error": str(e),
                "data": 0
            }

    def count(cls, conditions=None, alias=None):
        """统计记录数量"""
        import json
        if conditions is None:
            conditions = []
        if alias is None:
            alias = cls._database_alias

        conditions_str = json.dumps(conditions)

        try:
            response = _global_bridge.count(cls._table_name, conditions_str, alias)
            return _global_bridge._convert_response(response) if isinstance(response, str) else response
        except RuntimeError as e:
            return {
                "success": False,
                "error": str(e),
                "data": 0
            }

    # 将方法绑定到类
    cls.find = classmethod(find)
    cls.find_by_id = classmethod(find_by_id)
    cls.create = classmethod(create)
    cls.update = classmethod(update)
    cls.delete = classmethod(delete)
    cls.count = classmethod(count)

    return cls


def rat_dbmodel(table_name: Optional[str] = None,
                database_alias: str = "default",
                description: str = "",
                enable_cache: bool = True,
                cache_ttl: int = 300):
    """
    模型装饰器，将Python类转换为RAT QuickDB模型

    Args:
        table_name: 表名，默认为类名的小写
        database_alias: 数据库别名
        description: 模型描述
        enable_cache: 是否启用缓存
        cache_ttl: 缓存TTL

    Returns:
        装饰器函数
    """
    def decorator(cls: Type) -> Type:
        # 收集字段定义
        fields = {}

        # 获取所有类属性
        for name, value in cls.__dict__.items():
            if name.startswith('_'):  # 跳过私有属性
                continue

            # 检查是否是字段定义
            if hasattr(value, '__class__') and value.__class__.__name__ == 'FieldDefinition':
                fields[name] = value
            elif hasattr(value, '__class__') and value.__class__.__name__ == 'PyFieldDefinition':
                fields[name] = value

        # 收集索引定义和处理Meta类配置
        indexes = []
        meta_class = getattr(cls, 'Meta', None)
        final_table_name = table_name or cls.__name__.lower()
        final_database_alias = database_alias
        final_description = description

        if meta_class:
            indexes = getattr(meta_class, 'indexes', [])

            # 从Meta类获取其他配置
            table_name_from_meta = getattr(meta_class, 'table_name', None)
            if table_name_from_meta:
                final_table_name = table_name_from_meta

            database_alias_from_meta = getattr(meta_class, 'database_alias', None)
            if database_alias_from_meta:
                final_database_alias = database_alias_from_meta

            description_from_meta = getattr(meta_class, 'description', None)
            if description_from_meta:
                final_description = description_from_meta

        # 转换字段为正确格式
        fields_dict = {}
        for field_name, field_def in fields.items():
            if hasattr(field_def, '__class__') and field_def.__class__.__name__ == 'FieldDefinition':
                fields_dict[field_name] = field_def
            else:
                # 创建基本的字段定义
                from . import string_field
                basic_field = string_field(required=True)
                fields_dict[field_name] = basic_field

        # 转换索引为正确格式
        indexes_list = []
        for index in indexes:
            if isinstance(index, dict):
                # 需要创建IndexDefinition对象
                from . import IndexDefinition
                fields_list = index.get('fields', [])
                unique = index.get('unique', False)
                index_name = index.get('index_name', f"idx_{'_'.join(fields_list)}")
                index_def = IndexDefinition(fields_list, unique, index_name)
                indexes_list.append(index_def)
            elif hasattr(index, 'fields') and hasattr(index, 'unique') and hasattr(index, 'name'):
                # 直接是IndexDefinition对象，直接添加
                indexes_list.append(index)
            elif hasattr(index, 'to_dict'):
                indexes_list.append(index)

        # 创建ModelMeta对象
        model_meta_obj = ModelMeta(
            collection_name=final_table_name,
            fields=fields_dict,
            indexes=indexes_list,
            database_alias=final_database_alias,
            description=final_description or f"{cls.__name__}模型",
        )

        # 注册模型
        try:
            response = register_model(model_meta_obj)
            result = json.loads(response)

            if not result.get("success"):
                print(f"❌ 模型 {cls.__name__} 注册失败: {result.get('error')}")
                print(f"   提示：数据库别名 '{final_database_alias}' 可能尚未配置")
                import sys
                sys.exit(1)

        except Exception as e:
            print(f"❌ 注册模型 {cls.__name__} 时发生错误: {e}")
            print(f"   提示：数据库别名 '{final_database_alias}' 可能尚未配置")
            import sys
            sys.exit(1)

        # 为类添加有用的属性和方法
        cls._model_meta = model_meta_obj
        cls._fields = fields
        cls._table_name = final_table_name
        cls._database_alias = final_database_alias

        # 添加基础类方法
        @classmethod
        def get_table_name(cls):
            return cls._table_name

        @classmethod
        def get_fields(cls):
            return cls._fields

        @classmethod
        def get_model_meta(cls):
            return cls._model_meta

        cls.get_table_name = get_table_name
        cls.get_fields = get_fields
        cls.get_model_meta = get_model_meta

        # 添加类似主库的find()、create()等方法
        cls = add_model_find_methods(cls)

        return cls

    return decorator


class RatDbModelMeta(type):
    """
    RAT QuickDB 模型元类
    """

    def __new__(mcs, name, bases, namespace):
        # 收集字段定义
        fields = {}

        for attr_name, attr_value in namespace.items():
            if attr_name.startswith('_'):  # 跳过私有属性
                continue

            # 检查是否是字段定义
            if hasattr(attr_value, '__class__') and attr_value.__class__.__name__ == 'FieldDefinition':
                fields[attr_name] = attr_value
            elif hasattr(attr_value, '__class__') and attr_value.__class__.__name__ == 'PyFieldDefinition':
                fields[attr_name] = attr_value

        # 收集Meta类配置
        meta_class = namespace.get('Meta', None)
        table_name = name.lower()
        database_alias = "default"
        description = f"{name}模型"
        indexes = []

        if meta_class:
            table_name = getattr(meta_class, 'table_name', name.lower())
            database_alias = getattr(meta_class, 'database_alias', 'default')
            description = getattr(meta_class, 'description', f"{name}模型")
            indexes = getattr(meta_class, 'indexes', [])

        # 转换字段为正确格式
        fields_dict = {}
        for field_name, field_def in fields.items():
            if hasattr(field_def, '__class__') and field_def.__class__.__name__ == 'FieldDefinition':
                fields_dict[field_name] = field_def
            else:
                # 创建基本的字段定义
                from . import string_field
                basic_field = string_field(required=True)
                fields_dict[field_name] = basic_field

        # 转换索引为正确格式
        indexes_list = []
        for index in indexes:
            if isinstance(index, dict):
                # 需要创建IndexDefinition对象
                from . import IndexDefinition
                fields_list = index.get('fields', [])
                unique = index.get('unique', False)
                index_name = index.get('index_name', f"idx_{'_'.join(fields_list)}")
                index_def = IndexDefinition(fields_list, unique, index_name)
                indexes_list.append(index_def)
            elif hasattr(index, 'fields') and hasattr(index, 'unique') and hasattr(index, 'name'):
                # 直接是IndexDefinition对象，直接添加
                indexes_list.append(index)
            elif hasattr(index, 'to_dict'):
                indexes_list.append(index)

        # 创建ModelMeta对象
        model_meta_obj = ModelMeta(
            collection_name=table_name,
            fields=fields_dict,
            indexes=indexes_list,
            database_alias=database_alias,
            description=description,
        )

        # 创建类
        cls = super().__new__(mcs, name, bases, namespace)

        # 注册模型
        try:
            response = register_model(model_meta_obj)
            result = json.loads(response)

            if not result.get("success"):
                print(f"❌ 模型 {name} 注册失败: {result.get('error')}")
                print(f"   提示：数据库别名 '{database_alias}' 可能尚未配置")
                import sys
                sys.exit(1)

        except Exception as e:
            print(f"❌ 注册模型 {name} 时发生错误: {e}")
            print(f"   提示：数据库别名 '{database_alias}' 可能尚未配置")
            import sys
            sys.exit(1)

        # 为类添加有用的属性和方法
        cls._model_meta = model_meta_obj
        cls._fields = fields
        cls._table_name = table_name
        cls._database_alias = database_alias

        # 添加基础类方法
        @classmethod
        def get_table_name(cls):
            return cls._table_name

        @classmethod
        def get_fields(cls):
            return cls._fields

        @classmethod
        def get_model_meta(cls):
            return cls._model_meta

        cls.get_table_name = get_table_name
        cls.get_fields = get_fields
        cls.get_model_meta = get_model_meta

        # 添加类似主库的find()、create()等方法
        cls = add_model_find_methods(cls)

        return cls


# 为了更好的命名，提供一个别名
rat_dbmetaclass = RatDbModelMeta

# 添加到__all__以便导出
__all__ = ['rat_dbmodel', 'rat_dbmetaclass', 'RatQuickDB', 'get_app', 'add_model_find_methods']