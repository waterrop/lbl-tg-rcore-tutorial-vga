# lbl-tg-rcore-tutorial-vga 详细设计文档

## 1. 组件概述

`lbl-tg-rcore-tutorial-vga` 是一个面向 rCore-Tutorial 教学环境的最小图形显示组件，运行于 `riscv64gc-unknown-none-elf` 裸机目标，负责驱动 QEMU 提供的 `ramfb` 风格线性帧缓冲显示设备，并通过 `gtk` 图形后端将客户系统写入的像素数据显示到宿主机窗口。

该组件的设计目标如下：

- 为 RISC-V 64 裸机内核提供可复用的基础图形输出能力
- 在 `#![no_std]` 环境下完成显示设备初始化、像素写入和整屏清除
- 维护帧缓冲物理地址、大小、分辨率、stride、像素格式等核心信息
- 不依赖动态内存分配，不引入复杂图形协议栈，适合作为内核早期图形 bring-up 组件

组件提供的对外能力严格约束为以下最小接口：

```rust
pub fn init() -> Result<(), VgaError>
pub fn draw_pixel(x: u32, y: u32, color: u32) -> Result<(), VgaError>
pub fn clear_screen(color: u32) -> Result<(), VgaError>
```

同时提供只读辅助接口，用于获取当前显示分辨率和帧缓冲布局信息。

## 2. 硬件环境

### 2.1 目标平台

- CPU 架构：RISC-V 64
- 运行方式：QEMU `virt` 机器模型
- 执行模式：S 态或 M 态裸机内核，取决于上层教学框架
- 图形输出链路：Guest 内核驱动 → QEMU 虚拟显示设备 → QEMU `gtk` 后端 → 宿主机窗口

### 2.2 虚拟显示设备模型

本设计采用简单线性 framebuffer 设备模型。设备在 MMIO 配置空间暴露以下信息：

- 设备状态寄存器
- 当前分辨率：宽、高
- 行跨度 `stride`
- 帧缓冲物理地址
- 帧缓冲总大小
- 像素格式
- 使能命令寄存器

驱动初始化后，内核可直接向帧缓冲物理地址写入 32 位像素值，QEMU 检测到显存变化后交由 `gtk` 窗口后端显示。

### 2.3 建议的 MMIO 寄存器布局

为适配本项目当前实现与教学实验，约定显示设备 MMIO 基地址为 `0x1000_8000`，寄存器布局如下：

| 偏移 | 寄存器 | 位宽 | 含义 |
| --- | --- | --- | --- |
| `0x00` | `STATUS` | 32 | 设备状态，`bit0=1` 表示就绪 |
| `0x04` | `WIDTH` | 32 | 可见宽度，单位像素 |
| `0x08` | `HEIGHT` | 32 | 可见高度，单位像素 |
| `0x0C` | `STRIDE` | 32 | 每行像素跨度，单位像素 |
| `0x10` | `FB_ADDR_LOW` | 32 | 帧缓冲物理地址低 32 位 |
| `0x14` | `FB_ADDR_HIGH` | 32 | 帧缓冲物理地址高 32 位 |
| `0x18` | `FB_SIZE` | 32 | 帧缓冲总大小，单位字节 |
| `0x1C` | `FORMAT` | 32 | 像素格式 |
| `0x20` | `COMMAND` | 32 | 控制寄存器，`bit0=1` 表示使能 |

像素格式约定如下：

- `0`：`XRGB8888`
- `1`：`ARGB8888`

二者都按每像素 4 字节处理，供 `draw_pixel` 与 `clear_screen` 使用。

## 3. 总体架构设计

### 3.1 分层结构

组件采用四层结构，保证最小职责分离：

1. `mmio` 层  
   负责寄存器和帧缓冲的 volatile 读写，不包含业务逻辑。
2. `device` 层  
   负责显示设备探测、寄存器解析、格式校验和设备使能。
