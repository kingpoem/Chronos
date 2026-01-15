# OS2025-Chronos

**南京邮电大学**  
**学号**: T202510293997784  
**项目名称**: Chronos OS  
**开发方向**: 内核实现

---

## 📋 项目简介

Chronos 是一个基于 RISC-V 架构的教学型操作系统，使用 Rust 语言开发。项目目标是从零开始构建一个功能完整的操作系统内核，包括内存管理、进程调度、文件系统等核心功能。

### ✨ 当前特性 (v0.2.0)

- ✅ **引导加载** - 支持 RustSBI 启动，包含独立 Bootloader
- ✅ **内存管理系统** - 完整的物理/虚拟内存管理
  - **Buddy System 分配器** - 高效的内核堆分配 ⭐ NEW
  - 位图式物理页帧分配器
  - SV39 三级页表管理
  - **地址空间管理 (MemorySet)** - 支持独立地址空间 ⭐ NEW
  - 清晰的内存布局定义
- ✅ **Trap 处理** - 完整的中断和异常处理 ⭐ NEW
  - 陷入上下文保存/恢复
  - 系统调用处理
  - 页面错误处理
- ✅ **系统调用框架** - 基础系统调用支持 ⭐ NEW
  - sys_write, sys_exit, sys_yield, sys_get_time
- ✅ **任务管理基础** - 支持上下文切换 ⭐ NEW
  - TaskContext 和上下文切换
  - 为用户态程序做好准备
- ✅ **SBI 接口** - 与 RustSBI 交互
- ✅ **基础 I/O** - 串口输出支持
- ✅ **测试框架** - 自动化测试

### 🚀 开发路线图

- [x] **内存管理** - 完整实现 ✓
- [x] **Buddy Allocator** - 高效堆分配 ✓
- [x] **地址空间管理** - MemorySet 实现 ✓
- [x] **Trap 处理** - 中断和异常 ✓
- [x] **系统调用** - 基础接口 ✓
- [ ] **用户程序加载** - ELF 加载器 (开发中)
- [ ] **进程调度** - 时间片轮转调度器 (开发中)
- [ ] **文件系统** - VFS 和简单文件系统
- [ ] **设备驱动** - 完善的设备驱动

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
│   │   │   ├── heap.rs              # Buddy 堆分配器 ⭐
│   │   │   ├── frame_allocator.rs  # 物理帧分配
│   │   │   ├── page_table.rs       # 页表管理
│   │   │   └── memory_set.rs       # 地址空间 ⭐
│   │   ├── trap/            # 中断处理 ⭐
│   │   │   ├── trap.S              # 汇编入口
│   │   │   ├── context.rs          # TrapContext
│   │   │   └── mod.rs              # trap_handler
│   │   ├── task/            # 任务管理 ⭐
│   │   │   ├── switch.S            # 上下文切换
│   │   │   ├── context.rs          # TaskContext
│   │   │   └── mod.rs
│   │   ├── syscall/         # 系统调用 ⭐
│   │   │   ├── mod.rs              # 分发器
│   │   │   ├── fs.rs               # 文件系统调用
│   │   │   └── process.rs          # 进程调用
│   │   └── drivers/         # 设备驱动
│   ├── Cargo.toml           # 内核配置
│   └── linker.ld            # 内核链接脚本
├── rustsbi/                 # RustSBI 实现
├── Makefile                 # 项目构建脚本
├── docs/                    # 项目文档
│   ├── USER_MODE_IMPLEMENTATION.md # 用户态实现总结 📖
│   ├── MEMORY_MANAGEMENT.md        # 内存管理详细文档 📖
│   └── QUICKSTART.md               # 快速开始指南 🚀
├── QUICKREF.md              # 快速参考手册
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
Chronos OS Kernel v0.2.0
=================================
Hart ID: 0
DTB: 0x0

[Init] Initializing subsystems...
[MM] Memory management system initialized successfully
[Task] Task management initialized

[Kernel] All subsystems initialized!

