#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
RatQuickDB Python模型定义示例

本示例展示了如何使用RatQuickDB的Python装饰器模型定义系统，
包括字段定义、索引创建、模型验证等功能，对应主库model_definition.rs示例。
"""

import sys
import os
sys.path.insert(0, os.path.dirname(__file__))

import rat_quickdb_py as rq
from rat_quickdb_py import rat_dbmodel
import json
from datetime import datetime, timezone
import uuid

# 使用装饰器定义用户模型
@rat_dbmodel(table_name="users", database_alias="default", description="用户模型")
class User:
    # 基本信息字段
    id = rq.string_field(
        True,           # required
        True,           # unique
        None,           # max_length
        None,           # min_length
        "用户ID"         # description
    )

    username = rq.string_field(
        True,           # required
        True,           # unique
        None,           # max_length
        None,           # min_length
        "用户名"         # description
    )

    email = rq.string_field(
        True,           # required
        True,           # unique
        None,           # max_length
        None,           # min_length
        "邮箱地址"       # description
    )

    password_hash = rq.string_field(
        True,           # required
        False,          # unique
        None,           # max_length
        None,           # min_length
        "密码哈希"       # description
    )

    full_name = rq.string_field(
        True,           # required
        False,          # unique
        None,           # max_length
        None,           # min_length
        "全名"           # description
    )

    # 可选信息字段
    age = rq.integer_field(
        False,          # required
        None,           # min_value
        None,           # max_value
        "年龄"           # description
    )

    phone = rq.string_field(
        False,          # required
        False,          # unique
        None,           # max_length
        None,           # min_length
        "电话号码"       # description
    )

    avatar_url = rq.string_field(
        False,          # required
        False,          # unique
        None,           # max_length
        None,           # min_length
        "头像URL"        # description
    )

    # 状态字段
    is_active = rq.boolean_field(
        True,           # required
        "是否激活"       # description
    )

    # 时间字段
    created_at = rq.datetime_field(
        True,           # required
        "创建时间"       # description
    )

    updated_at = rq.datetime_field(
        False,          # required
        "更新时间"       # description
    )

    last_login = rq.datetime_field(
        False,          # required
        "最后登录时间"   # description
    )

    # JSON和数组字段
    profile = rq.json_field(
        False,          # required
        "用户配置信息"   # description
    )

    tags = rq.array_field(
        rq.FieldType.string(),  # array element type
        False,                  # required
        None,                   # max_items
        None,                   # min_items
        "用户标签"              # description
    )

    class Meta:
        database_alias = "default"
        description = "用户模型"
        indexes = [
            rq.IndexDefinition(
                ["username"],     # fields
                True,             # unique
                "idx_username"    # name
            ),
            rq.IndexDefinition(
                ["email"],        # fields
                True,             # unique
                "idx_email"       # name
            ),
            rq.IndexDefinition(
                ["created_at"],   # fields
                False,            # unique
                "idx_created_at"  # name
            ),
            rq.IndexDefinition(
                ["is_active", "created_at"],  # fields
                False,                        # unique
                "idx_active_created"          # name
            ),
        ]

# 使用装饰器定义文章模型
@rat_dbmodel(table_name="articles", database_alias="default", description="文章模型")
class Article:
    # 基本字段
    id = rq.string_field(
        True, True, None, None, "文章ID"
    )

    title = rq.string_field(
        True, False, None, None, "文章标题"
    )

    slug = rq.string_field(
        True, True, None, None, "文章URL别名"
    )

    content = rq.string_field(
        True, False, None, None, "文章内容"
    )

    summary = rq.string_field(
        False, False, None, None, "文章摘要"
    )

    # 关联字段
    author_id = rq.string_field(
        True, False, None, None, "作者ID"
    )

    category_id = rq.string_field(
        False, False, None, None, "分类ID"
    )

    # 状态和统计字段
    status = rq.string_field(
        True, False, None, None, "文章状态"
    )

    view_count = rq.integer_field(
        True, None, None, "浏览次数"
    )

    like_count = rq.integer_field(
        True, None, None, "点赞次数"
    )

    is_featured = rq.boolean_field(
        True, "是否推荐"
    )

    # 时间字段
    published_at = rq.datetime_field(
        False, "发布时间"
    )

    created_at = rq.datetime_field(
        True, "创建时间"
    )

    updated_at = rq.datetime_field(
        False, "更新时间"
    )

    # 元数据字段
    metadata = rq.json_field(
        False, "文章元数据"
    )

    tags = rq.array_field(
        rq.FieldType.string(), False, None, None, "文章标签"
    )

    class Meta:
        database_alias = "default"
        description = "文章模型"
        indexes = [
            rq.IndexDefinition(["slug"], True, "idx_slug"),
            rq.IndexDefinition(["author_id"], False, "idx_author"),
            rq.IndexDefinition(["category_id"], False, "idx_category"),
            rq.IndexDefinition(["status", "published_at"], False, "idx_status_published"),
            rq.IndexDefinition(["is_featured", "published_at"], False, "idx_featured_published"),
        ]

# 使用装饰器定义评论模型
@rat_dbmodel(table_name="comments", database_alias="default", description="评论模型")
class Comment:
    # 基本字段
    id = rq.string_field(
        True, True, None, None, "评论ID"
    )

    article_id = rq.string_field(
        True, False, None, None, "文章ID"
    )

    user_id = rq.string_field(
        True, False, None, None, "用户ID"
    )

    parent_id = rq.string_field(
        False, False, None, None, "父评论ID"
    )

    content = rq.string_field(
        True, False, None, None, "评论内容"
    )

    # 状态和统计字段
    is_approved = rq.boolean_field(
        True, "是否已审核"
    )

    like_count = rq.integer_field(
        True, None, None, "点赞次数"
    )

    # 时间字段
    created_at = rq.datetime_field(
        True, "创建时间"
    )

    updated_at = rq.datetime_field(
        False, "更新时间"
    )

    class Meta:
        database_alias = "default"
        description = "评论模型"
        indexes = [
            rq.IndexDefinition(["article_id"], False, "idx_article"),
            rq.IndexDefinition(["user_id"], False, "idx_user"),
            rq.IndexDefinition(["parent_id"], False, "idx_parent"),
            rq.IndexDefinition(["article_id", "is_approved"], False, "idx_article_approved"),
        ]

def demonstrate_json_serialization():
    """演示JSON序列化功能"""
    print("\n=== JSON序列化演示 ===")

    try:
        bridge = rq.create_native_db_queue_bridge()

        # 创建用户数据
        print("创建用户数据...")
        user_data = {
            "id": f"user_{uuid.uuid4().hex[:8]}",
            "username": f"zhangsan_{uuid.uuid4().hex[:8]}",
            "email": f"zhangsan_{uuid.uuid4().hex[:8]}@example.com",
            "password_hash": "hashed_password_here",
            "full_name": "张三",
            "age": 25,
            "phone": "+8613812345678",
            "avatar_url": "https://avatar.example.com/zhangsan.jpg",
            "is_active": True,
            "created_at": datetime.now(timezone.utc).isoformat(),
            "updated_at": datetime.now(timezone.utc).isoformat(),
            "last_login": None,
            "profile": {
                "preferences": {
                    "theme": "dark",
                    "language": "zh-CN"
                }
            },
            "tags": ["新用户", "活跃"]
        }

        # 插入用户数据
        insert_result = bridge.create("users", json.dumps(user_data), "default")

        if insert_result.get("success"):
            created_id = insert_result.get("data")
            print(f"✅ 用户创建成功，ID: {created_id}")

            # 查询用户数据
            print("\n查询用户数据...")
            query_result = bridge.find_by_id("users", created_id, "default")

            if query_result.get("success"):
                found_user = query_result.get("data")
                if found_user:
                    print(f"✅ 找到用户: {found_user.get('id')} - {found_user.get('username')}")

                    # 演示不同的序列化选项
                    print("\n序列化选项:")

                    # 1. 默认序列化（紧凑格式）
                    compact_json = json.dumps(found_user, ensure_ascii=False)
                    print(f"1. 默认序列化: {compact_json}")

                    # 2. 美化序列化
                    print("2. 美化序列化:")
                    pretty_json = json.dumps(found_user, indent=2, ensure_ascii=False)
                    print(pretty_json)

                    # 3. 展示数据映射的内容
                    print("3. 数据映射格式:")
                    print("数据映射:")
                    for key, value in found_user.items():
                        if value is None:
                            print(f"  {key}: null")
                        elif isinstance(value, str):
                            print(f"  {key}: \"{value}\"")
                        elif isinstance(value, (int, float)):
                            print(f"  {key}: {value}")
                        elif isinstance(value, bool):
                            print(f"  {key}: {value}")
                        elif isinstance(value, list):
                            print(f"  {key}: [{len(value)} 个元素]")
                        elif isinstance(value, dict):
                            print(f"  {key}: [{len(value)} 个字段]")
                        else:
                            print(f"  {key}: {type(value).__name__}")

                    # 清理测试数据
                    delete_result = bridge.delete("users", json.dumps([{"id": created_id}]), "default")
                    if delete_result.get("success"):
                        print("✅ 测试数据清理完成")
                else:
                    print("❌ 查询结果为空")
            else:
                print(f"❌ 查询用户失败: {query_result.get('error')}")
        else:
            print(f"❌ 用户创建失败: {insert_result.get('error')}")

    except Exception as e:
        print(f"❌ JSON序列化演示过程中发生错误: {e}")
        import traceback
        traceback.print_exc()

def demonstrate_json_field_types():
    """演示JSON字段类型功能"""
    print("\n=== JSON字段类型演示 ===")

    try:
        bridge = rq.create_native_db_queue_bridge()

        # 1. 创建包含复杂JSON数据的用户
        print("\n1. 创建包含复杂JSON数据的用户...")

        # 创建详细的用户配置JSON
        user_profile = {
            "personal_info": {
                "bio": "热爱编程的全栈开发者，专注于Rust和Web开发",
                "location": {
                    "country": "中国",
                    "city": "北京",
                    "coordinates": [116.4074, 39.9042]
                },
                "birth_date": "1995-06-15",
                "gender": "male"
            },
            "preferences": {
                "theme": "dark",
                "language": "zh-CN",
                "timezone": "Asia/Shanghai",
                "notifications": {
                    "email": True,
                    "push": False,
                    "sms": True
                },
                "privacy": {
                    "profile_visible": True,
                    "show_email": False,
                    "show_phone": False
                }
            },
            "skills": [
                {
                    "name": "Rust",
                    "level": "advanced",
                    "years_experience": 3,
                    "certifications": ["Rust Certified Developer"]
                },
                {
                    "name": "JavaScript",
                    "level": "intermediate",
                    "years_experience": 5
                },
                {
                    "name": "Python",
                    "level": "advanced",
                    "years_experience": 4
                }
            ],
            "social_links": {
                "github": "https://github.com/example_user",
                "linkedin": "https://linkedin.com/in/example_user",
                "twitter": "@example_user"
            },
            "settings": {
                "auto_save": True,
                "auto_backup": True,
                "api_keys": {
                    "weather_api": "sk-1234567890",
                    "maps_api": "mk-0987654321"
                }
            }
        }

        user_with_complex_profile = {
            "id": f"json_user_{uuid.uuid4().hex[:8]}",
            "username": f"json_user_{uuid.uuid4().hex[:8]}",
            "email": f"json_user_{uuid.uuid4().hex[:8]}@example.com",
            "password_hash": "hashed_password_here",
            "full_name": "JSON示例用户",
            "age": 28,
            "phone": "+8613812345678",
            "avatar_url": "https://avatar.example.com/json_user.jpg",
            "is_active": True,
            "created_at": datetime.now(timezone.utc).isoformat(),
            "updated_at": datetime.now(timezone.utc).isoformat(),
            "last_login": None,
            "profile": user_profile,
            "tags": ["JSON示例", "复杂配置", "开发者"]
        }

        insert_result = bridge.create("users", json.dumps(user_with_complex_profile), "default")

        if insert_result.get("success"):
            created_id = insert_result.get("data")
            print(f"✅ 复杂JSON用户创建成功，ID: {created_id}")

            # 2. 查询并验证JSON数据
            print("\n2. 查询并验证JSON数据...")
            query_result = bridge.find_by_id("users", created_id, "default")

            if query_result.get("success"):
                retrieved_user = query_result.get("data")
                if retrieved_user:
                    print("✅ 用户查询成功")

                    profile = retrieved_user.get('profile')
                    if profile and isinstance(profile, dict):
                        print("📋 用户配置信息:")

                        # 提取并展示个人信息
                        personal_info = profile.get("personal_info")
                        if personal_info:
                            bio = personal_info.get("bio", "未设置")
                            print(f"  📝 个人简介: {bio}")
                            location = personal_info.get("location", {})
                            city = location.get("city", "未知")
                            country = location.get("country", "未知")
                            print(f"  📍 位置: {city} - {country}")

                        # 提取并展示技能信息
                        skills = profile.get("skills", [])
                        if skills and isinstance(skills, list):
                            print("  💡 技能列表:")
                            for skill in skills:
                                name = skill.get("name", "未知")
                                years = skill.get("years_experience", 0)
                                level = skill.get("level", "未知")
                                print(f"    - {name} ({years}年经验, 级别: {level})")

                        # 提取并展示偏好设置
                        preferences = profile.get("preferences", {})
                        if preferences:
                            print("  ⚙️ 偏好设置:")
                            theme = preferences.get("theme", "未设置")
                            language = preferences.get("language", "未设置")
                            print(f"    主题: {theme}")
                            print(f"    语言: {language}")

                            notifications = preferences.get("notifications", {})
                            if notifications:
                                print("    通知设置:")
                                email_notif = notifications.get("email", False)
                                push_notif = notifications.get("push", False)
                                sms_notif = notifications.get("sms", False)
                                print(f"      邮件通知: {email_notif}")
                                print(f"      推送通知: {push_notif}")
                                print(f"      短信通知: {sms_notif}")

                    # 3. 演示JSON字段的部分更新
                    print("\n3. 演示JSON字段的部分更新...")

                    # 更新技能列表和偏好设置
                    updated_profile = profile.copy() if profile else {}

                    # 更新技能列表
                    if "skills" not in updated_profile:
                        updated_profile["skills"] = []
                    updated_profile["skills"].append({
                        "name": "Go",
                        "level": "beginner",
                        "years_experience": 1
                    })

                    # 更新偏好设置
                    if "preferences" in updated_profile:
                        if "notifications" in updated_profile["preferences"]:
                            updated_profile["preferences"]["notifications"]["push"] = True

                    update_data = {
                        "profile": updated_profile,
                        "updated_at": datetime.now(timezone.utc).isoformat()
                    }

                    conditions = [{"id": created_id}]
                    update_result = bridge.update("users", json.dumps(conditions), json.dumps(update_data), "default")

                    if update_result.get("success"):
                        print("✅ JSON字段更新成功")

                        # 验证更新结果
                        verify_result = bridge.find_by_id("users", created_id, "default")
                        if verify_result.get("success"):
                            updated_user = verify_result.get("data")
                            if updated_user:
                                updated_profile = updated_user.get("profile", {})
                                skills = updated_profile.get("skills", [])
                                print(f"🔄 更新后的技能数量: {len(skills)}")

                                preferences = updated_profile.get("preferences", {})
                                notifications = preferences.get("notifications", {})
                                push_status = notifications.get("push", False)
                                print(f"🔔 推送通知状态: {push_status}")

                    # 4. 演示基于标签的查询
                    print("\n4. 演示基于标签的查询...")

                    tag_conditions = [
                        {
                            "field": "tags",
                            "operator": "Contains",
                            "value": "开发者"
                        }
                    ]

                    find_result = bridge.find("users", json.dumps(tag_conditions), "default")

                    if find_result.get("success"):
                        dev_users = find_result.get("data", [])
                        print(f"✅ 标签包含'开发者'的用户数量: {len(dev_users)}")
                        for user in dev_users:
                            user_id = user.get("id", "未知")
                            username = user.get("username", "未知")
                            print(f"  用户: {user_id} - {username}")
                    else:
                        print(f"❌ 标签查询失败: {find_result.get('error')}")

                    # 5. 演示JSON数据的序列化和反序列化
                    print("\n5. 演示JSON数据的序列化和反序列化...")

                    if profile:
                        # 序列化为字符串
                        json_string = json.dumps(profile, indent=2, ensure_ascii=False)
                        print("📄 JSON序列化结果（前200字符）:")
                        preview = json_string[:200] + "..." if len(json_string) > 200 else json_string
                        print(preview)

                        # 反序列化回JSON值
                        parsed_json = json.loads(json_string)

                        # 验证数据完整性
                        skills = parsed_json.get("skills", [])
                        print(f"✅ 反序列化验证成功，技能数量: {len(skills)}")

                    # 清理测试数据
                    delete_result = bridge.delete("users", json.dumps([{"id": created_id}]), "default")
                    if delete_result.get("success"):
                        print("✅ 测试数据清理完成")
                else:
                    print("❌ 查询结果为空")
            else:
                print(f"❌ 查询用户失败: {query_result.get('error')}")
        else:
            print(f"❌ 复杂JSON用户创建失败: {insert_result.get('error')}")

        # 6. 创建包含简单JSON数据的文章
        print("\n6. 创建包含简单JSON数据的文章...")

        article_metadata = {
            "seo": {
                "title": "Rust JSON字段使用指南",
                "description": "详细介绍如何在RatQuickDB中使用JSON字段类型",
                "keywords": ["Rust", "JSON", "数据库", "RatQuickDB"],
                "og_image": "https://example.com/og-image.jpg"
            },
            "analytics": {
                "read_time_minutes": 8,
                "difficulty": "intermediate",
                "category": "技术教程",
                "tags": ["Rust", "数据库", "JSON"]
            },
            "version": {
                "current": "1.2.0",
                "history": ["1.0.0", "1.1.0", "1.2.0"]
            }
        }

        article_with_metadata = {
            "id": f"article_{uuid.uuid4().hex[:8]}",
            "title": "RatQuickDB JSON字段完全指南",
            "slug": f"rat-quickdb-json-guide-{uuid.uuid4().hex[:8]}",
            "content": "本文将详细介绍如何在RatQuickDB中有效使用JSON字段类型，包括数据建模、查询优化和最佳实践。",
            "summary": "学习RatQuickDB JSON字段的使用方法和技巧。",
            "author_id": "json_demo_author",
            "category_id": "database",
            "status": "published",
            "view_count": 150,
            "like_count": 42,
            "is_featured": True,
            "published_at": datetime.now(timezone.utc).isoformat(),
            "created_at": datetime.now(timezone.utc).isoformat(),
            "updated_at": datetime.now(timezone.utc).isoformat(),
            "metadata": article_metadata,
            "tags": ["Rust", "JSON", "数据库", "教程"]
        }

        article_insert_result = bridge.create("articles", json.dumps(article_with_metadata), "default")

        if article_insert_result.get("success"):
            article_id = article_insert_result.get("data")
            print(f"✅ 包含元数据的文章创建成功，ID: {article_id}")

            # 查询并展示文章元数据
            article_query_result = bridge.find_by_id("articles", article_id, "default")

            if article_query_result.get("success"):
                retrieved_article = article_query_result.get("data")
                if retrieved_article:
                    metadata = retrieved_article.get("metadata")
                    if metadata and isinstance(metadata, dict):
                        print("📊 文章元数据:")

                        seo = metadata.get("seo", {})
                        if seo:
                            title = seo.get("title", "未设置")
                            description = seo.get("description", "未设置")
                            print(f"  SEO标题: {title}")
                            print(f"  SEO描述: {description}")

                            keywords = seo.get("keywords", [])
                            if keywords and isinstance(keywords, list):
                                keyword_list = ", ".join(keywords)
                                print(f"  关键词: {keyword_list}")

                        analytics = metadata.get("analytics", {})
                        if analytics:
                            read_time = analytics.get("read_time_minutes", 0)
                            difficulty = analytics.get("difficulty", "未设置")
                            print(f"  阅读时间: {read_time}分钟")
                            print(f"  难度级别: {difficulty}")

                    # 清理测试数据
                    delete_result = bridge.delete("articles", json.dumps([{"id": article_id}]), "default")
                    if delete_result.get("success"):
                        print("✅ 文章测试数据清理完成")
                else:
                    print("❌ 文章查询结果为空")
            else:
                print(f"❌ 查询文章失败: {article_query_result.get('error')}")
        else:
            print(f"❌ 文章创建失败: {article_insert_result.get('error')}")

        print("✅ JSON字段类型演示完成")

    except Exception as e:
        print(f"❌ JSON字段类型演示过程中发生错误: {e}")
        import traceback
        traceback.print_exc()

def demonstrate_basic_crud():
    """演示基本CRUD操作"""
    print("\n=== 基本CRUD操作演示 ===")

    try:
        bridge = rq.create_native_db_queue_bridge()

        # 1. 创建用户
        print("\n1. 创建用户...")
        user_data = {
            "id": f"demo_user_{uuid.uuid4().hex[:8]}",
            "username": f"demo_user_{uuid.uuid4().hex[:8]}",
            "email": f"demo_user_{uuid.uuid4().hex[:8]}@example.com",
            "password_hash": "hashed_password_here",
            "full_name": "Demo User",
            "age": 25,
            "phone": "+8613811111111",
            "avatar_url": "https://avatar.example.com/demo.jpg",
            "is_active": True,
            "created_at": datetime.now(timezone.utc).isoformat(),
            "updated_at": datetime.now(timezone.utc).isoformat(),
            "last_login": None,
            "profile": {
                "preferences": {
                    "theme": "dark",
                    "language": "en-US"
                }
            },
            "tags": ["测试用户"]
        }

        insert_result = bridge.create("users", json.dumps(user_data), "default")

        if insert_result.get("success"):
            created_id = insert_result.get("data")
            print(f"✅ 用户创建成功，ID: {created_id}")

            # 2. 查询用户
            print("\n2. 查询用户...")
            query_result = bridge.find_by_id("users", created_id, "default")

            if query_result.get("success"):
                found_user = query_result.get("data")
                if found_user:
                    print(f"✅ 找到用户: {found_user.get('id')} - {found_user.get('username')}")

                    # 3. 更新用户
                    print("\n3. 更新用户...")
                    update_data = {
                        "age": 26,
                        "updated_at": datetime.now(timezone.utc).isoformat()
                    }

                    conditions = [{"id": created_id}]
                    update_result = bridge.update("users", json.dumps(conditions), json.dumps(update_data), "default")

                    if update_result.get("success"):
                        print("✅ 用户更新成功")
                    else:
                        print(f"❌ 用户更新失败: {update_result.get('error')}")

                    # 4. 删除用户
                    print("\n4. 删除用户...")
                    delete_result = bridge.delete("users", json.dumps([{"id": created_id}]), "default")

                    if delete_result.get("success"):
                        print("✅ 用户删除成功")
                    else:
                        print(f"❌ 用户删除失败: {delete_result.get('error')}")
                else:
                    print("❌ 用户未找到")
            else:
                print(f"❌ 查询用户失败: {query_result.get('error')}")
        else:
            print(f"❌ 用户创建失败: {insert_result.get('error')}")

    except Exception as e:
        print(f"❌ 基本CRUD操作演示过程中发生错误: {e}")
        import traceback
        traceback.print_exc()

def demonstrate_error_handling():
    """演示错误处理"""
    print("\n=== 错误处理演示 ===")

    try:
        bridge = rq.create_native_db_queue_bridge()

        # 1. 创建无效用户数据（违反字段约束）
        print("\n1. 创建无效用户数据...")
        invalid_user = {
            "id": "",  # 空ID，应该违反必填约束
            "username": "",  # 空用户名，应该违反必填约束
            "email": "invalid-email",  # 无效邮箱格式
            "password_hash": "",
            "full_name": "",
            "age": -1,  # 无效年龄
            "phone": None,
            "avatar_url": None,
            "is_active": True,
            "created_at": datetime.now(timezone.utc).isoformat(),
            "updated_at": datetime.now(timezone.utc).isoformat(),
            "last_login": None,
            "profile": None,
            "tags": None
        }

        insert_result = bridge.create("users", json.dumps(invalid_user), "default")
        if not insert_result.get("success"):
            print(f"✅ 预期错误（数据验证失败）: {insert_result.get('error')}")
        else:
            print("❌ 意外：无效用户数据创建成功")

        # 2. 尝试查询不存在的用户
        print("\n2. 查询不存在的用户...")
        query_result = bridge.find_by_id("users", "non_existent_id", "default")

        if query_result.get("success"):
            found_user = query_result.get("data")
            if found_user is None:
                print("✅ 预期结果：用户不存在")
            else:
                print("❌ 意外：找到了不存在的用户")
        else:
            print(f"查询错误: {query_result.get('error')}")

        # 3. 创建重复数据测试（测试唯一约束）
        print("\n3. 创建重复数据...")

        # 第一次创建
        first_user = {
            "id": f"unique_user_{uuid.uuid4().hex[:8]}",
            "username": f"unique_user_{uuid.uuid4().hex[:8]}",
            "email": f"unique_user_{uuid.uuid4().hex[:8]}@example.com",
            "password_hash": "hashed_password_here",
            "full_name": "Unique User",
            "age": 25,
            "phone": "+8613811111111",
            "avatar_url": "https://avatar.example.com/unique1.jpg",
            "is_active": True,
            "created_at": datetime.now(timezone.utc).isoformat(),
            "updated_at": datetime.now(timezone.utc).isoformat(),
            "last_login": None,
            "profile": None,
            "tags": None
        }

        first_result = bridge.create("users", json.dumps(first_user), "default")

        if first_result.get("success"):
            first_id = first_result.get("data")
            print(f"✅ 第一次创建成功: {first_id}")

            # 第二次创建相同用户名的用户
            duplicate_user = {
                "id": f"duplicate_user_{uuid.uuid4().hex[:8]}",
                "username": first_user["username"],  # 重复用户名
                "email": f"duplicate_user_{uuid.uuid4().hex[:8]}@example.com",
                "password_hash": "hashed_password_here",
                "full_name": "Duplicate User",
                "age": 30,
                "phone": "+8613822222222",
                "avatar_url": "https://avatar.example.com/unique2.jpg",
                "is_active": True,
                "created_at": datetime.now(timezone.utc).isoformat(),
                "updated_at": datetime.now(timezone.utc).isoformat(),
                "last_login": None,
                "profile": None,
                "tags": None
            }

            duplicate_result = bridge.create("users", json.dumps(duplicate_user), "default")

            if not duplicate_result.get("success"):
                print(f"✅ 预期错误（重复用户名）: {duplicate_result.get('error')}")
            else:
                print(f"❌ 意外成功：重复用户创建成功: {duplicate_result.get('data')}")

            # 清理测试数据
            delete_result = bridge.delete("users", json.dumps([{"id": first_id}]), "default")
            if delete_result.get("success"):
                print("✅ 测试数据清理完成")
        else:
            print(f"第一次创建失败: {first_result.get('error')}")

        # 4. 测试更新不存在的用户
        print("\n4. 更新不存在的用户...")
        update_data = {
            "age": 30
        }

        conditions = [{"id": "non_existent_id"}]
        update_result = bridge.update("users", json.dumps(conditions), json.dumps(update_data), "default")

        if not update_result.get("success"):
            print(f"✅ 预期错误（更新不存在的用户）: {update_result.get('error')}")
        else:
            print("❌ 意外成功：更新了不存在的用户")

        # 5. 测试删除不存在的用户
        print("\n5. 删除不存在的用户...")
        delete_result = bridge.delete("users", json.dumps([{"id": "non_existent_id"}]), "default")

        if not delete_result.get("success"):
            print(f"✅ 预期错误（删除不存在的用户）: {delete_result.get('error')}")
        else:
            print("❌ 意外成功：删除了不存在的用户")

    except Exception as e:
        print(f"❌ 错误处理演示过程中发生错误: {e}")
        import traceback
        traceback.print_exc()

def demonstrate_batch_operations():
    """演示批量操作"""
    print("\n=== 批量操作演示 ===")

    try:
        bridge = rq.create_native_db_queue_bridge()
        created_ids = []

        # 1. 批量创建用户
        print("\n1. 批量创建用户...")
        batch_users = []
        for i in range(1, 5):
            user = {
                "id": f"batch{i}_{uuid.uuid4().hex[:8]}",
                "username": f"batch{i}_{uuid.uuid4().hex[:8]}",
                "email": f"batch{i}_{uuid.uuid4().hex[:8]}@example.com",
                "password_hash": "hashed_password_here",
                "full_name": f"Batch User {i}",
                "age": 25 + i,
                "phone": f"+861381111111{i}",
                "avatar_url": f"https://avatar.example.com/batch{i}.jpg",
                "is_active": True,
                "created_at": datetime.now(timezone.utc).isoformat(),
                "updated_at": datetime.now(timezone.utc).isoformat(),
                "last_login": None,
                "profile": None,
                "tags": ["批量用户"]
            }
            batch_users.append(user)

        created_count = 0
        for i, user in enumerate(batch_users):
            result = bridge.create("users", json.dumps(user), "default")
            if result.get("success"):
                created_id = result.get("data")
                created_ids.append(created_id)
                created_count += 1
                print(f"✅ 创建用户 {i + 1}: {created_id}")
            else:
                print(f"❌ 创建用户 {i + 1} 失败: {result.get('error')}")

        print(f"✅ 批量创建完成，共创建 {created_count} 个用户")

        # 2. 批量查询用户
        print("\n2. 批量查询用户...")
        batch_conditions = [
            {
                "field": "username",
                "operator": "Contains",
                "value": "batch"
            }
        ]

        find_result = bridge.find("users", json.dumps(batch_conditions), "default")

        if find_result.get("success"):
            users = find_result.get("data", [])
            if len(users) > 0:
                print(f"✅ 查询结果（用户名包含'batch'）: {len(users)} 个用户")
                for user in users:
                    user_id = user.get("id", "未知")
                    username = user.get("username", "未知")
                    print(f"   用户: {user_id} - {username}")
            else:
                print("❌ 批量查询应该返回至少1个用户，但返回了0个用户")
        else:
            print(f"❌ 批量查询失败: {find_result.get('error')}")

        # 3. 批量更新用户状态
        print("\n3. 批量更新用户状态...")
        update_data = {
            "is_active": False,
            "updated_at": datetime.now(timezone.utc).isoformat()
        }

        update_conditions = [
            {
                "field": "username",
                "operator": "Contains",
                "value": "batch"
            }
        ]

        update_result = bridge.update("users", json.dumps(update_conditions), json.dumps(update_data), "default")

        if update_result.get("success"):
            print("✅ 批量更新成功")
        else:
            print(f"❌ 批量更新失败: {update_result.get('error')}")

        # 4. 批量统计操作
        print("\n4. 批量统计操作...")
        count_all_result = bridge.count("users", "[]", "default")

        if count_all_result.get("success"):
            total = count_all_result.get("data", 0)
            if total > 0:
                print(f"✅ 总用户数: {total}")
            else:
                print(f"❌ 总用户数应该大于0，但返回了{total}")
        else:
            print(f"❌ 统计总数失败: {count_all_result.get('error')}")

        batch_count_conditions = [
            {
                "field": "username",
                "operator": "Contains",
                "value": "batch"
            }
        ]

        count_batch_result = bridge.count("users", json.dumps(batch_count_conditions), "default")

        if count_batch_result.get("success"):
            batch_count = count_batch_result.get("data", 0)
            if batch_count > 0:
                print(f"✅ 批量用户数: {batch_count}")
            else:
                print(f"❌ 批量用户数应该大于0，但返回了{batch_count}")
        else:
            print(f"❌ 统计批量用户数失败: {count_batch_result.get('error')}")

        # 5. 批量删除演示
        print("\n5. 批量删除演示...")
        delete_conditions = [
            {
                "field": "username",
                "operator": "Contains",
                "value": "batch"
            }
        ]

        delete_result = bridge.delete("users", json.dumps(delete_conditions), "default")

        if delete_result.get("success"):
            print("✅ 批量删除成功")
        else:
            print(f"❌ 批量删除失败: {delete_result.get('error')}")

    except Exception as e:
        print(f"❌ 批量操作演示过程中发生错误: {e}")
        import traceback
        traceback.print_exc()

def main():
    """主函数"""
    print("=== RatQuickDB Python 模型定义系统演示 ===")

    # 清理旧数据库文件
    db_path = "./test_model.db"
    if os.path.exists(db_path):
        os.remove(db_path)
        print("🧹 清理旧数据库文件完成")

    try:
        # 初始化日志系统
        rq.init_logging_with_level("info")
        print("✅ 日志系统初始化成功")

        # 创建数据库桥接器
        bridge = rq.create_native_db_queue_bridge()
        print("✅ 数据库桥接器创建成功")

        # 添加SQLite数据库
        result = bridge.add_sqlite_database(
            alias="default",
            path=db_path,
            max_connections=10,
            min_connections=2,
            connection_timeout=5,
            idle_timeout=300,
            max_lifetime=1800,
            id_strategy="Uuid"
        )

        if result.get("success"):
            print("✅ SQLite数据库配置成功")

            print("\n1. 演示JSON序列化功能")
            demonstrate_json_serialization()

            print("\n2. 演示JSON字段类型功能")
            demonstrate_json_field_types()

            print("\n3. 演示基本CRUD操作")
            demonstrate_basic_crud()

            print("\n4. 演示错误处理")
            demonstrate_error_handling()

            print("\n5. 演示批量操作")
            demonstrate_batch_operations()

            print("\n=== 演示完成 ===")
            return True
        else:
            print(f"❌ 数据库配置失败: {result.get('error')}")
            return False

    except Exception as e:
        print(f"❌ 主函数执行过程中发生错误: {e}")
        import traceback
        traceback.print_exc()
        return False

if __name__ == "__main__":
    success = main()
    if success:
        print("\n✅ Python模型定义示例演示完成！")
        sys.exit(0)
    else:
        print("\n❌ Python模型定义示例演示失败！")
        sys.exit(1)