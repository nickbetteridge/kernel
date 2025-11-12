//! aarch64 hypervisor backend
//!
//! Supports ARM EL2 virtualization (Hypervisor exception level).

use crate::hypervisor::{HypervisorArch, HypervisorCaps, HypervisorError, Result};
use crate::hypervisor::vm::{MemoryRegion, VmId};
use crate::hypervisor::vcpu::{VcpuExit, VcpuRegs};

/// aarch64-specific VM data
pub struct ArchVmData {
    /// VTTBR_EL2 (Stage-2 translation table base)
    stage2_table_base: u64,
}

impl ArchVmData {
    /// Create new architecture-specific VM data
    pub fn new() -> Result<Self> {
        Ok(Self {
            stage2_table_base: 0, // TODO: Allocate Stage-2 page tables
        })
    }

    /// Map guest physical memory
    pub fn map_memory(&mut self, region: &MemoryRegion) -> Result<()> {
        // TODO: Update Stage-2 page tables
        log::debug!(
            "Mapping memory region: IPA={:#x}, PA={:#x}, size={:#x}",
            region.gpa,
            region.hpa,
            region.size
        );
        Ok(())
    }

    /// Unmap guest physical memory
    pub fn unmap_memory(&mut self, region: &MemoryRegion) -> Result<()> {
        // TODO: Update Stage-2 page tables
        log::debug!(
            "Unmapping memory region: IPA={:#x}, size={:#x}",
            region.gpa,
            region.size
        );
        Ok(())
    }
}

/// aarch64-specific VCPU data
pub struct ArchVcpuData {
    /// Parent VM ID
    vm_id: VmId,
    /// Saved guest system registers
    guest_sys_regs: GuestSysRegs,
}

/// Guest system registers
#[derive(Default)]
struct GuestSysRegs {
    // EL1 system registers
    sctlr_el1: u64,
    ttbr0_el1: u64,
    ttbr1_el1: u64,
    tcr_el1: u64,
    esr_el1: u64,
    far_el1: u64,
    // Add more as needed
}

impl ArchVcpuData {
    /// Create new architecture-specific VCPU data
    pub fn new(vm_id: VmId) -> Result<Self> {
        Ok(Self {
            vm_id,
            guest_sys_regs: GuestSysRegs::default(),
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
        // TODO: Enter guest at EL1
        // 1. Load guest context
        // 2. Execute ERET to drop to EL1
        // 3. Handle trap to EL2
        log::trace!("Running VCPU (VM ID: {})", self.vm_id);

        // Placeholder
        Ok(VcpuExit::Unknown)
    }
}

/// Check if EL2 is available
fn is_el2_available() -> bool {
    // TODO: Check CurrentEL register
    // If we're running at EL2 or higher, virtualization is available
    false
}

/// Detect hardware virtualization capabilities
pub fn detect_capabilities() -> Result<HypervisorCaps> {
    if !is_el2_available() {
        return Err(HypervisorError::NotSupported);
    }

    Ok(HypervisorCaps {
        hw_virt_available: true,
        arch: HypervisorArch::Aarch64,
        max_vms: 64,
        max_vcpus_per_vm: 256,
        nested_virt: false,
    })
}

/// Initialize aarch64 hypervisor backend
pub fn init(caps: &HypervisorCaps) -> Result<()> {
    // TODO: Initialize EL2
    // 1. Set up HCR_EL2 (Hypervisor Configuration Register)
    // 2. Set up VTCR_EL2 (Virtualization Translation Control Register)
    // 3. Set up exception vectors for EL2
    log::info!("Initializing aarch64 EL2 hypervisor");
    Ok(())
}
