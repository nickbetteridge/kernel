//! AMD SVM (Secure Virtual Machine) support

use crate::hypervisor::Result;
use core::arch::x86_64::__cpuid;

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

    log::info!("SVM: Available and enabled");

    // TODO: Enable SVM operation
    // 1. Set EFER.SVME[bit 12] = 1
    // 2. Set up VMCB structures for each VCPU
    // 3. Execute VMRUN to enter guest

    Ok(())
}
