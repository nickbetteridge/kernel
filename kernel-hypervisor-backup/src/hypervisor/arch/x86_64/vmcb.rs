//! VMCB (Virtual Machine Control Block) implementation for AMD SVM
//!
//! The VMCB is a 4KB data structure divided into two areas:
//! - Control Area (0x000-0x3FF): VM execution controls, intercepts, etc.
//! - State Save Area (0x400-0xFFF): Guest and host state

use crate::hypervisor::{HypervisorError, Result};
use crate::memory::{self, Frame};
use crate::paging::PhysicalAddress;

/// VMCB Control Area (first 1KB of VMCB)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
struct VmcbControlArea {
    // Intercept controls
    cr_read_intercept: u16,
    cr_write_intercept: u16,
    dr_read_intercept: u16,
    dr_write_intercept: u16,
    exception_intercept: u32,
    intercept_misc1: u32,
    intercept_misc2: u32,
    intercept_misc3: u32,

    _reserved1: [u8; 36],

    // Control fields
    pause_filter_thresh: u16,
    pause_filter_count: u16,
    iopm_base_pa: u64,
    msrpm_base_pa: u64,
    tsc_offset: u64,

    guest_asid: u32,
    tlb_control: u32,

    vintr: u64,
    interrupt_shadow: u64,
    exitcode: u64,
    exitinfo1: u64,
    exitinfo2: u64,
    exitintinfo: u64,

    np_enable: u64,
    avic_apic_bar: u64,
    guest_pa_of_ghcb: u64,
    eventinj: u64,
    n_cr3: u64,  // Nested page table CR3

    lbr_virt_enable: u64,
    vmcb_clean: u64,
    nrip: u64,

    num_of_bytes_fetched: u8,
    guest_instr_bytes: [u8; 15],

    avic_apic_backing_page_ptr: u64,
    _reserved2: u64,
    avic_logical_table_ptr: u64,
    avic_physical_table_ptr: u64,

    _reserved3: u64,
    vmcb_save_state_ptr: u64,

    _reserved4: [u8; 752],
}

/// VMCB State Save Area (second 3KB of VMCB)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
struct VmcbStateSaveArea {
    // Segment selectors
    es_selector: u16,
    es_attrib: u16,
    es_limit: u32,
    es_base: u64,

    cs_selector: u16,
    cs_attrib: u16,
    cs_limit: u32,
    cs_base: u64,

    ss_selector: u16,
    ss_attrib: u16,
    ss_limit: u32,
    ss_base: u64,

    ds_selector: u16,
    ds_attrib: u16,
    ds_limit: u32,
    ds_base: u64,

    fs_selector: u16,
    fs_attrib: u16,
    fs_limit: u32,
    fs_base: u64,

    gs_selector: u16,
    gs_attrib: u16,
    gs_limit: u32,
    gs_base: u64,

    gdtr_selector: u16,
    gdtr_attrib: u16,
    gdtr_limit: u32,
    gdtr_base: u64,

    ldtr_selector: u16,
    ldtr_attrib: u16,
    ldtr_limit: u32,
    ldtr_base: u64,

    idtr_selector: u16,
    idtr_attrib: u16,
    idtr_limit: u32,
    idtr_base: u64,

    tr_selector: u16,
    tr_attrib: u16,
    tr_limit: u32,
    tr_base: u64,

    _reserved1: [u8; 43],

    cpl: u8,
    _reserved2: u32,

    efer: u64,

    _reserved3: [u8; 112],

    // Control registers
    cr4: u64,
    cr3: u64,
    cr0: u64,
    dr7: u64,
    dr6: u64,
    rflags: u64,
    rip: u64,

    _reserved4: [u8; 88],

    rsp: u64,

    _reserved5: [u8; 24],

    rax: u64,
    star: u64,
    lstar: u64,
    cstar: u64,
    sfmask: u64,
    kernel_gs_base: u64,
    sysenter_cs: u64,
    sysenter_esp: u64,
    sysenter_eip: u64,
    cr2: u64,

    _reserved6: [u8; 32],

    g_pat: u64,
    dbgctl: u64,
    br_from: u64,
    br_to: u64,
    last_excp_from: u64,
    last_excp_to: u64,

