# Redox Hypervisor Initial Structure

## Overview

Initial implementation of the type-1 hypervisor module structure for Redox OS, located in the kernel at `redox-repos/kernel/src/hypervisor/`.

## Directory Structure

```
kernel/src/hypervisor/
├── mod.rs                      # Main hypervisor module
├── vm.rs                       # Virtual Machine management
├── vcpu.rs                     # Virtual CPU management
├── memory.rs                   # Guest memory management
├── arch/                       # Architecture-specific backends
│   ├── mod.rs                  # Architecture abstraction layer
│   ├── x86_64/                 # x86_64 backend
│   │   ├── mod.rs              # x86_64 main module
│   │   ├── vmx.rs              # Intel VMX support (stub)
│   │   └── svm.rs              # AMD SVM support (stub)
│   ├── aarch64/                # aarch64 backend
│   │   └── mod.rs              # EL2 virtualization (stub)
│   └── riscv64/                # riscv64 backend
│       └── mod.rs              # H-extension support (stub)
└── devices/                    # Virtual device emulation
    └── mod.rs                  # Device framework (stub)
```

## Modules Created

### 1. `hypervisor/mod.rs` - Core Hypervisor Module
**Purpose**: Main entry point for hypervisor functionality

**Key Components**:
- `HypervisorCaps` - Capability detection structure
- `HypervisorArch` - Enum for supported architectures (x86_64, aarch64, riscv64)
- `HypervisorError` - Error types for hypervisor operations
- `init()` - Initialize hypervisor subsystem
- `is_initialized()` - Check initialization status

### 2. `hypervisor/vm.rs` - Virtual Machine Management
**Purpose**: VM lifecycle and resource management

**Key Structures**:
- `Vm` - VM Control Block (VCB)
  - VM ID and state management
  - VCPU list tracking
  - Memory region management
  - Architecture-specific VM data

- `VmConfig` - VM configuration
  - Number of VCPUs
  - Memory size
  - VM name

- `VmState` - VM states
  - Creating, Stopped, Running, Paused, Destroying

- `MemoryRegion` - Guest physical memory regions
  - Guest Physical Address (GPA)
  - Host Physical Address (HPA)
  - Size and access flags

**Key Functions**:
- `Vm::new()` - Create new VM
- `Vm::map_memory()` - Map guest physical memory
- `Vm::unmap_memory()` - Unmap guest physical memory
- `Vm::add_vcpu()` - Add VCPU to VM
- `allocate_vm_id()` - Generate unique VM IDs

### 3. `hypervisor/vcpu.rs` - Virtual CPU Management
**Purpose**: VCPU execution and register management

**Key Structures**:
- `Vcpu` - Virtual CPU structure
  - VCPU ID and parent VM ID
  - State management
  - Register state
  - Last exit reason
  - Architecture-specific VCPU data

- `VcpuState` - VCPU states
  - Stopped, Running, Waiting, Exited

- `VcpuExit` - Exit reasons
  - External interrupt, Exception, I/O, MMIO
  - Halt, Shutdown, Hypercall, Debug

- `VcpuRegs` - Generic register state
  - Program counter (PC)
  - Stack pointer (SP)
  - General purpose registers
  - Flags/Status register

**Key Functions**:
- `Vcpu::new()` - Create new VCPU
- `Vcpu::run()` - Enter guest mode
- `Vcpu::resume()` - Resume after exit
- `Vcpu::stop()` - Stop VCPU
- `Vcpu::get_regs() / set_regs()` - Register access
- `allocate_vcpu_id()` - Generate unique VCPU IDs

### 4. `hypervisor/memory.rs` - Guest Memory Management
**Purpose**: Guest physical memory allocation and translation

**Key Components**:
- `GuestMemory` - Guest memory allocator
  - Memory size tracking
  - Base host physical address
  - GPA to HPA translation
  - Read/write operations

**Type Aliases**:
- `Gpa` - Guest Physical Address (u64)
- `Hpa` - Host Physical Address (u64)

### 5. `hypervisor/arch/mod.rs` - Architecture Abstraction Layer
**Purpose**: Provide unified interface across architectures

**Key Functions**:
- `detect_capabilities()` - Detect hardware virtualization support
- `init()` - Initialize architecture-specific backend

**Exports**:
- `ArchVmData` - Architecture-specific VM data
- `ArchVcpuData` - Architecture-specific VCPU data

### 6. `hypervisor/arch/x86_64/` - x86_64 Backend
**Purpose**: Intel VMX and AMD SVM support

**Components**:
- `VirtTech` - Enum for VMX vs SVM
- `ArchVmData` - EPT/NPT page table management
- `ArchVcpuData` - VMCS/VMCB management
- `vmx.rs` - Intel VMX operations (stub)
- `svm.rs` - AMD SVM operations (stub)

**Functions**:
- `detect_capabilities()` - Check VMX/SVM support
- `init()` - Initialize VMX or SVM
- `detect_virt_tech()` - Determine which technology is available

### 7. `hypervisor/arch/aarch64/` - aarch64 Backend
**Purpose**: ARM EL2 virtualization support

**Components**:
- `ArchVmData` - Stage-2 translation table management
- `ArchVcpuData` - Guest system register state
- `GuestSysRegs` - EL1 system registers

**Functions**:
- `detect_capabilities()` - Check EL2 availability
- `init()` - Initialize EL2 hypervisor
- `is_el2_available()` - Check current exception level

### 8. `hypervisor/arch/riscv64/` - riscv64 Backend
**Purpose**: RISC-V H-extension support

