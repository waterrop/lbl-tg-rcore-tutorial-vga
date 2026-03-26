# lbl-tg-rcore-tutorial-vga 使用说明

## 1. 组件简介

`lbl-tg-rcore-tutorial-vga` 是一个面向 RISC-V 64 裸机内核和 rCore-Tutorial 教学环境的最小图形显示组件，工作在 `#![no_std]` 环境下，基于 QEMU `virt` 机器的 `ramfb` 虚拟显示设备提供基础图形输出能力。

该组件适合以下场景：

- 在 QEMU 中为裸机内核提供最小图形输出
- 在 rCore-Tutorial 中完成图形 bring-up 实验
- 为后续字符绘制、窗口系统、简单 UI 组件提供底层像素接口

当前组件提供以下公共接口：

```rust
pub fn init() -> Result<(), VgaError>
pub fn draw_pixel(x: u32, y: u32, color: u32) -> Result<(), VgaError>
pub fn clear_screen(color: u32) -> Result<(), VgaError>
pub fn resolution() -> Result<(u32, u32), VgaError>
pub fn framebuffer_info() -> Result<FramebufferInfo, VgaError>
```

## 2. 依赖与适用环境

### 2.1 适用环境

- 目标架构：`riscv64gc-unknown-none-elf`
- 运行环境：QEMU RISC-V 64 `virt`
- 图形设备：`ramfb`
- 图形后端：`gtk`
- Rust 约束：`#![no_std]`

### 2.2 组件依赖

本组件自身依赖极少：

- `core`
- `tg-sbi`

其中 `tg-sbi` 主要用于示例入口中的串口输出和关机控制；如果你只把本组件作为库集成到自己的内核中，核心图形接口本身不依赖标准库，也不要求动态内存分配。

## 3. 快速集成步骤

### 3.1 Cargo 配置

如果你的内核工程和本组件位于同一工作区或本地目录，推荐使用路径依赖：

```toml
[dependencies]
lbl-tg-rcore-tutorial-vga = { path = "/home/hdu/study/rust/2026s-ai4ose-lab/lbl-tg-rcore-tutorial-vga" }
```

如果你的工程已经是 `no_std` 裸机内核，通常还需要确认目标平台为 RISC-V 64：

```toml
[build]
target = "riscv64gc-unknown-none-elf"
```

### 3.2 内核引入方式

在内核代码中引入本组件：

```rust
use lbl_tg_rcore_tutorial_vga::{
    clear_screen, draw_pixel, framebuffer_info, init, resolution, VgaError,
};
```

### 3.3 初始化时机

建议在以下时机调用：

- 平台基础初始化完成后
- 帧缓冲内存地址可访问后
- 正式进入图形输出逻辑前

不建议在多核并发阶段首次初始化本组件。推荐由单 hart 在启动早期完成一次初始化。

## 4. 初始化接口使用方法

初始化接口如下：

```rust
pub fn init() -> Result<(), VgaError>
```

接口行为：

- 通过 `fw_cfg DMA` 探测 QEMU `etc/ramfb`
- 配置 640×480、XRGB8888 帧缓冲
- 自动完成一次黑色清屏
- 初始化成功后即可调用绘图接口

基本用法：

```rust
use lbl_tg_rcore_tutorial_vga::{init, VgaError};

fn init_graphics() -> Result<(), VgaError> {
    init()?;
    Ok(())
}
```

错误处理建议：

```rust
use lbl_tg_rcore_tutorial_vga::{init, VgaError};

fn init_graphics() {
    match init() {
        Ok(()) => {}
        Err(VgaError::AlreadyInitialized) => {}
        Err(_error) => {
            panic!("graphics init failed");
        }
    }
}
```

## 5. 像素绘制接口使用方法

像素绘制接口如下：

```rust
pub fn draw_pixel(x: u32, y: u32, color: u32) -> Result<(), VgaError>
```

颜色格式使用 32 位整数表示，当前组件按 XRGB8888 写入：

```text
0x00RRGGBB
```

常见颜色示例：

- 红色：`0x00ff_0000`
- 绿色：`0x0000_ff00`
- 蓝色：`0x0000_00ff`
- 黄色：`0x00ff_ff00`
- 青色：`0x0000_ffff`
- 白色：`0x00ff_ffff`
- 黑色：`0x0000_0000`

绘制单个像素示例：

```rust
use lbl_tg_rcore_tutorial_vga::{draw_pixel, VgaError};

fn draw_demo_pixel() -> Result<(), VgaError> {
    draw_pixel(10, 10, 0x00ff_0000)?;
    Ok(())
}
```

绘制四角像素示例：

```rust
use lbl_tg_rcore_tutorial_vga::{draw_pixel, resolution, VgaError};

fn draw_corners() -> Result<(), VgaError> {
    let (width, height) = resolution()?;
    let max_x = width - 1;
    let max_y = height - 1;

    draw_pixel(0, 0, 0x00ff_0000)?;
    draw_pixel(max_x, 0, 0x0000_ff00)?;
    draw_pixel(0, max_y, 0x0000_00ff)?;
    draw_pixel(max_x, max_y, 0x00ff_ff00)?;
    Ok(())
}
```

## 6. 清屏接口使用方法

清屏接口如下：

```rust
pub fn clear_screen(color: u32) -> Result<(), VgaError>
```

基本用法：

```rust
use lbl_tg_rcore_tutorial_vga::{clear_screen, VgaError};

fn clear_to_black() -> Result<(), VgaError> {
    clear_screen(0x0000_0000)?;
    Ok(())
}
```

用纯蓝色清屏：

```rust
use lbl_tg_rcore_tutorial_vga::{clear_screen, VgaError};

fn clear_to_blue() -> Result<(), VgaError> {
    clear_screen(0x0000_00ff)?;
    Ok(())
}
```

