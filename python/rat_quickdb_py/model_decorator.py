#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
RAT QuickDB Python 模型装饰器框架

提供类装饰器和元类，支持模型定义方式
"""

import json
from typing import Dict, Any, Optional, Type
from . import register_model, ModelMeta


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

            if result.get("success"):
                print(f"✅ 模型 {cls.__name__} 注册成功")
            else:
                print(f"❌ 模型 {cls.__name__} 注册失败: {result.get('error')}")

        except Exception as e:
            print(f"❌ 注册模型 {cls.__name__} 时发生错误: {e}")

        # 为类添加有用的属性和方法
        cls._model_meta = model_meta_obj
        cls._fields = fields
        cls._table_name = final_table_name
        cls._database_alias = final_database_alias

        # 添加类方法
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

            if result.get("success"):
                print(f"✅ 模型 {name} 注册成功")
            else:
                print(f"❌ 模型 {name} 注册失败: {result.get('error')}")

        except Exception as e:
            print(f"❌ 注册模型 {name} 时发生错误: {e}")

        # 为类添加有用的属性和方法
        cls._model_meta = model_meta_obj
        cls._fields = fields
        cls._table_name = table_name
        cls._database_alias = database_alias

        # 添加类方法
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

        return cls


# 为了更好的命名，提供一个别名
rat_dbmetaclass = RatDbModelMeta

# 添加到__all__以便导出
__all__ = ['rat_dbmodel', 'rat_dbmetaclass']