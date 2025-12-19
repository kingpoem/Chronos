# Makefile 
TARGET := riscv64gc-unknown-none-elf
MODE := release

# RustSBI prototyper paths
RUSTSBI_DIR := rustsbi
RUSTSBI_ELF := $(RUSTSBI_DIR)/target/$(TARGET)/$(MODE)/rustsbi-prototyper
RUSTSBI_BIN := build/rustsbi-prototyper.bin

BOOTLOADER_ELF := bootloader/target/$(TARGET)/$(MODE)/bootloader
KERNEL_ELF := kernel/target/$(TARGET)/$(MODE)/chronos-kernel
BOOTLOADER_BIN := build/bootloader.bin
KERNEL_BIN := build/kernel.bin
OS_IMG := build/os.img

OBJDUMP := rust-objdump
OBJCOPY := rust-objcopy
GDB := riscv64-unknown-elf-gdb

.PHONY: all bootloader kernel rustsbi build run clean debug

all: build

# Build RustSBI prototyper from submodule
rustsbi:
	@echo "Initiatizing RustSBI repository"
	@git submodule update --init --recursive
	@echo "Building RustSBI prototyper..."
	@mkdir -p $(RUSTSBI_DIR)/target
	@cp config/rustsbi/config.toml $(RUSTSBI_DIR)/target/config.toml
	@cd $(RUSTSBI_DIR)/prototyper/prototyper && cargo build --release --target $(TARGET) -Zbuild-std=core,alloc -Zbuild-std-features=compiler-builtins-mem
	@mkdir -p build
	@$(OBJCOPY) --binary-architecture=riscv64 $(RUSTSBI_ELF) --strip-all -O binary $(RUSTSBI_BIN)
	@echo "RustSBI prototyper built: $(RUSTSBI_BIN)"

bootloader:
	@echo "Building bootloader..."
	@cd bootloader && cargo build --$(MODE) --target $(TARGET)

kernel:
	@echo "Building kernel..."
	@cd kernel && cargo build --$(MODE) --target $(TARGET)

build: rustsbi bootloader kernel
	@mkdir -p build
	@$(OBJCOPY) $(BOOTLOADER_ELF) --strip-all -O binary $(BOOTLOADER_BIN)
	@$(OBJCOPY) $(KERNEL_ELF) --strip-all -O binary $(KERNEL_BIN)
	@echo "Creating OS image..."
	@rm -f $(OS_IMG)
	@cat $(BOOTLOADER_BIN) > $(OS_IMG)
	@boot_sz=$$(stat -c%s $(BOOTLOADER_BIN)); \
		pad_sz=$$((131072 - boot_sz)); \
		if [ $$pad_sz -lt 0 ]; then echo "Bootloader too large: $$boot_sz bytes"; exit 1; fi; \
		dd if=/dev/zero bs=1 count=$$pad_sz 2>/dev/null >> $(OS_IMG)
	@cat $(KERNEL_BIN) >> $(OS_IMG)
	@echo "Build complete: $(OS_IMG)"

run: build
	@echo "Running Chronos OS in QEMU..."
	@qemu-system-riscv64 \
		-machine virt \
		-nographic \
		-serial mon:stdio \
		-bios $(RUSTSBI_ELF) \
		-kernel $(OS_IMG)

debug: build
	@echo "Starting QEMU in debug mode..."
	@qemu-system-riscv64 \
		-machine virt \
		-nographic \
		-serial mon:stdio \
		-bios $(RUSTSBI_ELF) \
		-kernel $(OS_IMG) \
		-s -S

gdb:
	@$(GDB) \
		-ex 'file $(KERNEL_ELF)' \
		-ex 'set arch riscv:rv64' \
		-ex 'target remote localhost:1234'

clean:
	@cd bootloader && cargo clean
	@cd kernel && cargo clean
	@cd $(RUSTSBI_DIR) && cargo clean
	@rm -rf build

disasm-bootloader:
	@$(OBJDUMP) -d $(BOOTLOADER_ELF) | less

disasm-kernel:
	@$(OBJDUMP) -d $(KERNEL_ELF) | less

info:
	@echo "Bootloader size:"
	@ls -lh $(BOOTLOADER_BIN)
	@echo "Kernel size:"
	@ls -lh $(KERNEL_BIN)
	@echo "Total OS image size:"
	@ls -lh $(OS_IMG)