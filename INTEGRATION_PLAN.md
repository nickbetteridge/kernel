# Hypervisor Integration with Redox Infrastructure

## Current Status

### ✅ Correctly Integrated
1. **Frame Allocation** - Using `memory::Frame::allocate()`
2. **Address Translation** - Using `RmmA::phys_to_virt()`
3. **Logging** - Using Redox `log::` macros

### ❌ Needs Integration
1. **Frame Deallocation** - Add `memory::deallocate_frame()` calls
2. **Address Types** - Use `PhysicalAddress`/`VirtualAddress` consistently
3. **Page Flags** - Leverage Redox `PageFlags` as base for EPT/NPT flags

### ✅ Genuinely New (No Redox Equivalent)
1. **VMCS/VMCB structures** - Hardware-specific control structures
2. **EPT/NPT page tables** - Second-level translation (GPA→HPA)
3. **VMX/SVM initialization** - Hardware virtualization enablement

## Why EPT/NPT Must Be New

### Architecture Comparison

```
Normal Redox Paging:
┌──────────────┐      CR3      ┌──────────────┐
│ Host Virtual │ ──────────────>│ Host Physical│
│   Address    │   (x86 PT)     │   Address    │
└──────────────┘                └──────────────┘
  (Managed by Redox kernel page mapper)

Hypervisor Paging (Two Levels):
┌──────────────┐      CR3      ┌──────────────┐    EPT/NPT   ┌──────────────┐
│Guest Virtual │ ──────────────>│Guest Physical│ ───────────> │ Host Physical│
│   Address    │  (Guest PT)    │   Address    │  (Our code)  │   Address    │
└──────────────┘                └──────────────┘              └──────────────┘
```

### EPT Structure (Intel) - Different Format!
```rust
// Standard x86_64 PTE (Redox uses this)
struct PageTableEntry {
    present: bool,      // bit 0
    writable: bool,     // bit 1
    user: bool,         // bit 2
    // ... standard x86_64 bits
}

// EPT PTE (We must implement)
struct EptEntry {
    read: bool,         // bit 0 - DIFFERENT!
    write: bool,        // bit 1 - DIFFERENT!
    execute: bool,      // bit 2 - DIFFERENT!
    memory_type: u8,    // bits 3-5 - NEW!
    ignore_pat: bool,   // bit 6 - NEW!
    // ... EPT-specific bits
}
```

## Integration Strategy

### 1. Extend Redox Types (Don't Replace)

```rust
// Good: Extend Redox's types
pub struct EptFlags {
    base: PageFlags,     // Use Redox's flags as base
    memory_type: MemoryType,  // Add EPT-specific
    execute_disable: bool,     // Add EPT-specific
}

// Bad: Ignore Redox completely
pub struct EptFlags {
    bits: u64,  // Re-implementing everything
}
```

### 2. Use Redox Frame Allocator Fully

```rust
// Current (incomplete):
impl Drop for VmcsHandle {
    fn drop(&mut self) {
        // TODO: Deallocate frame at phys_addr
    }
}

// Fixed:
impl Drop for VmcsHandle {
    fn drop(&mut self) {
        unsafe {
            let frame = Frame::containing(PhysicalAddress::new(self.phys_addr));
            memory::deallocate_frame(frame);
        }
    }
}
```

### 3. Leverage RMM PageMapper Pattern

```rust
// Redox has PageMapper for host page tables
// We should create EptMapper/NptMapper following same pattern

pub struct EptMapper<'a> {
    allocator: &'a mut dyn FrameAllocator,
    ept_root: PhysicalAddress,  // Use Redox's type!
}

impl<'a> EptMapper<'a> {
    pub fn map(&mut self, gpa: PhysicalAddress, hpa: PhysicalAddress, flags: EptFlags) {
        // Implementation using Redox's frame allocator
    }
}
```

### 4. Use Existing Redox Primitives

```rust
// From Redox's paging module:
use crate::paging::{PhysicalAddress, VirtualAddress, PageFlags, PAGE_SIZE};

// Don't redefine these!
const MY_PAGE_SIZE: usize = 4096;  // ❌ Bad
use crate::paging::PAGE_SIZE;      // ✅ Good
```

## Action Items

### Immediate (Next Changes)
1. ✅ Add proper frame deallocation in Drop impls
2. ✅ Use PhysicalAddress type consistently
3. ✅ Create EptMapper/NptMapper following Redox PageMapper pattern
4. ✅ Leverage existing PageFlags as base for EPT/NPT flags

### Medium Term
5. Integration with Redox's memory mapping functions
6. Use Redox's existing address space abstractions where applicable
7. Hook into Redox's frame allocator statistics

### Long Term
8. Consider contributing EPT/NPT support back to RMM library
9. Generalize second-level paging for all architectures (Stage-2, G-stage)

## Key Principle

> **Extend, don't replace**: Redox has excellent memory management primitives.
> Our hypervisor should build ON TOP of them, not duplicate them.

The hypervisor adds:
- Second-level page tables (EPT/NPT) - genuinely new
- Hardware control structures (VMCS/VMCB) - genuinely new
- VM/VCPU management - genuinely new

But uses Redox for:
- Frame allocation/deallocation
- Address types and arithmetic
- Page size constants
- Existing mapper patterns
