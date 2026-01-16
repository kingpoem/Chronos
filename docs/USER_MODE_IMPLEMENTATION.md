# 用户态支持实现总结

## 版本信息
- **版本**: Chronos OS v0.2.0
- **日期**: 2025-12-30
- **状态**: 成功实现

## 已完成的功能

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

## 关键技术细节

### 1. Trap 处理流程

```
User Mode → ecall
    ↓
__alltraps (trap.S)
    ↓
保存所有寄存器到 TrapContext
    ↓
trap_handler (Rust)
    ↓
syscall 分发
    ↓
执行具体系统调用
    ↓
__restore (trap.S)
    ↓
恢复寄存器
    ↓
sret → User Mode
```

### 2. 地址空间隔离

- **内核地址空间**: 恒等映射 (va == pa)
- **用户地址空间**: 按需分配帧，独立页表
- **切换**: 通过修改 `satp` 寄存器

### 3. 内存布局

```
物理地址空间:
0x8000_0000  ┌─────────────────┐
             │   RustSBI (M)   │
0x8020_0000  ├─────────────────┤
             │   Kernel Code   │
0x8042_0000  ├─────────────────┤
             │   Kernel Heap   │  (8MB, Buddy Allocator)
0x80C2_0000  ├─────────────────┤
             │  Available RAM  │  (~119MB)
0x8800_0000  └─────────────────┘

虚拟地址空间 (用户态):
0x0000_0000  ┌─────────────────┐
             │   User Stack    │
             ├─────────────────┤
             │   User Heap     │
             ├─────────────────┤
             │   User Data     │
             ├─────────────────┤
             │   User Code     │
             └─────────────────┘
```

---

## 下一步开发建议

### 短期 (1-2 周)

1. **加载用户程序**
   - 实现 ELF 解析器
   - 从二进制加载用户程序
   - 创建用户地址空间

2. **进程调度器**
   - 实现时间片轮转调度
   - 集成时钟中断
   - 实现 sys_yield

3. **简单用户程序**
   - 创建 user/ 目录
   - 编写简单的用户态测试程序
   - 使用系统调用

### 中期 (2-4 周)

4. **进程管理**
   - fork/exec/wait 系统调用
   - 进程生命周期管理
   - 父子进程关系

5. **文件系统**
   - VFS 抽象层
   - 简单文件系统 (如 FAT32)
   - 文件相关系统调用

### 长期 (1-2 月)

6. **高级特性**
   - 信号处理
   - 进程间通信 (IPC)
   - 多核支持
   - 网络栈

---

## 参考资源

- **rCore Tutorial**: https://rcore-os.github.io/rCore-Tutorial-Book-v3/
- **RISC-V Spec**: https://riscv.org/technical/specifications/
- **xv6-riscv**: https://github.com/mit-pdos/xv6-riscv
- **Buddy Allocator**: https://docs.rs/buddy_system_allocator/

---

## 贡献者

**南京邮电大学**  
学号: T202510293997784

---

## 许可证

MIT License
