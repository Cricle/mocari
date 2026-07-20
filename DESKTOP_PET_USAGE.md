# 桌面宠物使用指南

## 快速开始

```bash
cargo run --example desktop_pet --features wgpu --release
```

## 预期行为

运行桌宠后，你应该看到：

### ✅ 自动动画
1. **眨眼动画**: 每 2.5-6 秒自动眨眼一次
2. **呼吸动画**: 持续的呼吸动作
3. **鼠标追踪**: 模型眼睛跟随鼠标移动
4. **口型同步**: 可通过 API 控制（需要音频输入）

### ✅ 交互功能
- **拖动窗口**: 点击窗口任意位置拖动
- **退出**: 按 ESC 键

### ⚙️ 自定义配置

编辑 `examples/desktop_pet.rs`:

```rust
use mocari::engine::DesktopPetConfig;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    mocari::engine::run_desktop_pet_with_config(
        "assets/models/Ren/Ren.model3.json",
        DesktopPetConfig::new()
            .size(800, 800)           // 增大窗口减少锯齿
            .title("My Live2D Pet")   // 自定义标题
            .transparent(true)        // 透明背景
            .decorations(false)       // 无边框
            .always_on_top(true)      // 始终置顶
            .click_through(false),    // 允许交互
    )?;
    Ok(())
}
```

## 故障排查

### 问题: 模型不会动

**检查清单**:
1. 确保使用 `--release` 模式编译
2. 检查模型是否有标准参数名（ParamEyeLOpen, ParamEyeROpen 等）
3. 查看终端调试输出

**调试命令**:
```bash
RUST_LOG=debug cargo run --example desktop_pet --features wgpu --release 2>&1 | grep DEBUG
```

### 问题: 锯齿严重

**解决方案**:
1. 增大窗口尺寸（推荐 800x800 或更大）
2. MSAA 4x 已默认启用
3. 确保使用高分辨率模型纹理

### 问题: 内存占用高 (~230MB)

**说明**: 这是正常的，包括：
- GPU 纹理内存: ~64 MB
- 模型运行时: ~30 MB
- wgpu 上下文: ~40 MB
- MSAA 缓冲: ~16 MB
- 其他: ~80 MB

## 技术细节

### 自动动画系统

桌宠启动时自动启用以下系统：

```rust
engine.configure_eye_blink(&handle, Some(Default::default()));
engine.configure_breath(&handle, Some(Default::default()));
engine.configure_lip_sync(&handle, Some(Default::default()));
engine.configure_mouse_tracker(&handle, Some(Default::default()));
```

### 事件循环模式

- 使用持续轮询模式（非 Wait 模式）
- 确保动画持续更新
- 通过 `needs_continuous_redraw()` 优化性能

### 渲染配置

- **MSAA**: 4x 多重采样抗锯齿
- **格式**: Rgba8Unorm 纹理
- **帧率**: 根据动画需求动态调整

## 支持的模型

兼容标准 Cubism SDK 4 模型：
- `.model3.json` 格式
- `.moc3` 运行时
- `.physics3.json` 物理（可选）
- `.motion3.json` 动作（可选）
- `.exp3.json` 表情（可选）

## 性能提示

1. **Release 模式**: 始终使用 `--release` 编译
2. **窗口大小**: 较小窗口性能更好，但可能有锯齿
3. **模型复杂度**: 更多多边形 = 更多 GPU 开销
