# Chronos OS - 快速开始指南

本指南将帮助你从零开始搭建 Chronos OS 的开发环境。无论你是操作系统新手，还是经验丰富的开发者，这里都有你需要的步骤。

## 1. 准备工作：工具链

开发操作系统不同于编写普通应用，我们需要一套特殊的工具链来处理底层代码。本项目依赖 **Rust** 语言环境和 **QEMU** 模拟器。

### 1.1 安装 Rust 环境

Chronos OS 依赖 Rust 的许多底层特性（如内联汇编），因此我们需要使用 **Nightly（每夜构建版）** 的 Rust 编译器。

1.  **安装 Rustup** (如果尚未安装):
    ```bash
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    ```
    *解释：Rustup 是 Rust 的版本管理器，它可以让你在不同的 Rust 版本（Stable/Nightly）之间轻松切换。*

2.  **配置编译目标**:
    ```bash
    # 设置默认工具链为 nightly
    rustup default nightly
    
    # 添加 RISC-V 64位无操作系统目标
    rustup target add riscv64gc-unknown-none-elf
    ```
    *解释：我们需要生成的是能直接在 RISC-V 裸机上运行的代码，而不是运行在 Linux/Windows 上的程序，所以目标平台是 `unknown-none-elf`。*

3.  **安装辅助工具**:
    ```bash
    cargo install cargo-binutils
    rustup component add llvm-tools-preview
    rustup component add rust-src
    ```
    *解释：`cargo-binutils` 提供了 `objcopy` 等工具，用于将编译出的 ELF 文件剥离调试信息，转换为机器可直接执行的二进制文件。*

### 1.2 安装 QEMU 模拟器

QEMU 是一个通用的开源机器模拟器。我们将使用它来模拟一台 RISC-V 计算机。

*   **Ubuntu/Debian**:
    ```bash
    sudo apt update
    sudo apt install qemu-system-misc
    ```
*   **macOS (Homebrew)**:
    ```bash
    brew install qemu
    ```

安装完成后，请验证版本：
```bash
qemu-system-riscv64 --version
```
*建议使用 QEMU 5.0 或更高版本。*

---

## 2. 编译与运行

环境配置好后，只需几个简单的命令即可启动系统。

### 2.1 编译 (Build)

```bash
make build
```

**幕后发生了什么？**
1.  **Bootloader**: 编译 `bootloader` 目录下的引导程序。
2.  **Kernel**: 编译 `kernel` 目录下的内核源码，链接时会根据 `linker.ld` 脚本将代码段安排在指定的内存地址。
3.  **Objcopy**: 使用 `rust-objcopy` 丢弃 ELF 文件中的符号表和调试信息，生成纯净的二进制镜像 `kernel.bin`。

### 2.2 运行 (Run)

```bash
make run
```

这个命令会启动 QEMU，加载 RustSBI（作为 BIOS），然后加载我们的 `kernel.bin`。如果一切顺利，你应该能在终端看到内核的启动日志：

```text
[RustSBI] RustSBI version 0.3.0
...
[  0.003467] [kernel] Hello, Chronos OS!
...
```

**如何退出 QEMU？**
先按 `Ctrl+A`，然后迅速按 `X`。

---

## 3. 常见问题 (Troubleshooting)

**Q: 编译时提示 `error: can't find crate for 'core'`？**
A: 这通常是因为缺少标准库源码。请运行 `rustup component add rust-src`。由于我们的目标平台是裸机（no_std），编译器需要重新编译核心库。

**Q: 运行 `make run` 没有任何输出？**
A: 请检查是否已经成功构建了 bootloader。尝试运行 `make clean` 然后重新 `make build`。

**Q: GDB 调试时断点不生效？**
A: 确保你是通过 `make debug` 启动 QEMU（这会添加 `-s -S` 参数，挂起 CPU 等待连接），而不是 `make run`。

---

如有其他问题，欢迎查阅 [docs/LEARNING_PATH.md](LEARNING_PATH.md) 寻找阅读代码的线索。

---

MIT License
