//! Hypervisor mode abstraction
//!
//! This module defines the common interface that all hypervisor modes must implement.
//! Redox supports three virtualization modes:
//!
//! 1. Type 1 Hypervisor - Full hardware virtualization (VMX/SVM/EL2/H-extension)
//! 2. VirtIO Mode - Paravirtualization with VirtIO interfaces
//! 3. HVT Mode - Solo5-style hardware virtualized tender for unikernels

use super::{HypervisorError, Result};
use super::vm::{MemoryRegion, VmConfig, VmId};
use super::vcpu::{VcpuConfig, VcpuExit, VcpuId, VcpuRegs};

/// Hypervisor mode selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HypervisorMode {
    /// Full hardware virtualization
    /// Uses VMX (Intel), SVM (AMD), EL2 (ARM), or H-extension (RISC-V)
    Type1,

    /// Paravirtualization with VirtIO
    /// Guest aware of virtualization, uses VirtIO drivers
    VirtIO,

    /// Solo5-style hardware virtualized tender
    /// Lightweight execution environment for unikernels
    Hvt,
}

impl HypervisorMode {
    /// Get the mode name as a string
    pub fn name(&self) -> &'static str {
        match self {
            HypervisorMode::Type1 => "type1",
            HypervisorMode::VirtIO => "virtio",
            HypervisorMode::Hvt => "hvt",
        }
    }

    /// Parse mode from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "type1" => Some(HypervisorMode::Type1),
            "virtio" => Some(HypervisorMode::VirtIO),
            "hvt" => Some(HypervisorMode::Hvt),
            _ => None,
        }
    }
}

/// Mode-specific configuration
#[derive(Debug, Clone)]
pub enum ModeConfig {
    /// Type 1 hypervisor configuration
    Type1(Type1Config),

    /// VirtIO mode configuration
    VirtIO(VirtIOConfig),

    /// HVT mode configuration
    Hvt(HvtConfig),
}

/// Type 1 hypervisor configuration
#[derive(Debug, Clone)]
pub struct Type1Config {
    /// Enable nested virtualization
    pub nested_virt: bool,

    /// Use large pages where possible
    pub large_pages: bool,
}

impl Default for Type1Config {
    fn default() -> Self {
        Self {
            nested_virt: false,
            large_pages: true,
        }
    }
}

/// VirtIO mode configuration
#[derive(Debug, Clone)]
pub struct VirtIOConfig {
    /// Transport type (PCI or MMIO)
    pub transport: VirtIOTransport,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VirtIOTransport {
    Pci,
    Mmio,
}

impl Default for VirtIOConfig {
    fn default() -> Self {
        Self {
            transport: VirtIOTransport::Mmio,
        }
    }
}

/// HVT mode configuration
#[derive(Debug, Clone)]
pub struct HvtConfig {
    /// Path to unikernel binary
    pub unikernel_path: Option<[u8; 256]>,

    /// Use manifest for resource declaration
    pub use_manifest: bool,
}

impl Default for HvtConfig {
    fn default() -> Self {
        Self {
            unikernel_path: None,
            use_manifest: true,
        }
    }
}

/// Mode-specific capabilities
#[derive(Debug, Clone)]
pub struct ModeCapabilities {
    /// Mode type
    pub mode: HypervisorMode,

    /// Maximum VMs supported
    pub max_vms: usize,

    /// Maximum VCPUs per VM
    pub max_vcpus_per_vm: usize,

    /// Supports nested virtualization
    pub nested_virt: bool,

    /// Supports device passthrough
    pub device_passthrough: bool,

    /// Typical boot time (milliseconds)
    pub boot_time_ms: u64,
}

/// Common trait that all hypervisor modes must implement
pub trait HypervisorModeImpl: Send + Sync {
    /// Initialize the hypervisor mode
    fn init(config: &ModeConfig) -> Result<Self> where Self: Sized;

    /// Get mode type
    fn mode(&self) -> HypervisorMode;

    /// Create a new VM in this mode
    fn create_vm(&mut self, config: VmConfig) -> Result<VmId>;

    /// Destroy a VM
    fn destroy_vm(&mut self, vm_id: VmId) -> Result<()>;

    /// Start a VM
    fn start_vm(&mut self, vm_id: VmId) -> Result<()>;

    /// Stop a VM
    fn stop_vm(&mut self, vm_id: VmId) -> Result<()>;

    /// Pause a VM
    fn pause_vm(&mut self, vm_id: VmId) -> Result<()>;

    /// Resume a VM
    fn resume_vm(&mut self, vm_id: VmId) -> Result<()>;

    /// Create a VCPU for a VM
    fn create_vcpu(&mut self, vm_id: VmId, config: VcpuConfig) -> Result<VcpuId>;

    /// Run a VCPU (blocks until VM exit)
    fn run_vcpu(&mut self, vm_id: VmId, vcpu_id: VcpuId) -> Result<VcpuExit>;

    /// Get VCPU register state
    fn get_vcpu_regs(&self, vm_id: VmId, vcpu_id: VcpuId) -> Result<VcpuRegs>;

    /// Set VCPU register state
    fn set_vcpu_regs(&mut self, vm_id: VmId, vcpu_id: VcpuId, regs: &VcpuRegs) -> Result<()>;

    /// Map memory for a VM
    fn map_memory(&mut self, vm_id: VmId, region: MemoryRegion) -> Result<()>;

    /// Unmap memory for a VM
    fn unmap_memory(&mut self, vm_id: VmId, gpa: u64, size: usize) -> Result<()>;

    /// Inject an interrupt into a VCPU
    fn inject_interrupt(&mut self, vm_id: VmId, vcpu_id: VcpuId, vector: u32) -> Result<()>;

    /// Get mode-specific capabilities
    fn capabilities(&self) -> ModeCapabilities;
}