3. `framebuffer` 层  
   负责像素坐标到线性显存偏移的转换，以及像素写入和整屏清除。
4. `lib` 门面层  
   负责维护全局显示状态、提供稳定的公共 API、屏蔽底层初始化细节。

### 3.2 模块划分

建议模块组织如下：

```text
src/
├── lib.rs           // 对外 API 与全局状态
├── device.rs        // ramfb 设备探测与寄存器读取
├── framebuffer.rs   // 帧缓冲信息与绘制逻辑
├── mmio.rs          // MMIO 读写抽象
└── error.rs         // VgaError 定义
```

### 3.3 数据流

组件内部数据流如下：

```text
init()
  -> 使能 MMIO 设备
  -> 读取 STATUS/WIDTH/HEIGHT/STRIDE/FB_ADDR/FB_SIZE/FORMAT
  -> 校验显示参数
  -> 构造 FramebufferInfo
  -> 写入全局只读状态
  -> clear_screen(0x00000000)

draw_pixel(x, y, color)
  -> 读取全局 FramebufferInfo
  -> 检查初始化状态
  -> 检查坐标范围
  -> 计算字节偏移
  -> volatile 写入 framebuffer

clear_screen(color)
  -> 读取全局 FramebufferInfo
  -> 线性遍历整个 framebuffer
  -> 逐像素 volatile 写入
```

### 3.4 设计原则

- 最小接口：不暴露复杂图形对象，仅暴露初始化与基础像素操作
- 最小抽象：仅抽象寄存器、显示布局和错误类型
- 只读共享：初始化后显示元数据保持不可变，减少并发复杂度
- 无堆分配：所有状态使用静态存储
- 显式校验：任何分辨率、地址、大小和格式异常必须在初始化阶段返回错误

## 4. 核心数据结构定义

### 4.1 帧缓冲描述结构

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FramebufferInfo {
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub framebuffer_paddr: usize,
    pub framebuffer_size: usize,
    pub bytes_per_pixel: u8,
    pub pixel_format: PixelFormat,
}
```

字段说明：

- `width`：可见宽度
- `height`：可见高度
- `stride`：每行逻辑跨度，单位像素
- `framebuffer_paddr`：帧缓冲物理基地址
- `framebuffer_size`：帧缓冲总大小，单位字节
- `bytes_per_pixel`：当前固定为 `4`
- `pixel_format`：像素格式枚举

### 4.2 像素格式枚举

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PixelFormat {
    Xrgb8888,
    Argb8888,
}
```

对外 `color` 参数统一定义为 32 位颜色值。对于 `XRGB8888`，最高 8 位保留；对于 `ARGB8888`，按 alpha-red-green-blue 解释。由于 QEMU 简单 framebuffer 设备通常只关心颜色字节布局，因此驱动仅保证原样写入 32 位数据，不在驱动内部做混色。

### 4.3 全局状态结构

```rust
struct DisplayState {
    init_state: AtomicU8,
    info: UnsafeCell<FramebufferInfo>,
}
```

初始化状态建议使用三态：

- `0`：未初始化
- `1`：初始化中
- `2`：初始化完成

设计理由：

- 防止重复初始化覆盖状态
- 避免初始化过程中被其他调用方读取到半初始化数据
- 满足单核教学环境与后续多核扩展的兼容性

