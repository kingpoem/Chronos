# Chronos OS 用户态支持实现

## 版本信息
- **版本**: Chronos OS v0.2.0
- **日期**: 2025-12-30
- **状态**: 成功实现
## 1. 概述

本文档详细描述了 Chronos OS v0.2.0 在支持用户态（User Mode）程序运行方面的技术实现。用户态支持涉及特权级切换、地址空间隔离、系统调用接口以及用户运行时环境的构建。

### 1. Buddy System Allocator
- **替换**: 原有的简单链表分配器 → Buddy System Allocator
- **依赖**: `buddy_system_allocator = "0.9"`
- **优势**:
  - O(log n) 分配和释放时间复杂度
  - 减少内存碎片
  - 更高效的内存管理
- **位置**: `kernel/src/mm/heap.rs`

### 2. 完整的 Trap 处理
- **陷入上下文保存/恢复** (`trap/trap.S`)
  - `__alltraps`: 保存所有 32 个通用寄存器
  - `__restore`: 恢复寄存器并返回用户态
- **Trap Handler** (`trap/mod.rs`)
  - 处理系统调用 (UserEnvCall)
  - 处理页面错误 (PageFault)
  - 处理非法指令 (IllegalInstruction)
- **TrapContext 结构** (`trap/context.rs`)
  - 保存用户态所有寄存器
  - 保存 sstatus 和 sepc
  - 支持用户态初始化

### 3. 地址空间管理 (MemorySet)
- **MemorySet 实现** (`mm/memory_set.rs`)
  - 管理虚拟内存空间
  - 支持恒等映射和按帧映射
  - MapArea: 管理内存区域
  - FrameTracker: 自动回收物理帧
- **权限管理**
  - MapPermission: R/W/X/U 标志
  - 内核/用户态隔离
- **页表操作**
  - 创建内核地址空间
  - 支持用户地址空间创建

### 4. 任务管理基础设施
- **TaskContext** (`task/context.rs`)
  - 保存任务切换所需的寄存器 (ra, sp, s0-s11)
  - 支持跳转到 trap_return
- **上下文切换** (`task/switch.S`)
  - `__switch` 汇编函数
  - 保存当前任务上下文
  - 恢复下一个任务上下文
- **任务框架** (`task/mod.rs`)
  - 简单任务管理器
  - 支持任务切换

### 5. 系统调用框架
- **系统调用分发** (`syscall/mod.rs`)
  - SYSCALL_WRITE (64)
  - SYSCALL_EXIT (93)
  - SYSCALL_YIELD (124)
  - SYSCALL_GET_TIME (169)
- **实现的系统调用**:
  - `sys_write`: 输出到控制台
  - `sys_exit`: 退出进程
  - `sys_yield`: 让出 CPU (TODO: 需要调度器)
  - `sys_get_time`: 获取时间

---

## 代码统计

| 模块 | 文件 | 代码行数 | 说明 |
|------|------|---------|------|
| Buddy Allocator | `mm/heap.rs` | 25 | 使用 crate 实现 |
| 地址空间管理 | `mm/memory_set.rs` | 300+ | MemorySet 完整实现 |
| 页表管理 | `mm/page_table.rs` | 280 | SV39 三级页表 |
| Trap 处理 | `trap/trap.S` | 110 | 汇编陷入入口/出口 |
| Trap Handler | `trap/mod.rs` | 45 | Rust trap处理器 |
| 任务上下文 | `task/context.rs` | 40 | TaskContext 定义 |
| 上下文切换 | `task/switch.S` | 45 | 汇编上下文切换 |
| 系统调用 | `syscall/*.rs` | 70 | 分发和实现 |
| **总计** | | **~900** | 新增/修改代码 |

---

## 系统架构