[Kernel] Running tests...

=== Memory Management Tests ===
  Frame allocated at PPN: 0x80420
  Free frames: 31712 / 31712
  Heap allocation test: vec = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]

=== System Call Tests ===
  System call framework ready

=== All Tests Passed! ===

[Kernel] System features:
  ✓ Buddy System Allocator
  ✓ SV39 Page Table
  ✓ Trap Handling
  ✓ System Calls
  ✓ User Mode Support (Ready)
```

**退出 QEMU**: 按 `Ctrl-A` 然后按 `X`

---

## 📚 文档

- **[项目状态报告](docs/PROJECT_STATUS_REPORT.md)** - 完整的项目分析报告 (1200+ 行) ⭐
- **[用户态实现总结](docs/USER_MODE_IMPLEMENTATION.md)** - 详细的实现说明
- **[内存管理文档](docs/MEMORY_MANAGEMENT.md)** - 完整的内存管理系统说明
- **[快速参考](QUICKREF.md)** - 命令和概念速查
- **[快速开始指南](docs/QUICKSTART.md)** - 快速上手指南

---

## 🎓 技术栈

- **语言**: Rust (no_std)
- **架构**: RISC-V 64 (RV64GC)
- **内存模型**: SV39 (39-bit 虚拟地址)
- **堆分配器**: Buddy System Allocator
- **引导**: RustSBI
- **模拟器**: QEMU virt machine

---

## 📊 内存布局

```
物理地址空间 (128MB):
┌─────────────────────┬─────────────────┐
│ 0x8000_0000         │ RustSBI (M)     │
│         ↓           │                 │
│ 0x8020_0000         ├─────────────────┤
│         ↓           │ 内核代码段 (S)   │
│ 0x8042_0000         ├─────────────────┤
│         ↓           │ 内核堆 (Buddy)   │
│ 0x80C2_0000         │ (8MB)           │
│         ↓           ├─────────────────┤
│ ...                 │ 可用物理内存     │
│ 0x8800_0000         │ (~119MB)        │
└─────────────────────┴─────────────────┘
```

---

## 🔬 测试

项目包含完整的自动化测试：

- ✅ 物理帧分配/释放测试
- ✅ 页表映射/转换测试
- ✅ Buddy 堆分配测试 (Vec, String)
- ✅ 内存统计验证
- ✅ 系统调用框架测试

运行测试：
```bash
make run
```

---

## 🆕 最新更新 (v0.2.0)

### 新增功能
1. **Buddy System Allocator** - 替换原有链表分配器
2. **MemorySet** - 完整的地址空间管理
3. **Trap 处理** - 完整的陷入入口/出口
4. **System Call** - 系统调用分发和实现
5. **Task Context** - 任务上下文切换支持

### 改进
- 更高效的内存管理
- 更清晰的代码结构
- 更完善的文档

---

## 🛠️ 下一步开发

1. **用户程序加载器**
   - 实现 ELF 解析
   - 加载用户程序到独立地址空间
   - 创建第一个用户进程

2. **进程调度**
   - 实现时钟中断
   - 时间片轮转调度器
   - 实现 sys_yield

3. **完善系统调用**
   - fork/exec/wait
   - 进程管理系统调用

详细建议请查看 [QUICKSTART.md](docs/QUICKSTART.md)

---

## 📖 学习资源

- [RISC-V 规范](https://riscv.org/technical/specifications/)
- [rCore Tutorial Book](https://rcore-os.github.io/rCore-Tutorial-Book-v3/)
- [xv6 Book](https://pdos.csail.mit.edu/6.828/2021/xv6/book-riscv-rev2.pdf)
- [OSDev Wiki](https://wiki.osdev.org/)
- [The Rust Programming Language](https://doc.rust-lang.org/book/)

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

---

## 📈 项目统计

- **代码行数**: ~2000+ 行 Rust + 汇编
- **模块数**: 8 个核心模块
- **开发时间**: 持续开发中
- **最新版本**: v0.2.0 (2025-12-30)
