# Rust 统一缓存 + 本地产物的构建方案
#
# 使用方法：
#   just build      # 开发构建
#   just release    # 发布构建
#   just run        # 构建并运行
#   just clean      # 清理本地产物（保留缓存）
#   just info       # 显示配置信息
#   just cache-stats # 查看缓存统计

# 获取项目名称（从 Cargo.toml 读取）
export PROJECT_NAME := `grep '^name = ' Cargo.toml | head -1 | sed 's/name = "\(.*\)"/\1/'`

# 统一缓存目录（与 .cargo/config.toml 保持一致）
export CACHE_TARGET := "/home/oldmos/.cargo/cache/target"

# 本地产物目录
export LOCAL_TARGET := "target"

# 默认构建（开发模式）
build:
    #!/bin/bash
    set -e
    echo "📦 统一缓存编译..."
    cargo build

    echo "📋 复制最终产物到本地..."
    mkdir -p {{LOCAL_TARGET}}/debug

    # 复制二进制文件
    if [ -f "{{CACHE_TARGET}}/debug/{{PROJECT_NAME}}" ]; then
        cp "{{CACHE_TARGET}}/debug/{{PROJECT_NAME}}" "{{LOCAL_TARGET}}/debug/"
        echo "✅ 二进制: {{LOCAL_TARGET}}/debug/{{PROJECT_NAME}}"
        ls -lh "{{LOCAL_TARGET}}/debug/{{PROJECT_NAME}}"
    fi

# 发布构建
release:
    #!/bin/bash
    set -e
    echo "📦 统一缓存编译（Release）..."
    cargo build --release

    echo "📋 复制最终产物到本地..."
    mkdir -p {{LOCAL_TARGET}}/release

    # 复制二进制文件
    if [ -f "{{CACHE_TARGET}}/release/{{PROJECT_NAME}}" ]; then
        cp "{{CACHE_TARGET}}/release/{{PROJECT_NAME}}" "{{LOCAL_TARGET}}/release/"
        echo "✅ 二进制: {{LOCAL_TARGET}}/release/{{PROJECT_NAME}}"
        ls -lh "{{LOCAL_TARGET}}/release/{{PROJECT_NAME}}"
    fi

# 构建并运行
run args="":
    just build
    echo "🚀 运行..."
    ./{{LOCAL_TARGET}}/debug/{{PROJECT_NAME}} {{args}}

# 清理本地产物（保留缓存）
clean:
    #!/bin/bash
    set -e
    echo "🧹 清理本地产物..."
    rm -rf {{LOCAL_TARGET}}
    echo "✅ 已清理，缓存保留在 {{CACHE_TARGET}}"

# 完全清理（包括缓存）
clean-all:
    #!/bin/bash
    set -e
    echo "🧹 清理所有产物和缓存..."
    rm -rf {{LOCAL_TARGET}}
    cargo clean
    echo "✅ 已完全清理"

# 查看缓存统计
cache-stats:
    #!/bin/bash
    echo "📊 缓存统计:"
    echo "  缓存目录: {{CACHE_TARGET}}"
    du -sh {{CACHE_TARGET}} 2>/dev/null || echo "  缓存为空"
    echo ""
    echo "📊 本地产物:"
    du -sh {{LOCAL_TARGET}} 2>/dev/null || echo "  本地为空"

# 显示配置信息
info:
    #!/bin/bash
    echo "📋 构建配置:"
    echo "  项目名称: {{PROJECT_NAME}}"
    echo "  缓存目录: {{CACHE_TARGET}}"
    echo "  本地目录: {{LOCAL_TARGET}}"
    echo ""
    echo "📋 最终产物位置:"
    if [ -f "{{LOCAL_TARGET}}/debug/{{PROJECT_NAME}}" ]; then
        ls -lh "{{LOCAL_TARGET}}/debug/{{PROJECT_NAME}}"
    else
        echo "  (未构建)"
    fi

# 测试构建（清理后重新构建）
test:
    #!/bin/bash
    set -e
    echo "🧪 清理本地产物..."
    rm -rf {{LOCAL_TARGET}}
    echo "📦 重新构建..."
    just build
    echo ""
    echo "📊 构建后统计:"
    just cache-stats
