//! Virtual CPU (VCPU) management
//!
//! This module defines the VCPU structure and execution control.

use super::{HypervisorError, Result};
use super::vm::VmId;
use core::sync::atomic::{AtomicU64, AtomicU8, Ordering};

/// VCPU ID type
pub type VcpuId = u64;

/// VCPU state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum VcpuState {
    /// VCPU is stopped
    Stopped = 0,
    /// VCPU is running
    Running = 1,
    /// VCPU is waiting for an event
    Waiting = 2,
    /// VCPU has exited for handling
    Exited = 3,
}

impl From<u8> for VcpuState {
    fn from(val: u8) -> Self {
        match val {
            0 => VcpuState::Stopped,
            1 => VcpuState::Running,
            2 => VcpuState::Waiting,
            3 => VcpuState::Exited,
            _ => VcpuState::Stopped,
        }
    }
}

/// VCPU exit reason
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VcpuExit {
    /// Unknown exit reason
    Unknown,
    /// External interrupt
    ExternalInterrupt,
    /// Exception occurred
    Exception(u32),
    /// I/O instruction (port, size, direction, value)
    Io { port: u16, size: u8, write: bool },
    /// MMIO access (address, size, write, value)
    Mmio { addr: u64, size: u8, write: bool },
    /// Halt instruction
    Halt,
    /// Shutdown requested
    Shutdown,
    /// Hypercall from guest
    Hypercall { nr: u64 },
    /// Debug/breakpoint
    Debug,
    /// Internal error
    InternalError(u64),
}

/// VCPU configuration
#[derive(Debug, Clone)]
pub struct VcpuConfig {
    /// VCPU index within the VM
    pub index: usize,
    /// Initial register state
    pub initial_regs: Option<VcpuRegs>,
}

/// Generic VCPU register state
///
/// Architecture-specific register state is stored in ArchVcpuData
#[derive(Debug, Clone, Default)]
pub struct VcpuRegs {
    /// Instruction pointer / Program counter
    pub pc: u64,
    /// Stack pointer
    pub sp: u64,
    /// General purpose registers (architecture-specific interpretation)
    pub gpr: [u64; 32],
    /// Flags/Status register
    pub flags: u64,
}

/// Virtual CPU structure
pub struct Vcpu {
    /// Unique VCPU ID
    id: VcpuId,
    /// Parent VM ID
    vm_id: VmId,
    /// Current state
    state: AtomicU8,
    /// VCPU configuration
    config: VcpuConfig,
    /// Generic register state
    regs: VcpuRegs,
    /// Last exit reason
    last_exit: VcpuExit,
    /// Architecture-specific VCPU data
    arch_data: crate::hypervisor::arch::ArchVcpuData,
}

impl Vcpu {
    /// Create a new VCPU
    pub fn new(id: VcpuId, vm_id: VmId, config: VcpuConfig) -> Result<Self> {
        let arch_data = crate::hypervisor::arch::ArchVcpuData::new(vm_id)?;

        let regs = config.initial_regs.clone().unwrap_or_default();

        Ok(Self {
            id,
            vm_id,
            state: AtomicU8::new(VcpuState::Stopped as u8),
            config,
            regs,
            last_exit: VcpuExit::Unknown,
            arch_data,
        })
    }

    /// Get VCPU ID
    pub fn id(&self) -> VcpuId {
        self.id
    }

    /// Get parent VM ID
    pub fn vm_id(&self) -> VmId {
        self.vm_id
    }

    /// Get current state
    pub fn state(&self) -> VcpuState {
        self.state.load(Ordering::SeqCst).into()
    }

    /// Set VCPU state
    pub fn set_state(&self, state: VcpuState) {
        self.state.store(state as u8, Ordering::SeqCst);
    }

    /// Get register state
    pub fn regs(&self) -> &VcpuRegs {
        &self.regs
    }

    /// Set register state
    pub fn set_regs(&mut self, regs: VcpuRegs) {
        self.regs = regs;
    }

    /// Get last exit reason
    pub fn last_exit(&self) -> VcpuExit {
        self.last_exit
    }

    /// Run the VCPU
    ///
    /// This will enter guest mode and execute until a VM-exit occurs.
    pub fn run(&mut self) -> Result<VcpuExit> {
        if self.state() != VcpuState::Stopped {
            return Err(HypervisorError::ArchError(0));
        }

        self.set_state(VcpuState::Running);

        // Synchronize register state to architecture-specific structure
        self.arch_data.set_regs(&self.regs)?;

        // Enter guest mode (architecture-specific)
        let exit_reason = self.arch_data.run()?;

        // Synchronize register state from architecture-specific structure
        self.regs = self.arch_data.get_regs()?;

        self.last_exit = exit_reason;
        self.set_state(VcpuState::Exited);

        Ok(exit_reason)
    }

    /// Resume VCPU after handling an exit
    pub fn resume(&mut self) -> Result<VcpuExit> {
        if self.state() != VcpuState::Exited {
            return Err(HypervisorError::ArchError(1));
        }

        self.set_state(VcpuState::Running);

        // Synchronize register state to architecture-specific structure
        self.arch_data.set_regs(&self.regs)?;

        // Resume guest mode (architecture-specific)
        let exit_reason = self.arch_data.run()?;

        // Synchronize register state from architecture-specific structure
        self.regs = self.arch_data.get_regs()?;

        self.last_exit = exit_reason;
        self.set_state(VcpuState::Exited);

        Ok(exit_reason)
    }

    /// Stop the VCPU
    pub fn stop(&mut self) -> Result<()> {
        self.set_state(VcpuState::Stopped);
        Ok(())
    }
}

/// Global VCPU counter for generating unique VCPU IDs
static VCPU_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Allocate a new VCPU ID
pub fn allocate_vcpu_id() -> VcpuId {
    VCPU_COUNTER.fetch_add(1, Ordering::SeqCst)
}
