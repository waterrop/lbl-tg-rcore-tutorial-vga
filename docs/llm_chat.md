请生成一份Rust操作系统组件的详细设计文档，文档格式为Markdown，保存路径为：/home/hdu/study/rust/2026s-ai4ose-lab/lbl-tg-rcore-tutorial-vga/docs/vga-design.md

组件基本信息
1. 组件名称：lbl-tg-rcore-tutorial-vga
2. 编程语言：Rust，无标准库(std)依赖，仅使用core核心库
3. 目标架构：RISC-V 64 (rv64gc)
4. 运行环境：QEMU RISC-V 64 虚拟机
5. 虚拟硬件：QEMU ramfb 虚拟显示设备（VGA兼容帧缓冲）
6. 图形后端：gtk

功能需求
1. 实现QEMU ramfb虚拟显示设备驱动，完成硬件初始化
2. 实现帧缓冲（framebuffer）管理：维护帧缓冲物理地址、大小、分辨率、行跨度等核心信息
3. 提供可复用的图形显示接口，供其他RISC-V内核调用

对外接口定义（必须严格实现）
1. 初始化接口：pub fn init() -> Result<(), VgaError>
   功能：检测ramfb硬件、映射帧缓冲地址、初始化分辨率、清空屏幕
2. 像素绘制接口：pub fn draw_pixel(x: u32, y: u32, color: u32) -> Result<(), VgaError>
   功能：在指定(x,y)坐标写入32位RGBA像素数据
3. 清屏接口：pub fn clear_screen(color: u32) -> Result<(), VgaError>
   功能：使用指定颜色清空整个屏幕
4. 辅助接口：提供获取分辨率、帧缓冲信息的只读接口

核心设计约束
1. 无标准库依赖(#![no_std])，适配裸机RISC-V内核环境
2. 仅提供最小硬件抽象：帧缓冲地址、分辨率、行跨度、像素格式
3. 线程安全、无动态内存分配、无panic安全风险
4. 代码可复用、模块化，可直接集成到rCore等RISC-V操作系统内核

设计文档必须包含的章节
1. 组件概述
2. 硬件环境（QEMU ramfb + RISC-V64 + gtk）
3. 总体架构设计
4. 核心数据结构定义
5. 对外API接口详细设计
6. 初始化流程设计
7. 帧缓冲管理机制
8. 错误处理设计
9. 裸机适配与无std设计
10. 集成使用示例
11. 约束与限制

文档要求
1. 符合操作系统内核组件工程规范
2. 内容严谨、可直接用于代码开发
3. 纯技术设计文档，无冗余内容
4. 适配rCore-Tutorial教学环境

# question1
VirtIO GPU 探测与初始化
  - VirtIO 设备如何被内核发现？
  - 如何判断设备类型是 GPU？
  - Framebuffer 的物理地址如何获取？

# res1
- “VirtIO GPU 探测” = 扫 virtio-mmio + 看 DeviceID
- “GPU 类型判断” = DeviceID == 16
- “Framebuffer 地址获取” = 不是设备给，而是内核自己分配后挂给设备

# question2
告诉我qemu是如何进行图形化显示的。

# res2
Guest 内核/驱动
→ 虚拟显示设备接口
→ QEMU 设备模拟层
→ QEMU 图形后端
→ 宿主机窗口/远程显示

# question3
要求：
1.介绍qemu提供的虚拟显示设备
2.介绍qemu常见的图形后端
3.客户系统如何操作这些虚拟显示设备，在图形后端上显示图形？
4.给出3条建议选择的虚拟现实设备和图形后端的组合。

按要求回答，并保存到/home/hdu/study/rust/2026s-ai4ose-lab/lbl-tg-rcore-tutorial-vga/docs/grap-design.md

# question4
删除每个文件里的#[test]内容有影响吗？

# question5
请生成一份 lbl-tg-rcore-tutorial-vga 操作系统图形组件的使用说明文档，文档格式为标准 Markdown，保存路径为：/home/hdu/study/rust/2026s-ai4ose-lab/lbl-tg-rcore-tutorial-vga/docs/vga-usage.md

文档核心要求
适用环境：RISC-V 64 裸机内核 /rCore-Tutorial 操作系统、QEMU RISC-V 64 模拟器、ramfb 虚拟显示设备、无标准库（#![no_std]）
文档用途：供其他内核开发者快速集成、调用本图形组件
语言：Rust，严格匹配组件设计规范
内容必须完整包含
组件简介与依赖说明
快速集成步骤（Cargo 配置、内核引入方式）
初始化接口使用方法（含代码示例）
像素绘制接口使用方法（含代码示例）
清屏接口使用方法（含代码示例）
辅助信息获取接口使用（分辨率、帧缓冲信息）
完整可运行调用示例代码
QEMU 启动参数配置
常见问题与注意事项
格式规范：结构清晰、代码可直接复制使用、符合操作系统开发文档标准

# question6
在README.md中添加对vga-usage.md的引用

# question7
进行在crate.io上的发布测试
