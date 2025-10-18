#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Simple test for register_model functionality without Unicode characters
"""

import sys
import os
sys.path.insert(0, os.path.dirname(__file__))

import rat_quickdb_py as rq
import json

def test_register_model():
    """Test register_model functionality"""
    print("Starting register_model test")

    try:
        # 使用Python框架层的原生数据桥接器
        bridge = rq.create_native_db_queue_bridge()
        print("Native bridge created successfully")

        # Initialize logging
        try:
            rq.init_logging_with_level("debug")
            print("Logging initialized successfully")
        except:
            print("Logging initialization failed")

        # Add SQLite database
        result = bridge.add_sqlite_database(
            alias="test_sqlite",
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
            print(f"SQLite database addition failed: {result_data.get('error')}")
            return False

        print("SQLite database added successfully")

        # Create field definitions
        print("Creating field definitions...")

        # Create string field
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

        # Create JSON field
        json_field = rq.json_field(
            False,          # required
            "JSON Field"    # description
        )

        # Create index definition
        index_def = rq.IndexDefinition(
            ["id"],         # fields
            True,           # unique
            "idx_id"        # name
        )

        # Create field dictionary
        fields_dict = {
            "id": id_field,
            "name": name_field,
            "json_field": json_field
        }

        # Create model metadata
        table_name = "test_model_register"
        model_meta = rq.ModelMeta(
            table_name,
            fields_dict,
            [index_def],
            "test_sqlite",  # database_alias
            "Test Model Registration" # description
        )

        print("Model metadata created successfully")

        # Register model
        print("Registering ODM model...")
        register_result = bridge.register_model(model_meta)
        register_data = json.loads(register_result)

        if register_data.get("success"):
            print("ODM model registration successful")
            print(f"Message: {register_data.get('message')}")
        else:
            print(f"ODM model registration failed: {register_data.get('error')}")
            return False

        # Test data insertion
        test_data = {
            "id": "test_001",
            "name": "Model Registration Test",
            "json_field": {"key": "value", "number": 42}
        }

        print(f"Inserting test data into table {table_name}...")
        insert_result = bridge.create(table_name, json.dumps(test_data), "test_sqlite")

        if insert_result.get("success"):
            print("Data insertion successful")
            print(f"Returned ID: {insert_result.get('data')}")
        else:
            print(f"Data insertion failed: {insert_result.get('error')}")
            return False

        # Query data using the actual returned ID
        actual_id = insert_result.get('data')
        print(f"Querying data with actual ID: {actual_id}...")

        # First, let's try to find all records to see what's in the table
        print("Attempting to find all records in table...")
        find_all_result = bridge.find(table_name, "[]", "test_sqlite")

        if find_all_result.get("success"):
            records = find_all_result.get("data", [])
            print(f"Found {len(records)} records in table:")
            for i, record in enumerate(records):
                print(f"  Record {i+1}: {record}")
        else:
            print(f"Failed to find all records: {find_all_result.get('error')}")

        # Convert the ID to string for query
        if isinstance(actual_id, dict):
            if 'String' in actual_id:
                query_id = actual_id['String']
            elif 'Int' in actual_id:
                query_id = str(actual_id['Int'])
            else:
                query_id = str(actual_id)
        elif isinstance(actual_id, int):
            query_id = str(actual_id)
        elif isinstance(actual_id, str):
            query_id = actual_id
        else:
            query_id = str(actual_id)

        query_result = bridge.find_by_id(table_name, query_id, "test_sqlite")

        if query_result.get("success"):
            record = query_result.get("data")
            if record:
                print("Data query successful")
                print(f"Record type: {type(record)}")
                print(f"Native Python record: {record}")

                # Check if JSON field is correctly parsed
                json_field_value = record.get('json_field')
                if isinstance(json_field_value, dict):
                    print("JSON field correctly parsed as dict")
                    print(f"json_field: {json_field_value}")
                    return True
                else:
                    print(f"JSON field parsing failed: {type(json_field_value)}")
                    print(f"Value: {json_field_value}")
                    return False
            else:
                print("Query result is empty")
                return False
        else:
            print(f"Data query failed: {query_result.get('error')}")
            return False

    except Exception as e:
        print(f"Error occurred during test: {e}")
        import traceback
        traceback.print_exc()
        return False

if __name__ == "__main__":
    success = test_register_model()
    if success:
        print("\nRegister model test completed successfully!")
        sys.exit(0)
    else:
        print("\nRegister model test failed!")
        sys.exit(1)