# Chronos OS - 快速开始指南

## 项目结构

```
OS2025-Chronos/
├── bootloader/           # 引导程序
│   └── src/
├── kernel/               # 操作系统内核
│   └── src/
│       ├── mm/           # 内存管理
│       ├── trap/         # 中断处理
│       ├── task/         # 任务管理
│       ├── syscall/      # 系统调用
│       ├── loader/       # 程序加载
│       └── main.rs       # 内核入口
├── user/                 # 用户程序
└── docs/                 # 文档
```

## 快速命令

```bash
# 编译并运行
make run

# 仅编译
make build

# 调试模式
make debug

# 清理
make clean
```

**退出 QEMU**: 按 `Ctrl-A` 然后按 `X`

## 环境要求

```bash
# 安装 Rust nightly
rustup default nightly
rustup target add riscv64gc-unknown-none-elf

# 安装 QEMU
sudo apt install qemu-system-misc
```

## 核心功能

### 已实现

- **内存管理** - Buddy System 分配器、页表管理、地址空间
- **Trap 处理** - 中断和异常处理框架
- **系统调用** - sys_write, sys_exit, sys_yield, sys_get_time
- **任务管理** - 上下文切换、任务调度
- **用户程序** - ELF 加载和执行

### 技术指标

- **物理内存**: 128MB
- **页大小**: 4KB
- **堆分配器**: Buddy System
- **虚拟内存**: SV39 三级页表

## 学习资源

- [RISC-V 规范](https://riscv.org/technical/specifications/)
- [rCore Tutorial](https://rcore-os.github.io/rCore-Tutorial-Book-v3/)
- [OSDev Wiki](https://wiki.osdev.org/)

## 常见问题

**编译错误 "can't find crate"**
```bash
rustup default nightly
rustup target add riscv64gc-unknown-none-elf
```

**QEMU 没有输出**
```bash
make rustsbi
make clean && make build
```

**调试内核**
```bash
make debug  # 终端 1
make gdb    # 终端 2
```

---

MIT License
