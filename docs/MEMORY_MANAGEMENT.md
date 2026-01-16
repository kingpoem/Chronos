# Chronos OS - 内存管理系统

> **注意**: 本文档描述 v0.2.0 的内存管理系统。v0.1.0 使用链表分配器，v0.2.0 升级为 Buddy System Allocator。

## 概述

本项目为 Chronos OS 实现了一个完整的内存管理系统，包括：

1. **物理内存管理** (frame_allocator.rs)
2. **虚拟内存管理** (page_table.rs)
3. **堆分配器** (heap.rs) - Buddy System
4. **地址空间管理** (memory_set.rs)
5. **内存布局定义** (memory_layout.rs)

## 系统架构

```
┌─────────────────────────────────────┐
│         内存管理系统 (mm/)           │
├─────────────────────────────────────┤
│                                     │
│  ┌──────────────────────────────┐  │
│  │   内存布局 (memory_layout)    │  │
│  └──────────────────────────────┘  │
│                                     │
│  ┌──────────────────────────────┐  │
│  │ 物理帧分配器 (frame_allocator)│  │
│  └──────────────────────────────┘  │
│                                     │
│  ┌──────────────────────────────┐  │
│  │    页表管理 (page_table)      │  │
│  └──────────────────────────────┘  │
│                                     │
│  ┌──────────────────────────────┐  │
│  │   堆分配器 (heap)             │  │
│  │   Buddy System Allocator      │  │
│  └──────────────────────────────┘  │
│                                     │
│  ┌──────────────────────────────┐  │
│  │ 地址空间管理 (memory_set)     │  │
│  └──────────────────────────────┘  │
└─────────────────────────────────────┘
```

## 模块详解

### 1. 内存布局 (memory_layout.rs)

- **物理地址空间**: 0x8000_0000 - 0x8800_0000 (128MB)
- **内核区域**: 0x8020_0000+
- **堆区域**: 0x8042_0000 - 0x80C2_0000 (8MB)
- **页面大小**: 4KB

**核心类型**：
- `PhysAddr` / `PhysPageNum`: 物理地址和物理页号
- `VirtAddr` / `VirtPageNum`: 虚拟地址和虚拟页号

### 2. 物理帧分配器 (frame_allocator.rs)

**特性**：
- 基于位图的快速分配算法
- 原子操作保证线程安全
- 自动清零新分配的页帧

### 3. 页表管理 (page_table.rs)

**特性**：
- 支持 39 位虚拟地址空间
- 三级页表结构（512 entries per level）
- 页表项标志：V, R, W, X, U, G, A, D

### 4. 堆分配器 (heap.rs)

**Buddy System Allocator**：
- 使用 `buddy_system_allocator = "0.9"`
- 32 个分离子堆
- O(log n) 分配复杂度
- 支持 Vec, String 等

### 5. 地址空间管理 (memory_set.rs)

**MemorySet**：
- 管理虚拟内存空间
- 支持内核和用户地址空间
- MapArea 区域管理
- FrameTracker 自动回收

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
```

## 参考资料

- [RISC-V Privileged Specification](https://riscv.org/technical/specifications/)
- [rCore Tutorial](https://rcore-os.github.io/rCore-Tutorial-Book-v3/)
- [OSDev Wiki](https://wiki.osdev.org/Memory_Management)

---

MIT License
