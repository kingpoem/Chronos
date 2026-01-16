# Chronos OS

**南京邮电大学** | **队伍编号**: T202510293997784

基于 RISC-V 架构的教学型操作系统，使用 Rust 语言开发。

---

## 特性

- **引导加载** - 支持 RustSBI 启动，包含独立 Bootloader
- **内存管理** - 完整的物理/虚拟内存管理
  - Buddy System 分配器
  - 位图式物理页帧分配器
  - SV39 三级页表管理
  - 地址空间管理 (MemorySet)
- **Trap 处理** - 完整的中断和异常处理
- **系统调用** - 基础系统调用支持 (sys_write, sys_exit, sys_yield, sys_get_time)
- **任务管理** - 支持上下文切换

---

## 项目结构

```
OS2025-Chronos/
├── bootloader/              # 启动引导程序
├── kernel/                  # 操作系统内核
│   ├── src/
│   │   ├── mm/              # 内存管理
│   │   ├── trap/            # 中断处理
│   │   ├── task/            # 任务管理
│   │   ├── syscall/         # 系统调用
│   │   └── loader/          # 程序加载
├── user/                    # 用户程序
└── docs/                    # 项目文档
```

---

## 快速开始

```bash
# 编译并运行
make run

# 仅编译
make build

# 调试模式
make debug
```

**退出 QEMU**: 按 `Ctrl-A` 然后按 `X`

---

## 环境要求

- Rust (nightly)
- RISC-V 工具链
- QEMU (riscv64 支持)

```bash
rustup default nightly
rustup target add riscv64gc-unknown-none-elf
sudo apt install qemu-system-misc
```

---

## 文档

- [快速参考](QUICKREF.md) - 命令和概念速查
- [快速开始](docs/QUICKSTART.md) - 快速上手指南
- [内存管理](docs/MEMORY_MANAGEMENT.md) - 内存管理系统说明
- [用户态实现](docs/USER_MODE_IMPLEMENTATION.md) - 用户态实现说明
- [变更日志](docs/CHANGELOG.md) - 更新历史

---

## 技术栈

- **语言**: Rust (no_std)
- **架构**: RISC-V 64 (RV64GC)
- **内存模型**: SV39 (39-bit 虚拟地址)
- **堆分配器**: Buddy System Allocator
- **引导**: RustSBI
- **模拟器**: QEMU virt machine

---

## 学习资源

- [RISC-V 规范](https://riscv.org/technical/specifications/)
- [rCore Tutorial Book](https://rcore-os.github.io/rCore-Tutorial-Book-v3/)
- [xv6 Book](https://pdos.csail.mit.edu/6.828/2021/xv6/book-riscv-rev2.pdf)
- [OSDev Wiki](https://wiki.osdev.org/)

---

MIT License
