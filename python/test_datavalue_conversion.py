#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
测试DataValue转换功能
"""

import sys
import os
sys.path.insert(0, os.path.dirname(__file__))

from datetime import datetime, timezone
import uuid

# 导入必要的模块
from rat_quickdb_py import string_field, integer_field, boolean_field, datetime_field, uuid_field, ModelMeta, FieldDefinition

# 创建测试字段
test_string_field = string_field(required=True, description="测试字符串字段")
test_integer_field = integer_field(required=True, description="测试整数字段")
test_boolean_field = boolean_field(required=True, description="测试布尔字段")
test_datetime_field = datetime_field(required=True, description="测试时间字段")
test_uuid_field = uuid_field(required=True, description="测试UUID字段")

# 创建测试数据（包含tags数组字段）
test_data = {
    "id": "test_id",
    "name": "张三",
    "age": 25,
    "is_active": True,
    "created_at": datetime.now(timezone.utc).isoformat(),
    "user_uuid": str(uuid.uuid4()),
    "tags": ["tag1", "tag2", "tag3"]
}

# 创建模拟的模型元数据（添加tags数组字段）
fields = {
    "id": test_string_field,
    "name": test_string_field,
    "age": test_integer_field,
    "is_active": test_boolean_field,
    "created_at": test_datetime_field,
    "user_uuid": test_uuid_field,
    "tags": test_string_field  # 暂时作为字符串字段测试
}

model_meta = ModelMeta(
    collection_name="test_table",
    fields=fields,
    indexes=[],
    database_alias="test",
    description="测试模型"
)

# 测试转换功能
try:
    from rat_quickdb_py.utils import convert_dict_to_datavalue

    print("=== 测试DataValue转换功能 ===")
    print(f"原始数据: {test_data}")

    converted_data = convert_dict_to_datavalue(test_data, model_meta)
    print(f"转换后的带标签数据: {converted_data}")

    # 验证转换结果
    expected_types = {
        "id": {"String": str},
        "name": {"String": str},
        "age": {"Int": int},
        "is_active": {"Bool": bool},
        "created_at": {"String": str},
        "user_uuid": {"String": str},
        "tags": {"Array": list}
    }

    print("\n=== 验证转换结果 ===")
    all_correct = True
    for field, expected in expected_types.items():
        actual = converted_data.get(field)
        if actual:
            type_name = list(actual.keys())[0]
            value = list(actual.values())[0]
            expected_type_name = list(expected.keys())[0]
            expected_type = list(expected.values())[0]

            if type_name == expected_type_name and isinstance(value, expected_type):
                print(f"✅ {field}: {type_name} = {value}")
            else:
                print(f"❌ {field}: 期望 {expected_type_name}({expected_type}), 得到 {type_name}({type(value)})")
                all_correct = False
        else:
            print(f"❌ {field}: 缺少字段")
            all_correct = False

    if all_correct:
        print("\n🎉 所有字段转换正确！")
    else:
        print("\n❌ 部分字段转换失败！")

except Exception as e:
    print(f"❌ 转换过程中发生错误: {e}")
    import traceback
    traceback.print_exc()