```
┌─────────────────────────────────────────────┐
│          Chronos OS Kernel v0.2.0           │
├─────────────────────────────────────────────┤
│                                             │
│  ┌──────────────┐      ┌──────────────┐   │
│  │  User Space  │      │  User Space  │   │
│  │  Process 1   │      │  Process 2   │   │
│  └──────┬───────┘      └──────┬───────┘   │
│         │                     │            │
│         └──────────┬──────────┘            │
│                    │ System Call           │
│         ┌──────────▼──────────┐            │
│         │   Trap Handler      │            │
│         │  - UserEnvCall      │            │
│         │  - PageFault        │            │
│         │  - IllegalInsn      │            │
│         └──────────┬──────────┘            │
│                    │                        │
│         ┌──────────▼──────────┐            │
│         │  Syscall Dispatcher │            │
│         │  - sys_write        │            │
│         │  - sys_exit         │            │
│         │  - sys_yield        │            │
│         └──────────┬──────────┘            │
│                    │                        │
│  ┌─────────────────▼────────────────────┐  │
│  │        Memory Management              │  │
│  │  ┌────────────┐    ┌──────────────┐  │  │
│  │  │  Buddy     │    │  MemorySet   │  │  │
│  │  │  Allocator │    │  (Address    │  │  │
│  │  │            │    │   Spaces)    │  │  │
│  │  └────────────┘    └──────────────┘  │  │
│  │                                       │  │
│  │  ┌────────────┐    ┌──────────────┐  │  │
│  │  │  Frame     │    │  Page Table  │  │  │
│  │  │  Allocator │    │  (SV39)      │  │  │
│  │  └────────────┘    └──────────────┘  │  │
│  └───────────────────────────────────────┘  │
│                                             │
│  ┌───────────────────────────────────────┐  │
│  │        Task Management                │  │
│  │  ┌────────────┐    ┌──────────────┐  │  │
│  │  │  Task      │    │  Context     │  │  │
│  │  │  Manager   │    │  Switch      │  │  │
│  │  └────────────┘    └──────────────┘  │  │
│  └───────────────────────────────────────┘  │
│                                             │
└─────────────────────────────────────────────┘
```

---

## 测试结果

```
=================================
Chronos OS Kernel v0.2.0
=================================
Hart ID: 0
DTB: 0x0

[Init] Initializing subsystems...
[MM] Initializing memory management system...
[MM] Memory range: 0x80420000 - 0x88000000
[MM] Frame allocator initialized
[MM] Heap allocator initialized
[MM] Memory management system initialized successfully
[Task] Task management initialized

[Kernel] All subsystems initialized!

[Kernel] Running tests...

=== Memory Management Tests ===
  Frame allocated at PPN: 0x80420
  Second frame allocated at PPN: 0x80421
  Frames deallocated
  Free frames: 31712 / 31712
  Heap allocation test: vec = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]

=== System Call Tests ===
  Testing system calls...
  System call framework ready

=== All Tests Passed! ===

[Kernel] Tests completed!
[Kernel] System features:
  - Buddy System Allocator
  - SV39 Page Table
  - Trap Handling
  - System Calls
  - User Mode Support (Ready)

[Kernel] Shutting down...
```

---
=======
目标是通过这些机制，让操作系统能够加载、运行并安全管控不可信的应用程序。

## 2. 特权级切换机制

### 2.1 硬件基础
RISC-V 架构提供了 M (Machine), S (Supervisor), U (User) 三种特权级。Chronos OS 内核运行在 S 态，用户程序运行在 U 态。
`sstatus` 寄存器的 `SPP` (Previous Privilege) 位用于在 S 与 U 之间记录切换前的状态。

### 2.2 Trap 上下文保存与恢复 (`trap/trap.S`)
当 U 态程序执行系统调用 (`ecall`) 或发生异常时，硬件会自动跳转到 `stvec` 寄存器指向的地址。由于我们采用了页表机制，这个地址必须指向 Trampoline 页面（跳板页）。

