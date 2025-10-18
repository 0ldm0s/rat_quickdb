#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
简单测试register_model功能
"""

import sys
import os
sys.path.insert(0, os.path.dirname(__file__))

import rat_quickdb_py as rq
from rat_quickdb_py import create_native_db_queue_bridge
import json

def test_register_model():
    """测试register_model功能"""
    print("🚀 开始测试register_model功能")

    try:
        # 创建数据库桥接器（使用原生数据桥接器）
        bridge = create_native_db_queue_bridge()
        print("✅ 原生数据桥接器创建成功")

        # 初始化日志
        try:
            rq.init_logging_with_level("debug")
            print("✅ 日志初始化成功")
        except:
            print("⚠️ 日志初始化失败")

        # 添加SQLite数据库（使用文件数据库以便检查）
        db_path = "./test_register_debug.db"
        if os.path.exists(db_path):
            os.remove(db_path)

        result = bridge.add_sqlite_database(
            alias="test_sqlite",
            path=db_path,
            max_connections=5,
            min_connections=1,
            connection_timeout=30,
            idle_timeout=600,
            max_lifetime=3600
        )

        # 原生数据桥接器可能仍然返回JSON字符串
        if isinstance(result, str):
            result_data = json.loads(result)
        else:
            result_data = result

        if not result_data.get("success"):
            print(f"❌ SQLite数据库添加失败: {result_data.get('error')}")
            return

        print("✅ SQLite数据库添加成功")

        # 创建简单的字段定义
        # 注意：这里使用位置参数而不是关键字参数
        print("📝 创建字段定义...")

        # 创建整数字段 (required, unique, description) - 匹配AutoIncrement策略
        id_field = rq.integer_field(
            True,           # required
            None,           # min_value
            None,           # max_value
            True,           # unique
            "主键ID"         # description
        )

        name_field = rq.string_field(
            True,           # required
            False,          # unique
            None,           # max_length
            None,           # min_length
            "名称字段"       # description
        )

        # 创建JSON字段
        json_field = rq.json_field(
            False,          # required
            "JSON字段"      # description
        )

        # 创建索引定义
        index_def = rq.IndexDefinition(
            ["id"],         # fields
            True,           # unique
            "idx_id"        # name
        )

        # 创建模型元数据
        table_name = "test_model_register"

        # 创建字段字典
        fields_dict = {
            "id": id_field,
            "name": name_field,
            "json_field": json_field
        }

        model_meta = rq.ModelMeta(
            table_name,
            fields_dict,
            [index_def],
            "test_sqlite",  # database_alias
            "测试模型注册"   # description
        )

        print("✅ 模型元数据创建成功")

        # 注册模型
        print("📝 注册ODM模型...")
        register_result = bridge.register_model(model_meta)
        if isinstance(register_result, str):
            register_data = json.loads(register_result)
        else:
            register_data = register_result

        if register_data.get("success"):
            print("✅ ODM模型注册成功")
            print(f"   消息: {register_data.get('message')}")
        else:
            print(f"❌ ODM模型注册失败: {register_data.get('error')}")
            return

        # 测试数据插入 - 不包含ID，让AutoIncrement策略自动生成
        test_data = {
            "name": "模型注册测试",
            "json_field": {"key": "value", "number": 42}
        }

        print(f"📝 插入测试数据到表 {table_name}...")
        insert_result = bridge.create(table_name, json.dumps(test_data), "test_sqlite")
        if isinstance(insert_result, str):
            insert_data = json.loads(insert_result)
        else:
            insert_data = insert_result

        if insert_data.get("success"):
            print("✅ 数据插入成功")
            generated_id = insert_data.get('data')
            print(f"   数据库生成的ID: {generated_id}")

            # 提取实际的ID值用于查询
            actual_id = None
            if isinstance(generated_id, dict):
                if 'Int' in generated_id:
                    actual_id = generated_id['Int']
                elif 'String' in generated_id:
                    actual_id = generated_id['String']
            else:
                actual_id = str(generated_id)

            print(f"   提取的查询ID: {actual_id}")
        else:
            print(f"❌ 数据插入失败: {insert_data.get('error')}")
            return

        # 查询数据 - 使用数据库实际生成的ID
        print("🔍 查询数据...")
        if actual_id:
            print(f"📋 桥接器类型: {type(bridge)}")
            print(f"📋 查询ID: {actual_id} (类型: {type(actual_id)})")
            query_result = bridge.find_by_id(table_name, str(actual_id), "test_sqlite")
            print(f"📋 查询结果类型: {type(query_result)}")
        else:
            print("❌ 无法提取有效的ID进行查询")
            return
        if isinstance(query_result, str):
            query_data = json.loads(query_result)
        else:
            query_data = query_result

        if query_data.get("success"):
            record = query_data.get("data")
            if record:
                print("✅ 数据查询成功")
                print(f"   完整记录: {record}")
                print(f"   ID字段值: {record.get('id')} (类型: {type(record.get('id')).__name__})")
                print(f"   JSON字段: {record.get('json_field')} (类型: {type(record.get('json_field')).__name__})")
            else:
                print("❌ 查询结果为空")
        else:
            print(f"❌ 数据查询失败: {query_data.get('error')}")

        print("\n🎉 register_model功能测试完成")

    except Exception as e:
        print(f"❌ 测试过程中发生错误: {e}")
        import traceback
        traceback.print_exc()

if __name__ == "__main__":
    test_register_model()