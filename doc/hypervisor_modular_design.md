# Modular Hypervisor Architecture for Redox OS

## Overview

This document describes the modular hypervisor architecture that supports three distinct virtualization modes on Redox OS:

1. **Type 1 Hypervisor** - Full hardware virtualization (VMX/SVM/EL2/H-extension)
2. **VirtIO Mode** - Paravirtualization with VirtIO interfaces
3. **HVT Mode** - Solo5-style hardware virtualized tender for unikernels

## Design Goals

- **Modularity**: Clean separation between modes
- **Shared Components**: Reuse common code across modes
- **Runtime Selection**: Choose mode at initialization or per-VM
- **Minimal Overhead**: Each mode optimized for its use case
- **Compatibility**: Support for OCaml-Solo5 compiled unikernels (HVT mode)

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                    User Space Applications                      │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐         │
│  │ VM Manager   │  │  Unikernel   │  │   MirageOS   │         │
│  │    CLI       │  │   Builder    │  │   Runtime    │         │
│  └──────────────┘  └──────────────┘  └──────────────┘         │
└─────────────────────────────────────────────────────────────────┘
                         ↕ System Calls
┌─────────────────────────────────────────────────────────────────┐
│                    Redox Microkernel                            │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │          Hypervisor Core (Mode-Agnostic)                 │  │
│  │  ┌────────────┐  ┌──────────────┐  ┌────────────────┐  │  │
│  │  │    VM      │  │    Memory    │  │    Device      │  │  │
│  │  │ Management │  │  Management  │  │   Framework    │  │  │
│  │  └────────────┘  └──────────────┘  └────────────────┘  │  │
│  └──────────────────────────────────────────────────────────┘  │
│                           ↕                                     │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │        Hypervisor Mode Abstraction (Trait)               │  │
│  │        - HypervisorMode trait                            │  │
│  │        - Common interface for all modes                  │  │
│  └──────────────────────────────────────────────────────────┘  │
│       ↕                    ↕                    ↕               │
│  ┌──────────┐       ┌──────────┐       ┌──────────┐           │
│  │  Type 1  │       │  VirtIO  │       │   HVT    │           │
│  │   Mode   │       │   Mode   │       │   Mode   │           │
│  └──────────┘       └──────────┘       └──────────┘           │
│       ↕                    ↕                    ↕               │
│  ┌──────────────────────────────────────────────────────┐     │
│  │   Architecture-Specific Backends                     │     │
│  │   x86_64 │ aarch64 │ riscv64                        │     │
│  └──────────────────────────────────────────────────────┘     │
└─────────────────────────────────────────────────────────────────┘
                         ↕ Hardware
