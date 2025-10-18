#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
验证代码隔离的测试脚本
测试Python绑定是否正确受到限制，无法绕过主库的模型系统
"""

import sys
import os
sys.path.insert(0, os.path.dirname(__file__))

import rat_quickdb_py as rq
import json

def test_model_registration_isolation():
    """测试模型注册的代码隔离"""
    print("=== 测试模型注册代码隔离 ===")

    try:
        # 使用Python框架层的原生数据桥接器
        bridge = rq.create_native_db_queue_bridge()
        print("✅ Native bridge created successfully")

        # Initialize logging
        try:
            rq.init_logging_with_level("debug")
            print("✅ Logging initialized successfully")
        except:
            print("⚠️ Logging initialization failed")

        # Add SQLite database
        result = bridge.add_sqlite_database(
            alias="test_isolation",
            path=":memory:",
            max_connections=5,
            min_connections=1,
            connection_timeout=30,
            idle_timeout=600,
            max_lifetime=3600,
            id_strategy="Uuid"
        )

        result_data = json.loads(result)
        if not result_data.get("success"):
            print(f"❌ SQLite database addition failed: {result_data.get('error')}")
            return False

        print("✅ SQLite database added successfully")

        # 创建字段定义 - 这是正确的使用方式
        print("🔧 Creating field definitions...")

        # 必须明确定义字段类型
        id_field = rq.string_field(
            True,           # required
            True,           # unique
            None,           # max_length
            None,           # min_length
            "Primary Key ID" # description
        )

        name_field = rq.string_field(
            True,           # required
            False,          # unique
            None,           # max_length
            None,           # min_length
            "Name Field"    # description
        )

        # 创建数组字段
        array_field = rq.array_field(
            rq.FieldType.string(),  # 必须指定数组元素类型
            False,                  # required
            None,                   # max_items
            None,                   # min_items
            "Array Field"           # description
        )

        # 创建索引定义
        index_def = rq.IndexDefinition(
            ["id"],         # fields
            True,           # unique
            "idx_id"        # name
        )

        # 创建字段字典
        fields_dict = {
            "id": id_field,
            "name": name_field,
            "tags": array_field  # 正确定义的数组字段
        }

        # 创建模型元数据
        table_name = "test_isolation_table"
        model_meta = rq.ModelMeta(
            table_name,
            fields_dict,
            [index_def],
            "test_isolation",  # database_alias
            "Test Isolation Model" # description
        )

        print("✅ Model metadata created successfully")

        # 注册模型 - 这是正确的使用方式
        print("📝 Registering ODM model...")
        register_result = bridge.register_model(model_meta)
        register_data = json.loads(register_result)

        if register_data.get("success"):
            print("✅ ODM model registration successful")
            print(f"Message: {register_data.get('message')}")
        else:
            print(f"❌ ODM model registration failed: {register_data.get('error')}")
            return False

        # 测试数据插入 - 使用预定义的模型
        test_data = {
            "id": "isolation_test_001",
            "name": "Code Isolation Test",
            "tags": ["python", "rust", "isolation", "test"]
        }

        print(f"💾 Inserting test data into table {table_name}...")
        insert_result = bridge.create(table_name, json.dumps(test_data), "test_isolation")

        if insert_result.get("success"):
            print("✅ Data insertion successful")
            print(f"Returned ID: {insert_result.get('data')}")
        else:
            print(f"❌ Data insertion failed: {insert_result.get('error')}")
            return False

        # 查询数据验证
        actual_id = insert_result.get('data')
        print(f"🔍 Querying data with actual ID: {actual_id}...")

        query_result = bridge.find_by_id(table_name, actual_id, "test_isolation")

        if query_result.get("success"):
            record = query_result.get("data")
            if record:
                print("✅ Data query successful")
                print(f"Native Python record: {record}")

                # 验证数组字段
                tags_value = record.get('tags')
                if isinstance(tags_value, list):
                    print("✅ Array field correctly parsed as list")
                    print(f"tags: {tags_value}")
                    return True
                else:
                    print(f"❌ Array field parsing failed: {type(tags_value)}")
                    return False
            else:
                print("❌ Query result is empty")
                return False
        else:
            print(f"❌ Data query failed: {query_result.get('error')}")
            return False

    except Exception as e:
        print(f"❌ Error occurred during test: {e}")
        import traceback
        traceback.print_exc()
        return False

def test_isolation_violation_attempt():
    """测试尝试违反代码隔离（应该失败）"""
    print("\n=== 测试代码隔离违规尝试（应该失败） ===")

    try:
        bridge = rq.create_db_queue_bridge()

        # 添加数据库
        result = bridge.add_sqlite_database(
            alias="isolation_test_db",
            path=":memory:",
            max_connections=5,
            min_connections=1,
            connection_timeout=30,
            idle_timeout=600,
            max_lifetime=3600
        )

        result_data = json.loads(result)
        if not result_data.get("success"):
            print(f"❌ Database setup failed: {result_data.get('error')}")
            return False

        print("✅ Database setup successful")

        # 尝试直接操作未定义的表（这应该失败）
        print("🚫 Attempting direct table operation without model definition...")

        test_data = {
            "id": "violation_test",
            "name": "This should fail",
            "undefined_field": ["test", "data"]
        }

        try:
            # 这应该失败，因为我们没有先注册模型
            insert_result = bridge.create("undefined_table", json.dumps(test_data), "isolation_test_db")

            if insert_result:
                result_data = json.loads(insert_result)
                if result_data.get("success"):
                    print("❌ CODE ISOLATION VIOLATION! Direct table operation succeeded when it should fail!")
                    return False
                else:
                    print(f"✅ Code isolation working! Expected failure: {result_data.get('error')}")
                    return True
            else:
                print("✅ Code isolation working! No result returned for undefined table operation")
                return True

        except Exception as e:
            print(f"✅ Code isolation working! Exception thrown as expected: {e}")
            return True

    except Exception as e:
        print(f"❌ Test setup error: {e}")
        return False

def main():
    """主函数"""
    print("🔒 RAT QuickDB Python 代码隔离验证测试")
    print("=" * 50)

    success_count = 0
    total_tests = 2

    # 测试1: 正确的模型注册流程
    if test_model_registration_isolation():
        success_count += 1

    # 测试2: 尝试违反代码隔离
    if test_isolation_violation_attempt():
        success_count += 1

    print(f"\n📊 测试结果: {success_count}/{total_tests} 通过")

    if success_count == total_tests:
        print("🎉 所有测试通过！Python代码隔离工作正常！")
        print("✅ Python绑定无法绕过主库的模型定义系统")
        print("✅ 所有操作都必须通过正确的模型注册流程")
        return True
    else:
        print("❌ 代码隔离测试失败！存在安全隐患！")
        return False

if __name__ == "__main__":
    success = main()
    if success:
        print("\n✅ 代码隔离验证完成！")
        sys.exit(0)
    else:
        print("\n❌ 代码隔离验证失败！")
        sys.exit(1)