#!/bin/bash
echo "=== Development Environment Check ==="
echo ""
echo "1. Redox Repositories:"
ls -d redox-repos/*/ 2>/dev/null | wc -l | xargs echo "   Cloned repositories:"
echo ""
echo "2. QEMU Installations:"
echo -n "   x86_64: " && qemu-system-x86_64 --version | head -1
echo -n "   aarch64: " && qemu-system-aarch64 --version | head -1
echo -n "   riscv64: " && qemu-system-riscv64 --version | head -1
echo ""
echo "3. Rust Toolchain:"
echo "   " && rustc --version
echo "   Targets installed:"
rustup target list --installed | grep -E "(x86_64|aarch64|riscv)" | sed 's/^/      /'
echo ""
echo "4. Build Tools:"
echo -n "   cargo: " && cargo --version
echo -n "   rust-lld: " && which rust-lld >/dev/null 2>&1 && echo "available" || echo "not found"
echo ""
echo "=== Environment Ready ==="
