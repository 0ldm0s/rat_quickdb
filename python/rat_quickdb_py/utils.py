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