    _reserved7: [u8; 2408],
}

/// Complete VMCB structure (4KB)
#[repr(C, align(4096))]
pub struct Vmcb {
    control: VmcbControlArea,
    save: VmcbStateSaveArea,
}

impl Vmcb {
    /// Allocate a new VMCB
    pub fn new() -> Result<VmcbHandle> {
        // Allocate 4KB-aligned frame
        let frame = memory::allocate_frame()
            .ok_or(HypervisorError::OutOfMemory)?;
        let phys_addr = frame.base().data();

        // Clear VMCB
        let virt_addr = crate::memory::phys_to_virt(phys_addr);
        unsafe {
            core::ptr::write_bytes(virt_addr as *mut u8, 0, 4096);
        }

        Ok(VmcbHandle {
            phys_addr: phys_addr as u64,
            virt_addr: virt_addr as u64,
        })
    }
}

/// Handle to a VMCB
pub struct VmcbHandle {
    phys_addr: u64,
    virt_addr: u64,
}

impl VmcbHandle {
    /// Get physical address
    pub fn phys_addr(&self) -> u64 {
        self.phys_addr
    }

    /// Get mutable reference to control area
    pub fn control_mut(&mut self) -> &mut VmcbControlArea {
        unsafe { &mut *(self.virt_addr as *mut Vmcb) }.control_mut()
    }

    /// Get mutable reference to state save area
    pub fn save_mut(&mut self) -> &mut VmcbStateSaveArea {
        unsafe { &mut *(self.virt_addr as *mut Vmcb) }.save_mut()
    }

    /// Get reference to control area
    pub fn control(&self) -> &VmcbControlArea {
        unsafe { &*(self.virt_addr as *const Vmcb) }.control()
    }

    /// Get reference to state save area
    pub fn save(&self) -> &VmcbStateSaveArea {
        unsafe { &*(self.virt_addr as *const Vmcb) }.save()
    }

    /// Initialize VMCB with default values
    pub fn initialize(&mut self, guest_asid: u32) -> Result<()> {
        // Set up control area
        let control = self.control_mut();

        // Intercept all exceptions for now
        control.exception_intercept = 0xFFFFFFFF;

        // Intercept important CR accesses
        control.cr_read_intercept = 0x0001;  // CR0
        control.cr_write_intercept = 0x0011; // CR0, CR4

        // Set guest ASID (Address Space ID)
        control.guest_asid = guest_asid;

        // Enable nested paging (NPT)
        control.np_enable = 1;

        // TLB control: flush all
        control.tlb_control = 1;

        // Set up state save area with host state
        let save = self.save_mut();

        // Copy current control registers
        unsafe {
            save.cr0 = read_cr0();
            save.cr3 = read_cr3();
            save.cr4 = read_cr4();
            save.efer = read_msr(0xC0000080); // IA32_EFER
        }

        // Set up segment registers
        unsafe {
            save.cs_selector = read_cs();
            save.ss_selector = read_ss();
            save.ds_selector = read_ds();
            save.es_selector = read_es();
            save.fs_selector = read_fs();
            save.gs_selector = read_gs();

            // Set up descriptor table registers
            let (gdtr_base, gdtr_limit) = read_gdtr();
            save.gdtr_base = gdtr_base;
            save.gdtr_limit = gdtr_limit;

            let (idtr_base, idtr_limit) = read_idtr();
            save.idtr_base = idtr_base;
            save.idtr_limit = idtr_limit;

            // Set up MSRs
            save.fs_base = read_msr(0xC0000100); // IA32_FS_BASE
            save.gs_base = read_msr(0xC0000101); // IA32_GS_BASE
            save.kernel_gs_base = read_msr(0xC0000102); // IA32_KERNEL_GS_BASE
        }

        log::debug!("VMCB: Initialized at {:#x} with ASID {}", self.phys_addr, guest_asid);
        Ok(())
    }

