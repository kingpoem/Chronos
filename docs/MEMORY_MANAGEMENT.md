# Chronos OS - 内存管理系统

## 概述

本项目为 Chronos OS 实现了一个完整的内存管理系统，包括：

1. **物理内存管理** (frame_allocator.rs)
2. **虚拟内存管理** (page_table.rs)
3. **堆分配器** (heap.rs)
4. **内存布局定义** (memory_layout.rs)

## 系统架构

```
┌─────────────────────────────────────┐
│         内存管理系统 (mm/)           │
├─────────────────────────────────────┤
│                                     │
│  ┌──────────────────────────────┐  │
│  │   内存布局 (memory_layout)    │  │
│  │  - 物理地址类型               │  │
│  │  - 虚拟地址类型               │  │
│  │  - 页面大小和地址转换         │  │
│  └──────────────────────────────┘  │
│                                     │
│  ┌──────────────────────────────┐  │
│  │  物理帧分配器 (frame_allocator)│ │
│  │  - 位图分配算法               │  │
│  │  - 页帧分配/释放              │  │
│  │  - 内存统计                   │  │
│  └──────────────────────────────┘  │
│                                     │
│  ┌──────────────────────────────┐  │
│  │    页表管理 (page_table)      │  │
│  │  - SV39 三级页表              │  │
│  │  - 地址映射                   │  │
│  │  - 地址转换                   │  │
│  └──────────────────────────────┘  │
│                                     │
│  ┌──────────────────────────────┐  │
│  │     堆分配器 (heap)           │  │
│  │  - 链表分配算法               │  │
│  │  - 动态内存分配               │  │
│  │  - 支持 Vec/String 等         │  │
│  └──────────────────────────────┘  │
└─────────────────────────────────────┘
```

## 模块详解

### 1. 内存布局 (memory_layout.rs)

定义了系统的内存布局常量和地址类型：

- **物理地址空间**: 0x8000_0000 - 0x8800_0000 (128MB)
- **内核区域**: 0x8020_0000+ (从 0x80200000 开始)
- **堆区域**: 0x8042_0000 - 0x80C2_0000 (8MB)
- **页面大小**: 4KB

**核心类型**：
- `PhysAddr` / `PhysPageNum`: 物理地址和物理页号
- `VirtAddr` / `VirtPageNum`: 虚拟地址和虚拟页号

### 2. 物理帧分配器 (frame_allocator.rs)

使用位图管理物理页帧：

**特性**：
- 基于位图的快速分配算法
- 原子操作保证线程安全
- 自动清零新分配的页帧（安全性）
- 支持内存统计（已用/空闲页帧数）

**API**：
```rust
// 分配一个物理页帧
let frame = FRAME_ALLOCATOR.alloc();

// 释放一个物理页帧
FRAME_ALLOCATOR.dealloc(ppn);

// 查询内存统计
let free = FRAME_ALLOCATOR.free_frames();
let total = FRAME_ALLOCATOR.total_frames();
```

### 3. 页表管理 (page_table.rs)

实现 RISC-V SV39 三级页表：

**特性**：
- 支持 39 位虚拟地址空间
- 三级页表结构（512 entries per level）
- 页表项标志：V, R, W, X, U, G, A, D
- 自动分配中间页表

**API**：
```rust
let mut pt = PageTable::new();

// 映射虚拟页到物理页
pt.map(vpn, ppn, PTEFlags::V | PTEFlags::R | PTEFlags::W)?;

// 取消映射
pt.unmap(vpn)?;

// 地址转换
if let Some((ppn, flags)) = pt.translate(vpn) {
    // 转换成功
}
```

### 4. 堆分配器 (heap.rs)

基于链表的动态内存分配器：

**特性**：
- 实现 Rust 的 GlobalAlloc trait
- 支持标准库集合类型 (Vec, String, etc.)
- 首次适配(First-Fit)算法
- 自动处理对齐需求

**使用示例**：
```rust
// 在初始化后可以直接使用
let mut vec = Vec::new();
vec.push(42);

let s = String::from("Hello, heap!");
```

## 内存管理初始化流程

```rust
// 在 boot.rs 的 rust_main 中：

// 1. 初始化内存管理系统
mm::init(dtb);

// 内部流程：
// a. 初始化物理帧分配器
// b. 初始化堆分配器
// c. 打印内存范围信息
```

## 测试功能

系统包含了全面的测试代码，验证以下功能：

1. **物理帧分配测试**
   - 分配多个页帧
   - 验证页帧地址
   - 释放页帧

2. **堆分配测试**
   - Vec 动态数组分配
   - String 字符串分配
   - 验证数据完整性

3. **页表操作测试**
   - 创建页表
   - 映射虚拟页
   - 地址转换
   - 取消映射

4. **内存统计**
   - 显示总页帧数
   - 显示空闲页帧数