### 4.4 错误类型

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VgaError {
    DeviceNotReady,
    InvalidResolution,
    InvalidPixelPosition,
    InvalidFramebuffer,
    UnsupportedFormat,
    MmioFault,
    NotInitialized,
    AlreadyInitialized,
}
```

其中 `AlreadyInitialized` 建议作为重复初始化保护错误；如实现希望允许幂等初始化，也可改为“重复调用直接返回 `Ok(())`”，但必须保证不会重新写坏全局状态。

## 5. 对外 API 接口详细设计

### 5.1 初始化接口

```rust
pub fn init() -> Result<(), VgaError>
```

职责：

- 检测并使能 `ramfb` 显示设备
- 读取设备寄存器
- 校验分辨率、stride、地址、大小和像素格式
- 构建并保存 `FramebufferInfo`
- 使用默认背景色清空屏幕

输入：

- 无

输出：

- 成功返回 `Ok(())`
- 失败返回 `VgaError`

语义约束：

- 必须先于任何绘图接口调用
- 成功后组件进入可用状态
- 若检测到设备未就绪、地址非法或格式不支持，必须立即失败

### 5.2 像素绘制接口

```rust
pub fn draw_pixel(x: u32, y: u32, color: u32) -> Result<(), VgaError>
```

职责：

- 将二维坐标转换为帧缓冲线性偏移
- 向指定像素地址写入一个 32 位颜色值

输入：

- `x`：横坐标，范围 `[0, width)`
- `y`：纵坐标，范围 `[0, height)`
- `color`：32 位颜色值

输出：

- 成功返回 `Ok(())`
- 越界、未初始化、地址溢出等情况返回错误

实现约束：

- 必须使用边界检查
- 必须使用溢出安全的偏移计算
- 必须使用 volatile 写入，防止编译器优化丢失显存写操作

偏移计算公式：

```text
pixel_index = y * stride + x
byte_offset = pixel_index * bytes_per_pixel
pixel_addr  = framebuffer_paddr + byte_offset
```

### 5.3 清屏接口

```rust
pub fn clear_screen(color: u32) -> Result<(), VgaError>
```

职责：

- 使用指定颜色填充整个帧缓冲区

处理方式：

- 以 `framebuffer_size / bytes_per_pixel` 为循环次数顺序写入
- 或以 `(height, stride)` 二重循环写入可见区域

建议策略：

- 若目标是“使整个硬件缓冲区保持一致”，使用 `framebuffer_size` 遍历
- 若目标是“仅清空可见区域”，使用 `height * stride` 遍历

本项目建议采用第一种，即按整个缓冲区写入，原因如下：

- 可避免尾部未初始化区域残留旧数据
- 与简单线性 framebuffer 的设备模型更一致
- 实现简单，适合教学环境

### 5.4 辅助只读接口

建议提供以下辅助接口：

```rust
pub fn resolution() -> Result<(u32, u32), VgaError>
pub fn framebuffer_info() -> Result<FramebufferInfo, VgaError>
```

职责：

- 为上层内核或图形子系统提供显示能力探测入口
- 保持状态只读，避免外部直接修改帧缓冲元数据

返回复制值而不是可变引用，便于维持封装边界。

## 6. 初始化流程设计

### 6.1 启动时序

初始化流程定义如下：

1. 进入 `init()`
2. 检查全局状态是否已初始化
3. 向 `COMMAND` 写入 `ENABLE`
4. 读取 `STATUS`
5. 读取 `WIDTH`、`HEIGHT`、`STRIDE`
6. 读取 `FB_ADDR_LOW/HIGH` 并合成 64 位物理地址
7. 读取 `FB_SIZE`
8. 读取 `FORMAT`
9. 执行参数合法性校验
10. 记录 `FramebufferInfo`
11. 调用 `clear_screen(0x0000_0000)` 清屏
12. 将状态切换为初始化完成

### 6.2 初始化流程图

```text
init
 ├─ 检查 init_state
 │   ├─ 已完成 -> 返回 Ok 或 AlreadyInitialized
 │   └─ 未初始化 -> 继续
 ├─ 写 COMMAND=ENABLE
 ├─ 读 STATUS
 │   └─ 未就绪 -> DeviceNotReady
 ├─ 读取宽高/stride
 │   └─ 非法 -> InvalidResolution
 ├─ 读取 framebuffer 地址和大小
 │   └─ 非法 -> InvalidFramebuffer
 ├─ 读取 FORMAT
 │   └─ 不支持 -> UnsupportedFormat
 ├─ 保存 FramebufferInfo
 ├─ 清屏
 └─ 返回 Ok
