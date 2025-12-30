# Chronos OS - 快速参考

## 🚀 快速开始

```bash
# 编译整个系统
make build

# 运行
make run

# 调试模式
make debug
# 另一个终端: make gdb

# 清理
make clean
```

## 📁 项目结构

```
OS2025-Chronos/
├── bootloader/          # 启动引导程序
├── kernel/              # 操作系统内核
│   ├── src/
│   │   ├── mm/          # 内存管理 ⭐
│   │   │   ├── heap.rs           # Buddy 分配器
│   │   │   ├── frame_allocator.rs  # 物理帧
│   │   │   ├── page_table.rs     # 页表
│   │   │   └── memory_set.rs     # 地址空间 ⭐
│   │   ├── trap/        # 中断处理 ⭐
│   │   │   ├── trap.S           # 汇编入口
│   │   │   ├── context.rs       # TrapContext
│   │   │   └── mod.rs           # trap_handler
│   │   ├── task/        # 任务管理 ⭐
│   │   │   ├── switch.S         # 上下文切换
│   │   │   ├── context.rs       # TaskContext
│   │   │   └── mod.rs
│   │   ├── syscall/     # 系统调用 ⭐
│   │   │   ├── mod.rs           # 分发器
│   │   │   ├── fs.rs            # 文件系统调用
│   │   │   └── process.rs       # 进程调用
│   │   └── main.rs      # 内核入口
└── docs/                # 文档
    ├── USER_MODE_IMPLEMENTATION.md  # 实现总结 📖
    └── MEMORY_MANAGEMENT.md         # 内存管理详解

⭐ = 本次新增/重大修改
```

## 🎯 核心功能

### 1. Buddy System Allocator
```rust
// kernel/src/mm/heap.rs
use buddy_system_allocator::LockedHeap;

#[global_allocator]
static HEAP_ALLOCATOR: LockedHeap<32> = LockedHeap::empty();
```

### 2. 地址空间管理
```rust
// 创建内核地址空间
let kernel_space = MemorySet::new_kernel();

// 创建用户地址空间
let mut user_space = MemorySet::new_bare();
user_space.push(MapArea::new(...), Some(data));
```

### 3. Trap 处理
```
用户态 → ecall → __alltraps → trap_handler → syscall → __restore → 用户态
```

### 4. 系统调用
```rust
// 支持的系统调用
SYSCALL_WRITE (64)    // sys_write(fd, buf, len)
SYSCALL_EXIT (93)     // sys_exit(code)
SYSCALL_YIELD (124)   // sys_yield()
SYSCALL_GET_TIME (169) // sys_get_time()
```

## 🔧 常用命令

### 编译相关
```bash
# 仅编译内核
cd kernel && cargo build --release --target riscv64gc-unknown-none-elf

# 查看反汇编
make disasm-kernel
make disasm-bootloader

# 查看大小
make info
```

### 调试相关
```bash
# GDB 调试
make debug  # 启动 QEMU 等待 GDB
make gdb    # 连接 GDB

# GDB 常用命令
(gdb) b trap_handler      # 断点
(gdb) c                   # 继续
(gdb) info registers      # 查看寄存器
(gdb) x/10gx $sp          # 查看栈
```

## 📊 内存布局

```
物理地址:
0x8000_0000  RustSBI
0x8020_0000  Kernel Code
0x8042_0000  Kernel Heap (8MB, Buddy)
0x80C2_0000  Available Memory (~119MB)
0x8800_0000  End
```

## 🐛 常见问题

### Q: 编译错误 "can't find crate"
A: 确保使用 nightly 工具链
```bash
rustup default nightly
rustup target add riscv64gc-unknown-none-elf
```

### Q: QEMU 没有输出
A: 检查 RustSBI 是否正确编译
```bash
make rustsbi
```

### Q: Trap handler 没有响应
A: 检查 stvec 是否正确设置
```rust
// trap/mod.rs
stvec::write(__alltraps as usize, TrapMode::Direct);
```

## 📚 学习资源

- **rCore Tutorial**: https://rcore-os.github.io/rCore-Tutorial-Book-v3/
- **xv6 Book**: https://pdos.csail.mit.edu/6.828/2021/xv6/book-riscv-rev2.pdf
- **RISC-V Spec**: https://riscv.org/technical/specifications/

## 🎓 关键概念

### Trap vs Exception vs Interrupt
- **Trap**: 所有导致控制转移的事件
- **Exception**: 同步事件（如系统调用、页错误）
- **Interrupt**: 异步事件（如时钟中断）

### 特权级
- **M-mode**: Machine (RustSBI)
- **S-mode**: Supervisor (Kernel) ← 我们在这里
- **U-mode**: User (Applications) ← 目标

### 页表
- **SV39**: 39-bit 虚拟地址
- **3-level**: VPN[2] → VPN[1] → VPN[0]
- **Page size**: 4KB

## 📈 性能指标

| 操作 | 复杂度 |
|------|--------|
| Buddy 分配 | O(log n) |
| Frame 分配 | O(n) 平均 |
| 页表查找 | O(1) (3次访问) |
| 上下文切换 | O(1) |

## 🔐 安全特性

- ✓ 页表隔离
- ✓ 特权级隔离
- ✓ 帧自动回收 (FrameTracker)
- ✓ 用户指针验证 (TODO)
- ✓ 栈溢出保护 (TODO)

## 下一步

1. **用户程序加载**
   - [ ] ELF 解析器
   - [ ] 加载到用户空间
   
2. **进程调度**
   - [ ] 时钟中断
   - [ ] Round-Robin 调度器

3. **测试程序**
   - [ ] Hello World 用户程序
   - [ ] 系统调用测试

---

**版本**: v0.2.0  
**作者**: T202510293997784  
**日期**: 2025-12-30
