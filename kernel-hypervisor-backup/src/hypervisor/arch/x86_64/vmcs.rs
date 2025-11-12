//! VMCS (Virtual Machine Control Structure) implementation
//!
//! The VMCS is a 4KB data structure that stores:
//! - Guest state (registers, segment descriptors, control registers)
//! - Host state (where to return on VM exit)
//! - VM execution controls
//! - VM exit controls
//! - VM entry controls

use crate::hypervisor::{HypervisorError, Result};
use crate::memory;

/// VMCS region (4KB aligned)
#[repr(C, align(4096))]
pub struct Vmcs {
    revision_id: u32,
    _data: [u8; 4092],
}

impl Vmcs {
    /// Allocate a new VMCS
    pub fn new(revision_id: u32) -> Result<VmcsHandle> {
        // Allocate 4KB-aligned frame
        let frame = memory::Frame::allocate()
            .ok_or(HypervisorError::OutOfMemory)?;
        let phys_addr = frame.start_address().data();

        // Initialize VMCS header
        let virt_addr = crate::memory::phys_to_virt(phys_addr);
        unsafe {
            // Write revision ID (bit 31 must be 0)
            *(virt_addr as *mut u32) = revision_id & 0x7FFFFFFF;
            // Clear the rest
            core::ptr::write_bytes((virt_addr as *mut u8).add(4), 0, 4092);
        }

        Ok(VmcsHandle {
            phys_addr: phys_addr as u64,
            virt_addr: virt_addr as u64,
        })
    }
}

/// Handle to a VMCS
pub struct VmcsHandle {
    phys_addr: u64,
    virt_addr: u64,
}

impl VmcsHandle {
    /// Get physical address
    pub fn phys_addr(&self) -> u64 {
        self.phys_addr
    }

    /// Clear the VMCS (make it inactive and ready for initialization)
    pub fn clear(&self) -> Result<()> {
        unsafe {
            let mut result: u8;
            core::arch::asm!(
                "vmclear [{}]",
                "setna {}",
                in(reg) &self.phys_addr,
                out(reg_byte) result,
                options(nostack)
            );

            if result != 0 {
                log::error!("VMCS: VMCLEAR failed");
                return Err(HypervisorError::InitializationFailed);
            }
        }

        log::trace!("VMCS: VMCLEAR successful at {:#x}", self.phys_addr);
        Ok(())
    }

    /// Load the VMCS (make it active and current)
    pub fn load(&self) -> Result<()> {
        unsafe {
            let mut result: u8;
            core::arch::asm!(
                "vmptrld [{}]",
                "setna {}",
                in(reg) &self.phys_addr,
                out(reg_byte) result,
                options(nostack)
            );

            if result != 0 {
                log::error!("VMCS: VMPTRLD failed");
                return Err(HypervisorError::InitializationFailed);
            }
        }

        log::trace!("VMCS: VMPTRLD successful at {:#x}", self.phys_addr);
        Ok(())
    }

    /// Read a VMCS field
    pub fn read(&self, field: VmcsField) -> Result<u64> {
        let value: u64;
        unsafe {
            let mut result: u8;
            core::arch::asm!(
                "vmread {value}, {field}",
                "setna {result}",
                field = in(reg) field as u64,
                value = out(reg) value,
                result = out(reg_byte) result,
                options(nostack)
            );

            if result != 0 {
                log::error!("VMCS: VMREAD failed for field {:#x}", field as u64);
                return Err(HypervisorError::InvalidParameter);
            }
        }

        Ok(value)
    }

    /// Write a VMCS field
    pub fn write(&self, field: VmcsField, value: u64) -> Result<()> {
        unsafe {
            let mut result: u8;
            core::arch::asm!(
                "vmwrite {field}, {value}",
                "setna {result}",
                field = in(reg) field as u64,
                value = in(reg) value,
                result = out(reg_byte) result,
                options(nostack)
            );

            if result != 0 {
                log::error!("VMCS: VMWRITE failed for field {:#x}", field as u64);
                return Err(HypervisorError::InvalidParameter);
            }
        }

        Ok(())
    }

