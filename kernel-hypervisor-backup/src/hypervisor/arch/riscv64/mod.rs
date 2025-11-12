//! riscv64 hypervisor backend
//!
//! Supports RISC-V H-extension (Hypervisor extension).

use crate::hypervisor::{HypervisorArch, HypervisorCaps, HypervisorError, Result};
use crate::hypervisor::vm::{MemoryRegion, VmId};
use crate::hypervisor::vcpu::{VcpuExit, VcpuRegs};

/// riscv64-specific VM data
pub struct ArchVmData {
    /// hgatp (Hypervisor Guest Address Translation and Protection)
    /// Contains the G-stage page table root
    hgatp: u64,
}

impl ArchVmData {
    /// Create new architecture-specific VM data
    pub fn new() -> Result<Self> {
        Ok(Self {
            hgatp: 0, // TODO: Allocate G-stage page tables
        })
    }

    /// Map guest physical memory
    pub fn map_memory(&mut self, region: &MemoryRegion) -> Result<()> {
        // TODO: Update G-stage page tables
        log::debug!(
            "Mapping memory region: GPA={:#x}, PA={:#x}, size={:#x}",
            region.gpa,
            region.hpa,
            region.size
        );
        Ok(())
    }

    /// Unmap guest physical memory
    pub fn unmap_memory(&mut self, region: &MemoryRegion) -> Result<()> {
        // TODO: Update G-stage page tables
        log::debug!(
            "Unmapping memory region: GPA={:#x}, size={:#x}",
            region.gpa,
            region.size
        );
        Ok(())
    }
}

/// riscv64-specific VCPU data
pub struct ArchVcpuData {
    /// Parent VM ID
    vm_id: VmId,
    /// Saved guest CSRs (Control and Status Registers)
    guest_csrs: GuestCsrs,
}

/// Guest CSRs
#[derive(Default)]
struct GuestCsrs {
    // Supervisor-level CSRs
    sstatus: u64,
    sie: u64,
    stvec: u64,
    sscratch: u64,
    sepc: u64,
    scause: u64,
    stval: u64,
    sip: u64,
    satp: u64,
    // Add more as needed
}

impl ArchVcpuData {
    /// Create new architecture-specific VCPU data
    pub fn new(vm_id: VmId) -> Result<Self> {
        Ok(Self {
            vm_id,
            guest_csrs: GuestCsrs::default(),
        })
    }

    /// Get register state
    pub fn get_regs(&self) -> Result<VcpuRegs> {
        // TODO: Read guest registers
        Ok(VcpuRegs::default())
    }

    /// Set register state
    pub fn set_regs(&mut self, regs: &VcpuRegs) -> Result<()> {
        // TODO: Write guest registers
        Ok(())
    }

    /// Run the VCPU
    pub fn run(&mut self) -> Result<VcpuExit> {
        // TODO: Enter guest in VS-mode
        // 1. Load guest context
        // 2. Set hstatus.SPV to enter VS-mode
        // 3. Execute SRET
        // 4. Handle trap to HS-mode
        log::trace!("Running VCPU (VM ID: {})", self.vm_id);

        // Placeholder
        Ok(VcpuExit::Unknown)
    }
}

/// Check if H-extension is available
fn is_h_extension_available() -> bool {
    // TODO: Check for H-extension support
    // Read misa CSR and check bit 7 (H)
    false
}

/// Detect hardware virtualization capabilities
pub fn detect_capabilities() -> Result<HypervisorCaps> {
    if !is_h_extension_available() {
        return Err(HypervisorError::NotSupported);
    }

    // All three modes are supported on riscv64 with H-extension
    let supported_modes = crate::hypervisor::ModeSupportFlags::TYPE1
        | crate::hypervisor::ModeSupportFlags::VIRTIO
        | crate::hypervisor::ModeSupportFlags::HVT;

    Ok(HypervisorCaps {
        hw_virt_available: true,
        arch: HypervisorArch::Riscv64,
        max_vms: 64,
        max_vcpus_per_vm: 256,
        nested_virt: false,
        supported_modes,
    })
}

/// Initialize riscv64 hypervisor backend
pub fn init(caps: &HypervisorCaps) -> Result<()> {
    // TODO: Initialize H-extension
    // 1. Set up hstatus (Hypervisor Status Register)
    // 2. Set up hedeleg (Hypervisor Exception Delegation)
    // 3. Set up hideleg (Hypervisor Interrupt Delegation)
    // 4. Set up hgatp (Hypervisor Guest Address Translation)
    log::info!("Initializing RISC-V H-extension hypervisor");
    Ok(())
}
