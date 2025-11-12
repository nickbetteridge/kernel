//! Architecture-specific hypervisor backends
//!
//! This module provides the architecture abstraction layer for the hypervisor.

use super::{HypervisorCaps, HypervisorError, Result};
use super::vm::MemoryRegion;
use super::vcpu::{VcpuExit, VcpuRegs};

// Architecture-specific modules
#[cfg(target_arch = "x86_64")]
pub mod x86_64;

#[cfg(target_arch = "aarch64")]
pub mod aarch64;

#[cfg(target_arch = "riscv64")]
pub mod riscv64;

// Re-export architecture-specific types
#[cfg(target_arch = "x86_64")]
pub use self::x86_64::{ArchVcpuData, ArchVmData};

#[cfg(target_arch = "aarch64")]
pub use self::aarch64::{ArchVcpuData, ArchVmData};

#[cfg(target_arch = "riscv64")]
pub use self::riscv64::{ArchVcpuData, ArchVmData};

/// Detect hardware virtualization capabilities
pub fn detect_capabilities() -> Result<HypervisorCaps> {
    #[cfg(target_arch = "x86_64")]
    return x86_64::detect_capabilities();

    #[cfg(target_arch = "aarch64")]
    return aarch64::detect_capabilities();

    #[cfg(target_arch = "riscv64")]
    return riscv64::detect_capabilities();

    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64", target_arch = "riscv64")))]
    Err(HypervisorError::NotSupported)
}

/// Initialize architecture-specific hypervisor backend
pub fn init(caps: &HypervisorCaps) -> Result<()> {
    #[cfg(target_arch = "x86_64")]
    return x86_64::init(caps);

    #[cfg(target_arch = "aarch64")]
    return aarch64::init(caps);

    #[cfg(target_arch = "riscv64")]
    return riscv64::init(caps);

    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64", target_arch = "riscv64")))]
    Err(HypervisorError::NotSupported)
}
