//! Redox OS Modular Hypervisor
//!
//! This module implements a modular hypervisor for Redox OS supporting three modes:
//!
//! ## Mode 1: Type 1 Hypervisor (Full Virtualization)
//! - x86_64 (Intel VMX and AMD SVM)
//! - aarch64 (ARM EL2 virtualization)
//! - riscv64 (RISC-V H-extension)
//! - Full hardware virtualization for unmodified guest OSes
//!
//! ## Mode 2: VirtIO (Paravirtualization)
//! - Guest-aware virtualization using VirtIO interfaces
//! - High-performance I/O with virtio-net, virtio-blk, etc.
//! - Reduced VM-exit overhead
//!
//! ## Mode 3: HVT (Hardware Virtualized Tender)
//! - Solo5-style lightweight execution for unikernels
//! - Minimal attack surface, fast boot times (<10ms)
//! - Compatible with OCaml-Solo5 compiled unikernels
//!
//! The hypervisor follows a modular design with mode-specific implementations
//! sharing common components, inspired by xvisor and Solo5 while integrating
//! with Redox's microkernel architecture.

#![allow(dead_code)]

pub mod vm;
pub mod vcpu;
pub mod memory;
pub mod arch;
pub mod devices;
pub mod mode;
pub mod modes;

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
    /// Supported modes
    pub supported_modes: ModeSupportFlags,
}

bitflags::bitflags! {
    /// Flags indicating which virtualization modes are supported
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ModeSupportFlags: u8 {
        /// Type 1 hypervisor mode supported
        const TYPE1 = 1 << 0;
        /// VirtIO mode supported
        const VIRTIO = 1 << 1;
        /// HVT mode supported
        const HVT = 1 << 2;
    }
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
