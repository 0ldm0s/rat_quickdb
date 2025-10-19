#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
RatQuickDB Python模型定义示例（MySQL版本）

本示例展示了如何使用RatQuickDB的应用模式进行模型定义，
包括字段定义、索引创建、模型验证等功能，对应主库model_definition_mysql.rs示例。
"""

import sys
import os
sys.path.insert(0, os.path.dirname(__file__))

import rat_quickdb_py as rq
from rat_quickdb_py.model_decorator import RatQuickDB
import json
from datetime import datetime, timezone
import uuid

# 创建应用实例
app = RatQuickDB()

# 使用应用装饰器定义用户模型
@app.model(table_name="users", database_alias="default", description="用户模型")
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
        False,          # unique
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
                ["is_active"],    # fields
                False,            # unique
                "idx_is_active"   # name
            ),
        ]

# 使用应用装饰器定义文章模型
@app.model(table_name="articles", database_alias="default", description="文章模型")
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
        True, None, None, False, "浏览次数"
    )

    like_count = rq.integer_field(
        True, None, None, False, "点赞次数"
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
            rq.IndexDefinition(["status"], False, "idx_status"),
            rq.IndexDefinition(["published_at"], False, "idx_published_at"),
            rq.IndexDefinition(["is_featured"], False, "idx_is_featured"),
        ]

# 使用应用装饰器定义评论模型
@app.model(table_name="comments", database_alias="default", description="评论模型")
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
        True, None, None, False, "点赞次数"
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
            rq.IndexDefinition(["is_approved"], False, "idx_is_approved"),
        ]

def demonstrate_json_serialization():
    """演示JSON序列化功能"""
    print("\n=== JSON序列化演示 ===")

    try:

        # 创建用户数据
        print("创建用户数据...")
        # MySQL兼容的datetime格式（去掉时区信息）
        now = datetime.now()
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
            "created_at": now.strftime("%Y-%m-%d %H:%M:%S"),
            "updated_at": now.strftime("%Y-%m-%d %H:%M:%S"),
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
        insert_result = User.create(user_data)

        if insert_result.get("success"):
            created_id = insert_result.get("data")
            print(f"✅ 用户创建成功，ID: {created_id}")

            # 查询用户数据
            print("\n查询用户数据...")
            query_result = User.find_by_id(created_id)

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
                    delete_result = User.delete([{"id": created_id}])
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

        # MySQL兼容的datetime格式
        now = datetime.now()
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
            "created_at": now.strftime("%Y-%m-%d %H:%M:%S"),
            "updated_at": now.strftime("%Y-%m-%d %H:%M:%S"),
            "last_login": None,
            "profile": user_profile,
            "tags": ["JSON示例", "复杂配置", "开发者"]
        }

        insert_result = User.create(user_with_complex_profile)

        if insert_result.get("success"):
            created_id = insert_result.get("data")
            print(f"✅ 复杂JSON用户创建成功，ID: {created_id}")

            # 2. 查询并验证JSON数据
            print("\n2. 查询并验证JSON数据...")
            query_result = User.find_by_id(created_id)

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
                        "updated_at": datetime.now().strftime("%Y-%m-%d %H:%M:%S")
                    }

                    conditions = [{"id": created_id}]
                    update_result = User.update(conditions, update_data)

                    if update_result.get("success"):
                        print("✅ JSON字段更新成功")

                        # 验证更新结果
                        verify_result = User.find_by_id(created_id)
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

                    find_result = User.find(tag_conditions)

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
                    delete_result = User.delete([{"id": created_id}])
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

        # MySQL兼容的datetime格式
        now = datetime.now()
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
            "published_at": now.strftime("%Y-%m-%d %H:%M:%S"),
            "created_at": now.strftime("%Y-%m-%d %H:%M:%S"),
            "updated_at": now.strftime("%Y-%m-%d %H:%M:%S"),
            "metadata": article_metadata,
            "tags": ["Rust", "JSON", "数据库", "教程"]
        }

        article_insert_result = Article.create(article_with_metadata)

        if article_insert_result.get("success"):
            article_id = article_insert_result.get("data")
            print(f"✅ 包含元数据的文章创建成功，ID: {article_id}")

            # 查询并展示文章元数据
            article_query_result = Article.find_by_id(article_id)

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
                    delete_result = Article.delete([{"id": article_id}])
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

        # 1. 创建用户
        print("\n1. 创建用户...")
        # MySQL兼容的datetime格式
        now = datetime.now()
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
            "created_at": now.strftime("%Y-%m-%d %H:%M:%S"),
            "updated_at": now.strftime("%Y-%m-%d %H:%M:%S"),
            "last_login": None,
            "profile": {
                "preferences": {
                    "theme": "dark",
                    "language": "en-US"
                }
            },
            "tags": ["测试用户"]
        }

        insert_result = User.create(user_data)

        if insert_result.get("success"):
            created_id = insert_result.get("data")
            print(f"✅ 用户创建成功，ID: {created_id}")

            # 2. 查询用户
            print("\n2. 查询用户...")
            query_result = User.find_by_id(created_id)

            if query_result.get("success"):
                found_user = query_result.get("data")
                if found_user:
                    print(f"✅ 找到用户: {found_user.get('id')} - {found_user.get('username')}")

                    # 3. 更新用户
                    print("\n3. 更新用户...")
                    update_data = {
                        "age": 26,
                        "updated_at": datetime.now().strftime("%Y-%m-%d %H:%M:%S")
                    }

                    conditions = [{"id": created_id}]
                    update_result = User.update(conditions, update_data)

                    if update_result.get("success"):
                        print("✅ 用户更新成功")
                    else:
                        print(f"❌ 用户更新失败: {update_result.get('error')}")

                    # 4. 删除用户
                    print("\n4. 删除用户...")
                    delete_result = User.delete([{"id": created_id}])

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

        # 1. 创建无效用户数据（违反字段约束）
        print("\n1. 创建无效用户数据...")
        # MySQL兼容的datetime格式
        now = datetime.now()
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
            "created_at": now.strftime("%Y-%m-%d %H:%M:%S"),
            "updated_at": now.strftime("%Y-%m-%d %H:%M:%S"),
            "last_login": None,
            "profile": None,
            "tags": None
        }

        insert_result = User.create(invalid_user)
        if not insert_result.get("success"):
            print(f"✅ 预期错误（数据验证失败）: {insert_result.get('error')}")
        else:
            print("❌ 意外：无效用户数据创建成功")

            # 二次校验：检查数据是否真的被创建了
            created_id = insert_result.get("data")
            print(f"🔍 二次校验：检查用户是否真的创建了，ID: {created_id}")

            if created_id:
                verify_result = User.find_by_id(created_id)
                if verify_result.get("success") and verify_result.get("data"):
                    print("❌ 确认：无效数据确实被创建了，但这可能是SQLite的容错机制")
                    invalid_data = verify_result.get("data")
                    print(f"   实际创建的数据: {invalid_data}")
                else:
                    print("✅ 确认：虽然返回成功，但数据实际上并未创建（容错返回）")
            else:
                print("✅ 确认：没有返回有效ID，数据可能未实际创建")

        # 2. 尝试查询不存在的用户
        print("\n2. 查询不存在的用户...")
        query_result = User.find_by_id("non_existent_id")

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
        # MySQL兼容的datetime格式
        now = datetime.now()
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
            "created_at": now.strftime("%Y-%m-%d %H:%M:%S"),
            "updated_at": now.strftime("%Y-%m-%d %H:%M:%S"),
            "last_login": None,
            "profile": None,
            "tags": None
        }

        first_result = User.create(first_user)

        if first_result.get("success"):
            first_id = first_result.get("data")
            print(f"✅ 第一次创建成功: {first_id}")

            # 第二次创建相同用户名的用户
            # MySQL兼容的datetime格式
            now = datetime.now()
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
                "created_at": now.strftime("%Y-%m-%d %H:%M:%S"),
                "updated_at": now.strftime("%Y-%m-%d %H:%M:%S"),
                "last_login": None,
                "profile": None,
                "tags": None
            }

            duplicate_result = User.create(duplicate_user)

            if not duplicate_result.get("success"):
                print(f"✅ 预期错误（重复用户名）: {duplicate_result.get('error')}")
            else:
                print(f"❌ 意外成功：重复用户创建成功: {duplicate_result.get('data')}")

                # 二次校验：检查是否真的创建了重复用户
                duplicate_id = duplicate_result.get("data")
                print(f"🔍 二次校验：检查重复用户是否真的创建了，ID: {duplicate_id}")

                if duplicate_id:
                    verify_duplicate = User.find_by_id(duplicate_id)
                    if verify_duplicate.get("success") and verify_duplicate.get("data"):
                        duplicate_data = verify_duplicate.get("data")
                        print("❌ 确认：重复用户确实被创建了")
                        print(f"   重复用户数据: {duplicate_data}")

                        # 三次校验：检查是否真的有重复用户名
                        find_by_username = User.find([
                            {"field": "username", "operator": "Eq", "value": first_user["username"]}
                        ])

                        if find_by_username.get("success"):
                            duplicate_users = find_by_username.get("data", [])
                            print(f"🔍 三次校验：用户名'{first_user['username']}'的用户数量: {len(duplicate_users)}")
                            if len(duplicate_users) > 1:
                                print("❌ 确认：确实存在重复用户名的记录")
                                for i, user in enumerate(duplicate_users):
                                    print(f"   记录{i+1}: {user.get('id')} - {user.get('username')}")
                            else:
                                print("✅ 确认：实际上没有重复用户名，可能是自动处理了或UUID策略避免了冲突")
                    else:
                        print("✅ 确认：虽然返回成功，但重复用户实际未创建")

            # 清理测试数据
            delete_result = User.delete([{"id": first_id}])
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
        update_result = User.update(conditions, update_data)

        if not update_result.get("success"):
            print(f"✅ 预期错误（更新不存在的用户）: {update_result.get('error')}")
        else:
            print("❌ 意外成功：更新了不存在的用户")

            # 二次校验：检查是否真的更新了不存在的用户
            print("🔍 二次校验：检查是否真的更新了不存在的用户...")
            verify_after_update = User.find_by_id("non_existent_id")
            if verify_after_update.get("success") and verify_after_update.get("data"):
                print("❌ 确认：不存在用户被意外更新了（这不应该发生）")
            else:
                print("✅ 确认：不存在用户确实没有被更新（容错返回成功）")

        # 5. 测试删除不存在的用户
        print("\n5. 删除不存在的用户...")
        delete_result = User.delete([{"id": "non_existent_id"}])

        if not delete_result.get("success"):
            print(f"✅ 预期错误（删除不存在的用户）: {delete_result.get('error')}")
        else:
            print("❌ 意外成功：删除了不存在的用户")

            # 二次校验：检查是否真的删除了不存在的用户
            print("🔍 二次校验：检查是否真的删除了不存在的用户...")
            # 这种情况容错是合理的，因为删除不存在的记录在语义上是成功的
            print("✅ 确认：删除不存在的用户返回成功是合理的容错行为")

    except Exception as e:
        print(f"❌ 错误处理演示过程中发生错误: {e}")
        import traceback
        traceback.print_exc()

def demonstrate_batch_operations():
    """演示批量操作"""
    print("\n=== 批量操作演示 ===")

    try:
        created_ids = []

        # 1. 批量创建用户
        print("\n1. 批量创建用户...")
        batch_users = []
        now = datetime.now()
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
                "created_at": now.strftime("%Y-%m-%d %H:%M:%S"),
                "updated_at": now.strftime("%Y-%m-%d %H:%M:%S"),
                "last_login": None,
                "profile": None,
                "tags": ["批量用户"]
            }
            batch_users.append(user)

        created_count = 0
        for i, user in enumerate(batch_users):
            result = User.create(user)
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

        find_result = User.find(batch_conditions)

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
            "updated_at": datetime.now().strftime("%Y-%m-%d %H:%M:%S")
        }

        update_conditions = [
            {
                "field": "username",
                "operator": "Contains",
                "value": "batch"
            }
        ]

        update_result = User.update(update_conditions, update_data)

        if update_result.get("success"):
            print("✅ 批量更新成功")
        else:
            print(f"❌ 批量更新失败: {update_result.get('error')}")

        # 4. 批量统计操作
        print("\n4. 批量统计操作...")
        count_all_result = User.count()

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

        count_batch_result = User.count(batch_count_conditions)

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

        delete_result = User.delete(delete_conditions)

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
    print("RAT QuickDB Python绑定 - 模型定义示例（MySQL版本）")
    print("=" * 60)

    try:
        # 初始化日志系统
        rq.init_logging_with_level("info")
        print("✅ 日志系统初始化成功")

        # 添加MySQL数据库到应用
        result = app.add_mysql_database(
            alias="default",
            host="172.16.0.21",
            port=3306,
            database="testdb",
            username="testdb",
            password="yash2vCiBA&B#h$#i&gb@IGSTh&cP#QC^",
            max_connections=10,
            min_connections=2,
            connection_timeout=5,
            idle_timeout=300,
            max_lifetime=1800,
            id_strategy="Uuid"
        )

        if result.get("success"):
            print("✅ MySQL数据库配置成功")

            # 清理旧的测试表，确保干净的测试环境
            print("🧹 清理旧测试表...")

            # 删除可能存在的旧表
            old_tables = ["users", "articles", "comments"]
            for table_name in old_tables:
                drop_result = app.drop_table(table_name, "default")
                if drop_result.get("success"):
                    print(f"✅ 删除旧表: {table_name}")
                else:
                    print(f"ℹ️ 表 {table_name} 不存在或已删除")

            print("✅ 测试环境清理完成")

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
            print(f"❌ MySQL数据库配置失败: {result.get('error')}")
            return False

    except Exception as e:
        print(f"❌ 主函数执行过程中发生错误: {e}")
        import traceback
        traceback.print_exc()
        return False

if __name__ == "__main__":
    success = main()
    if success:
        print("\n✅ Python模型定义示例（MySQL版本）演示完成！")
        sys.exit(0)
    else:
        print("\n❌ Python模型定义示例（MySQL版本）演示失败！")
        sys.exit(1)