**Components**:
- `ArchVmData` - G-stage page table management (hgatp)
- `ArchVcpuData` - Guest CSR state
- `GuestCsrs` - Supervisor-level CSRs

**Functions**:
- `detect_capabilities()` - Check H-extension support
- `init()` - Initialize H-extension hypervisor
- `is_h_extension_available()` - Check H-extension in misa

### 9. `hypervisor/devices/mod.rs` - Device Emulation Framework
**Purpose**: Virtual device emulation (stub)

**Traits**:
- `VirtualDevice` - Trait for emulated devices
  - `mmio_read()` / `mmio_write()` - MMIO operations
  - `io_read()` / `io_write()` - I/O port operations (x86)

## Design Patterns

### Architecture Abstraction
- **Trait-based design**: Architecture-specific operations are abstracted through traits
- **Conditional compilation**: Uses `#[cfg(target_arch = "...")]` for architecture selection
- **Common interface**: All architectures implement the same interface for VM/VCPU operations

### State Management
- **Atomic state**: Uses `AtomicU8` for thread-safe state transitions
- **State enums**: Clear state machines for VM and VCPU lifecycle
- **Error handling**: Comprehensive error type for all hypervisor operations

### Memory Safety
- **No unsafe code yet**: All stubs use safe Rust
- **Future**: Will need unsafe for hardware access (VMCS, CSRs, etc.)
- **Validation**: Memory region overlap checking

## Integration Points

### With Redox Kernel
1. **System Calls**: Need to add hypervisor system calls to `src/syscall/`
2. **Module Registration**: Need to add `mod hypervisor;` to `src/main.rs`
3. **Memory Manager**: Integration with Redox Memory Manager (RMM)
4. **Interrupt Handling**: Integration with IRQ scheme

### With Bootloader
- Early virtualization detection and enablement
- Pass capabilities to kernel

### With Drivers
- VirtIO driver framework
- Device emulation integration

## Current Status

### Completed ✓
- [x] Core module structure created
- [x] VM lifecycle framework
- [x] VCPU execution framework
- [x] Memory management stubs
- [x] Architecture abstraction layer
- [x] x86_64 backend stubs (VMX/SVM)
- [x] aarch64 backend stubs (EL2)
- [x] riscv64 backend stubs (H-extension)
- [x] Device emulation framework stub

### TODO (Phase 1 - Foundation)
- [ ] Add hypervisor module to kernel build
- [ ] Implement actual VMX detection and initialization
- [ ] Implement actual SVM detection and initialization
- [ ] Implement EL2 detection and initialization
- [ ] Implement H-extension detection and initialization
- [ ] Add hypervisor system calls
- [ ] Integrate with Redox memory manager
- [ ] Add unit tests

### Future Phases
- **Phase 2**: x86_64 full implementation (EPT/NPT, APIC, devices)
- **Phase 3**: aarch64 full implementation (Stage-2, GIC, devices)
- **Phase 4**: riscv64 full implementation (G-stage, PLIC, devices)
- **Phase 5**: Bootloader integration
- **Phase 6**: VM management daemon
- **Phase 7**: QEMU testing
- **Phase 8**: Advanced features

## Key Design Decisions

### 1. Monolithic Hypervisor Core
Following xvisor's design, the hypervisor is implemented as a monolithic component within the kernel, handling CPU virtualization, memory virtualization, and device emulation.

### 2. Architecture Abstraction
Common hypervisor core with architecture-specific backends allows code reuse while supporting platform-specific optimizations.

### 3. Rust-First Implementation
Leverages Rust's type system and memory safety for hypervisor implementation, minimizing security vulnerabilities.

### 4. Integration with Microkernel
Despite the monolithic hypervisor core, integration with Redox's microkernel architecture is maintained through:
- System call interface
- Scheme-based device management
- User-space VM management daemon

## Next Steps

1. **Verify Compilation**: Add module to kernel and ensure it compiles
2. **Implement x86_64 Detection**: Start with CPUID-based VMX/SVM detection
3. **Memory Integration**: Connect with Redox Memory Manager
4. **Testing Framework**: Set up unit tests for core functionality

## References

- Implementation Plan: `/home/user/ockham/doc/hypervisor_implementation_plan.md`
- Project Intent: `/home/user/ockham/doc/Intent_redox_hypervisor_1.md`
- Environment Setup: `/home/user/ockham/ENVIRONMENT.md`
- Kernel Source: `/home/user/ockham/redox-repos/kernel/`

## Files Created

Total: 14 files

**Core Modules** (5 files):
1. `src/hypervisor/mod.rs` - 97 lines
2. `src/hypervisor/vm.rs` - 187 lines
3. `src/hypervisor/vcpu.rs` - 171 lines
4. `src/hypervisor/memory.rs` - 61 lines
5. `src/hypervisor/devices/mod.rs` - 31 lines

**Architecture Layer** (9 files):
6. `src/hypervisor/arch/mod.rs` - 55 lines
7. `src/hypervisor/arch/x86_64/mod.rs` - 149 lines
8. `src/hypervisor/arch/x86_64/vmx.rs` - 16 lines
9. `src/hypervisor/arch/x86_64/svm.rs` - 16 lines
10. `src/hypervisor/arch/aarch64/mod.rs` - 123 lines
11. `src/hypervisor/arch/riscv64/mod.rs` - 131 lines

**Total Lines of Code**: ~1,037 lines

---

**Created**: November 12, 2025
**Status**: Initial structure complete, ready for Phase 1 implementation
