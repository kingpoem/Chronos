# Chronos OS

**南京邮电大学** | **队伍编号**: T202510293997784

Chronos OS 是一个基于 RISC-V 64 位架构的教学型操作系统，主要使用 Rust 语言编写。

本项目的初衷是提供一个结构清晰、功能完备但又不过度复杂的内核实现，既适合操作系统初学者理解底层原理，也适合进阶开发者研究 Rust 在系统编程中的应用。我们致力于在代码可读性与系统性能之间寻找平衡。项目展示了作系统的基本概念，包括内存管理、进程调度、系统调用和权限级转换。它运行在 QEMU 的 virt 机器上，固件层是 RustSBI。

---

## 快速上手

如果你已经配置好了 Rust 开发环境和 QEMU，那么可以直接运行以下命令运行Chronos OS：

```bash
# 编译并启动 QEMU 运行
make run
```

如果你是第一次接触 Rust 语言开发，可以查看 **[快速开始指南](docs/QUICKSTART.md)** 其中有详细的环境配置教程。

---

## 核心特性

我们实现了一个现代操作系统的核心子系统，主要包括：

### 启动与引导 (Booting)
*   **RustSBI集成**: 遵循 SBI 标准，使用 RustSBI 作为底层固件（Machine Mode），有效屏蔽底层硬件差异。
*   **独立 Bootloader**: 包含一个轻量级的引导加载程序，负责内核的加载与初始化。

### 内存管理 (Memory Management)
*   **页式内存管理**: 完整实现 SV39 页表、地址空间隔离和内存映射。
*   **物理内存算符**: 采用位图（Bitmap）算法管理物理页帧，高效处理内存分配与回收。
*   **内核堆分配**: 集成 Buddy System Allocator（伙伴系统分配器），支持动态内存分配（Vec, Box, etc.）。
*   **地址空间抽象**: 实现了 `MemorySet` 和 `MapArea` 抽象，支持内核与用户进程的地址空间隔离。

### 中断与系统调用 (Trap Handling)
*   **陷阱处理机制**: 低级上下文切换、中断处理和系统调用调度。
*   **上下文保存**: 汇编级（`trap.S`）的通用寄存器保存与恢复机制。
*   **系统调用**: 提供了符合 UNIX 标准的基础系统调用接口（如 `sys_write`, `sys_exit`, `sys_yield`）。

### 多任务与调度 (Multitasking)
*   **协作式调度**: 实现了基于上下文切换（Context Switch）的任务调度器。
*   **任务元数据**: 完整的任务控制块（TCB）设计，管理任务状态、上下文及栈空间。

---

## 项目文档

为了方便不同背景的开发者阅读，我们提供了多层次的文档：

*   **对于初学者**:
    *   [docs/QUICKSTART.md](docs/QUICKSTART.md): 从零开始的环境配置指南。
    *   [docs/LEARNING_PATH.md](docs/LEARNING_PATH.md): 建议的学习路径和代码阅读顺序。
    
*   **对于进阶开发者**:
    *   [docs/ARCHITECTURE_DESIGN.md](docs/ARCHITECTURE_DESIGN.md): 系统整体架构与设计哲学。
    *   [docs/MEMORY_MANAGEMENT.md](docs/MEMORY_MANAGEMENT.md): 内存管理模块的详细设计文档。
    *   [docs/KERNEL_STRUCTURE.md](docs/KERNEL_STRUCTURE.md): 内核架构的设计文档。
    *   [docs/USER_MODE_IMPLEMENTATION.md](ocs/USER_MODE_IMPLEMENTATION.md):用户态支持的设计文档。

---

## 目录结构


```text
Chronos OS Repository Structure

chronos/
├── bootloader/          # RustSBI boot entry
│   └── src/
│       └── entry.S      # Assembly entry point (_start)
│
├── kernel/              # Main kernel implementation
│   └── src/
│       ├── main.rs          # kernel_main() entry
│       ├── entry.S          # Kernel assembly entry
│       ├── link_app.S       # Embedded user binaries
│       ├── console.rs       # Debug output macros
│       ├── config.rs        # Memory layout constants
│       ├── sbi.rs           # SBI interface wrappers
│       ├── mm/              # Memory management subsystem
│       │   ├── frame_allocator.rs
│       │   ├── heap_allocator.rs
│       │   ├── page_table.rs
│       │   ├── memory_set.rs
│       │   └── address.rs
│       ├── trap/            # Trap handling subsystem
│       │   ├── mod.rs       # trap_handler()
│       │   ├── trap.S       # __alltraps, __restore
│       │   └── context.rs   # TrapContext struct
│       ├── task/            # Task management subsystem
│       │   ├── mod.rs       # TASK_MANAGER global
│       │   ├── task.rs      # TaskControlBlock
│       │   ├── switch.S     # __switch assembly
│       │   ├── context.rs   # TaskContext struct
│       │   └── scheduler.rs # SCHEDULER global
│       ├── syscall/         # System call subsystem
│       │   ├── mod.rs       # syscall() dispatcher
│       │   ├── process.rs   # Process syscalls
│       │   ├── fs.rs        # File/IO syscalls
│       │   └── mm.rs        # Memory syscalls
│       └── loader.rs        # ELF parsing
│
├── user/                # User application binaries
│   └── src/
│       └── bin/
│           ├── 00poweroff.rs
│           └── 01hello.rs
│
└── docs/                # Project documentation
    ├── PROJECT_STATUS_REPORT.md
    ├── MEMORY_MANAGEMENT.md
    └── USER_MODE_IMPLEMENTATION.md
```

---

## 本项目相关学习资源

- [RISC-V 规范](https://riscv.org/technical/specifications/)
- [rCore Tutorial Book](https://rcore-os.github.io/rCore-Tutorial-Book-v3/)
- [xv6 Book](https://pdos.csail.mit.edu/6.828/2021/xv6/book-riscv-rev2.pdf)
- [OSDev Wiki](https://wiki.osdev.org/)

---

MIT License
