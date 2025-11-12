//! x86_64 hypervisor backend
//!
//! Supports Intel VMX and AMD SVM virtualization.

use crate::hypervisor::{HypervisorArch, HypervisorCaps, HypervisorError, Result};
use crate::hypervisor::vm::{MemoryRegion, VmId};
use crate::hypervisor::vcpu::{VcpuExit, VcpuRegs};

pub mod vmx;
pub mod svm;
pub mod vmcs;
pub mod vmcb;

/// Virtualization technology type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VirtTech {
    /// Intel VMX (Virtual Machine Extensions)
    Vmx,
    /// AMD SVM (Secure Virtual Machine)
    Svm,
}

/// x86_64-specific VM data
pub struct ArchVmData {
    /// Virtualization technology in use
    virt_tech: VirtTech,
    /// EPT/NPT pointer (for memory virtualization)
    page_table_root: u64,
}

impl ArchVmData {
    /// Create new architecture-specific VM data
    pub fn new() -> Result<Self> {
        let virt_tech = detect_virt_tech()?;

        Ok(Self {
            virt_tech,
            page_table_root: 0, // TODO: Allocate page tables
        })
    }

    /// Map guest physical memory
    pub fn map_memory(&mut self, region: &MemoryRegion) -> Result<()> {
        // TODO: Update EPT/NPT page tables
        log::debug!(
            "Mapping memory region: GPA={:#x}, HPA={:#x}, size={:#x}",
            region.gpa,
            region.hpa,
            region.size
        );
        Ok(())
    }

    /// Unmap guest physical memory
    pub fn unmap_memory(&mut self, region: &MemoryRegion) -> Result<()> {
        // TODO: Update EPT/NPT page tables
        log::debug!(
            "Unmapping memory region: GPA={:#x}, size={:#x}",
            region.gpa,
            region.size
        );
        Ok(())
    }
}

/// x86_64-specific VCPU data
pub struct ArchVcpuData {
    /// Parent VM ID
    vm_id: VmId,
    /// Virtualization technology in use
    virt_tech: VirtTech,
    /// VMCS (VMX) or VMCB (SVM) physical address
    control_structure: u64,
}

impl ArchVcpuData {
    /// Create new architecture-specific VCPU data
    pub fn new(vm_id: VmId) -> Result<Self> {
        let virt_tech = detect_virt_tech()?;

        Ok(Self {
            vm_id,
            virt_tech,
            control_structure: 0, // TODO: Allocate VMCS/VMCB
        })
    }

    /// Get register state
    pub fn get_regs(&self) -> Result<VcpuRegs> {
        // TODO: Read registers from VMCS/VMCB
        Ok(VcpuRegs::default())
    }

    /// Set register state
    pub fn set_regs(&mut self, regs: &VcpuRegs) -> Result<()> {
        // TODO: Write registers to VMCS/VMCB
        Ok(())
    }

    /// Run the VCPU
    pub fn run(&mut self) -> Result<VcpuExit> {
        // TODO: Execute VMLAUNCH/VMRESUME (VMX) or VMRUN (SVM)
        log::trace!("Running VCPU (VM ID: {})", self.vm_id);

        // Placeholder: return immediately with unknown exit
        Ok(VcpuExit::Unknown)
    }
}

/// Detect which virtualization technology is available
fn detect_virt_tech() -> Result<VirtTech> {
    // Check for VMX
    if vmx::is_available() {
        return Ok(VirtTech::Vmx);
    }

    // Check for SVM
    if svm::is_available() {
        return Ok(VirtTech::Svm);
    }

    Err(HypervisorError::NotSupported)
}

/// Detect hardware virtualization capabilities
pub fn detect_capabilities() -> Result<HypervisorCaps> {
    let virt_tech = detect_virt_tech()?;

    // All three modes are supported on x86_64 with hardware virtualization
    let supported_modes = crate::hypervisor::ModeSupportFlags::TYPE1
        | crate::hypervisor::ModeSupportFlags::VIRTIO
        | crate::hypervisor::ModeSupportFlags::HVT;

    Ok(HypervisorCaps {
        hw_virt_available: true,
        arch: HypervisorArch::X86_64,
        max_vms: 64,           // Arbitrary limit for now
        max_vcpus_per_vm: 256, // Arbitrary limit for now
        nested_virt: false,    // Not implemented yet
        supported_modes,
    })
}

/// Initialize x86_64 hypervisor backend
pub fn init(caps: &HypervisorCaps) -> Result<()> {
    let virt_tech = detect_virt_tech()?;

    match virt_tech {
        VirtTech::Vmx => {
            log::info!("Initializing Intel VMX");
            vmx::init()?;
        }
        VirtTech::Svm => {
            log::info!("Initializing AMD SVM");
            svm::init()?;
        }
    }

    Ok(())
}
