//! Intel VMX (Virtual Machine Extensions) support

use crate::hypervisor::Result;
use crate::memory::{self, Frame};
use crate::paging::PhysicalAddress;
use core::arch::x86_64::__cpuid;
use core::sync::atomic::{AtomicU64, Ordering};

/// Check if VMX is available
pub fn is_available() -> bool {
    // Check CPUID.1:ECX.VMX[bit 5] = 1
    unsafe {
        let cpuid = __cpuid(1);
        // Bit 5 of ECX indicates VMX support
        (cpuid.ecx & (1 << 5)) != 0
    }
}

/// Check if VMX is enabled in BIOS/UEFI
pub fn is_enabled_in_firmware() -> bool {
    // Check IA32_FEATURE_CONTROL MSR (MSR 0x3A)
    // Bit 0: Lock bit (must be set)
    // Bit 2: Enable VMX outside SMX operation
    unsafe {
        let msr_value = read_msr(0x3A);
        let locked = (msr_value & 1) != 0;
        let vmx_enabled = (msr_value & (1 << 2)) != 0;
        locked && vmx_enabled
    }
}

/// Read Model Specific Register
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

/// Write Model Specific Register
#[inline]
unsafe fn write_msr(msr: u32, value: u64) {
    let low = value as u32;
    let high = (value >> 32) as u32;
    core::arch::asm!(
        "wrmsr",
        in("ecx") msr,
        in("eax") low,
        in("edx") high,
        options(nomem, nostack)
    );
}

/// VMXON region (4KB aligned, must be in low memory)
/// Format: First 4 bytes are VMCS revision identifier
#[repr(C, align(4096))]
struct VmxonRegion {
    revision_id: u32,
    _reserved: [u8; 4092],
}

/// Per-CPU VMXON region (static for now, will be per-CPU later)
static VMXON_REGION: AtomicU64 = AtomicU64::new(0);

/// Enable VMX operation by setting CR4.VMXE and executing VMXON
unsafe fn enable_vmx_operation() -> Result<()> {
    // Read IA32_VMX_BASIC MSR to get VMCS revision identifier
    const IA32_VMX_BASIC: u32 = 0x480;
    let vmx_basic = read_msr(IA32_VMX_BASIC);
    let vmcs_revision_id = (vmx_basic & 0x7FFFFFFF) as u32;

    log::debug!("VMX: VMCS revision ID: {:#x}", vmcs_revision_id);

    // Allocate VMXON region (4KB aligned)
    let vmxon_frame = memory::allocate_frame()
        .ok_or(crate::hypervisor::HypervisorError::OutOfMemory)?;
    let vmxon_phys = vmxon_frame.base().data();

    // Write VMCS revision ID to VMXON region
    let vmxon_virt = crate::memory::phys_to_virt(vmxon_phys);
    *(vmxon_virt as *mut u32) = vmcs_revision_id;

    // Clear bit 31 (must be 0 for VMXON region)
    *(vmxon_virt as *mut u32) &= 0x7FFFFFFF;

    // Store VMXON region for later cleanup
    VMXON_REGION.store(vmxon_phys as u64, Ordering::Release);

    // Set CR4.VMXE[bit 13] = 1
    core::arch::asm!(
        "mov rax, cr4",
        "or rax, {vmxe_bit}",
        "mov cr4, rax",
        vmxe_bit = const (1u64 << 13),
        out("rax") _,
        options(nostack, preserves_flags)
    );

    log::debug!("VMX: CR4.VMXE set");

    // Execute VMXON instruction
    let result: u8;
    core::arch::asm!(
        "vmxon [{}]",
        "setna {}",
        in(reg) &vmxon_phys,
        out(reg_byte) result,
        options(nostack)
    );

    if result != 0 {
        log::error!("VMX: VMXON instruction failed");
        return Err(crate::hypervisor::HypervisorError::InitializationFailed);
    }

    log::info!("VMX: VMXON successful, VMX operation enabled");
    Ok(())
}

/// Disable VMX operation
pub unsafe fn disable() -> Result<()> {
    // Execute VMXOFF instruction
    core::arch::asm!("vmxoff", options(nostack, nomem));

    // Clear CR4.VMXE
    core::arch::asm!(
        "mov rax, cr4",
        "and rax, {vmxe_mask}",
        "mov cr4, rax",
        vmxe_mask = const !(1u64 << 13),
        out("rax") _,
        options(nostack, preserves_flags)
    );

    // Free VMXON region
    let vmxon_phys = VMXON_REGION.swap(0, Ordering::AcqRel);
    if vmxon_phys != 0 {
        let frame = Frame::containing(PhysicalAddress::new(vmxon_phys));
        memory::deallocate_frame(frame);
    }

    log::info!("VMX: VMX operation disabled");
    Ok(())
}

/// Initialize VMX
pub fn init() -> Result<()> {
    if !is_available() {
        log::warn!("VMX: Not supported by CPU");
        return Err(crate::hypervisor::HypervisorError::NotSupported);
    }

    if !is_enabled_in_firmware() {
        log::warn!("VMX: Not enabled in BIOS/UEFI firmware");
        return Err(crate::hypervisor::HypervisorError::NotSupported);
    }

    log::info!("VMX: Available and enabled in firmware");

    // Enable VMX operation
    unsafe {
        enable_vmx_operation()?;
    }

    log::info!("VMX: Initialization complete");
    Ok(())
}