    /// Initialize VMCS with default values
    pub fn initialize(&self) -> Result<()> {
        // Clear VMCS first
        self.clear()?;

        // Load VMCS to make it current
        self.load()?;

        // Set up minimal required fields
        // Host state fields (where to return on VM exit)
        self.write(VmcsField::HostCr0, read_cr0())?;
        self.write(VmcsField::HostCr3, read_cr3())?;
        self.write(VmcsField::HostCr4, read_cr4())?;

        // Read segment selectors
        let cs = read_cs();
        let ss = read_ss();
        let ds = read_ds();
        let es = read_es();
        let fs = read_fs();
        let gs = read_gs();
        let tr = read_tr();

        self.write(VmcsField::HostCsSelector, cs as u64)?;
        self.write(VmcsField::HostSsSelector, ss as u64)?;
        self.write(VmcsField::HostDsSelector, ds as u64)?;
        self.write(VmcsField::HostEsSelector, es as u64)?;
        self.write(VmcsField::HostFsSelector, fs as u64)?;
        self.write(VmcsField::HostGsSelector, gs as u64)?;
        self.write(VmcsField::HostTrSelector, tr as u64)?;

        // Read segment bases
        self.write(VmcsField::HostFsBase, read_msr(0xC0000100))?; // IA32_FS_BASE
        self.write(VmcsField::HostGsBase, read_msr(0xC0000101))?; // IA32_GS_BASE
        self.write(VmcsField::HostTrBase, 0)?; // TODO: Read from GDT

        // Read GDTR and IDTR
        let gdtr = read_gdtr();
        let idtr = read_idtr();
        self.write(VmcsField::HostGdtrBase, gdtr)?;
        self.write(VmcsField::HostIdtrBase, idtr)?;

        // Set up VM execution controls (minimal)
        // TODO: Read from MSRs and set appropriate values
        self.write(VmcsField::PinBasedVmExecControl, 0)?;
        self.write(VmcsField::PrimaryProcBasedVmExecControl, 0)?;

        // Set up VM exit controls
        // Bit 9: Host address-space size (1 = 64-bit mode)
        self.write(VmcsField::VmExitControls, 1 << 9)?;

        // Set up VM entry controls
        // Bit 9: IA-32e mode guest (1 = 64-bit guest)
        self.write(VmcsField::VmEntryControls, 1 << 9)?;

        log::debug!("VMCS: Initialized at {:#x}", self.phys_addr);
        Ok(())
    }
}

impl Drop for VmcsHandle {
    fn drop(&mut self) {
        // TODO: Deallocate frame at phys_addr
        log::trace!("VMCS: Dropping VMCS at {:#x}", self.phys_addr);
    }
}

/// VMCS field encodings
#[repr(u64)]
#[allow(dead_code)]
pub enum VmcsField {
    // 16-bit control fields
    VirtualProcessorId = 0x0000,
    PostedIntrNotifVector = 0x0002,
    EptpIndex = 0x0004,

    // 16-bit guest state
    GuestEsSelector = 0x0800,
    GuestCsSelector = 0x0802,
    GuestSsSelector = 0x0804,
    GuestDsSelector = 0x0806,
    GuestFsSelector = 0x0808,
    GuestGsSelector = 0x080A,
    GuestLdtrSelector = 0x080C,
    GuestTrSelector = 0x080E,

    // 16-bit host state
    HostEsSelector = 0x0C00,
    HostCsSelector = 0x0C02,
    HostSsSelector = 0x0C04,
    HostDsSelector = 0x0C06,
    HostFsSelector = 0x0C08,
    HostGsSelector = 0x0C0A,
    HostTrSelector = 0x0C0C,

    // 64-bit control fields
    IobitMapA = 0x2000,
    IobitMapB = 0x2002,
    MsrBitmap = 0x2004,
    VmExitMsrStoreAddr = 0x2006,
    VmExitMsrLoadAddr = 0x2008,
    VmEntryMsrLoadAddr = 0x200A,
    ExecutiveVmcsPointer = 0x200C,
    TscOffset = 0x2010,
    VirtualApicPageAddr = 0x2012,
    ApicAccessAddr = 0x2014,
    EptPointer = 0x201A,

    // 64-bit guest state
    VmcsLinkPointer = 0x2800,
    GuestIa32Debugctl = 0x2802,
    GuestIa32Pat = 0x2804,
    GuestIa32Efer = 0x2806,

    // 64-bit host state
    HostIa32Pat = 0x2C00,
    HostIa32Efer = 0x2C02,

    // 32-bit control fields
    PinBasedVmExecControl = 0x4000,
    PrimaryProcBasedVmExecControl = 0x4002,
    ExceptionBitmap = 0x4004,
    PageFaultErrorCodeMask = 0x4006,
    PageFaultErrorCodeMatch = 0x4008,
    Cr3TargetCount = 0x400A,
    VmExitControls = 0x400C,
    VmExitMsrStoreCount = 0x400E,
    VmExitMsrLoadCount = 0x4010,
    VmEntryControls = 0x4012,
    VmEntryMsrLoadCount = 0x4014,
    VmEntryIntrInfoField = 0x4016,
    SecondaryProcBasedVmExecControl = 0x401E,

    // 32-bit guest state
    GuestEsLimit = 0x4800,
    GuestCsLimit = 0x4802,
    GuestSsLimit = 0x4804,
    GuestDsLimit = 0x4806,
    GuestFsLimit = 0x4808,
    GuestGsLimit = 0x480A,
    GuestLdtrLimit = 0x480C,
    GuestTrLimit = 0x480E,
    GuestGdtrLimit = 0x4810,
    GuestIdtrLimit = 0x4812,
    GuestEsArBytes = 0x4814,
    GuestCsArBytes = 0x4816,
    GuestSsArBytes = 0x4818,
    GuestDsArBytes = 0x481A,
    GuestFsArBytes = 0x481C,
    GuestGsArBytes = 0x481E,
    GuestLdtrArBytes = 0x4820,
    GuestTrArBytes = 0x4822,
    GuestInterruptibilityInfo = 0x4824,
    GuestActivityState = 0x4826,
    GuestSysenterCs = 0x482A,

