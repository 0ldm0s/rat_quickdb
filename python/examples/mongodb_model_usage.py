#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""RAT QuickDB Python MongoDB ODM绑定使用示例

本示例展示了如何使用 RAT QuickDB 的 Python ODM 绑定在 MongoDB 环境下：
- 字段定义和属性访问
- 模型元数据创建
- 索引定义
- MongoDB数据库连接和基本操作
- MongoDB特有的字段类型和索引配置

基于 SQLite 版本的 ODM 使用示例改写为 MongoDB 版本
"""

import json
import time
import os
import signal
import threading
import sys
from datetime import datetime, timezone
from typing import Dict, List, Optional

# 导入优雅关闭机制
from graceful_shutdown import GracefulShutdownMixin, ShutdownConfig, with_graceful_shutdown

# 全局变量用于强制退出机制
shutdown_lock = threading.Lock()
shutdown_timeout = 15  # 强制退出超时时间（秒）
test_instance = None

try:
    import rat_quickdb_py
    from rat_quickdb_py import (
        create_db_queue_bridge,
        get_version,
        get_info,
        get_name,
        # 字段创建函数
        string_field,
        integer_field,
        boolean_field,
        datetime_field,
        uuid_field,
        reference_field,
        array_field,
        json_field,
        # 类型定义
        FieldDefinition,
        IndexDefinition,
        ModelMeta,
        FieldType,
        # 缓存配置
        PyCacheConfig,
        PyL1CacheConfig,
        PyL2CacheConfig,
        PyTtlConfig,
        PyCompressionConfig,
        PyTlsConfig,
        PyZstdConfig,
    )
except ImportError as e:
    print(f"错误：无法导入 rat_quickdb_py 模块: {e}")
    print("请确保已正确安装 rat-quickdb-py 包")
    print("安装命令：maturin develop")
    exit(1)


def force_exit():
    """强制退出函数"""
    print(f"\n优雅关闭超时（{shutdown_timeout}秒），强制退出程序...")
    os._exit(1)


def signal_handler(signum, frame):
    """信号处理器，支持强制退出机制"""
    global test_instance
    
    with shutdown_lock:
        print(f"\n收到信号 {signum}，开始优雅关闭...")
        
        # 启动强制退出定时器
        force_exit_timer = threading.Timer(shutdown_timeout, force_exit)
        force_exit_timer.daemon = True
        force_exit_timer.start()
        
        try:
            if test_instance:
                test_instance.shutdown()
        except Exception as e:
            print(f"优雅关闭过程中出错: {e}")
        finally:
            force_exit_timer.cancel()
            print("优雅关闭完成")
            sys.exit(0)


def demonstrate_field_creation():
    """演示字段创建和属性访问"""
    print("=== MongoDB字段创建和属性访问演示 ===")
    
    # 创建各种类型的字段
    print("\n1. 创建字符串字段（用户名）:")
    username_field = string_field(
        required=True,
        unique=True,
        max_length=50,
        min_length=3,
        description="MongoDB用户名字段，支持唯一索引"
    )
    print(f"  字段类型: StringField")
    print(f"  是否必填: {username_field.is_required}")
    print(f"  是否唯一: {username_field.is_unique}")
    print(f"  是否索引: {username_field.is_indexed}")
    print(f"  字段描述: {username_field.description}")
    
    print("\n2. 创建整数字段（年龄）:")
    age_field = integer_field(
        required=False,
        min_value=0,
        max_value=150,
        description="年龄字段，支持范围查询"
    )
    print(f"  字段类型: IntegerField")
    print(f"  是否必填: {age_field.is_required}")
    print(f"  是否唯一: {age_field.is_unique}")
    print(f"  字段描述: {age_field.description}")
    
    print("\n3. 创建布尔字段（激活状态）:")
    active_field = boolean_field(
        required=True,
        description="用户激活状态字段"
    )
    print(f"  字段类型: BooleanField")
    print(f"  是否必填: {active_field.is_required}")
    print(f"  字段描述: {active_field.description}")
    
    print("\n4. 创建日期时间字段（创建时间）:")
    created_at_field = datetime_field(
        required=True,
        description="创建时间字段，MongoDB ISODate格式"
    )
    print(f"  字段类型: DateTimeField")
    print(f"  是否必填: {created_at_field.is_required}")
    print(f"  字段描述: {created_at_field.description}")
    
    print("\n5. 创建UUID字段（文档ID）:")
    id_field = uuid_field(
        required=True,
        unique=True,
        description="MongoDB文档唯一标识字段"
    )
    print(f"  字段类型: UuidField")
    print(f"  是否必填: {id_field.is_required}")
    print(f"  是否唯一: {id_field.is_unique}")
    print(f"  字段描述: {id_field.description}")
    
    print("\n6. 创建引用字段（关联文档）:")
    author_field = reference_field(
        target_collection="users",
        required=True,
        description="作者引用字段，MongoDB ObjectId引用"
    )
    print(f"  字段类型: ReferenceField")
    print(f"  是否必填: {author_field.is_required}")
    print(f"  字段描述: {author_field.description}")
    
    print("\n7. 创建JSON字段（元数据）:")
    metadata_field = json_field(
        required=False,
        description="MongoDB嵌套文档元数据字段"
    )
    print(f"  字段类型: JsonField")
    print(f"  是否必填: {metadata_field.is_required}")
    print(f"  字段描述: {metadata_field.description}")
    
    print("\n8. 创建数组字段（标签列表）:")
    tags_field = array_field(FieldType.string(), description="标签数组字段，MongoDB原生数组支持")
    print(f"  字段类型: ArrayField")
    print(f"  是否必填: {tags_field.is_required}")
    print(f"  字段描述: {tags_field.description}")
    
    print("\n=== MongoDB 原生数组字段支持演示 ===")
    
    # MongoDB 原生支持的数组字段类型
    # 字符串数组 - MongoDB 原生支持
    tags_array = array_field(
        FieldType.string(),
        description="标签数组 - MongoDB原生数组存储"
    )
    print(f"字符串数组字段: ArrayField(String)")
    
    # 整数数组 - MongoDB 原生支持
    scores_array = array_field(
        FieldType.integer(),
        description="分数数组 - MongoDB原生数组存储"
    )
    print(f"整数数组字段: ArrayField(Integer)")
    
    # 布尔数组 - MongoDB 原生支持
    flags_array = array_field(
        FieldType.boolean(),
        description="标志数组 - MongoDB原生数组存储"
    )
    print(f"布尔数组字段: ArrayField(Boolean)")
    
    # JSON字段示例 - MongoDB 灵活存储
    metadata_json = json_field(
        required=False,
        description="元数据 - MongoDB灵活JSON存储"
    )
    print(f"JSON字段示例: JsonField")
    
    print("\n=== MongoDB 数组字段优势 ===")
    print("1. 原生数组支持，无需序列化")
    print("2. 支持数组元素查询和索引")
    print("3. 支持嵌套文档数组")
    print("4. 支持混合类型数组")
    print("5. 高效的数组操作（$push, $pull, $addToSet等）")
    
    return {
        '_id': id_field,
        'username': username_field,
        'age': age_field,
        'is_active': active_field,
        'created_at': created_at_field,
        'author_id': author_field,
        'metadata': metadata_field,
        'tags': tags_field,
        'tags_array': tags_array,
        'scores_array': scores_array,
        'flags_array': flags_array,
        'metadata_json': metadata_json
    }


def demonstrate_mongodb_index_creation():
    """演示MongoDB索引创建"""
    print("\n=== MongoDB索引创建演示 ===")
    
    # 创建单字段唯一索引
    print("\n1. 创建用户名唯一索引（MongoDB单字段索引）:")
    username_index = IndexDefinition(
        fields=["username"],
        unique=True,
        name="idx_username_unique"
    )
    print(f"  索引字段: {username_index.fields}")
    print(f"  是否唯一: {username_index.unique}")
    print(f"  索引名称: {username_index.name}")
    print(f"  MongoDB索引类型: 单字段唯一索引")
    
    # 创建复合索引
    print("\n2. 创建复合索引（MongoDB复合索引）:")
    compound_index = IndexDefinition(
        fields=["is_active", "created_at"],
        unique=False,
        name="idx_active_created"
    )
    print(f"  索引字段: {compound_index.fields}")
    print(f"  是否唯一: {compound_index.unique}")
    print(f"  索引名称: {compound_index.name}")
    print(f"  MongoDB索引类型: 复合索引，支持高效范围查询")
    
    # 创建时间索引（支持排序）
    print("\n3. 创建创建时间索引（MongoDB时间索引）:")
    created_index = IndexDefinition(
        fields=["created_at"],
        unique=False,
        name="idx_created_at"
    )
    print(f"  索引字段: {created_index.fields}")
    print(f"  是否唯一: {created_index.unique}")
    print(f"  索引名称: {created_index.name}")
    print(f"  MongoDB索引类型: 时间索引，支持时间范围查询和排序")
    
    # 创建数组索引
    print("\n4. 创建标签数组索引（MongoDB多键索引）:")
    tags_index = IndexDefinition(
        fields=["tags"],
        unique=False,
        name="idx_tags_multikey"
    )
    print(f"  索引字段: {tags_index.fields}")
    print(f"  是否唯一: {tags_index.unique}")
    print(f"  索引名称: {tags_index.name}")
    print(f"  MongoDB索引类型: 多键索引，支持数组元素查询")
    
    # 创建文本索引（如果支持）
    print("\n5. 创建文本搜索索引（MongoDB文本索引）:")
    text_index = IndexDefinition(
        fields=["username", "metadata"],
        unique=False,
        name="idx_text_search"
    )
    print(f"  索引字段: {text_index.fields}")
    print(f"  是否唯一: {text_index.unique}")
    print(f"  索引名称: {text_index.name}")
    print(f"  MongoDB索引类型: 文本索引，支持全文搜索")
    
    return [username_index, compound_index, created_index, tags_index, text_index]


def demonstrate_mongodb_model_meta_creation(fields: Dict, indexes: List):
    """演示MongoDB模型元数据创建"""
    print("\n=== MongoDB模型元数据创建演示 ===")
    
    # 创建用户模型元数据
    print("\n1. 创建MongoDB用户模型元数据:")
    user_meta = ModelMeta(
        collection_name="mongodb_users",
        fields=fields,
        indexes=indexes,
        database_alias="mongodb_default",
        description="MongoDB用户信息模型，支持复杂查询和索引"
    )
    
    print(f"  集合名称: {user_meta.collection_name}")
    print(f"  数据库别名: {user_meta.database_alias}")
    print(f"  模型描述: {user_meta.description}")
    print(f"  MongoDB特性: 支持嵌套文档、数组字段、复合索引")
    
    # 访问字段和索引信息
    try:
        fields_info = user_meta.fields
        indexes_info = user_meta.indexes
        print(f"  字段数量: {len(fields_info) if hasattr(fields_info, '__len__') else 'N/A'}")
        print(f"  索引数量: {len(indexes_info) if hasattr(indexes_info, '__len__') else 'N/A'}")
        print(f"  MongoDB集合特点: 无模式约束，动态字段支持")
    except Exception as e:
        print(f"  访问字段/索引信息时出错: {e}")
    
    return user_meta


class MongoDBDemoManager(GracefulShutdownMixin):
    """MongoDB演示管理器，支持优雅关闭"""
    
    def __init__(self):
        super().__init__(ShutdownConfig(
            shutdown_timeout=10,  # 减少关闭超时时间到10秒
            verbose_logging=True,
            auto_cleanup_on_exit=True
        ))
        self.bridge = None
        self.cache_dir = "./mongodb_odm_cache"
        self.add_temp_dir(self.cache_dir)
    
    def cleanup_resources(self):
        """清理MongoDB测试数据（实现 GracefulShutdownMixin 的抽象方法）"""
        print("清理MongoDB测试数据...")
        
        if not self.bridge:
            print("  桥接器不可用，跳过清理")
            return
        
        try:
            # 删除测试文档（添加超时限制）
            import signal
            
            def timeout_handler(signum, frame):
                raise TimeoutError("清理操作超时")
            
            signal.signal(signal.SIGALRM, timeout_handler)
            signal.alarm(5)  # 5秒超时
            
            try:
                delete_conditions = json.dumps([
                    {"field": "_id", "operator": "Eq", "value": "test_connection_doc"}
                ])
                response = self.bridge.delete("odm_test_collection", delete_conditions, "mongodb_default")
                result = json.loads(response)
                if result.get("success"):
                    print("  MongoDB测试文档清理成功")
                else:
                    print(f"  MongoDB测试文档清理失败: {result.get('error')}")
            finally:
                signal.alarm(0)  # 取消超时
                
        except TimeoutError:
            print("  MongoDB测试文档清理超时，跳过")
        except Exception as e:
            print(f"  清理MongoDB测试数据失败: {e}")


def demonstrate_mongodb_database_operations():
    """演示MongoDB数据库操作"""
    print("\n=== MongoDB数据库操作演示 ===")
    
    # 创建演示管理器
    demo_manager = MongoDBDemoManager()
    
    # 创建数据库队列桥接器
    print("\n1. 创建数据库队列桥接器:")
    try:
        bridge = create_db_queue_bridge()
        demo_manager.bridge = bridge
        demo_manager.add_database_connection(bridge)
        print("  队列桥接器创建成功")
    except Exception as e:
        print(f"  队列桥接器创建失败: {e}")
        return None
    
    # 创建缓存配置
    print("\n2. 创建MongoDB缓存配置:")
    try:
        cache_config = PyCacheConfig()
        cache_config.enable()
        cache_config.strategy = "lru"
        
        # L1缓存配置
        l1_config = PyL1CacheConfig(500)  # 最大容量500条记录
        l1_config.max_memory_mb = 50  # 最大内存50MB
        l1_config.enable_stats = True  # 启用统计
        cache_config.l1_config = l1_config
        
        # L2缓存配置
        cache_dir = demo_manager.cache_dir
        os.makedirs(cache_dir, exist_ok=True)
        l2_config = PyL2CacheConfig(cache_dir)
        l2_config.max_disk_mb = 200  # 最大磁盘200MB
        l2_config.compression_level = 6
        l2_config.enable_wal = True
        l2_config.clear_on_startup = False  # 启动时不清空缓存目录
        cache_config.l2_config = l2_config
        
        # TTL配置
        ttl_config = PyTtlConfig(600)  # 默认TTL 10分钟
        ttl_config.max_ttl_secs = 3600  # 最大TTL 1小时
        ttl_config.check_interval_secs = 120  # 检查间隔2分钟
        cache_config.ttl_config = ttl_config
        
        # 压缩配置
        compression_config = PyCompressionConfig("zstd")
        compression_config.enabled = True
        compression_config.threshold_bytes = 512
        cache_config.compression_config = compression_config
        
        print("  MongoDB缓存配置创建成功")
        print(f"    缓存策略: {cache_config.strategy}")
        print(f"    L1缓存容量: {l1_config.max_capacity} 条记录")
        print(f"    L2缓存目录: {cache_dir}")
        
    except Exception as e:
        print(f"  缓存配置创建失败: {e}")
        cache_config = None
    
    # 创建TLS配置
    tls_config = PyTlsConfig()
    tls_config.enable()
    tls_config.ca_cert_path = "/etc/ssl/certs/ca-certificates.crt"
    tls_config.client_cert_path = ""
    tls_config.client_key_path = ""
    
    # 创建ZSTD配置
    zstd_config = PyZstdConfig()
    zstd_config.enable()
    zstd_config.compression_level = 3
    zstd_config.compression_threshold = 1024
    
    # 添加MongoDB数据库
    print("\n3. 添加MongoDB数据库:")
    try:
        response = bridge.add_mongodb_database(
            alias="mongodb_default",
            host="db0.0ldm0s.net",
            port=27017,
            database="testdb",
            username="testdb",
            password="yash2vCiBA&B#h$#i&gb@IGSTh&cP#QC^",
            auth_source="testdb",
            direct_connection=True,
            max_connections=10,
            min_connections=2,
            connection_timeout=5,   # 减少连接超时时间到5秒
            idle_timeout=60,        # 减少空闲超时时间到1分钟
            max_lifetime=300,       # 减少最大生命周期到5分钟
            cache_config=cache_config,
            tls_config=tls_config,
            zstd_config=zstd_config
        )
        result = json.loads(response)
        if result.get("success"):
            print("  MongoDB数据库添加成功")
            print(f"    主机: db0.0ldm0s.net:27017")
            print(f"    数据库: testdb")
            print(f"    TLS: 启用")
            print(f"    ZSTD压缩: 启用（级别3）")
            print(f"    缓存: {'启用' if cache_config else '禁用'}")
        else:
            print(f"  MongoDB数据库添加失败: {result.get('error')}")
            return None
    except Exception as e:
        print(f"  MongoDB数据库添加失败: {e}")
        return None
    
    # 测试MongoDB连接
    print("\n4. 测试MongoDB连接:")
    try:
        # 创建一个测试文档
        test_doc = {
            "_id": "test_connection_doc",
            "test_field": "MongoDB连接测试",
            "created_at": datetime.now(timezone.utc).isoformat(),
            "test_number": 42,
            "test_boolean": True,
            "test_array": ["tag1", "tag2", "tag3"],
            "test_nested": {
                "nested_field": "嵌套文档测试",
                "nested_number": 123
            }
        }
        
        # 创建测试文档
        response = bridge.create("odm_test_collection", json.dumps(test_doc), "mongodb_default")
        result = json.loads(response)
        if result.get("success"):
            print("  MongoDB连接测试成功")
            print(f"    测试文档创建成功: {test_doc['_id']}")
            print(f"    支持嵌套文档: ✓")
            print(f"    支持数组字段: ✓")
            print(f"    支持多种数据类型: ✓")
        else:
            print(f"  MongoDB连接测试失败: {result.get('error')}")
        
        # 查询测试文档
        response = bridge.find_by_id("odm_test_collection", "test_connection_doc", "mongodb_default")
        result = json.loads(response)
        if result.get("success") and result.get("data"):
            print("  MongoDB查询测试成功")
            retrieved_doc = json.loads(result["data"][0]) if result["data"] else {}
            print(f"    查询到的文档ID: {retrieved_doc.get('_id')}")
            print(f"    嵌套文档字段: {retrieved_doc.get('test_nested', {}).get('nested_field')}")
        
    except Exception as e:
        print(f"  MongoDB连接测试失败: {e}")
    
    return bridge, demo_manager


def demonstrate_mongodb_field_builder_pattern():
    """演示MongoDB字段构建器模式"""
    print("\n=== MongoDB字段构建器模式演示 ===")
    
    # 演示MongoDB特有的字段配置
    print("\n1. 创建MongoDB复杂字段配置:")
    
    # 创建一个复杂的邮箱字段（支持MongoDB文本索引）
    email_field = string_field(
        required=True,
        unique=True,
        max_length=255,
        min_length=5,
        description="邮箱地址字段，MongoDB唯一索引，支持文本搜索"
    )
    
    print(f"  邮箱字段配置:")
    print(f"    必填: {email_field.is_required}")
    print(f"    唯一: {email_field.is_unique}")
    print(f"    索引: {email_field.is_indexed}")
    print(f"    描述: {email_field.description}")
    print(f"    MongoDB特性: 支持正则表达式查询")
    
    # 创建一个带范围限制的分数字段（支持MongoDB数值索引）
    score_field = integer_field(
        required=True,
        min_value=0,
        max_value=100,
        description="分数字段，MongoDB数值索引，支持范围查询"
    )
    
    print(f"\n  分数字段配置:")
    print(f"    必填: {score_field.is_required}")
    print(f"    唯一: {score_field.is_unique}")
    print(f"    描述: {score_field.description}")
    print(f"    MongoDB特性: 支持 $gte, $lte, $in 等操作符")
    
    # 创建地理位置字段（如果支持）
    location_field = json_field(
        required=False,
        description="地理位置字段，MongoDB GeoJSON格式，支持地理空间查询"
    )
    
    print(f"\n  地理位置字段配置:")
    print(f"    必填: {location_field.is_required}")
    print(f"    描述: {location_field.description}")
    print(f"    MongoDB特性: 支持 $near, $geoWithin 等地理查询")
    
    return {
        'email': email_field, 
        'score': score_field,
        'location': location_field
    }


def demonstrate_version_info():
    """演示版本信息获取"""
    print("\n=== 版本信息演示 ===")
    
    try:
        version = get_version()
        info = get_info()
        name = get_name()
        
        print(f"  库名称: {name}")
        print(f"  版本号: {version}")
        print(f"  库信息: {info}")
        print(f"  MongoDB支持: ✓")
        print(f"  缓存支持: ✓")
        print(f"  ODM支持: ✓")
    except Exception as e:
        print(f"  获取版本信息失败: {e}")


def demonstrate_mongodb_performance_test():
    """演示MongoDB性能测试"""
    print("\n=== MongoDB性能测试演示 ===")
    
    # 测试字段创建性能
    print("\n1. MongoDB字段创建性能测试:")
    start_time = time.time()
    
    fields = []
    for i in range(100):
        field = string_field(
            required=i % 2 == 0,
            unique=i % 10 == 0,
            description=f"MongoDB测试字段{i}，支持文档嵌套"
        )
        fields.append(field)
    
    end_time = time.time()
    duration = end_time - start_time
    print(f"  创建100个MongoDB字段耗时: {duration:.4f} 秒")
    print(f"  平均每个字段创建时间: {duration/100:.6f} 秒")
    print(f"  MongoDB字段特性: 支持动态类型、嵌套文档")
    
    # 测试MongoDB索引创建性能
    print("\n2. MongoDB索引创建性能测试:")
    start_time = time.time()
    
    indexes = []
    for i in range(50):
        index = IndexDefinition(
            fields=[f"mongodb_field_{i}"],
            unique=i % 5 == 0,
            name=f"idx_mongodb_field_{i}"
        )
        indexes.append(index)
    
    end_time = time.time()
    duration = end_time - start_time
    print(f"  创建50个MongoDB索引耗时: {duration:.4f} 秒")
    print(f"  平均每个索引创建时间: {duration/50:.6f} 秒")
    print(f"  MongoDB索引特性: 支持复合索引、文本索引、地理索引")
    
    # 测试MongoDB特有的复合字段
    print("\n3. MongoDB复合字段测试:")
    start_time = time.time()
    
    complex_fields = []
    for i in range(20):
        # 创建包含多种类型的复合字段
        string_f = string_field(description=f"MongoDB字符串字段{i}")
        int_f = integer_field(description=f"MongoDB整数字段{i}")
        bool_f = boolean_field(description=f"MongoDB布尔字段{i}")
        json_f = json_field(description=f"MongoDB嵌套文档字段{i}")
        array_f = array_field(FieldType.string(), description=f"MongoDB数组字段{i}")
        
        complex_fields.extend([string_f, int_f, bool_f, json_f, array_f])
    
    end_time = time.time()
    duration = end_time - start_time
    print(f"  创建20组复合字段（共{len(complex_fields)}个）耗时: {duration:.4f} 秒")
    print(f"  平均每组复合字段创建时间: {duration/20:.6f} 秒")
    print(f"  MongoDB复合字段优势: 无模式约束，灵活的数据结构")
    
    return len(fields), len(indexes), len(complex_fields)


def cleanup_mongodb_test_data(demo_manager):
    """清理MongoDB测试数据（兼容性函数）"""
    if demo_manager:
        demo_manager.shutdown()


def cleanup_existing_collections():
    """清理现有的测试集合"""
    print("清理现有的MongoDB测试集合...")
    try:
        # 创建临时桥接器进行清理
        temp_bridge = create_db_queue_bridge()
        
        # 添加数据库连接
        response = temp_bridge.add_mongodb_database(
            alias="mongodb_cleanup",
            host="db0.0ldm0s.net",
            port=27017,
            database="testdb",
            username="testdb",
            password="yash2vCiBA&B#h$#i&gb@IGSTh&cP#QC^",
            auth_source="testdb",
            direct_connection=True,
            max_connections=5,
            min_connections=1,
            connection_timeout=5,
            idle_timeout=30,
            max_lifetime=120
        )
        
        result = json.loads(response)
        if result.get("success"):
            # 删除测试集合中的文档
            collections_to_clean = ["odm_test_collection", "mongodb_users"]
            for collection in collections_to_clean:
                try:
                    # 使用delete方法删除集合中的所有文档
                    delete_conditions = json.dumps([])  # 空条件删除所有文档
                    delete_response = temp_bridge.delete(collection, delete_conditions, "mongodb_cleanup")
                    delete_result = json.loads(delete_response)
                    if delete_result.get("success"):
                        print(f"  清理集合 {collection} 成功")
                    else:
                        print(f"  清理集合 {collection} 失败: {delete_result.get('error')}")
                except Exception as e:
                    print(f"  清理集合 {collection} 时出错: {e}")
        else:
            print(f"  无法连接到MongoDB进行清理: {result.get('error')}")
            
    except Exception as e:
        print(f"  清理过程中出错: {e}")
    
    print("  清理完成")


@with_graceful_shutdown(ShutdownConfig(verbose_logging=True))
def main():
    """主函数"""
    global test_instance
    
    # 注册信号处理器
    signal.signal(signal.SIGINT, signal_handler)
    signal.signal(signal.SIGTERM, signal_handler)
    
    print("=== RAT QuickDB Python MongoDB ODM绑定演示 ===")
    
    # 清理现有的测试集合
    cleanup_existing_collections()
    
    bridge = None
    demo_manager = None
    
    try:
        # 显示版本信息
        demonstrate_version_info()
        
        # 演示MongoDB字段创建
        fields = demonstrate_field_creation()
        
        # 演示MongoDB索引创建
        indexes = demonstrate_mongodb_index_creation()
        
        # 演示MongoDB模型元数据创建
        model_meta = demonstrate_mongodb_model_meta_creation(fields, indexes)
        
        # 演示MongoDB字段构建器模式
        builder_fields = demonstrate_mongodb_field_builder_pattern()
        
        # 演示MongoDB数据库操作
        bridge, demo_manager = demonstrate_mongodb_database_operations()
        
        # 设置全局实例用于信号处理
        test_instance = demo_manager
        
        # 演示MongoDB性能测试
        field_count, index_count, complex_field_count = demonstrate_mongodb_performance_test()
        
        print(f"\n=== MongoDB ODM演示完成 ===")
        print(f"总共创建了 {len(fields)} 个模型字段")
        print(f"总共创建了 {len(indexes)} 个MongoDB索引")
        print(f"性能测试创建了 {field_count} 个字段、{index_count} 个索引、{complex_field_count} 个复合字段")
        print(f"MongoDB数据库桥接器状态: {'已连接' if bridge else '未连接'}")
        
        print(f"\nMongoDB ODM特性总结:")
        print(f"   - 支持MongoDB原生数据类型（字符串、数字、布尔、日期、数组、嵌套文档）")
        print(f"   - 支持MongoDB索引类型（单字段、复合、多键、文本、地理空间）")
        print(f"   - 支持MongoDB查询操作符（$eq, $ne, $gt, $gte, $lt, $lte, $in, $nin等）")
        print(f"   - 支持MongoDB聚合管道和复杂查询")
        print(f"   - 集成缓存机制，提升MongoDB查询性能")
        print(f"   - 支持TLS加密和ZSTD压缩")
        print(f"   - 无模式约束，灵活的文档结构")
        print(f"   - 支持优雅关闭和资源清理")
        
    except KeyboardInterrupt:
        print("\n演示被用户中断")
        # 键盘中断时也要设置全局实例
        if demo_manager:
            test_instance = demo_manager
    except Exception as e:
        print(f"\n演示过程中发生错误: {e}")
        import traceback
        traceback.print_exc()
    finally:
        # 清理测试数据
        try:
            cleanup_mongodb_test_data(demo_manager)
        except Exception as e:
            print(f"清理过程中出错: {e}")


if __name__ == "__main__":
    main()