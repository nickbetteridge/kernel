//! Redox OS Type-1 Hypervisor
//!
//! This module implements a type-1 hypervisor for Redox OS, supporting:
//! - x86_64 (Intel VMX and AMD SVM)
//! - aarch64 (ARM EL2 virtualization)
//! - riscv64 (RISC-V H-extension)
//!
//! The hypervisor follows a monolithic design with architecture-specific backends,
//! inspired by xvisor while integrating with Redox's microkernel architecture.

#![allow(dead_code)]

pub mod vm;
pub mod vcpu;
pub mod memory;
pub mod arch;
pub mod devices;

use core::sync::atomic::{AtomicBool, Ordering};

/// Global flag indicating if hypervisor is initialized
static HYPERVISOR_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Hypervisor capability flags
#[derive(Debug, Clone, Copy)]
pub struct HypervisorCaps {
    /// Hardware virtualization support available
    pub hw_virt_available: bool,
    /// Architecture type
    pub arch: HypervisorArch,
    /// Maximum number of VMs supported
    pub max_vms: usize,
    /// Maximum number of VCPUs per VM
    pub max_vcpus_per_vm: usize,
    /// Nested virtualization support
    pub nested_virt: bool,
}

/// Supported hypervisor architectures
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HypervisorArch {
    X86_64,
    Aarch64,
    Riscv64,
}

/// Hypervisor error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HypervisorError {
    /// Hardware virtualization not supported
    NotSupported,
    /// Hypervisor already initialized
    AlreadyInitialized,
    /// Invalid VM ID
    InvalidVmId,
    /// Invalid VCPU ID
    InvalidVcpuId,
    /// Maximum VMs reached
    MaxVmsReached,
    /// Maximum VCPUs reached
    MaxVcpusReached,
    /// Memory allocation failed
    MemoryAllocationFailed,
    /// Invalid memory region
    InvalidMemoryRegion,
    /// Architecture-specific error
    ArchError(u64),
}

pub type Result<T> = core::result::Result<T, HypervisorError>;

/// Initialize the hypervisor subsystem
///
/// This function detects hardware virtualization capabilities and initializes
/// the architecture-specific backend.
pub fn init() -> Result<HypervisorCaps> {
    if HYPERVISOR_INITIALIZED.swap(true, Ordering::SeqCst) {
        return Err(HypervisorError::AlreadyInitialized);
    }

    // Detect architecture and capabilities
    let caps = arch::detect_capabilities()?;

    if !caps.hw_virt_available {
        HYPERVISOR_INITIALIZED.store(false, Ordering::SeqCst);
        return Err(HypervisorError::NotSupported);
    }

    // Initialize architecture-specific backend
    arch::init(&caps)?;

    log::info!("Hypervisor initialized: {:?}", caps);

    Ok(caps)
}

/// Check if hypervisor is initialized
pub fn is_initialized() -> bool {
    HYPERVISOR_INITIALIZED.load(Ordering::SeqCst)
}