┌─────────────────────────────────────────────────────────────────┐
│              Hardware (CPU, Memory, Devices)                    │
└─────────────────────────────────────────────────────────────────┘
```

## Virtualization Modes

### Mode 1: Type 1 Hypervisor (Full Virtualization)

**Purpose**: Run complete guest operating systems with full isolation

**Features**:
- Full CPU virtualization (VMX/SVM/EL2/H-extension)
- Extended/Nested/Stage-2 page tables for memory virtualization
- Virtual interrupt controllers (APIC/GIC/PLIC)
- Full device emulation or passthrough
- Support for unmodified guest OSes

**Use Cases**:
- Running Linux, BSD, or multiple Redox instances
- Testing and development environments
- Server consolidation
- Security isolation

**Architecture-Specific**:
- **x86_64**: Intel VMX or AMD SVM with EPT/NPT
- **aarch64**: ARM EL2 with Stage-2 translation
- **riscv64**: H-extension with G-stage translation

### Mode 2: VirtIO Mode (Paravirtualization)

**Purpose**: High-performance virtualization with guest awareness

**Features**:
- Guest uses VirtIO drivers (aware of virtualization)
- Shared memory for efficient I/O
- Hypercalls for optimized operations
- Reduced VM-exit overhead
- Lightweight compared to full virtualization

**Use Cases**:
- High-performance guest VMs
- Cloud/container-like workloads
- Guests designed for virtualized environments
- Development and testing with paravirt-aware guests

**Components**:
- VirtIO transport layer (PCI, MMIO)
- VirtIO devices:
  - virtio-net (networking)
  - virtio-blk (block storage)
  - virtio-console (serial console)
  - virtio-fs (filesystem)
  - virtio-gpu (graphics)

**Implementation**:
- Can run on top of Type 1 hypervisor or standalone
- Uses hardware virtualization but with paravirt interfaces
- Guest drivers from virtio project

### Mode 3: HVT Mode (Hardware Virtualized Tender)

**Purpose**: Fast, lightweight execution of unikernels (especially OCaml-Solo5)

**Features**:
- Minimal "tender" (not a full hypervisor)
- Uses hardware virtualization (KVM-like on Redox)
- Single address space per unikernel
- Minimal syscall interface
- Very fast boot times (<1ms)
- Minimal attack surface

**Use Cases**:
- Running MirageOS unikernels
- OCaml-Solo5 compiled applications
- Serverless/FaaS workloads
- Microservices with strict isolation
- Research and experimental OSes

**Solo5 HVT Architecture**:
- Tender loads unikernel into memory
- Sets up minimal VM execution environment
- Mediates I/O through manifest
- Only needs /dev/kvm equivalent on Linux (native on Redox)
- Simple hypercall interface for I/O

**Components**:
- HVT Tender: Loads and manages unikernel
- Manifest System: Declares required resources
- Hypercall Interface: Minimal I/O operations
- Block I/O: Simple block device interface
- Network I/O: Simple network interface

**Comparison to Solo5**:
- Solo5 HVT uses KVM on Linux
- Redox HVT uses native Redox virtualization
- Compatible with Solo5 unikernel ABI
- Can run OCaml-Solo5 binaries

## Mode Selection

### Configuration

Modes can be selected:

1. **Globally**: Set default mode for the system
```rust
HypervisorConfig {
    default_mode: HypervisorMode::Hvt,
    ...
}
```

2. **Per-VM**: Each VM can use a different mode
```rust
VmConfig {
    mode: HypervisorMode::Type1,
    ...
}
```

3. **Auto-detection**: Based on VM image type
- ELF with Solo5 header → HVT mode
- ISO/disk image → Type 1 mode
- VirtIO manifest → VirtIO mode

### Mode Enum

```rust
pub enum HypervisorMode {
    /// Full hardware virtualization
    Type1,
    /// Paravirtualization with VirtIO
    VirtIO,
    /// Solo5-style hardware virtualized tender
    Hvt,
}
```

## Shared Components

### VM Management (Mode-Agnostic)
- VM lifecycle (create, start, stop, destroy)
- VM ID allocation
- State management
- Resource tracking

### Memory Management (Partially Shared)
- Guest physical memory allocation
- Host physical address mapping
- Common GPA→HPA translation interface
- Mode-specific page table management

### Device Framework (Partially Shared)
- Device abstraction layer
- MMIO/PIO emulation framework
- Interrupt injection interface
- Mode-specific device implementations

## Mode-Specific Implementations

### Type 1 Mode Structure

```
hypervisor/
├── modes/
│   └── type1/
│       ├── mod.rs              # Type 1 mode implementation
│       ├── vm.rs               # Type 1 VM control
│       ├── vcpu.rs             # Type 1 VCPU management
│       ├── memory.rs           # EPT/NPT/Stage-2 management
│       └── arch/
│           ├── x86_64/         # VMX/SVM
│           ├── aarch64/        # EL2
│           └── riscv64/        # H-extension
```

### VirtIO Mode Structure

```
hypervisor/
├── modes/
│   └── virtio/
│       ├── mod.rs              # VirtIO mode implementation
│       ├── vm.rs               # VirtIO VM control
│       ├── transport.rs        # VirtIO transport (PCI/MMIO)
│       ├── devices/
│       │   ├── net.rs          # virtio-net
│       │   ├── blk.rs          # virtio-blk
│       │   ├── console.rs      # virtio-console
│       │   ├── fs.rs           # virtio-fs
│       │   └── gpu.rs          # virtio-gpu
│       └── queue.rs            # VirtQueue implementation
```

### HVT Mode Structure

```
hypervisor/
├── modes/
│   └── hvt/
│       ├── mod.rs              # HVT mode implementation
│       ├── tender.rs           # HVT tender (loader/manager)
│       ├── manifest.rs         # Resource manifest parsing
│       ├── hypercall.rs        # Hypercall interface
│       ├── solo5_abi.rs        # Solo5 ABI compatibility
│       ├── devices/
│       │   ├── block.rs        # Simple block I/O
│       │   └── net.rs          # Simple network I/O
│       └── arch/
│           ├── x86_64/         # x86_64 HVT backend
│           ├── aarch64/        # aarch64 HVT backend
│           └── riscv64/        # riscv64 HVT backend
```

## Common Hypervisor Trait

All modes implement a common trait:

```rust
pub trait HypervisorMode {
    /// Initialize the hypervisor mode
    fn init(config: &ModeConfig) -> Result<Self> where Self: Sized;