    // Natural-width control fields
    Cr0GuestHostMask = 0x6000,
    Cr4GuestHostMask = 0x6002,
    Cr0ReadShadow = 0x6004,
    Cr4ReadShadow = 0x6006,

    // Natural-width guest state
    GuestCr0 = 0x6800,
    GuestCr3 = 0x6802,
    GuestCr4 = 0x6804,
    GuestEsBase = 0x6806,
    GuestCsBase = 0x6808,
    GuestSsBase = 0x680A,
    GuestDsBase = 0x680C,
    GuestFsBase = 0x680E,
    GuestGsBase = 0x6810,
    GuestLdtrBase = 0x6812,
    GuestTrBase = 0x6814,
    GuestGdtrBase = 0x6816,
    GuestIdtrBase = 0x6818,
    GuestDr7 = 0x681A,
    GuestRsp = 0x681C,
    GuestRip = 0x681E,
    GuestRflags = 0x6820,
    GuestPendingDbgExceptions = 0x6822,
    GuestSysenterEsp = 0x6824,
    GuestSysenterEip = 0x6826,

    // Natural-width host state
    HostCr0 = 0x6C00,
    HostCr3 = 0x6C02,
    HostCr4 = 0x6C04,
    HostFsBase = 0x6C06,
    HostGsBase = 0x6C08,
    HostTrBase = 0x6C0A,
    HostGdtrBase = 0x6C0C,
    HostIdtrBase = 0x6C0E,
    HostSysenterEsp = 0x6C10,
    HostSysenterEip = 0x6C12,
    HostRsp = 0x6C14,
    HostRip = 0x6C16,
}

// Helper functions to read CPU state

#[inline]
unsafe fn read_cr0() -> u64 {
    let value: u64;
    core::arch::asm!("mov {}, cr0", out(reg) value, options(nomem, nostack));
    value
}

#[inline]
unsafe fn read_cr3() -> u64 {
    let value: u64;
    core::arch::asm!("mov {}, cr3", out(reg) value, options(nomem, nostack));
    value
}

#[inline]
unsafe fn read_cr4() -> u64 {
    let value: u64;
    core::arch::asm!("mov {}, cr4", out(reg) value, options(nomem, nostack));
    value
}

#[inline]
unsafe fn read_cs() -> u16 {
    let value: u16;
    core::arch::asm!("mov {:x}, cs", out(reg) value, options(nomem, nostack));
    value
}

#[inline]
unsafe fn read_ss() -> u16 {
    let value: u16;
    core::arch::asm!("mov {:x}, ss", out(reg) value, options(nomem, nostack));
    value
}

#[inline]
unsafe fn read_ds() -> u16 {
    let value: u16;
    core::arch::asm!("mov {:x}, ds", out(reg) value, options(nomem, nostack));
    value
}

#[inline]
unsafe fn read_es() -> u16 {
    let value: u16;
    core::arch::asm!("mov {:x}, es", out(reg) value, options(nomem, nostack));
    value
}

#[inline]
unsafe fn read_fs() -> u16 {
    let value: u16;
    core::arch::asm!("mov {:x}, fs", out(reg) value, options(nomem, nostack));
    value
}

#[inline]
unsafe fn read_gs() -> u16 {
    let value: u16;
    core::arch::asm!("mov {:x}, gs", out(reg) value, options(nomem, nostack));
    value
}

#[inline]
unsafe fn read_tr() -> u16 {
    let value: u16;
    core::arch::asm!("str {:x}", out(reg) value, options(nomem, nostack));
    value
}

#[inline]
unsafe fn read_gdtr() -> u64 {
    let mut gdt: [u8; 10] = [0; 10];
    core::arch::asm!(
        "sgdt [{}]",
        in(reg) &mut gdt,
        options(nostack)
    );
    // Extract base address (bytes 2-9)
    u64::from_le_bytes([
        gdt[2], gdt[3], gdt[4], gdt[5],
        gdt[6], gdt[7], gdt[8], gdt[9],
    ])
}

#[inline]
unsafe fn read_idtr() -> u64 {
    let mut idt: [u8; 10] = [0; 10];
    core::arch::asm!(
        "sidt [{}]",
        in(reg) &mut idt,
        options(nostack)
    );
    // Extract base address (bytes 2-9)
    u64::from_le_bytes([
        idt[2], idt[3], idt[4], idt[5],
        idt[6], idt[7], idt[8], idt[9],
    ])
}

#[inline]
unsafe fn read_msr(msr: u32) -> u64 {
    let (high, low): (u32, u32);
    core::arch::asm!(
        "rdmsr",
        in("ecx") msr,
        out("eax") low,
        out("edx") high,
        options(nomem, nostack)
    );
    ((high as u64) << 32) | (low as u64)
}