## 编译和运行

### 编译

```bash
cd bootloader
cargo build --target riscv64gc-unknown-none-elf
```

或使用构建脚本：

```bash
./build.sh
```

### 运行测试

```bash
qemu-system-riscv64 \
    -machine virt \
    -nographic \
    -bios default \
    -kernel output/bootloader
```

### 预期输出

```
=================================
Chronos OS v0.1.0
=================================
RustSBI Bootloader initialized
Hart ID: 0
DTB address: 0x82200000

[MM] Initializing memory management system...
[MM] Memory range: 0x80200000 - 0x88000000
[MM] Frame allocator initialized
[MM] Heap allocator initialized
[MM] Memory management system initialized successfully

[Test] Testing memory management system...
[Test] 1. Testing frame allocation...
[Test]    ✓ Frame allocation successful
[Test]    Frame 1 PPN: 0x80200
[Test]    Frame 2 PPN: 0x80201
[Test]    Frame 3 PPN: 0x80202
[Test]    ✓ Frame deallocation successful
[Test] 2. Testing heap allocation...
[Test]    ✓ Vec allocation successful (length: 10)
[Test]    ✓ String allocation: "Hello from heap!"
[Test] 3. Testing page table operations...
[Test]    ✓ Page table created at PPN: 0x80200
[Test]    ✓ Page mapping successful
[Test]    ✓ Page translation successful
[Test]    ✓ Page unmapping successful
[Test] Memory statistics:
[Test]    Total frames: 129536
[Test]    Free frames: 129536
[Test] ✓ All memory management tests completed!

...
=================================
All tests passed! Shutting down...
=================================
```

## 下一步改进建议

### 1. 内存管理优化
- [ ] 实现伙伴系统(Buddy System)算法
- [ ] 添加 Slab 分配器用于小对象分配
- [ ] 实现内存碎片整理
- [ ] 支持大页(Huge Pages)

### 2. 虚拟内存增强
- [ ] 实现完整的虚拟地址空间管理
- [ ] 添加写时复制(Copy-on-Write)支持
- [ ] 实现页面换出(Page Swapping)
- [ ] 添加内存映射文件支持

### 3. 内核功能扩展
- [ ] **进程管理**：实现进程结构、调度器、上下文切换
- [ ] **文件系统**：实现 VFS、文件描述符、基础文件系统
- [ ] **设备驱动**：串口、块设备、网络设备驱动
- [ ] **系统调用**：实现基本系统调用接口
- [ ] **中断处理**：完善中断和异常处理机制

### 4. 安全性增强
- [ ] 实现地址空间布局随机化(ASLR)
- [ ] 添加栈保护(Stack Guard)
- [ ] 实现内存隔离和权限控制
- [ ] 添加安全审计功能

### 5. 性能优化
- [ ] 实现 TLB 管理优化
- [ ] 添加内存预分配机制
- [ ] 实现缓存友好的数据结构
- [ ] 优化页表遍历算法

## 技术细节

### RISC-V SV39 页表格式

```
Virtual Address (39 bits):
┌────────┬─────────┬─────────┬─────────┬────────────┐
│  EXT   │  VPN[2] │  VPN[1] │  VPN[0] │   Offset   │
│ 63-39  │  38-30  │  29-21  │  20-12  │    11-0    │
└────────┴─────────┴─────────┴─────────┴────────────┘

Page Table Entry (64 bits):
┌──────────────┬──────┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┐
│     PPN      │ RSW  │D│A│G│U│X│W│R│V│
│   53-10      │  9-8 │7│6│5│4│3│2│1│0│
└──────────────┴──────┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┘

Flags:
V - Valid       有效位
R - Read        可读
W - Write       可写
X - Execute     可执行
U - User        用户模式可访问
G - Global      全局映射
A - Accessed    已访问
D - Dirty       已修改
```

### 位图分配算法

```
Bitmap Array:
┌──────────────────────────────────────┐
│ 0 0 1 0 1 1 0 0 ... (1 = allocated) │
└──────────────────────────────────────┘
  │ │ │ │ │ │ │ │
  Frame 0-7 ...

算法：
1. 从 next 位置开始搜索
2. 找到第一个为 0 的位（空闲页帧）
3. 使用原子操作设置为 1
4. 返回页帧号
5. 更新 next 指针
```

## 参考资料

- [RISC-V Privileged Specification](https://riscv.org/technical/specifications/)
- [The Rust Programming Language](https://doc.rust-lang.org/book/)
- [OSDev Wiki - Memory Management](https://wiki.osdev.org/Memory_Management)
- [rCore Tutorial](https://rcore-os.github.io/rCore-Tutorial-Book-v3/)

## 贡献

欢迎提交 Issue 和 Pull Request！

## 许可证

MIT License
