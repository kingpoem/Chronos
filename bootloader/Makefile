# Makefile for RustSBI Bootloader

TARGET := riscv64gc-unknown-none-elf
BINARY := target/$(TARGET)/release/bootloader
QEMU := qemu-system-riscv64

.PHONY: all build clean run check install-toolchain

all: build

build:
	cargo build --release --target $(TARGET)
	rust-objcopy --strip-all -O binary $(BINARY) bootloader.bin

check:
	cargo check --target $(TARGET)

clean:
	cargo clean
	rm -f bootloader.bin

clean-all: clean
	rm -f $(RUSTSBI)

# RustSBI binary path
RUSTSBI := rustsbi-qemu.bin

# Download RustSBI if not exists
$(RUSTSBI):
	@echo "Downloading RustSBI..."
	@curl -L -o $(RUSTSBI) "https://github.com/rustsbi/rustsbi-qemu/releases/download/v0.0.1/rustsbi-qemu.bin" || \
	(echo "Failed to download RustSBI. Please download manually from:" && \
	 echo "https://github.com/rustsbi/rustsbi-qemu/releases" && exit 1)
	@echo "RustSBI downloaded successfully"

run: build $(RUSTSBI)
	@echo "Starting QEMU with RustSBI and bootloader..."
	$(QEMU) \
		-machine virt \
		-cpu rv64 \
		-m 128M \
		-bios $(RUSTSBI) \
		-kernel bootloader.bin \
		-nographic \
		-monitor none \
		-serial stdio

# Install RISC-V toolchain (macOS)
# Note: riscv-gnu-toolchain may not be available in default Homebrew
# Try these methods in order:
install-toolchain-macos:
	@echo "Attempting to install riscv-gnu-toolchain..."
	@echo "Method 1: Try riscv/riscv tap"
	@brew tap riscv/riscv 2>/dev/null || true
	@brew install riscv-gnu-toolchain 2>/dev/null || \
	(echo "Method 1 failed. Trying Method 2: Install from source..." && \
	 echo "Please visit: https://github.com/riscv-collab/riscv-gnu-toolchain" && \
	 echo "Or use: brew install --build-from-source riscv-gnu-toolchain" && \
	 echo "" && \
	 echo "Alternative: Use pre-built binaries from:" && \
	 echo "  https://github.com/riscv-collab/riscv-gnu-toolchain/releases" && \
	 echo "  Extract and add to PATH")

# Install Rust target
install-rust-target:
	rustup target add $(TARGET)

# Install required tools
install-tools: install-rust-target
	@echo "Checking required tools..."
	@which rust-objcopy >/dev/null 2>&1 || (echo "Installing rust-objcopy..." && cargo install cargo-binutils)
	@which qemu-system-riscv64 >/dev/null 2>&1 || (echo "ERROR: qemu-system-riscv64 not found. Install with: brew install qemu")
	@which riscv64-unknown-elf-gcc >/dev/null 2>&1 || (echo "WARNING: riscv64-unknown-elf-gcc not found (optional, only needed if using assembly files)")
	@echo "All required tools checked!"

help:
	@echo "Available targets:"
	@echo "  build          - Build the bootloader"
	@echo "  check          - Check the code without building"
	@echo "  clean          - Clean build artifacts"
	@echo "  run            - Build and run in QEMU"
	@echo "  install-toolchain-macos - Install RISC-V toolchain (macOS)"
	@echo "  install-rust-target - Install Rust RISC-V target"
	@echo "  install-tools  - Show required tools"