    /// Run the guest (execute VMRUN)
    pub fn run(&mut self) -> Result<u64> {
        unsafe {
            // VMRUN expects physical address in RAX
            let vmcb_pa = self.phys_addr;
            let exit_code: u64;

            core::arch::asm!(
                "vmrun",
                inout("rax") vmcb_pa => exit_code,
                options(nostack)
            );

            // Exit code is in VMCB control area
            Ok(self.control().exitcode)
        }
    }
}

impl Vmcb {
    fn control_mut(&mut self) -> &mut VmcbControlArea {
        &mut self.control
    }

    fn save_mut(&mut self) -> &mut VmcbStateSaveArea {
        &mut self.save
    }

    fn control(&self) -> &VmcbControlArea {
        &self.control
    }

    fn save(&self) -> &VmcbStateSaveArea {
        &self.save
    }
}

impl Drop for VmcbHandle {
    fn drop(&mut self) {
        unsafe {
            let frame = Frame::containing(PhysicalAddress::new(self.phys_addr));
            memory::deallocate_frame(frame);
        }
        log::trace!("VMCB: Dropped VMCB at {:#x}", self.phys_addr);
    }
}

/// VMEXIT codes
#[repr(u64)]
#[allow(dead_code)]
pub enum VmexitCode {
    CrRead = 0x00,
    CrWrite = 0x10,
    DrRead = 0x20,
    DrWrite = 0x30,
    Exception = 0x40,
    Intr = 0x60,
    Nmi = 0x61,
    Smi = 0x62,
    Init = 0x63,
    Vintr = 0x64,
    Cr0Selective = 0x65,
    IdtrRead = 0x66,
    GdtrRead = 0x67,
    LdtrRead = 0x68,
    TrRead = 0x69,
    IdtrWrite = 0x6A,
    GdtrWrite = 0x6B,
    LdtrWrite = 0x6C,
    TrWrite = 0x6D,
    Rdtsc = 0x6E,
    Rdpmc = 0x6F,
    Pushf = 0x70,
    Popf = 0x71,
    Cpuid = 0x72,
    Rsm = 0x73,
    Iret = 0x74,
    Int = 0x75,
    Invd = 0x76,
    Pause = 0x77,
    Hlt = 0x78,
    Invlpg = 0x79,
    Invlpga = 0x7A,
    Ioio = 0x7B,
    Msr = 0x7C,
    TaskSwitch = 0x7D,
    FerrFreeze = 0x7E,
    Shutdown = 0x7F,
    Vmrun = 0x80,
    Vmmcall = 0x81,
    Vmload = 0x82,
    Vmsave = 0x83,
    Stgi = 0x84,
    Clgi = 0x85,
    Skinit = 0x86,
    Rdtscp = 0x87,
    Icebp = 0x88,
    Wbinvd = 0x89,
    Monitor = 0x8A,
    Mwait = 0x8B,
    MwaitCond = 0x8C,
    Xsetbv = 0x8D,
    Rdpru = 0x8E,
    Efer = 0x8F,
    Cr0Trap = 0x90,
    NptFault = 0x400,
    Invalid = 0xFFFFFFFFFFFFFFFF,
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
unsafe fn read_gdtr() -> (u64, u32) {
    let mut gdt: [u8; 10] = [0; 10];
    core::arch::asm!(
        "sgdt [{}]",
        in(reg) &mut gdt,
        options(nostack)
    );
    // Extract limit (bytes 0-1) and base (bytes 2-9)
    let limit = u16::from_le_bytes([gdt[0], gdt[1]]) as u32;
    let base = u64::from_le_bytes([
        gdt[2], gdt[3], gdt[4], gdt[5],
        gdt[6], gdt[7], gdt[8], gdt[9],
    ]);
    (base, limit)
}

#[inline]
unsafe fn read_idtr() -> (u64, u32) {
    let mut idt: [u8; 10] = [0; 10];
    core::arch::asm!(
        "sidt [{}]",
        in(reg) &mut idt,
        options(nostack)
    );
    // Extract limit (bytes 0-1) and base (bytes 2-9)
    let limit = u16::from_le_bytes([idt[0], idt[1]]) as u32;
    let base = u64::from_le_bytes([
        idt[2], idt[3], idt[4], idt[5],
        idt[6], idt[7], idt[8], idt[9],
    ]);
    (base, limit)
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
