#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Python框架层工具模块

提供处理Rust DataValue格式的转换工具和其他Python框架层功能
"""

def convert_datavalue_to_python(value):
    """
    Python框架层：将Rust DataValue格式转换为Python原生类型

    这是标准的数据转换工具，用于处理从Rust ODM层返回的DataValue格式数据。

    转换规则：
    - {"String": "value"} -> "value"
    - {"Int": 42} -> 42
    - {"Float": 3.14} -> 3.14
    - {"Bool": true} -> True
    - {"Null": null} -> None
    - {"Object": {...}} -> {...} (递归转换)
    - {"Array": [...]} -> [...] (递归转换)

    Args:
        value: 从Rust ODM层返回的DataValue格式数据

    Returns:
        转换后的Python原生类型

    Examples:
        >>> convert_datavalue_to_python({"String": "test"})
        'test'
        >>> convert_datavalue_to_python({"Int": 42})
        42
        >>> convert_datavalue_to_python({"Object": {"key": {"String": "value"}}})
        {'key': 'value'}
    """
    if isinstance(value, dict):
        if len(value) == 1:
            # 单一类型DataValue
            for key, val in value.items():
                if key == 'String':
                    return val
                elif key == 'Int':
                    return val
                elif key == 'Float':
                    return val
                elif key == 'Bool':
                    return val
                elif key == 'Object':
                    return convert_datavalue_to_python(val)
                elif key == 'Array':
                    return [convert_datavalue_to_python(item) for item in val]
                elif key == 'Null':
                    return None
                else:
                    # 未知类型，原样返回
                    return val
        else:
            # 复杂对象，递归转换每个字段
            return {k: convert_datavalue_to_python(v) for k, v in value.items()}
    elif isinstance(value, list):
        return [convert_datavalue_to_python(item) for item in value]
    else:
        return value


def convert_python_to_datavalue_with_metadata(value, field_metadata):
    """
    利用表格元数据将Python原生类型转换为Rust DataValue格式

    严格按照元数据进行转换，不做任何类型推断，缺少字段就报错！
    """
    if value is None:
        return {"Null": None}

    # 检查字段定义字符串，直接从字符串中获取类型信息
    metadata_str = str(field_metadata)

    # 直接从字段定义字符串中获取类型信息
    if "field_type:" in metadata_str:
        # 提取类型部分
        type_part = metadata_str.split("field_type:")[1].split(",")[0].strip()
        type_str = type_part

        # 调试输出
        print(f"🔍 字段定义字符串: {metadata_str}")
        print(f"🔍 提取的类型部分: {type_str}")
    else:
        raise ValueError(f"字段缺少类型定义: {field_metadata}")

    # 根据字符串判断类型
    if "Array" in type_str:
        print(f"🔍 检测到Array类型字段")
        # 优先检查Array类型
        if not isinstance(value, list):
            raise ValueError(f"字段期望数组类型，但得到: {type(value)} - {value}")

        # 获取数组项类型 - 直接使用String作为默认类型，因为tags是字符串数组
        converted = []
        for item in value:
            # 对于简单的字符串数组，直接转换为String格式
            if isinstance(item, str):
                converted.append({"String": item})
            elif isinstance(item, int):
                converted.append({"Int": item})
            elif isinstance(item, bool):
                converted.append({"Bool": item})
            elif item is None:
                converted.append({"Null": None})
            else:
                # 其他类型转换为String
                converted.append({"String": str(item)})
        return {"Array": converted}
    elif "String" in type_str:
        return {"String": str(value)}
    elif "Integer" in type_str:
        if not isinstance(value, int):
            raise ValueError(f"字段期望整数类型，但得到: {type(value)} - {value}")
        return {"Int": value}
    elif "Float" in type_str:
        if not isinstance(value, (int, float)):
            raise ValueError(f"字段期望浮点类型，但得到: {type(value)} - {value}")
        return {"Float": float(value)}
    elif "Boolean" in type_str:
        if not isinstance(value, bool):
            raise ValueError(f"字段期望布尔类型，但得到: {type(value)} - {value}")
        return {"Bool": value}
    elif "DateTime" in type_str:
        # DateTime需要转换为专门的DateTime格式
        if isinstance(value, str):
            # 检查是否为有效的ISO格式datetime字符串
            import re
            # ISO 8601格式检测 (如: 2025-10-21T19:59:47.097075+00:00)
            iso_pattern = r'^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?([+-]\d{2}:\d{2}|Z)?$'
            if re.match(iso_pattern, value):
                return {"DateTime": value}
            else:
                raise ValueError(f"字段期望ISO格式datetime字符串，但得到: {value}")
        else:
            raise ValueError(f"字段期望datetime字符串，但得到: {type(value)} - {value}")
    elif "Uuid" in type_str:
        # UUID需要转换为专门的Uuid格式
        if isinstance(value, str):
            # 空字符串让ODM自动生成UUID
            if value == "":
                return {"String": value}

            # 非空时检查UUID格式
            import uuid as uuid_lib
            try:
                # 验证UUID格式
                uuid_lib.UUID(value)
                return {"Uuid": value}
            except ValueError:
                raise ValueError(f"字段期望UUID格式字符串，但得到: {value}")
        else:
            raise ValueError(f"字段期望UUID字符串，但得到: {type(value)} - {value}")
    elif "Json" in type_str:
        if isinstance(value, dict):
            converted = {}
            for k, v in value.items():
                converted[k] = convert_python_to_datavalue_with_metadata(v, field_metadata)
            return {"Object": converted}
        elif isinstance(value, list):
            converted = []
            for item in value:
                converted.append(convert_python_to_datavalue_with_metadata(item, field_metadata))
            return {"Array": converted}
        else:
            return {"String": str(value)}
    elif "Array" in type_str:
        if not isinstance(value, list):
            raise ValueError(f"字段期望数组类型，但得到: {type(value)} - {value}")

        # 获取数组项类型 - 直接使用String作为默认类型，因为tags是字符串数组
        converted = []
        for item in value:
            # 对于简单的字符串数组，直接转换为String格式
            if isinstance(item, str):
                converted.append({"String": item})
            elif isinstance(item, int):
                converted.append({"Int": item})
            elif isinstance(item, bool):
                converted.append({"Bool": item})
            elif item is None:
                converted.append({"Null": None})
            else:
                # 其他类型转换为String
                converted.append({"String": str(item)})
        return {"Array": converted}
    else:
        raise ValueError(f"不支持的字段类型: {type_str}")


def convert_dict_to_datavalue(data_dict, model_meta):
    """
    将整个数据字典转换为带标签的DataValue格式

    严格使用元数据，缺少字段或类型不匹配就报错！
    """
    converted_dict = {}
    fields = getattr(model_meta, 'fields', {})

    for key, value in data_dict.items():
        field_metadata = fields.get(key)
        if not field_metadata:
            raise ValueError(f"数据中包含未定义的字段: {key}")

        converted_dict[key] = convert_python_to_datavalue_with_metadata(value, field_metadata)

    return converted_dict