    /// Create a new VM in this mode
    fn create_vm(&mut self, config: VmConfig) -> Result<VmId>;

    /// Destroy a VM
    fn destroy_vm(&mut self, vm_id: VmId) -> Result<()>;

    /// Create a VCPU for a VM
    fn create_vcpu(&mut self, vm_id: VmId, config: VcpuConfig) -> Result<VcpuId>;

    /// Run a VCPU
    fn run_vcpu(&mut self, vm_id: VmId, vcpu_id: VcpuId) -> Result<VcpuExit>;

    /// Map memory for a VM
    fn map_memory(&mut self, vm_id: VmId, region: MemoryRegion) -> Result<()>;

    /// Inject an interrupt
    fn inject_interrupt(&mut self, vm_id: VmId, vcpu_id: VcpuId, vector: u32) -> Result<()>;

    /// Get mode-specific capabilities
    fn capabilities(&self) -> ModeCapabilities;
}
```

## Implementation Strategy

### Phase 1: Refactor Existing Code (Week 1-2)
1. ✅ Already have: Core VM/VCPU structures
2. Create mode abstraction trait
3. Move Type 1 code into modes/type1/
4. Ensure Type 1 mode still works

### Phase 2: VirtIO Mode (Week 3-6)
1. Implement VirtIO transport layer
2. Create virtio-net device
3. Create virtio-blk device
4. Create virtio-console device
5. Test with VirtIO-aware guests

### Phase 3: HVT Mode (Week 7-10)
1. Study Solo5 HVT implementation in detail
2. Implement HVT tender
3. Implement Solo5 ABI compatibility
4. Implement manifest system
5. Implement hypercall interface
6. Test with OCaml-Solo5 unikernels

### Phase 4: Integration and Testing (Week 11-12)
1. System call interface for mode selection
2. VM management daemon updates
3. Integration testing
4. Documentation
5. Benchmarking all three modes

## System Call Interface

New system calls for mode selection:

```rust
// Query available modes
sys_hypervisor_query_modes() -> Result<ModeFlags>

// Create VM with specific mode
sys_vm_create(config: &VmConfig) -> Result<VmId>
// VmConfig now includes:
struct VmConfig {
    mode: HypervisorMode,  // NEW: mode selection
    num_vcpus: usize,
    memory_size: usize,
    name: [u8; 64],
    // Mode-specific config
    mode_config: ModeSpecificConfig,
}
```

## Configuration Examples

### Type 1 VM (Full Virtualization)

```toml
[vm]
name = "linux-vm"
mode = "type1"
vcpus = 4
memory = "2G"

[vm.type1]
# Type 1 specific config
virtualization = "vmx"  # or "svm", "el2", "h-ext"
nested = false
```

### VirtIO VM (Paravirtualization)

```toml
[vm]
name = "virtio-vm"
mode = "virtio"
vcpus = 2
memory = "1G"

