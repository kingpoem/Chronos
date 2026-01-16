# Chronos OS 内核架构

## 1. 概述

Chronos OS 的内核设计遵循宏内核（Monolithic Kernel）架构，但通过 Rust 的强类型系统和模块化设计，实现了解耦和高内聚。内核运行在 RISC-V 架构的 Supervisor Mode (S-Mode) 下，向下管理底层硬件资源，向上并通过系统调用为用户态程序提供服务。

本文档深入剖析 `kernel/src` 目录下的核心模块，阐述其职责、交互关系及关键实现细节。

## 2. 内核模块全景图

内核源码采用清晰的树状结构组织，主要模块如下：

```text
kernel/src/
├── main.rs              # 启动入口与主函数
├── entry.S              # 内核汇编入口
├── config.rs            # 全局配置参数
├── console.rs           # 格式化输出 (printk)
├── sbi.rs               # SBI 接口封装 (OpenSBI/RustSBI)
├── drivers/             # 设备驱动框架
├── lang_items.rs        # Rust 核心库适配 (Panic Handler)
├── mm/                  # 内存管理子系统
│   ├── frame_allocator.rs # 物理页帧分配
│   ├── page_table.rs    # 多级页表映射
│   ├── heap.rs          # 动态堆分配器
│   └── memory_set.rs    # 地址空间抽象
├── task/                # 任务管理子系统
│   ├── manager.rs       # 任务控制块管理
│   ├── scheduler.rs     # CPU 调度算法
│   ├── context.rs       # 任务上下文保存
│   └── switch.S         # 任务切换汇编
├── trap/                # 异常与中断处理
│   ├── trap.S           # Trap 上下文保存与恢复 (Trampoline)
│   └── mod.rs           # Trap 分发与处理逻辑
└── syscall/             # 系统调用接口
    ├── dispatch.rs      # 系统调用分发
    ├── fs.rs            # 文件与IO相关调用
    └── process.rs       # 进程控制相关调用
```

## 3. 核心子系统详解

### 3.1 启动与初始化 (`main.rs`, `entry.S`)

内核的生命周期始于 `entry.S` 中的 `_start` 标签。
1. **汇编阶段 (`entry.S`)**: 设置启动栈指针 (sp)，初始化全局指针 (gp)，然后跳转至 Rust 入口。
2. **Rust 入口 (`main.rs`)**: `kernel_main` 函数按序执行初始化：
   - 清空 `.bss` 段。
   - 初始化串口控制台 (`console::init`)。
   - 初始化内存管理子系统 (`mm::init`)：激活页表机制。
   - 初始化中断向量表 (`trap::init`)。
   - 初始化任务管理器 (`task::init`)：加载并调度第一个用户进程。

### 3.2 内存管理子系统 (`mm/`)

负责所有物理和虚拟内存资源的分配与映射。
- **物理内存**: 使用位图 (Bitmap) 算法管理 4KB 物理页帧。
- **虚拟内存**: 实现 RISC-V SV39 分页模式，管理三级页表。
- **动态分配**: 集成 Buddy System Allocator，支持内核动态数据结构（`Vec`, `Box` 等）。
- **空间隔离**: 抽象出 `MemorySet` 概念，区分内核地址空间与用户地址空间。

### 3.3 任务调度子系统 (`task/`)

负责并发任务的管理与调度，当前实现为**非抢占式多道程序** (v0.2.0)。
- **任务状态**: 维护 `Ready`, `Running`, `Exited` 等状态机。
- **任务上下文**: `TaskContext` 保存 callee-saved 寄存器 (ra, sp, s0-s11)，用于 `__switch` 切换。
- **调度策略**: 简单的 Round-Robin (轮转) 或 FIFO 策略，从就绪队列中选择下一个任务。

### 3.4 异常与系统调用 (`trap/`, `syscall/`)

实现用户态与内核态的特权级切换。
- **Trampoline 机制**: 在虚拟地址空间的顶端映射统一的 `trap.S` 代码，解决通过页表切换回内核时的地址映射问题。
- **Trap Context**: `TrapContext` 结构体保存通用寄存器 (x0-x31)、CSR 寄存器 (sstatus, sepc) 以及内核栈指针。
- **系统调用**: 遵循 standard RISC-V syscall convention (a0-a7 传参，a7 为系统调用号)，支持 `write`, `exit`, `yield`, `get_time`, `mmap` 等基础服务。

## 4. 关键数据流

### 4.1 系统调用流程
1. 用户程序通过 `ecall` 指令触发 Trap。
2. 硬件原子性地跳转到 `stvec` 指向的 `__alltraps` (位于 Trampoline 页)。
3. `__alltraps` 保存当前寄存器均到内核栈上的 `TrapContext`。
4. 切换页表到内核地址空间，跳转到 Rust 函数 `trap_handler`。
5. `trap_handler` 根据 `scause` 识别为 System Call，分发给 `syscall` 函数处理。
6. 处理完成后，调用 `trap_return` -> `__restore` 恢复用户上下文并返回用户态。

### 4.2 任务切换流程
1. 当前任务时间片耗尽或主动 `yield`。
2. 内核调用 `__switch(current_ctx, next_ctx)`。
3. 汇编代码保存当前内核栈的 callee-saved 寄存器到 `current_ctx`。
4. 加载 `next_ctx` 中的寄存器，恢复栈指针，跳转到新任务的切出点继续执行。

## 5. 设计规范与约定

- **地址空间**:
  - 内核高半区: `0x80200000` 向上。
  - 用户低半区: 用户程序加载至低地址。
  - Trampoline: 物理内存与虚拟内存最高页双重映射。
- **栈**:
  - 每个任务拥有独立的内核栈 (`KERNEL_STACK_SIZE = 16KB`) 和用户栈 (`USER_STACK_SIZE = 8KB`)。
- **安全**:
  - 所有物理页分配时自动清零。
  - 用户态无法直接访问内核段。

---
Copyright © 2025 Chronos OS Developers
