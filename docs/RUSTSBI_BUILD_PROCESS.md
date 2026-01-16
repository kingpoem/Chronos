# RustSBI 构建产物处理流程

本文档详细说明 `rustsbi/` 目录生成的构建产物以及它们在整个系统启动流程中的处理过程。

## 构建流程概览

```
rustsbi 源码目录
    ↓ (cargo build --release)
rustsbi/target/riscv64gc-unknown-none-elf/release/rustsbi-prototyper (ELF)
    ↓ (rust-objcopy)
build/rustsbi-prototyper.bin (二进制，可选)
    ↓ (QEMU 加载)
运行时内存中 (0x80000000)
```

## 详细步骤分析

### 1. 构建阶段

#### 1.1 源代码编译

**命令**（来自 `Makefile`）：
```makefile
cd rustsbi/prototyper/prototyper && \
cargo build --release --target riscv64gc-unknown-none-elf \
    -Zbuild-std=core,alloc \
    -Zbuild-std-features=compiler-builtins-mem
```

**构建产物位置**：
```
rustsbi/target/riscv64gc-unknown-none-elf/release/rustsbi-prototyper
```

**构建模式**：
- 默认使用 **dynamic 模式**（未启用 `payload` 或 `jump` feature）
- 这意味着 RustSBI 不会在编译时嵌入内核镜像
- 而是通过运行时动态信息（DynamicInfo）来加载下一阶段

#### 1.2 ELF 转换为二进制（可选步骤）

**命令**：
```makefile
rust-objcopy --binary-architecture=riscv64 \
    $(RUSTSBI_ELF) \
    --strip-all \
    -O binary \
    build/rustsbi-prototyper.bin
```

**说明**：
- 这一步将 ELF 文件转换为纯二进制格式
- 但在实际使用中，**直接使用 ELF 文件**（见运行时部分）
- `.bin` 文件可能用于某些特殊情况或调试

### 2. 内存布局（链接脚本分析）

RustSBI 的链接脚本（`build.rs` 生成）定义了以下内存布局：

```
0x80000000 (sbi_start)
    ├─ .text              (代码段，可执行)
    ├─ .rodata            (只读数据，受 PMP 保护)
    ├─ .rela.dyn          (重定位信息)
    ├─ .data              (数据段)
    ├─ .bss               (未初始化数据)
    │   ├─ .bss.stack     (栈区域)
    │   ├─ .bss.heap      (堆区域，RustSBI 内部使用)
    │   └─ .bss.*         (其他 BSS 数据)
    ├─ .fdt               (设备树，可选)
0x80100000+ (sbi_end，具体取决于大小)
    
0x80200000                (独立的 payload 段)
    └─ .payload           (仅在 payload 模式下使用)
```

**关键点**：
- RustSBI 从 `0x80000000` 开始
- 在 dynamic 模式下，`.payload` 段为空
- RustSBI 自身的代码和数据占用约 1MB 空间（取决于编译选项）

### 3. 运行时处理

#### 3.1 QEMU 启动命令

**来自 `Makefile` 的 run 目标**：
```bash
qemu-system-riscv64 \
    -machine virt \
    -nographic \
    -serial mon:stdio \
    -bios bootloader/rustsbi-prototyper.bin      # RustSBI 二进制文件
    -kernel build/kernel.bin                      # 内核二进制文件
```

#### 3.2 QEMU 的加载行为

1. **加载 RustSBI**：
   - QEMU 将 `-bios` 指定的 ELF 文件加载到内存 `0x80000000`
   - 设置程序计数器（PC）到 RustSBI 的入口点 `_start`
   - 初始化寄存器：`a0 = hartid`, `a1 = dtb_address`

2. **加载内核镜像**：
   - QEMU 将 `-kernel` 指定的 `kernel.bin` 加载到内存 `0x80200000`
   - 这是由 QEMU 的固件加载逻辑自动完成的

3. **传递动态信息**：
   - QEMU 在内存中创建 DynamicInfo 结构
   - 通过 `a2` 寄存器传递 DynamicInfo 的物理地址
   - DynamicInfo 包含：
     - `next_addr`: 下一阶段的入口地址（0x80200000）
     - `next_mode`: 特权模式（Supervisor）
     - `boot_hart`: 启动 hart ID

#### 3.3 RustSBI 的初始化流程

```
_start (entry.S)
    ├─ 关闭中断
    ├─ 初始化 BSS（仅 boot hart）
    ├─ 重定位处理（位置无关代码）
    ├─ 设置栈指针
    └─ 跳转到 rust_main

rust_main
    ├─ 获取初始化信息（hart ID, DTB 地址）
    ├─ [如果是 boot hart]
    │   ├─ 初始化 SBI 堆
    │   ├─ 解析设备树 (DTB)
    │   ├─ 初始化平台（控制台、IPI、复位等）
    │   ├─ 设置 PMP（物理内存保护）
    │   └─ 检测硬件特性
    ├─ [所有 hart]
    │   ├─ 准备陷阱栈
    │   └─ 配置 CSR（控制状态寄存器）
    ├─ 读取 DynamicInfo（从 a2 寄存器）
    ├─ 获取下一阶段地址 (0x80200000)
    └─ 跳转到 bootloader (local_remote_hsm().start())
```

