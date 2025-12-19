# Chronos OS - 快速开始指南

## 项目结构

```
OS2025-Chronos/
├── bootloader/           # 内核启动器
│   ├── src/
│   │   ├── main.rs      # 入口点
│   │   ├── boot.rs      # 启动逻辑和测试
│   │   ├── sbi.rs       # SBI 接口
│   │   ├── lib.rs       # 库入口
│   │   └── mm/          # 内存管理模块 ⭐
│   │       ├── mod.rs             # 模块入口
│   │       ├── memory_layout.rs   # 内存布局定义
│   │       ├── frame_allocator.rs # 物理帧分配器
│   │       ├── page_table.rs      # 页表管理
│   │       └── heap.rs            # 堆分配器
│   ├── Cargo.toml
│   ├── linker.ld        # 链接脚本
│   └── build.sh         # 构建脚本
├── MEMORY_MANAGEMENT.md # 详细文档
└── README.md
```

## 快速命令

### 编译项目
```bash
cd bootloader
cargo build --target riscv64gc-unknown-none-elf
```

### 编译 Release 版本
```bash
cargo build --target riscv64gc-unknown-none-elf --release
```

### 运行测试（需要 QEMU）
```bash
qemu-system-riscv64 \
    -machine virt \
    -nographic \
    -bios default \
    -kernel target/riscv64gc-unknown-none-elf/debug/bootloader
```

### 退出 QEMU
按 `Ctrl-A` 然后按 `X`

## 新增的内存管理功能

### ✅ 已实现

1. **物理内存分配器** - 使用位图管理 128MB 物理内存
2. **虚拟内存管理** - SV39 三级页表支持
3. **堆分配器** - 支持动态内存分配（Vec, String 等）
4. **内存布局** - 清晰的地址空间划分
5. **完整测试** - 自动测试所有内存管理功能

### 📊 关键指标

- **物理内存**: 128MB (0x8000_0000 - 0x8800_0000)
- **内核大小**: 2MB
- **堆大小**: 8MB
- **页大小**: 4KB
- **总页帧数**: 32,768 帧

## 代码示例

### 使用物理帧分配器
```rust
use crate::mm::FRAME_ALLOCATOR;

// 分配一个物理页帧
if let Some(frame) = FRAME_ALLOCATOR.alloc() {
    println!("Allocated frame at PPN: 0x{:x}", frame.as_usize());
    
    // 使用完后释放
    FRAME_ALLOCATOR.dealloc(frame);
}
```

### 使用页表
```rust
use crate::mm::{PageTable, PTEFlags, VirtPageNum};

let mut page_table = PageTable::new();
let vpn = VirtPageNum::new(0x1000);
let ppn = FRAME_ALLOCATOR.alloc().unwrap();

// 映射虚拟页到物理页
let flags = PTEFlags::V | PTEFlags::R | PTEFlags::W;
page_table.map(vpn, ppn, flags).unwrap();

// 地址转换
if let Some((translated_ppn, _)) = page_table.translate(vpn) {
    assert_eq!(translated_ppn, ppn);
}
```

### 使用堆分配
```rust
use alloc::vec::Vec;
use alloc::string::String;

// 在内存管理初始化后，可以直接使用
let mut vec = Vec::new();
for i in 0..10 {
    vec.push(i);
}

let s = String::from("Hello from kernel!");
```

## 下一步开发建议

### 1. 进程管理 🚀
```rust
// 创建新模块: src/process/
- mod.rs           # 进程管理入口
- task.rs          # 任务结构体
- scheduler.rs     # 调度器
- context.rs       # 上下文切换
```

**核心功能**：
- 进程控制块 (PCB)
- 时间片轮转调度
- 上下文保存/恢复
- 进程创建和销毁

### 2. 系统调用 📞
```rust
// 创建新模块: src/syscall/
- mod.rs           # 系统调用入口
- process.rs       # 进程相关系统调用
- fs.rs            # 文件系统系统调用
- memory.rs        # 内存管理系统调用
```

**基础系统调用**：
- `sys_write()` - 输出
- `sys_exit()` - 退出进程
- `sys_fork()` - 创建进程
- `sys_exec()` - 执行程序
- `sys_wait()` - 等待子进程

### 3. 文件系统 📁
```rust
// 创建新模块: src/fs/
- mod.rs           # 文件系统入口
- inode.rs         # 索引节点
- file.rs          # 文件描述符
- pipe.rs          # 管道
```

### 4. 设备驱动 🔧
```rust
// 创建新模块: src/drivers/
- mod.rs           # 驱动管理
- uart.rs          # 串口驱动
- virtio_blk.rs    # 块设备驱动
```

## 学习资源

- 📚 [RISC-V 手册](https://riscv.org/technical/specifications/)
- 📚 [rCore Tutorial](https://rcore-os.github.io/rCore-Tutorial-Book-v3/)
- 📚 [OSDev Wiki](https://wiki.osdev.org/)
- 📚 [The Rust Book](https://doc.rust-lang.org/book/)

## 常见问题

### Q: 编译时出现 "linker error"
A: 确保安装了 RISC-V 工具链：
```bash
rustup target add riscv64gc-unknown-none-elf
```

### Q: QEMU 无法运行
A: 安装 QEMU RISC-V 支持：
```bash
# Ubuntu/Debian
sudo apt install qemu-system-misc

# macOS
brew install qemu
```

### Q: 如何调试内核？
A: 使用 GDB 远程调试：
```bash
# 终端 1: 启动 QEMU 并等待 GDB
qemu-system-riscv64 -machine virt -nographic -bios default \
    -kernel bootloader -s -S

# 终端 2: 启动 GDB
riscv64-unknown-elf-gdb bootloader
(gdb) target remote :1234
(gdb) break rust_main
(gdb) continue
```

## 贡献者

欢迎贡献代码！请提交 Pull Request。

## 许可证

MIT License
