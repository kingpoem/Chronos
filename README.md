# OS2025-Chronos

**南京邮电大学**  
**学号**: T202510293997784  
**项目名称**: Chronos OS  
**开发方向**: 内核实现

---

## 📋 项目简介

Chronos 是一个基于 RISC-V 架构的教学型操作系统，使用 Rust 语言开发。项目目标是从零开始构建一个功能完整的操作系统内核，包括内存管理、进程调度、文件系统等核心功能。

### ✨ 当前特性

- ✅ **引导加载** - 支持 RustSBI 启动，包含独立 Bootloader
- ✅ **内存管理系统** - 完整的物理/虚拟内存管理
  - 位图式物理页帧分配器
  - SV39 三级页表管理
  - 堆分配器（支持动态内存分配）
  - 清晰的内存布局定义
- ✅ **SBI 接口** - 与 RustSBI 交互
- ✅ **基础 I/O** - 串口输出支持
- ✅ **测试框架** - 自动化测试所有内存管理功能

### 🚀 开发路线图

- [ ] **进程管理** - 进程控制块、调度器、上下文切换 (开发中)
- [ ] **系统调用** - 基础系统调用接口 (开发中)
- [ ] **中断处理** - 完善的中断和异常处理 (开发中)
- [ ] **文件系统** - VFS 和简单文件系统
- [ ] **设备驱动** - 串口、块设备驱动 (开发中)
- [ ] **用户程序** - 用户态程序支持

---

## 🏗️ 项目结构

```
OS2025-Chronos/
├── bootloader/              # 启动引导程序
│   ├── src/
│   │   ├── main.rs          # 引导程序入口
│   │   └── loader.rs        # 内核加载逻辑
├── kernel/                  # 操作系统内核源代码
│   ├── src/
│   │   ├── main.rs          # 内核入口
│   │   ├── mm/              # 内存管理模块 ⭐
│   │   │   ├── frame_allocator.rs  # 物理帧分配
│   │   │   ├── page_table.rs       # 页表管理
│   │   │   └── heap.rs             # 堆分配器
│   │   ├── task/            # 进程管理 (TODO)
│   │   ├── syscall/         # 系统调用 (TODO)
│   │   ├── trap/            # 中断处理 (TODO)
│   │   └── drivers/         # 设备驱动 (TODO)
│   ├── Cargo.toml           # 内核配置
│   └── linker.ld            # 内核链接脚本
├── rustsbi/                 # RustSBI 实现
├── Makefile                 # 项目构建脚本
├── docs/                    # 项目文档
│   ├── MEMORY_MANAGEMENT.md # 内存管理详细文档 📖
│   └── QUICKSTART.md        # 快速开始指南 🚀
└── README.md                # 本文件
```

---

## 🔧 环境要求

### 必需工具

- **Rust** (nightly)
- **RISC-V 工具链**
- **QEMU** (riscv64 支持)

### 安装步骤

```bash
# 1. 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup default nightly

# 2. 添加 RISC-V 目标
rustup target add riscv64gc-unknown-none-elf

# 3. 安装 QEMU (Ubuntu/Debian)
sudo apt install qemu-system-misc

# 或者 macOS
brew install qemu
```

---

## 🚀 快速开始

### 编译并运行

项目提供了 `Makefile` 来简化构建和运行流程：

```bash
# 编译并运行 (QEMU)
make run

# 仅编译
make build

# 调试模式 (启动 QEMU 并等待 GDB 连接)
make debug
```

### 预期输出

```
=================================
Chronos OS Kernel v0.1.0
=================================
Hart ID: 0
DTB: 0x82200000

[MM] Initializing memory management system...
[MM] Memory range: 0x80300000 - 0x88000000
[MM] Frame allocator initialized
[MM] Heap allocator initialized
[MM] Memory management system initialized successfully

[Kernel] All subsystems initialized!
[Kernel] Running tests...

...
```

**退出 QEMU**: 按 `Ctrl-A` 然后按 `X`

---

## 📚 文档

- **[内存管理详细文档](docs/MEMORY_MANAGEMENT.md)** - 完整的内存管理系统说明
- **[快速开始指南](docs/QUICKSTART.md)** - 快速上手指南和常用命令

---

## 🎓 技术栈

- **语言**: Rust (no_std)
- **架构**: RISC-V 64 (RV64GC)
- **内存模型**: SV39 (39-bit 虚拟地址)
- **引导**: RustSBI
- **模拟器**: QEMU virt machine

---

## 📊 内存布局

```
物理地址空间 (128MB):
┌─────────────────────┬─────────────────┐
│ 0x8000_0000         │ RustSBI         │
│         ↓           │                 │
│ 0x8020_0000         ├─────────────────┤
│         ↓           │ 内核代码段       │
│ 0x8030_0000         ├─────────────────┤
│         ↓           │ 内核堆           │
│ 0x80B0_0000         │ (8MB)           │
│         ↓           ├─────────────────┤
│ ...                 │ 可用物理内存     │
│ 0x8800_0000         │                 │
└─────────────────────┴─────────────────┘
```

---

## 🔬 测试

项目包含完整的自动化测试：

- ✅ 物理帧分配/释放测试
- ✅ 页表映射/转换测试
- ✅ 堆分配测试 (Vec, String)
- ✅ 内存统计验证

运行测试：
```bash
make run
```

---

## 🛠️ 开发建议

### 下一步可以实现的功能

1. **进程管理**
   - 实现进程控制块 (PCB)
   - 实现调度器（时间片轮转）
   - 实现上下文切换

2. **系统调用**
   - 实现系统调用框架
   - 实现基础系统调用（write, exit, fork, exec）

3. **文件系统**
   - 实现 VFS 虚拟文件系统层
   - 实现简单的文件系统（如 FAT32）

详细建议请查看 [QUICKSTART.md](docs/QUICKSTART.md)

---

## 📖 学习资源

- [RISC-V 规范](https://riscv.org/technical/specifications/)
- [rCore Tutorial Book](https://rcore-os.github.io/rCore-Tutorial-Book-v3/)
- [OSDev Wiki](https://wiki.osdev.org/)
- [The Rust Programming Language](https://doc.rust-lang.org/book/)
- [Rust Embedded Book](https://rust-embedded.github.io/book/)

---

## 🤝 贡献

欢迎提交 Issue 和 Pull Request！

---

## 📄 许可证

MIT License

---

## 👨‍💻 作者

**南京邮电大学**  
**学号**: T202510293997784


