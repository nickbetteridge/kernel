//! HVT (Hardware Virtualized Tender) mode
//!
//! Solo5-style minimal execution environment for unikernels.
//! Provides fast boot times (<10ms) and minimal attack surface.
//! Compatible with OCaml-Solo5 compiled unikernels (MirageOS).

use crate::hypervisor::{HypervisorError, Result};
use crate::hypervisor::mode::{HypervisorMode, HypervisorModeImpl, ModeCapabilities, ModeConfig};
use crate::hypervisor::vm::{MemoryRegion, VmConfig, VmId};
use crate::hypervisor::vcpu::{VcpuConfig, VcpuExit, VcpuId, VcpuRegs};
use alloc::vec::Vec;

/// HVT tender implementation
///
/// The "tender" is responsible for loading the unikernel into memory,
/// setting up its execution environment, and mediating its access to I/O resources.
pub struct HvtTender {
    /// List of running unikernels
    unikernels: Vec<Unikernel>,
}

/// Represents a running unikernel
struct Unikernel {
    /// VM ID
    vm_id: VmId,
    /// VCPU ID (unikernels typically use only one VCPU)
    vcpu_id: VcpuId,
    /// Entry point address
    entry_point: u64,
    /// Memory size
    memory_size: usize,
}

impl HypervisorModeImpl for HvtTender {
    fn init(config: &ModeConfig) -> Result<Self> {
        log::info!("Initializing HVT (Hardware Virtualized Tender) mode");

        // HVT mode uses minimal hardware virtualization
        // Just enough to provide isolation

        Ok(Self {
            unikernels: Vec::new(),
        })
    }

    fn mode(&self) -> HypervisorMode {
        HypervisorMode::Hvt
    }

    fn create_vm(&mut self, config: VmConfig) -> Result<VmId> {
        let vm_id = crate::hypervisor::vm::allocate_vm_id();

        log::debug!("Creating HVT unikernel: VM {}", vm_id);

        // TODO: Parse unikernel binary (ELF with Solo5 header)
        // TODO: Load unikernel into memory
        // TODO: Parse manifest for resource requirements

        Ok(vm_id)
    }

    fn destroy_vm(&mut self, vm_id: VmId) -> Result<()> {
        self.unikernels.retain(|u| u.vm_id != vm_id);
        log::debug!("Destroyed HVT unikernel: VM {}", vm_id);
        Ok(())
    }

    fn start_vm(&mut self, vm_id: VmId) -> Result<()> {
        log::debug!("Starting HVT unikernel: VM {}", vm_id);
        // HVT unikernels have very fast boot times
        // TODO: Jump to entry point
        Ok(())
    }

    fn stop_vm(&mut self, vm_id: VmId) -> Result<()> {
        log::debug!("Stopping HVT unikernel: VM {}", vm_id);
        Ok(())
    }

    fn pause_vm(&mut self, _vm_id: VmId) -> Result<()> {
        // HVT unikernels don't typically support pause/resume
        Err(HypervisorError::NotSupported)
    }

    fn resume_vm(&mut self, _vm_id: VmId) -> Result<()> {
        // HVT unikernels don't typically support pause/resume
        Err(HypervisorError::NotSupported)
    }

    fn create_vcpu(&mut self, vm_id: VmId, config: VcpuConfig) -> Result<VcpuId> {
        // HVT unikernels typically use only one VCPU
        let vcpu_id = crate::hypervisor::vcpu::allocate_vcpu_id();
        log::debug!("Created VCPU {} for HVT unikernel VM {}", vcpu_id, vm_id);
        Ok(vcpu_id)
    }

    fn run_vcpu(&mut self, vm_id: VmId, vcpu_id: VcpuId) -> Result<VcpuExit> {
        log::trace!("Running VCPU {} of HVT unikernel VM {}", vcpu_id, vm_id);

        // TODO: Enter unikernel execution
        // TODO: Handle hypercalls (Solo5 ABI)
        // Hypercalls include:
        // - solo5_hypercall_puts (console output)
        // - solo5_hypercall_blkinfo/blkread/blkwrite (block I/O)
        // - solo5_hypercall_netinfo/netread/netwrite (network I/O)
        // - solo5_hypercall_exit (terminate unikernel)

        Ok(VcpuExit::Hypercall { nr: 0 })
    }

    fn get_vcpu_regs(&self, vm_id: VmId, vcpu_id: VcpuId) -> Result<VcpuRegs> {
        Ok(VcpuRegs::default())
    }

    fn set_vcpu_regs(&mut self, vm_id: VmId, vcpu_id: VcpuId, regs: &VcpuRegs) -> Result<()> {
        Ok(())
    }

    fn map_memory(&mut self, vm_id: VmId, region: MemoryRegion) -> Result<()> {
        log::debug!(
            "Mapping memory for HVT unikernel VM {}: GPA={:#x}, size={:#x}",
            vm_id,
            region.gpa,
            region.size
        );

        // HVT uses a simple single address space
        // No complex page table management needed

        Ok(())
    }

    fn unmap_memory(&mut self, vm_id: VmId, gpa: u64, size: usize) -> Result<()> {
        log::debug!(
            "Unmapping memory for HVT unikernel VM {}: GPA={:#x}, size={:#x}",
            vm_id,
            gpa,
            size
        );
        Ok(())
    }

    fn inject_interrupt(&mut self, vm_id: VmId, vcpu_id: VcpuId, vector: u32) -> Result<()> {
        // HVT unikernels typically don't use traditional interrupts
        // I/O is handled via hypercalls
        log::trace!("HVT mode: interrupt injection not supported");
        Err(HypervisorError::NotSupported)
    }

    fn capabilities(&self) -> ModeCapabilities {
        ModeCapabilities {
            mode: HypervisorMode::Hvt,
            max_vms: 256,            // Can run many lightweight unikernels
            max_vcpus_per_vm: 1,     // Unikernels typically use single VCPU
            nested_virt: false,
            device_passthrough: false,
            boot_time_ms: 5,         // Very fast boot time (<10ms)
        }
    }
}

/// Solo5 ABI hypercall numbers
#[allow(dead_code)]
mod solo5_hypercalls {
    pub const SOLO5_HYPERCALL_PUTS: u64 = 0;
    pub const SOLO5_HYPERCALL_BLKINFO: u64 = 1;
    pub const SOLO5_HYPERCALL_BLKREAD: u64 = 2;
    pub const SOLO5_HYPERCALL_BLKWRITE: u64 = 3;
    pub const SOLO5_HYPERCALL_NETINFO: u64 = 4;
    pub const SOLO5_HYPERCALL_NETREAD: u64 = 5;
    pub const SOLO5_HYPERCALL_NETWRITE: u64 = 6;
    pub const SOLO5_HYPERCALL_EXIT: u64 = 7;
}

/// Solo5 unikernel ELF header marker
#[allow(dead_code)]
const SOLO5_ELF_NOTE_NAME: &[u8] = b"Solo5";

// TODO: Implement Solo5 ABI compatibility
// TODO: Implement manifest parsing
// TODO: Implement unikernel ELF loader
// TODO: Implement hypercall handling