说明：

- `clear_screen` 会遍历整个帧缓冲并逐像素写入
- 适合初始化背景、场景切换或测试显示链路
- 若在高频路径中频繁调用，可能带来明显性能开销

## 7. 辅助信息获取接口使用

### 7.1 获取分辨率

```rust
use lbl_tg_rcore_tutorial_vga::{resolution, VgaError};

fn read_resolution() -> Result<(u32, u32), VgaError> {
    let (width, height) = resolution()?;
    Ok((width, height))
}
```

### 7.2 获取帧缓冲信息

```rust
use lbl_tg_rcore_tutorial_vga::{framebuffer_info, VgaError};

fn read_framebuffer() -> Result<(), VgaError> {
    let info = framebuffer_info()?;
    let _width = info.width;
    let _height = info.height;
    let _stride = info.stride;
    let _paddr = info.framebuffer_paddr;
    let _size = info.framebuffer_size;
    Ok(())
}
```

`FramebufferInfo` 主要包含：

- `width`
- `height`
- `stride`
- `framebuffer_paddr`
- `framebuffer_size`
- `bytes_per_pixel`
- `pixel_format`

适合用于：

- 上层图形模块查询显示参数
- 内核日志打印
- 后续字符绘制、位图显示模块复用

## 8. 完整可运行调用示例代码

下面给出一个最小可运行的调用示例，展示初始化、清屏、绘制十字线和中心点：

```rust
#![no_std]

use lbl_tg_rcore_tutorial_vga::{
    clear_screen, draw_pixel, init, resolution, VgaError,
};

pub fn graphics_demo() -> Result<(), VgaError> {
    init()?;

    let (width, height) = resolution()?;
    let center_x = width / 2;
    let center_y = height / 2;

    clear_screen(0x0000_0000)?;

    for x in 0..width {
        draw_pixel(x, center_y, 0x0000_ffff)?;
    }

    for y in 0..height {
        draw_pixel(center_x, y, 0x0000_ffff)?;
    }

    draw_pixel(center_x, center_y, 0x00ff_ffff)?;
    Ok(())
}
```

如果你想直接使用本仓库自带的演示入口，可以参考当前示例实现 [main.rs](file:///home/hdu/study/rust/2026s-ai4ose-lab/lbl-tg-rcore-tutorial-vga/src/main.rs#L1-L111)。

## 9. QEMU 启动参数配置

### 9.1 当前仓库默认配置

当前仓库已经在 [.cargo/config.toml](file:///home/hdu/study/rust/2026s-ai4ose-lab/lbl-tg-rcore-tutorial-vga/.cargo/config.toml#L1-L20) 中配置了图形运行参数，核心参数如下：

```bash
qemu-system-riscv64 \
  -machine virt \
  -m 128M \
  -serial stdio \
  -monitor none \
  -display gtk \
  -device ramfb \
  -bios none \
  -kernel <kernel-elf>
```

### 9.2 推荐运行方式

直接使用 Cargo：

```bash
cargo run
```

或者使用仓库自带脚本 [run-graphic.sh](file:///home/hdu/study/rust/2026s-ai4ose-lab/lbl-tg-rcore-tutorial-vga/run-graphic.sh#L1-L31)：

```bash
./run-graphic.sh
```

可选环境变量：

```bash
QEMU_DISPLAY=gtk ./run-graphic.sh
PROFILE=release ./run-graphic.sh
QEMU_MEMORY=256M ./run-graphic.sh
```

## 10. 常见问题与注意事项

### 10.1 为什么 `init()` 之后再次调用会失败

当前实现采用一次性初始化策略。重复调用会返回 `VgaError::AlreadyInitialized`。

### 10.2 为什么绘图前必须先调用 `init()`

因为只有在 `init()` 成功后，组件才会完成：

- `ramfb` 配置
- 帧缓冲地址建立
- 全局显示信息写入

如果未初始化直接调用绘图接口，会返回 `VgaError::NotInitialized`。

### 10.3 为什么看不到图形窗口

请检查以下几点：

- 宿主机是否支持 `gtk`
- 是否使用了 `-display gtk`
- 是否带上了 `-device ramfb`
- 程序是否在绘制后立即退出

本仓库当前示例会在绘制完成后保持运行，因此窗口应持续可见。

### 10.4 为什么不是传统 MMIO VGA 驱动

当前实现接入的是 QEMU `ramfb` 的真实工作方式：通过 `fw_cfg DMA` 将帧缓冲配置写入 `etc/ramfb`，而不是访问一组自定义 VGA 寄存器。

这更符合 QEMU `ramfb` 的实际设备模型，也更适合 rCore-Tutorial 教学环境。

### 10.5 使用时的注意事项

- 该组件当前固定使用 640×480、XRGB8888 帧缓冲
- 当前适合单显示面、单缓冲、基础像素绘制场景
- `clear_screen` 是全缓冲遍历操作，不适合高频刷新路径
- 若后续接入分页映射或更复杂地址空间模型，需要确认帧缓冲地址在当前内核上下文中可访问

## 11. 小结

集成本组件的推荐流程如下：

1. 在内核工程中加入 `lbl-tg-rcore-tutorial-vga` 依赖
2. 启动早期调用 `init()`
3. 通过 `clear_screen()` 和 `draw_pixel()` 进行基础图形输出
4. 通过 `resolution()` 和 `framebuffer_info()` 获取显示元数据
5. 使用 `cargo run` 或 `./run-graphic.sh` 在 QEMU `gtk + ramfb` 环境中验证结果

如果你要进一步扩展字符渲染、位图绘制或简单窗口系统，建议直接以 `draw_pixel()` 为底层基础接口继续封装。
