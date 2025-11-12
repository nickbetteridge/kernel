//! Virtual device emulation
//!
//! This module will contain device emulation for guests.

use crate::hypervisor::Result;

/// Virtual device trait
pub trait VirtualDevice {
    /// Handle MMIO read
    fn mmio_read(&mut self, addr: u64, size: u8) -> Result<u64>;

    /// Handle MMIO write
    fn mmio_write(&mut self, addr: u64, size: u8, value: u64) -> Result<()>;

    /// Handle I/O port read (x86-specific)
    fn io_read(&mut self, port: u16, size: u8) -> Result<u32> {
        // Default implementation: not supported
        Ok(0xFFFFFFFF)
    }

    /// Handle I/O port write (x86-specific)
    fn io_write(&mut self, port: u16, size: u8, value: u32) -> Result<()> {
        // Default implementation: not supported
        Ok(())
    }
}

// TODO: Implement specific devices:
// - Serial console (16550 UART, PL011)
// - VirtIO transport (PCI, MMIO)
// - Timer devices
// - Interrupt controllers (virtual APIC, GIC, PLIC)