```

### 6.3 合法性校验规则

初始化阶段必须至少完成以下校验：

- `width > 0`
- `height > 0`
- `stride >= width`
- `framebuffer_paddr != 0`
- `framebuffer_size != 0`
- `bytes_per_pixel == 4`
- `framebuffer_size >= stride * height * bytes_per_pixel`
- 相关乘法和加法均不得溢出 `usize`

## 7. 帧缓冲管理机制

### 7.1 物理地址管理

本组件不负责分配 framebuffer，而是使用设备暴露的物理地址。驱动只做三件事：

- 读取设备提供的 framebuffer 基址
- 记录该物理地址及大小
- 按物理地址直接写入像素

在 rCore-Tutorial 这类恒等映射或线性映射教学环境下，可直接把该物理地址视为当前可访问地址；如果上层内核启用了分页并区分物理地址与虚拟地址，则应在集成层增加地址转换逻辑，再把可访问的线性地址传给本组件。

### 7.2 坐标到偏移的映射

帧缓冲使用行优先线性布局：

```text
byte_offset = ((y * stride) + x) * bytes_per_pixel
```

注意事项：

- `stride` 可能大于 `width`，因此不能用 `width` 替代
- 所有计算必须使用 `checked_mul` 和 `checked_add`
- 计算出的 `end_offset` 不能超过 `framebuffer_size`

### 7.3 清屏策略

清屏通过线性遍历写入统一颜色值实现：

- 时间复杂度：`O(framebuffer_size / 4)`
- 空间复杂度：`O(1)`

该策略简单稳定，适合裸机早期阶段。后续如果需要优化，可增加如下增强路径：

- 使用机器字宽批量写入
- 使用缓存行对齐优化
- 仅清理脏区而非全屏

这些优化不属于本组件首版范围。

### 7.4 可见区域与总缓冲区

为保证设计清晰，区分以下两个概念：

- 可见区域大小：`height * stride * bytes_per_pixel`
- 实际缓冲区大小：`framebuffer_size`

约束为：

```text
framebuffer_size >= height * stride * bytes_per_pixel
```

若存在额外 padding，本组件应允许其存在，但在像素写入时只允许访问有效坐标对应区域。

## 8. 错误处理设计

### 8.1 错误分类

| 错误 | 触发阶段 | 含义 |
| --- | --- | --- |
| `NotInitialized` | 绘图阶段 | 初始化前调用绘图接口 |
| `AlreadyInitialized` | 初始化阶段 | 重复初始化且策略为非幂等 |
| `DeviceNotReady` | 初始化阶段 | 设备状态未就绪 |
| `InvalidResolution` | 初始化阶段 | 分辨率或 stride 非法 |
| `InvalidFramebuffer` | 初始化或绘图阶段 | 地址、大小或范围不合法 |
| `InvalidPixelPosition` | 绘图阶段 | 坐标越界 |
| `UnsupportedFormat` | 初始化或绘图阶段 | 像素格式不支持 |
| `MmioFault` | 设备访问阶段 | MMIO 访问异常或平台不支持 |

### 8.2 错误处理原则

- 不使用 `panic!` 作为常规错误处理路径
- 所有外部接口统一返回 `Result`
- 所有计算溢出一律视为 `InvalidFramebuffer`
- 初始化失败后状态必须回退为“未初始化”或保持不可用状态

### 8.3 故障隔离

组件只处理自身可判定的硬件参数错误，不处理上层内核的内存管理错误。例如：

- 若分页未正确映射 framebuffer，属于上层集成问题
- 若 QEMU 设备未按约定暴露寄存器，属于运行环境问题

组件在这类场景下只返回可观测错误码，不承担恢复职责。

## 9. 裸机适配与无 std 设计

### 9.1 `#![no_std]` 适配

本组件必须满足以下约束：

- 仅依赖 `core`
- 不使用堆分配
- 不依赖文件系统、线程库、时间库或同步原语库
- 不使用格式化日志作为核心执行路径的一部分