**关键操作**：

1. **PMP 设置**（物理内存保护）：
   ```rust
   // 保护 RustSBI 自身的内存区域
   [sbi_start..sbi_end]: NONE (不可访问，防止被覆盖)
   // 允许访问其他区域
   [0..sbi_start]: RWX
   [sbi_end..memory_end]: RWX
   ```

2. **设备树处理**：
   - 解析设备树获取内存范围
   - 在设备树中添加 reserved-memory 节点
   - 标记 RustSBI 占用的内存为保留区域

3. **跳转到下一阶段**：
   - 读取 DynamicInfo 获取 `next_addr` (0x80200000)
   - 设置 Supervisor 模式
   - 跳转到 bootloader

### 4. 构建产物的最终用途

#### 4.1 主要产物：ELF 文件

**文件**：`rustsbi/target/riscv64gc-unknown-none-elf/release/rustsbi-prototyper`

**用途**：
- 直接作为 QEMU 的 `-bios` 参数
- ELF 格式保留了符号和调试信息
- QEMU 能够正确解析 ELF 并加载到正确的内存地址

**为什么使用 ELF 而不是 BIN**：
- ELF 包含完整的段信息和加载地址
- QEMU 可以直接解析 ELF 格式
- 便于调试（保留符号表）

#### 4.2 次要产物：BIN 文件

**文件**：`build/rustsbi-prototyper.bin`

**用途**：
- 当前项目中的 `run` 命令不使用此文件
- 可能用于：
  - 某些不支持 ELF 的引导程序
  - 直接烧录到硬件
  - 特殊调试场景

#### 4.3 其他构建产物

**在 `rustsbi/target/` 目录下**：
- `*.rlib`: 静态库文件
- `*.rmeta`: 元数据文件
- `incremental/`: 增量编译缓存
- `build/`: 构建脚本输出（如链接脚本）

**处理方式**：
- 这些文件在构建过程中自动生成
- 通过 `make clean` 可以清理
- 通常不需要手动处理

### 5. 内存占用分析

#### 5.1 RustSBI 自身内存占用

```
0x80000000 ────────────────────────────────
          │ RustSBI 代码段 (.text)
          │ 大小：约 50-100KB
          ├────────────────────────────────
          │ RustSBI 只读数据 (.rodata)
          │ 大小：约 20-50KB
          ├────────────────────────────────
          │ RustSBI 数据段 (.data)
          │ 大小：约 10-20KB
          ├────────────────────────────────
          │ RustSBI BSS 段 (.bss)
          │  ├─ 栈区域（每个 hart 16KB）
          │  ├─ 堆区域（约 84KB）
          │  └─ 其他 BSS 数据
          │ 总大小：约 100-200KB
          ├────────────────────────────────
          │ 设备树 (可选，.fdt)
          │ 大小：约 10-50KB
          ├────────────────────────────────
0x80100000+ (sbi_end，实际大小可变)
```

**总占用**：通常在 200KB - 500KB 之间

#### 5.2 对 0x80000000 之前空间的影响

**重要**：RustSBI **不影响** `0x80000000` 之前的内存空间。

**原因**：
1. RustSBI 从 `0x80000000` 开始加载
2. 在 QEMU virt 机器上，`0x80000000` 之前通常是：
   - 设备映射区域（MMIO）
   - ROM 区域（只读）
   - 保留区域

3. RustSBI 的 PMP 配置：
   ```rust
   // [0..memory_range.start] RWX  (允许访问)
   // [memory_range.start..sbi_start] RWX  (RAM 开始到 RustSBI)
   // [sbi_start..sbi_end] NONE  (保护 RustSBI)
   ```

**结论**：
- `0x80000000` 之前的空间由 QEMU 硬件模拟器管理
- RustSBI 不会修改或使用这些区域
- 这些区域通常映射到虚拟硬件设备（如 UART、CLINT 等）

### 6. 清理和重建

#### 6.1 清理构建产物

```bash
# 清理所有构建产物（包括 RustSBI）
make clean

# 或单独清理 RustSBI
cd rustsbi && cargo clean
```

#### 6.2 重建流程

```bash
# 完整重建（包括 RustSBI）
make build

# 只重建 RustSBI
make rustsbi
```

### 7. 总结

| 阶段 | 文件/产物 | 位置 | 最终用途 |
|------|-----------|------|----------|
| 源码编译 | ELF 文件 | `rustsbi/target/.../rustsbi-prototyper` | 主要产物 |
| 格式转换 | BIN 文件 | `build/rustsbi-prototyper.bin` | 次要产物（当前未使用） |
| 运行时 | 内存镜像 | `0x80000000+` | 作为 BIOS/Firmware |
| 功能 | - | - | 初始化硬件、加载 bootloader |

**关键点**：
1. RustSBI ELF 文件是主要构建产物，直接用于 QEMU 运行
2. 使用 dynamic 模式，不嵌入内核镜像
3. 通过 DynamicInfo 动态获取下一阶段地址
4. 设置 PMP 保护自身内存区域
5. 不影响 `0x80000000` 之前的内存空间

