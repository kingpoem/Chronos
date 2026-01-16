# Chronos OS - 快速参考

## 快速开始

```bash
make build  # 编译
make run    # 运行
make debug  # 调试
make clean  # 清理
```

## 项目结构

```
OS2025-Chronos/
├── bootloader/          # 启动引导程序
├── kernel/              # 操作系统内核
│   └── src/
│       ├── mm/          # 内存管理
│       ├── trap/        # 中断处理
│       ├── task/        # 任务管理
│       ├── syscall/     # 系统调用
│       └── loader/      # 程序加载
└── user/                # 用户程序
```

## 核心功能

### 系统调用

| 编号 | 系统调用 | 说明 |
|------|----------|------|
| 64 | sys_write | 写入输出 |
| 93 | sys_exit | 退出进程 |
| 124 | sys_yield | 主动让出 CPU |
| 169 | sys_get_time | 获取时间 |

## 常用命令

### 调试

```bash
make debug  # 启动 QEMU 等待 GDB
make gdb    # 连接 GDB
```

### GDB 命令

```bash
(gdb) b trap_handler   # 断点
(gdb) c                # 继续
(gdb) info registers   # 查看寄存器
(gdb) x/10gx $sp       # 查看栈
```

## 内存布局

```
0x8000_0000  RustSBI
0x8020_0000  Kernel Code
0x8042_0000  Kernel Heap (8MB)
0x80C2_0000  Available Memory
0x8800_0000  End
```

## 常见问题

**编译错误 "can't find crate"**
```bash
rustup default nightly
rustup target add riscv64gc-unknown-none-elf
```

**QEMU 没有输出**
```bash
make rustsbi
```

## 学习资源

- rCore Tutorial: https://rcore-os.github.io/rCore-Tutorial-Book-v3/
- xv6 Book: https://pdos.csail.mit.edu/6.828/2021/xv6/book-riscv-rev2.pdf
- RISC-V Spec: https://riscv.org/technical/specifications/

---

**版本**: v0.2.0
