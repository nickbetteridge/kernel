//! Intel VMX (Virtual Machine Extensions) support

use crate::hypervisor::Result;
use core::arch::x86_64::__cpuid;

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

    log::info!("VMX: Available and enabled");

    // TODO: Enable VMX operation
    // 1. Set CR4.VMXE[bit 13] = 1
    // 2. Execute VMXON with VMXON region
    // 3. Set up VMCS regions for each VCPU

    Ok(())
}
