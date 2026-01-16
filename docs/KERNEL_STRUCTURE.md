# Chronos OS 内核结构

> **注意**: 本文档描述 v0.2.0 的内核结构。

## 内核模块概览

```
kernel/src/
├── main.rs              # 内核入口点和主逻辑
├── entry.S              # 汇编入口代码
├── lang_items.rs        # Rust 语言项（panic handler）
├── config.rs            # 内核配置常量
├── console.rs           # 控制台输出
├── sbi.rs               # SBI 接口封装
├── link_app.S           # 用户程序链接脚本
├── trap/                # 中断和异常处理
│   ├── trap.S           # 陷入入口/出口汇编
│   ├── context.rs       # TrapContext
│   └── mod.rs           # trap_handler
├── mm/                  # 内存管理
│   ├── mod.rs           # 模块入口
│   ├── memory_layout.rs # 内存布局定义
│   ├── frame_allocator.rs # 物理帧分配器
│   ├── page_table.rs    # 页表管理（SV39）
│   ├── heap.rs          # Buddy 堆分配器
│   └── memory_set.rs    # 地址空间管理
├── task/                # 任务管理
│   ├── mod.rs           # 任务管理入口
│   ├── task.rs          # TaskControlBlock
│   ├── context.rs       # TaskContext
│   ├── switch.S         # 上下文切换汇编
│   ├── switch.rs        # 上下文切换Rust
│   ├── manager.rs       # 任务管理器
│   ├── scheduler.rs     # 调度器
│   └── loader.rs        # 程序加载
├── syscall/             # 系统调用
│   ├── mod.rs           # 分发器
│   ├── fs.rs            # 文件系统调用
│   ├── process.rs       # 进程调用
│   └── memory.rs        # 内存调用
└── loader/              # 程序加载器
    └── mod.rs           # ELF 加载
```

## 模块详细说明

### 核心模块

**main.rs**
- 内核入口 `kernel_main(hartid, dtb)`
- 初始化流程：
  1. 清理 BSS 段
  2. 初始化控制台
  3. 初始化内存管理
  4. 初始化 Trap 处理
  5. 初始化任务管理
  6. 加载并运行用户程序

**entry.S**
- 汇编入口代码 `_start`
- 设置全局指针（gp）
- 设置栈指针（sp）
- 跳转到 `kernel_main`

**config.rs**
- 栈大小：用户栈 8KB，内核栈 16KB
- 堆大小：8MB
- 内存结束地址：0x8800_0000
- 页面大小：4KB

### 内存管理模块 (mm/)

**memory_layout.rs**
- 地址类型：`PhysAddr`, `VirtAddr`, `PhysPageNum`, `VirtPageNum`
- 地址转换工具函数
- 内存布局常量

**frame_allocator.rs**
- 物理帧分配器（位图算法）
- 快速分配/释放
- 线程安全

**page_table.rs**
- SV39 三级页表
- 39 位虚拟地址空间
- 页表项标志：V, R, W, X, U

**heap.rs**
- Buddy System 分配器
- 32 个分离子堆
- O(log n) 分配复杂度

**memory_set.rs**
- 地址空间管理
- 支持内核和用户空间
- MapArea 区域管理

### Trap 处理 (trap/)

**trap.S**
- `__alltraps`: 陷入入口，保存上下文
- `__restore`: 陷入出口，恢复上下文

**context.rs**
- `TrapContext`: 保存用户态寄存器
- `__restore` 参数准备

**mod.rs**
- `trap_handler`: 陷阱分发处理
- 中断和异常处理

### 任务管理 (task/)

**task.rs**
- `TaskControlBlock`: 任务控制块
- 任务状态：Ready, Running, Zombie

**context.rs**
- `TaskContext`: 任务上下文
- 寄存器保存/恢复

**switch.S / switch.rs**
- 上下文切换汇编代码
- 栈指针切换

**manager.rs**
- 任务管理器
- 任务状态维护

**scheduler.rs**
- 简单调度器
- FIFO 调度策略

**loader.rs**
- ELF 程序加载

### 系统调用 (syscall/)

**mod.rs**
- `syscall(syscall_id, args)`: 系统调用分发

**fs.rs**
- `sys_write(fd, buf, len)`

**process.rs**
- `sys_exit(status)`
- `sys_yield()`
- `sys_get_time()`

**memory.rs**
- `sys_brk(addr)`

## 依赖项

- `riscv` - RISC-V 寄存器访问
- `buddy_system_allocator` - 伙伴系统分配器
- `spin` - 自旋锁
- `sbi-rt` - SBI 运行时库

---

**版本**: v0.2.0
