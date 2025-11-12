//! AMD SVM (Secure Virtual Machine) support

use crate::hypervisor::Result;
use crate::memory::{self, Frame};
use crate::paging::PhysicalAddress;
use core::arch::x86_64::__cpuid;
use core::sync::atomic::{AtomicU64, Ordering};

/// Check if SVM is available
pub fn is_available() -> bool {
    // Check CPUID.80000001h:ECX.SVM[bit 2] = 1
    unsafe {
        // First check if extended CPUID is available
        let max_extended = __cpuid(0x80000000).eax;
        if max_extended < 0x80000001 {
            return false;
        }

        let cpuid = __cpuid(0x80000001);
        // Bit 2 of ECX indicates SVM support
        (cpuid.ecx & (1 << 2)) != 0
    }
}

/// Check if SVM is disabled in BIOS
pub fn is_disabled_in_firmware() -> bool {
    // Check VM_CR MSR (MSR 0xC0010114)
    // Bit 4: SVMDIS - if set, SVM is disabled
    unsafe {
        let msr_value = read_msr(0xC0010114);
        (msr_value & (1 << 4)) != 0
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

/// Host Save Area (4KB aligned, per-CPU)
/// AMD SVM requires a host state save area for each CPU
#[repr(C, align(4096))]
struct HostSaveArea {
    _data: [u8; 4096],
}

/// Per-CPU Host Save Area (static for now, will be per-CPU later)
static HOST_SAVE_AREA: AtomicU64 = AtomicU64::new(0);

/// MSR addresses for SVM
const MSR_EFER: u32 = 0xC0000080;      // Extended Feature Enable Register
const MSR_VM_HSAVE_PA: u32 = 0xC0010117; // Host Save Area Physical Address

/// Enable SVM operation by setting EFER.SVME
unsafe fn enable_svm_operation() -> Result<()> {
    // Allocate Host Save Area (4KB aligned)
    let host_save_frame = memory::allocate_frame()
        .ok_or(crate::hypervisor::HypervisorError::OutOfMemory)?;
    let host_save_phys = host_save_frame.base().data();

    // Clear the host save area
    let host_save_virt = crate::memory::phys_to_virt(host_save_phys);
    core::ptr::write_bytes(host_save_virt as *mut u8, 0, 4096);

    // Store host save area for later cleanup
    HOST_SAVE_AREA.store(host_save_phys as u64, Ordering::Release);

    // Set VM_HSAVE_PA MSR to point to host save area
    write_msr(MSR_VM_HSAVE_PA, host_save_phys as u64);

    log::debug!("SVM: Host Save Area at {:#x}", host_save_phys);

    // Set EFER.SVME[bit 12] = 1 to enable SVM
    let mut efer = read_msr(MSR_EFER);
    efer |= 1 << 12; // Set SVME bit
    write_msr(MSR_EFER, efer);

    log::info!("SVM: EFER.SVME set, SVM operation enabled");

    // Verify EFER.SVME is set
    let efer_verify = read_msr(MSR_EFER);
    if (efer_verify & (1 << 12)) == 0 {
        log::error!("SVM: Failed to set EFER.SVME");
        return Err(crate::hypervisor::HypervisorError::InitializationFailed);
    }

    Ok(())
}

/// Disable SVM operation
pub unsafe fn disable() -> Result<()> {
    // Clear EFER.SVME
    let mut efer = read_msr(MSR_EFER);
    efer &= !(1 << 12); // Clear SVME bit
    write_msr(MSR_EFER, efer);

    // Free Host Save Area
    let host_save_phys = HOST_SAVE_AREA.swap(0, Ordering::AcqRel);
    if host_save_phys != 0 {
        let frame = Frame::containing(PhysicalAddress::new(host_save_phys));
        memory::deallocate_frame(frame);
    }

    log::info!("SVM: SVM operation disabled");
    Ok(())
}

/// Initialize SVM
pub fn init() -> Result<()> {
    if !is_available() {
        log::warn!("SVM: Not supported by CPU");
        return Err(crate::hypervisor::HypervisorError::NotSupported);
    }

    if is_disabled_in_firmware() {
        log::warn!("SVM: Disabled in BIOS/UEFI firmware");
        return Err(crate::hypervisor::HypervisorError::NotSupported);
    }

    log::info!("SVM: Available and not disabled in firmware");

    // Enable SVM operation
    unsafe {
        enable_svm_operation()?;
    }

    log::info!("SVM: Initialization complete");
    Ok(())
}
