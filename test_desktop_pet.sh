#!/bin/bash
set -e

echo "=========================================="
echo "Desktop Pet 测试脚本"
echo "=========================================="
echo ""

# 检查模型文件
echo "1. 检查模型文件..."
if [ -f "assets/models/Ren/Ren.model3.json" ]; then
    echo "   ✓ 模型文件存在"
else
    echo "   ✗ 模型文件不存在！"
    exit 1
fi

# 编译
echo ""
echo "2. 编译 desktop_pet 示例 (release 模式)..."
cargo build --example desktop_pet --features wgpu --release 2>&1 | tail -3

if [ $? -eq 0 ]; then
    echo "   ✓ 编译成功"
else
    echo "   ✗ 编译失败！"
    exit 1
fi

# 检查二进制文件
echo ""
echo "3. 检查生成的二进制文件..."
BINARY="target/release/examples/desktop_pet"
if [ -f "$BINARY" ]; then
    SIZE=$(du -h "$BINARY" | cut -f1)
    echo "   ✓ 二进制文件: $BINARY ($SIZE)"
else
    echo "   ✗ 二进制文件不存在！"
    exit 1
fi

# 运行测试
echo ""
echo "4. 启动桌面宠物 (10秒后自动关闭)..."
echo "   请观察："
echo "   - 模型是否显示"
echo "   - 是否有眨眼动画 (2.5-6秒一次)"
echo "   - 是否有呼吸动画"
echo "   - 鼠标移动时眼睛是否追踪"
echo "   - 是否可以拖动窗口"
echo ""
echo "   按 ESC 键可以提前退出"
echo ""

# 运行并捕获调试输出
timeout 10s $BINARY 2>&1 | grep -E "\[DEBUG\]|failed|error" || true

echo ""
echo "=========================================="
echo "测试完成"
echo "=========================================="
echo ""
echo "如果模型不会动，请检查："
echo "1. 调试输出中是否显示 'eye_blink: true'"
echo "2. 调试输出中是否显示 'breath: true'"
echo "3. 模型是否有 ParamEyeLOpen/ParamEyeROpen 参数"
echo ""
echo "如果锯齿严重，尝试："
echo "1. 增大窗口尺寸到 800x800"
echo "2. 使用高分辨率显示器"
echo "3. 检查 MSAA 是否启用 (应该默认为 4x)"