### 9.2 MMIO 访问方式

所有寄存器和显存写入必须通过 volatile 语义完成：

```rust
read_volatile(addr)
write_volatile(addr, value)
```

原因：

- MMIO 和 framebuffer 属于具有副作用的外设内存
- 普通读写可能被编译器重排或消除
- volatile 能保证每次访问都保留为真实总线操作

### 9.3 线程安全策略

在 rCore-Tutorial 教学环境下，通常由单 hart 在启动阶段完成 `init()`，后续执行读写操作。为满足线程安全约束，本设计采用以下策略：

- 初始化状态使用原子变量控制
- 全局显示信息仅在初始化时写入一次
- 初始化完成后只读共享
- 绘图接口不进行堆分配和可重入状态修改

如果未来扩展到多核并发绘图，建议在上层引入更细粒度的同步机制；本组件首版不实现复杂锁。

### 9.4 无 panic 风险约束

必须避免以下风险：

- 数组或切片越界
- 未检查的整数溢出
- 未初始化状态下访问全局显示信息
- 由于空指针或地址错误导致的显存越界写入

因此，所有地址计算必须显式检查，所有接口必须先校验初始化状态。

## 10. 集成使用示例

### 10.1 典型调用顺序

```rust
use lbl_tg_rcore_tutorial_vga::{
    clear_screen, draw_pixel, framebuffer_info, init, resolution,
};

pub fn kernel_graphics_init() -> Result<(), VgaError> {
    init()?;

    let (width, height) = resolution()?;
    clear_screen(0x0000_0000)?;

    draw_pixel(0, 0, 0x00ff_0000)?;
    draw_pixel(width - 1, 0, 0x0000_ff00)?;
    draw_pixel(0, height - 1, 0x0000_00ff)?;
    draw_pixel(width / 2, height / 2, 0x00ff_ffff)?;

    let _info = framebuffer_info()?;
    Ok(())
}
```

### 10.2 rCore-Tutorial 集成建议

- 在内核早期初始化阶段调用 `init()`
- 在页表启用前后保持 framebuffer 地址可访问
- 若引入地址空间隔离，在平台层提供物理地址到内核虚拟地址的映射
- 保持绘图接口纯同步调用，避免在中断上下文中执行大面积清屏

### 10.3 QEMU 启动建议

建议以“RISC-V 虚拟机 + 简单 framebuffer 设备 + gtk 后端”的组合运行，验证链路如下：

```text
内核 draw_pixel/clear_screen
  -> framebuffer 显存变化
  -> QEMU 显示设备模型更新
  -> gtk 窗口刷新
```

该组合最适合课程实验和基础驱动 bring-up。

## 11. 约束与限制

本组件明确具有以下限制：

- 仅支持 32 位像素格式，首版只接受 `XRGB8888` 和 `ARGB8888`
- 仅支持单显示面、单 framebuffer，不支持多显示输出
- 不支持硬件加速、2D blit、字体、窗口系统和合成器
- 不支持旋转、缩放、alpha 混合和双缓冲管理
- 不处理复杂中断机制，仅采用“写显存即显示”的简单模型
- 默认面向 rCore-Tutorial 的单核早期图形场景，不覆盖完整桌面图形栈需求

## 12. 结论

`lbl-tg-rcore-tutorial-vga` 的首版设计定位为一个最小、可靠、可复用的裸机图形组件。其核心在于：

- 使用 MMIO 完成显示设备发现与初始化
- 使用只读 `FramebufferInfo` 维护稳定的显示元数据
- 使用线性 framebuffer 实现像素绘制和清屏
- 使用 `Result` + `VgaError` 进行可恢复错误传播
- 使用 `no_std`、无堆分配、最小抽象适配 rCore-Tutorial 教学内核

该设计可以直接指导后续代码实现，也为将来扩展更复杂的图形能力保留了清晰边界。
