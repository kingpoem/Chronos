# Chronos OS 内核结构

本文档详细说明 Chronos OS 内核的模块结构和功能。

## 内核模块概览

```
kernel/src/
├── main.rs              # 内核入口点和主逻辑
├── entry.S              # 汇编入口代码
├── lang_items.rs        # Rust 语言项（panic handler）
├── config.rs            # 内核配置常量
├── console.rs           # 控制台输出（println! 宏）
├── sbi.rs               # SBI 接口封装
│
├── mm/                  # 内存管理模块 ⭐
│   ├── mod.rs           # 模块入口
│   ├── memory_layout.rs # 内存布局定义
│   ├── frame_allocator.rs # 物理帧分配器
│   ├── page_table.rs    # 页表管理（SV39）
│   └── heap.rs          # 堆分配器
│
├── trap/                # 中断和异常处理 🔧（预留）
│   └── mod.rs
│
├── syscall/             # 系统调用处理 🔧（预留）
│   └── mod.rs
│
├── task/                # 进程/任务管理 🔧（预留）
│   └── mod.rs
│
└── drivers/             # 设备驱动 🔧（预留）
    └── mod.rs
```

## 模块详细说明

### ✅ 已实现模块

#### 1. 核心模块

**main.rs**
- 内核入口点 `kernel_main(hartid, dtb)`
- 初始化流程：
  1. 清理 BSS 段
  2. 初始化控制台
  3. 初始化内存管理
  4. 初始化陷阱处理
  5. 运行测试
  6. 关闭系统

**entry.S**
- 汇编入口代码 `_start`
- 设置全局指针（gp）
- 设置栈指针（sp）
- 跳转到 `kernel_main`

**lang_items.rs**
- Panic handler（内核恐慌处理）
- 错误时调用 SBI shutdown

**config.rs**
- 内核配置常量
  - 栈大小：用户栈 8KB，内核栈 16KB
  - 堆大小：8MB
  - 内存结束地址：0x8800_0000
  - 页面大小：4KB
  - 最大应用数：16
  - 最大系统调用数：500

**console.rs**
- 控制台输出实现
- 提供 `print!` 和 `println!` 宏
- 通过 SBI console_putchar 输出

**sbi.rs**
- SBI (Supervisor Binary Interface) 接口封装
- 功能：
  - `console_putchar/putstr/getchar` - 控制台 I/O
  - `get_time/set_timer` - 定时器
  - `shutdown` - 系统关闭

#### 2. 内存管理模块 (mm/)

**memory_layout.rs**
- 定义内存布局常量
  - `MEMORY_START`: 0x8000_0000
  - `MEMORY_END`: 0x8800_0000
  - `KERNEL_START`: 0x8020_0000
  - `KERNEL_HEAP_START`: 0x8042_0000
  - `KERNEL_HEAP_SIZE`: 8MB
- 地址类型：`PhysAddr`, `VirtAddr`, `PhysPageNum`, `VirtPageNum`
- 地址转换工具函数

**frame_allocator.rs**
- 物理帧分配器（位图算法）
- 特性：
  - 快速分配/释放
  - 线程安全（原子操作）
  - 自动清零新分配的页帧
  - 内存统计（已用/空闲页帧数）
- API：
  - `FRAME_ALLOCATOR.alloc()` - 分配一个页帧
  - `FRAME_ALLOCATOR.dealloc(ppn)` - 释放一个页帧
  - `FRAME_ALLOCATOR.free_frames()` - 查询空闲页帧数

**page_table.rs**
- SV39 三级页表实现
- 特性：
  - 39 位虚拟地址空间
  - 三级页表结构（512 entries per level）
  - 页表项标志：V, R, W, X, U, G, A, D
  - 自动分配中间页表
- API：
  - `PageTable::new()` - 创建页表
  - `pt.map(vpn, ppn, flags)` - 映射虚拟页到物理页
  - `pt.unmap(vpn)` - 取消映射
  - `pt.translate(vpn)` - 虚拟地址转换

**heap.rs**
- 堆分配器（链表算法）
- 特性：
  - 首次适配算法
  - 支持动态内存分配（Vec, String 等）
  - 对齐到 8 字节
- 状态：⚠️ 当前有已知问题，测试中跳过

**mod.rs**
- 内存管理模块入口
- `mm::init(dtb)` - 初始化内存管理系统
- `mm::test()` - 内存管理测试

### 🔧 预留模块（待实现）

#### trap/ - 中断和异常处理
- 当前状态：空实现，只有 `init()` 占位符
- 计划功能：
  - 中断向量表设置
  - 异常处理
  - 中断处理程序
  - 时钟中断

#### syscall/ - 系统调用处理
- 当前状态：空实现，只有 `syscall()` 占位符
- 计划功能：
  - 系统调用入口
  - 系统调用分发
  - 基础系统调用实现（write, exit, fork 等）

#### task/ - 进程/任务管理
- 当前状态：空实现，只有 `init()` 占位符
- 计划功能：
  - 进程控制块（PCB）
  - 进程调度器
  - 上下文切换
  - 进程创建/销毁

#### drivers/ - 设备驱动
- 当前状态：空实现，只有 `init()` 占位符
- 计划功能：
  - UART 驱动
  - 块设备驱动
  - 设备抽象层

## 内核初始化流程

```
_start (entry.S)
    ↓
kernel_main (main.rs)
    ├─ clear_bss()                    # 清理 BSS 段
    ├─ console::init()                # 初始化控制台
    ├─ mm::init(dtb)                  # 初始化内存管理
    │   ├─ frame_allocator::init()    # 初始化帧分配器
    │   └─ heap::init_heap()          # 初始化堆分配器
    ├─ trap::init()                   # 初始化陷阱处理（预留）
    ├─ test_kernel()                  # 运行测试
    │   └─ mm::test()                 # 内存管理测试
    └─ sbi::shutdown()                # 关闭系统
```

## 内存布局

```
物理地址空间 (128MB):
┌─────────────────────┬─────────────────┐
│ 0x8000_0000        │ RustSBI         │
│         ↓           │                 │
│ 0x8020_0000        ├─────────────────┤
│         ↓           │ 内核代码段       │
│ 0x8042_0000        ├─────────────────┤
│         ↓           │ 内核堆           │
│ 0x80C2_0000        │ (8MB)           │
│         ↓           ├─────────────────┤
│ ...                 │ 可用物理内存     │
│ 0x8800_0000        │                 │
└─────────────────────┴─────────────────┘
```

## 构建产物

- **ELF 文件**: `kernel/target/riscv64gc-unknown-none-elf/release/chronos-kernel`
- **二进制文件**: `build/kernel.bin`
- **入口地址**: 0x80200000

## 测试功能

当前内核包含以下测试：

1. **内存管理测试** (`mm::test()`)
   - 物理帧分配/释放测试
   - 内存统计验证

2. **待添加测试**
   - 页表映射测试
   - 堆分配测试
   - 系统调用测试

## 依赖项

- `riscv` - RISC-V 寄存器访问
- `lazy_static` - 静态变量初始化
- `spin` - 自旋锁
- `sbi-rt` - SBI 运行时库

---

**最后更新**: 2025-12-21
**状态**: ✅ 基础内存管理完成，其他模块预留

