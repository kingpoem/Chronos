# Chronos OS

**南京邮电大学** | **队伍编号**: T202510293997784

Chronos OS 是一个基于 RISC-V 64 位架构的教学型操作系统，完全使用 Rust 语言编写。

本项目的初衷是提供一个结构清晰、功能完备但又不过度复杂的内核实现，既适合操作系统初学者理解底层原理，也适合进阶开发者研究 Rust 在系统编程中的应用。我们致力于在代码可读性与系统性能之间寻找平衡。

---

## 快速上手

如果你已经配置好了 Rust 开发环境和 QEMU，可以直接运行以下命令体验：

```bash
# 编译并启动 QEMU 运行
make run
```

如果是第一次接触 Rust 开发，请移步 **[快速开始指南](docs/QUICKSTART.md)** 查看详细的环境配置教程。

---

## 核心特性

我们实现了一个现代操作系统的核心子系统，主要包括：

### 1. 启动与引导 (Booting)
*   **RustSBI集成**: 遵循 SBI 标准，使用 RustSBI 作为底层固件（Machine Mode），屏蔽底层硬件差异。
*   **独立 Bootloader**: 包含一个轻量级的引导加载程序，负责内核的加载与初始化。

### 2. 内存管理 (Memory Management)
*   **页式内存管理**: 实现了基于 SV39 标准的三级页表机制，支持 39 位虚拟地址空间。
*   **物理内存算符**: 采用位图（Bitmap）算法管理物理页帧，高效处理内存分配与回收。
*   **内核堆分配**: 集成 Buddy System Allocator（伙伴系统分配器），支持动态内存分配（Vec, Box, etc.）。
*   **地址空间抽象**: 实现了 `MemorySet` 和 `MapArea` 抽象，支持内核与用户进程的地址空间隔离。

### 3. 未定义行为与中断 (Trap Handling)
*   **统一 Trap 分发**: 实现了内核态与用户态的 Trap 统一入口。
*   **上下文保存**: 汇编级（`trap.S`）的通用寄存器保存与恢复机制。
*   **系统调用**: 提供了符合 UNIX 标准的基础系统调用接口（如 `sys_write`, `sys_exit`, `sys_yield`）。

### 4. 多任务与调度 (Multitasking)
*   **协作式调度**: 实现了基于上下文切换（Context Switch）的任务调度器。
*   **任务元数据**: 完整的任务控制块（TCB）设计，管理任务状态、上下文及栈空间。

---

## 项目导航

为了方便不同背景的开发者阅读，我们提供了多层次的文档：

*   **对于初学者**:
    *   [docs/QUICKSTART.md](docs/QUICKSTART.md): 从零开始的环境配置指南。
    *   [docs/LEARNING_PATH.md](docs/LEARNING_PATH.md): 建议的代码阅读顺序。
    
*   **对于进阶开发者**:
    *   [docs/ARCHITECTURE_DESIGN.md](docs/ARCHITECTURE_DESIGN.md): 系统整体架构与设计哲学。
    *   [docs/MEMORY_MANAGEMENT.md](docs/MEMORY_MANAGEMENT.md): 内存管理模块的详细设计文档。
    *   [docs/KERNEL_STRUCTURE.md](docs/KERNEL_STRUCTURE.md): 内核各模块的源码索引。

---

## 目录结构

代码库采用了 Rust 惯用的 Workspace 结构：

```text
OS2025-Chronos/
├── bootloader/     # 引导程序 (Bootloader)
├── kernel/         # 核心内核代码 (Kernel)
│   ├── src/
│   │   ├── mm/     # 内存管理子系统
│   │   ├── trap/   # 中断与异常处理
│   │   ├── task/   # 任务调度子系统
│   │   ├── syscall/# 系统调用接口
│   │   └── drivers/# 设备驱动 (UART等)
├── user/           # 用户态应用程序与库
└── docs/           # 设计文档与说明
```

---

## 本项目相关学习资源

- [RISC-V 规范](https://riscv.org/technical/specifications/)
- [rCore Tutorial Book](https://rcore-os.github.io/rCore-Tutorial-Book-v3/)
- [xv6 Book](https://pdos.csail.mit.edu/6.828/2021/xv6/book-riscv-rev2.pdf)
- [OSDev Wiki](https://wiki.osdev.org/)

---

MIT License