- **陷入 (Trap Entry)**:
  `__alltraps` 是 Trap 处理的汇编入口：
  1. 将用户栈指针 (sp) 保存到 `sscratch`，并从 `sscratch` 获取内核栈指针。
  2. 在内核栈上保存所有 31 个通用寄存器、`sstatus` 和 `sepc`。
  3. 读取 `KERNEL_SATP`，切换页表到内核地址空间。
  4. 跳转到 Rust 实现的 `trap_handler`。

- **恢复 (Trap Return)**:
  `__restore` 负责执行逆向操作：
  1. 切换页表回用户地址空间。
  2. 从内核栈恢复保存的寄存器环境。
  3. 执行 `sret` 指令，硬件自动将 PC 设置为 `sepc`，并将特权级降回 U 态。
>>>>>>> Stashed changes

## 3. 地址空间隔离

为了保护内核与各用户进程，Chronos OS 实现了严格的地址空间隔离。

### 3.1 虚拟内存布局
- **用户空间**: 位于虚拟地址的低端 (`0x0` - `0x8000_0000` 以下)。
- **内核空间**: 位于虚拟地址的高端 (`0x8020_0000` 向上)。
- **Trampoline**: 统一映射在虚拟地址空间的最高页 (`MAX_VIRT_ADDR - PAGE_SIZE`)。

### 3.2 MemorySet 实现 (`mm/memory_set.rs`)
`MemorySet` 是地址空间的逻辑抽象，包含：
- **页表根节点 (PageTable)**: 存储页表的物理页帧号。
- **逻辑段 (MapArea)**: 如代码段、数据段、用户栈等。
- **帧追踪器 (FrameTracker)**: 确保物理页资源的 RAII 管理。

## 4. 用户运行时库 (`user/src/lib.rs`)

为了让 Rust 编写的应用程序能在裸机上运行，我们需要提供最小运行时支持（类似于 Linux 的 C Runtime）。

### 4.1 链接与入口
- **Linker Script (`linker.ld`)**: 指定程序入口点为 `_start`。
- **Entry Point (`lib.rs`)**: `_start` 函数负责初始化堆（若需）、读取参数，最后调用 `main`。
- **Language Items**: 提供 `panic_handler`，当用户程序崩溃时调用 `sys_exit`。

### 4.2 系统调用封装
用户库封装了 `ecall` 指令，向应用程序提供 Rustic 的 API：
```rust
pub fn sys_write(fd: usize, buffer: &[u8]) -> isize;
pub fn sys_exit(exit_code: i32) -> !;
pub fn sys_yield() -> isize;
pub fn sys_get_time() -> isize;
```

## 5. 系统调用实现 (`kernel/src/syscall/`)

内核通过 System Call Interface (SBI/ABI) 为用户提供服务。

### 5.1 调用约定
- **调用号**: 寄存器 `a7`。
- **参数**: 寄存器 `a0` - `a5`。
- **返回值**: 寄存器 `a0`。

### 5.2 核心系统调用
| 编号 | 名称 | 功能描述 | 实现文件 |
| :--- | :--- | :--- | :--- |
| 64 | `sys_write` | 将缓冲区数据写入指定文件描述符 (目前仅支持 stdout) | `syscall/fs.rs` |
| 93 | `sys_exit` | 终止当前进程并返回退出码 | `syscall/process.rs` |
| 124 | `sys_yield` | 主动让出 CPU 使用权 | `syscall/process.rs` |
| 169 | `sys_get_time`| 获取当前系统时间 (毫秒) | `syscall/process.rs` |
| 222 | `sys_mmap` | 申请内存映射 (Anonymous) | `syscall/memory.rs` |
| 215 | `sys_munmap` | 释放内存映射 | `syscall/memory.rs` |

## 6. 测试应用

目前 `user/src/bin` 目录下包含若干测试程序：
- `01hello.rs`: 基础输出测试。
- `power_*.rs`: 幂运算计算测试，验证计算密集型任务。
- `00poweroff.rs`: 关机测试。

---
Copyright © 2025 Chronos OS Developers
