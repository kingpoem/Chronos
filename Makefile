# Makefile 
TARGET := riscv64gc-unknown-none-elf
MODE := release

# RustSBI prototyper paths
RUSTSBI_DIR := rustsbi
RUSTSBI_ELF := $(RUSTSBI_DIR)/target/$(TARGET)/$(MODE)/rustsbi-prototyper
RUSTSBI_BIN := bootloader/rustsbi-prototyper.bin

KERNEL_ELF := kernel/target/$(TARGET)/$(MODE)/chronos-kernel
KERNEL_BIN := build/kernel.bin

OBJDUMP := rust-objdump
OBJCOPY := rust-objcopy
GDB := riscv64-unknown-elf-gdb

.PHONY: all kernel rustsbi build run clean debug

all: build

# Build RustSBI prototyper from submodule
rustsbi:
	@echo "Initiatizing RustSBI repository"
	@git submodule update --init --recursive
	@echo "Building RustSBI prototyper..."
	@mkdir -p $(RUSTSBI_DIR)/target
	@cp config/rustsbi/config.toml $(RUSTSBI_DIR)/target/config.toml
	@cd $(RUSTSBI_DIR)/prototyper/prototyper && cargo build --release --target $(TARGET) -Zbuild-std=core,alloc -Zbuild-std-features=compiler-builtins-mem
	@mkdir -p bootloader
	@$(OBJCOPY) --binary-architecture=riscv64 $(RUSTSBI_ELF) --strip-all -O binary $(RUSTSBI_BIN)
	@echo "RustSBI prototyper built: $(RUSTSBI_BIN)"

kernel:
	@echo "Building kernel..."
	@cd kernel && cargo build --$(MODE) --target $(TARGET)

build: rustsbi kernel
	@mkdir -p build
	@$(OBJCOPY) $(KERNEL_ELF) --strip-all -O binary $(KERNEL_BIN)
	@echo "Build complete: $(KERNEL_BIN)"

run: build
	@echo "Running Chronos OS in QEMU..."
	@qemu-system-riscv64 \
		-machine virt \
		-nographic \
		-serial mon:stdio \
		-bios $(RUSTSBI_BIN) \
		-kernel $(KERNEL_BIN)

debug: build
	@echo "Starting QEMU in debug mode..."
	@qemu-system-riscv64 \
		-machine virt \
		-nographic \
		-serial mon:stdio \
		-bios $(RUSTSBI_BIN) \
		-kernel $(KERNEL_BIN) \
		-s -S

gdb:
	@$(GDB) \
		-ex 'file $(KERNEL_ELF)' \
		-ex 'set arch riscv:rv64' \
		-ex 'target remote localhost:1234'

clean:
	@cd kernel && cargo clean
	@cd $(RUSTSBI_DIR) && cargo clean
	@rm -rf build
	@rm -f $(RUSTSBI_BIN)

disasm-kernel:
	@$(OBJDUMP) -d $(KERNEL_ELF) | less

info:
	@echo "Kernel binary size:"
	@ls -lh $(KERNEL_BIN)
	@echo "RustSBI binary size:"
	@ls -lh $(RUSTSBI_BIN)