[vm.virtio]
# VirtIO specific config
transport = "pci"  # or "mmio"

[[vm.virtio.devices]]
type = "net"
driver = "virtio-net"

[[vm.virtio.devices]]
type = "blk"
driver = "virtio-blk"
path = "/path/to/disk.img"
```

### HVT Unikernel

```toml
[vm]
name = "mirageos-unikernel"
mode = "hvt"
memory = "64M"

[vm.hvt]
# HVT specific config
unikernel = "/path/to/unikernel.hvt"
manifest = "auto"  # or path to manifest

[[vm.hvt.resources]]
type = "block"
path = "/path/to/data.img"

[[vm.hvt.resources]]
type = "net"
interface = "tap0"
```

## Performance Characteristics

| Mode | Boot Time | Memory Overhead | CPU Overhead | Use Case |
|------|-----------|-----------------|--------------|----------|
| Type 1 | ~1-5s | High (full VM) | Higher (full virt) | Full OS guests |
| VirtIO | ~0.5-2s | Medium | Medium (paravirt) | Para-aware guests |
| HVT | <10ms | Low (unikernel) | Low (minimal) | Unikernels |

## Compatibility Matrix

| Mode | Guest Type | Required Guest Support |
|------|------------|------------------------|
| Type 1 | Any OS | None (unmodified) |
| VirtIO | Para-aware OS | VirtIO drivers |
| HVT | Unikernel | Solo5 ABI |

## Solo5 HVT Compatibility

### Solo5 ABI Support

The HVT mode implements Solo5's hypercall ABI:

- `solo5_hypercall_puts()` - Console output
- `solo5_hypercall_blkinfo()` - Block device info
- `solo5_hypercall_blkread()` - Block read
- `solo5_hypercall_blkwrite()` - Block write
- `solo5_hypercall_netinfo()` - Network info
- `solo5_hypercall_netread()` - Network read
- `solo5_hypercall_netwrite()` - Network write
- `solo5_hypercall_exit()` - Exit unikernel

### Unikernel Binary Format

HVT mode recognizes Solo5 ELF binaries:
- ELF64 format
- Solo5 note section
- Single load segment
- Entry point at solo5_start

### OCaml-Solo5 Support

Full support for unikernels compiled with ocaml-solo5:
- MirageOS applications
- OCaml programs compiled to Solo5 target
- Compatible with Solo5 0.6.x+ ABI

## Migration Path from Solo5

For users coming from Solo5:

1. **Compile unikernel**:
   ```bash
   mirage configure -t solo5-hvt
   make
   ```

2. **Run on Redox**:
   ```bash
   # Instead of: solo5-hvt unikernel.hvt
   # Use:
   redox-hvt unikernel.hvt
   ```

3. **Configuration**: Manifest-based resource declaration (compatible with Solo5)

## Development Roadmap

### Immediate (Weeks 1-2)
- Refactor existing code for modularity
- Implement mode abstraction trait
- Move Type 1 code to modes/type1/

### Short-term (Weeks 3-6)
- Implement VirtIO mode
- Basic VirtIO devices (net, blk, console)

### Medium-term (Weeks 7-10)
- Implement HVT mode
- Solo5 ABI compatibility
- Test with OCaml-Solo5 unikernels

### Long-term (Weeks 11-12+)
- Integration and testing
- Performance optimization
- Documentation and examples

## References

- **Solo5**: https://github.com/Solo5/solo5
- **Solo5 Architecture**: https://github.com/Solo5/solo5/blob/master/docs/architecture.md
- **VirtIO Specification**: https://docs.oasis-open.org/virtio/virtio/
- **MirageOS**: https://mirage.io/
- **OCaml-Solo5**: https://github.com/mirage/ocaml-solo5

## Next Steps

1. Review and approve this modular design
2. Begin refactoring existing hypervisor code
3. Implement mode abstraction trait
4. Create mode-specific directory structure
5. Implement each mode incrementally

---

**Document Version**: 1.0
**Date**: November 12, 2025
**Status**: Design Proposal
