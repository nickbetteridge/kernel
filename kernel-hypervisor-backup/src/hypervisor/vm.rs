//! Virtual Machine (VM) management
//!
//! This module defines the VM control block and lifecycle management.

use super::{HypervisorError, Result};
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, AtomicU8, Ordering};

/// VM ID type
pub type VmId = u64;

/// VM state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum VmState {
    /// VM is being created
    Creating = 0,
    /// VM is stopped
    Stopped = 1,
    /// VM is running
    Running = 2,
    /// VM is paused
    Paused = 3,
    /// VM is being destroyed
    Destroying = 4,
}

impl From<u8> for VmState {
    fn from(val: u8) -> Self {
        match val {
            0 => VmState::Creating,
            1 => VmState::Stopped,
            2 => VmState::Running,
            3 => VmState::Paused,
            4 => VmState::Destroying,
            _ => VmState::Stopped,
        }
    }
}

/// VM configuration
#[derive(Debug, Clone)]
pub struct VmConfig {
    /// Number of virtual CPUs
    pub num_vcpus: usize,
    /// Guest physical memory size in bytes
    pub memory_size: usize,
    /// VM name
    pub name: [u8; 64],
}

impl Default for VmConfig {
    fn default() -> Self {
        Self {
            num_vcpus: 1,
            memory_size: 128 * 1024 * 1024, // 128 MB default
            name: [0; 64],
        }
    }
}

/// VM Control Block (VCB)
///
/// This structure represents a virtual machine instance.
pub struct Vm {
    /// Unique VM ID
    id: VmId,
    /// Current VM state
    state: AtomicU8,
    /// Configuration
    config: VmConfig,
    /// List of VCPU IDs belonging to this VM
    vcpu_ids: Vec<u64>,
    /// Guest physical memory regions
    memory_regions: Vec<MemoryRegion>,
    /// Architecture-specific VM data
    arch_data: crate::hypervisor::arch::ArchVmData,
}

/// Guest physical memory region
#[derive(Debug, Clone)]
pub struct MemoryRegion {
    /// Guest physical address
    pub gpa: u64,
    /// Host physical address
    pub hpa: u64,
    /// Size in bytes
    pub size: usize,
    /// Memory flags
    pub flags: MemoryFlags,
}

bitflags::bitflags! {
    /// Memory region flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct MemoryFlags: u32 {
        /// Region is readable
        const READ = 1 << 0;
        /// Region is writable
        const WRITE = 1 << 1;
        /// Region is executable
        const EXEC = 1 << 2;
        /// Region is device memory
        const DEVICE = 1 << 3;
        /// Region uses write-back caching
        const CACHED = 1 << 4;
    }
}

impl Vm {
    /// Create a new VM
    pub fn new(id: VmId, config: VmConfig) -> Result<Self> {
        let arch_data = crate::hypervisor::arch::ArchVmData::new()?;

        Ok(Self {
            id,
            state: AtomicU8::new(VmState::Creating as u8),
            config,
            vcpu_ids: Vec::new(),
            memory_regions: Vec::new(),
            arch_data,
        })
    }

    /// Get VM ID
    pub fn id(&self) -> VmId {
        self.id
    }

    /// Get current VM state
    pub fn state(&self) -> VmState {
        self.state.load(Ordering::SeqCst).into()
    }

    /// Set VM state
    pub fn set_state(&self, state: VmState) {
        self.state.store(state as u8, Ordering::SeqCst);
    }

    /// Get VM configuration
    pub fn config(&self) -> &VmConfig {
        &self.config
    }

    /// Add a VCPU to this VM
    pub fn add_vcpu(&mut self, vcpu_id: u64) -> Result<()> {
        if self.vcpu_ids.len() >= self.config.num_vcpus {
            return Err(HypervisorError::MaxVcpusReached);
        }
        self.vcpu_ids.push(vcpu_id);
        Ok(())
    }

    /// Get VCPU IDs
    pub fn vcpu_ids(&self) -> &[u64] {
        &self.vcpu_ids
    }

    /// Map guest physical memory
    pub fn map_memory(&mut self, region: MemoryRegion) -> Result<()> {
        // Validate region doesn't overlap
        for existing in &self.memory_regions {
            if regions_overlap(existing, &region) {
                return Err(HypervisorError::InvalidMemoryRegion);
            }
        }

        // Map in architecture-specific backend
        self.arch_data.map_memory(&region)?;

        self.memory_regions.push(region);
        Ok(())
    }

    /// Unmap guest physical memory
    pub fn unmap_memory(&mut self, gpa: u64, size: usize) -> Result<()> {
        // Find and remove region
        let idx = self
            .memory_regions
            .iter()
            .position(|r| r.gpa == gpa && r.size == size)
            .ok_or(HypervisorError::InvalidMemoryRegion)?;

        let region = self.memory_regions.remove(idx);

        // Unmap in architecture-specific backend
        self.arch_data.unmap_memory(&region)?;

        Ok(())
    }

    /// Get memory regions
    pub fn memory_regions(&self) -> &[MemoryRegion] {
        &self.memory_regions
    }
}

/// Check if two memory regions overlap
fn regions_overlap(a: &MemoryRegion, b: &MemoryRegion) -> bool {
    let a_end = a.gpa + a.size as u64;
    let b_end = b.gpa + b.size as u64;

    (a.gpa < b_end) && (b.gpa < a_end)
}

/// Global VM counter for generating unique VM IDs
static VM_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Allocate a new VM ID
pub fn allocate_vm_id() -> VmId {
    VM_COUNTER.fetch_add(1, Ordering::SeqCst)
}
