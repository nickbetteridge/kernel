//! Nested Page Tables (NPT) for AMD SVM
//!
//! NPT provides second-level address translation (Guest Physical → Host Physical).
//! Unlike EPT, NPT uses the same page table format as standard x86_64 paging.
//!
//! ## Architecture
//! ```
//! Guest Virtual  --[Guest PT]--> Guest Physical --[NPT]--> Host Physical
//!      (GVA)                           (GPA)                   (HPA)
//! ```
//!
//! ## Integration with Redox
//! - Uses Redox's `PhysicalAddress` types
//! - Leverages Redox's frame allocator
//! - Extends Redox's `PageFlags` concept
//! - Follows Redox's `PageMapper` pattern

use crate::hypervisor::{HypervisorError, Result};
use crate::memory::{self, Frame};
use crate::paging::{PhysicalAddress, PageFlags, PAGE_SIZE};

/// NPT-specific flags extending Redox's PageFlags
#[derive(Debug, Clone, Copy)]
pub struct NptFlags {
    /// Base Redox page flags (for compatibility)
    base_flags: PageFlags,
    /// Present bit
    present: bool,
    /// Writable bit
    writable: bool,
    /// User/Supervisor bit (typically set for guest pages)
    user: bool,
    /// No-execute bit (inverted - if set, execution disabled)
    no_execute: bool,
}

impl NptFlags {
    /// Create new NPT flags
    pub fn new(present: bool, writable: bool, user: bool) -> Self {
        Self {
            base_flags: PageFlags::empty(),
            present,
            writable,
            user,
            no_execute: false,
        }
    }

    /// Create read-write flags (common for guest pages)
    pub fn read_write() -> Self {
        Self::new(true, true, true)
    }

    /// Create read-only flags
    pub fn read_only() -> Self {
        Self::new(true, false, true)
    }

    /// Create read-execute flags (W^X)
    pub fn read_execute() -> Self {
        Self::new(true, false, true).with_execute()
    }

    /// Enable execution
    pub fn with_execute(mut self) -> Self {
        self.no_execute = false;
        self
    }

    /// Disable execution
    pub fn no_execute(mut self) -> Self {
        self.no_execute = true;
        self
    }

    /// Convert to raw NPT PTE bits
    pub fn to_npt_entry(&self) -> u64 {
        let mut entry = 0u64;

        // Bit 0: Present
        if self.present {
            entry |= 1 << 0;
        }

        // Bit 1: Writable
        if self.writable {
            entry |= 1 << 1;
        }

        // Bit 2: User
        if self.user {
            entry |= 1 << 2;
        }

        // Bit 63: NX (No-Execute)
        if self.no_execute {
            entry |= 1 << 63;
        }

        entry
    }

    /// Parse NPT entry bits
    pub fn from_npt_entry(entry: u64) -> Self {
        Self {
            base_flags: PageFlags::empty(),
            present: (entry & (1 << 0)) != 0,
            writable: (entry & (1 << 1)) != 0,
            user: (entry & (1 << 2)) != 0,
            no_execute: (entry & (1 << 63)) != 0,
        }
    }
}

/// NPT Page Table Entry
#[derive(Clone, Copy)]
#[repr(transparent)]
struct NptEntry(u64);

impl NptEntry {
    /// Create empty entry
    fn new() -> Self {
        Self(0)
    }

    /// Check if entry is present
    fn is_present(&self) -> bool {
        (self.0 & (1 << 0)) != 0
    }

    /// Get physical address from entry
    fn address(&self) -> PhysicalAddress {
        // Bits 12-51 contain the physical address
        PhysicalAddress::new(self.0 & 0x000F_FFFF_FFFF_F000)
    }

    /// Set physical address and flags
    fn set_address(&mut self, addr: PhysicalAddress, flags: NptFlags) {
        // Clear old address and flags
        self.0 = 0;

        // Set new address (bits 12-51)
        self.0 |= addr.data() & 0x000F_FFFF_FFFF_F000;

        // Set flags (bits 0-11 and bit 63)
        self.0 |= flags.to_npt_entry();
    }

    /// Check if this is a huge page (2MB or 1GB)
    fn is_huge_page(&self) -> bool {
        // Bit 7 indicates a huge page
        (self.0 & (1 << 7)) != 0
    }
}

/// NPT Page Table (512 entries, 4KB)
#[repr(C, align(4096))]
struct NptPageTable {
    entries: [NptEntry; 512],
}

impl NptPageTable {
    /// Create a new empty page table
    fn new() -> Self {
        Self {
            entries: [NptEntry::new(); 512],
        }
    }
}

/// NPT Mapper - manages NPT page tables following Redox's PageMapper pattern
pub struct NptMapper {
    /// Root page table (PML4) physical address
    pml4_addr: PhysicalAddress,
}

impl NptMapper {
    /// Create new NPT mapper with allocated root table
    pub fn new() -> Result<Self> {
        // Allocate PML4 (root) table
        let pml4_frame = memory::allocate_frame()
            .ok_or(HypervisorError::OutOfMemory)?;
        let pml4_addr = pml4_frame.base();

        // Zero the PML4 table
        let pml4_virt = crate::memory::phys_to_virt(pml4_addr.data());
        unsafe {
            core::ptr::write_bytes(pml4_virt as *mut u8, 0, PAGE_SIZE);
        }

        log::debug!("NPT: Created new NPT structure at {:#x}", pml4_addr.data());

        Ok(Self { pml4_addr })
    }

    /// Get the NPT CR3 value for VMCB (N_CR3 field)
    pub fn npt_cr3(&self) -> u64 {
        // NPT CR3 is simply the physical address of the PML4 table
        // (unlike EPT which has additional control bits)
        self.pml4_addr.data() & 0x000F_FFFF_FFFF_F000
    }

    /// Map a guest physical address to a host physical address
    pub fn map(&mut self, gpa: PhysicalAddress, hpa: PhysicalAddress, flags: NptFlags) -> Result<()> {
        // NPT uses 4-level paging like x86_64
        // Calculate indices for each level
        let gpa_val = gpa.data();
        let pml4_index = (gpa_val >> 39) & 0x1FF;
        let pdpt_index = (gpa_val >> 30) & 0x1FF;
        let pd_index = (gpa_val >> 21) & 0x1FF;
        let pt_index = (gpa_val >> 12) & 0x1FF;

        log::trace!(
            "NPT: Mapping GPA {:#x} -> HPA {:#x} (indices: PML4={} PDPT={} PD={} PT={})",
            gpa_val, hpa.data(), pml4_index, pdpt_index, pd_index, pt_index
        );

        // Walk page tables, allocating as needed
        let pml4 = unsafe { &mut *(crate::memory::phys_to_virt(self.pml4_addr.data()) as *mut NptPageTable) };

        // PML4 -> PDPT
        let pdpt_addr = self.get_or_create_table(&mut pml4.entries[pml4_index])?;
        let pdpt = unsafe { &mut *(crate::memory::phys_to_virt(pdpt_addr.data()) as *mut NptPageTable) };

        // PDPT -> PD
        let pd_addr = self.get_or_create_table(&mut pdpt.entries[pdpt_index])?;
        let pd = unsafe { &mut *(crate::memory::phys_to_virt(pd_addr.data()) as *mut NptPageTable) };

        // PD -> PT
        let pt_addr = self.get_or_create_table(&mut pd.entries[pd_index])?;
        let pt = unsafe { &mut *(crate::memory::phys_to_virt(pt_addr.data()) as *mut NptPageTable) };

        // Set final mapping in PT
        pt.entries[pt_index].set_address(hpa, flags);

        Ok(())
    }

    /// Helper: Get existing table or create new one
    fn get_or_create_table(&mut self, entry: &mut NptEntry) -> Result<PhysicalAddress> {
        if entry.is_present() {
            Ok(entry.address())
        } else {
            // Allocate new table
            let frame = memory::allocate_frame()
                .ok_or(HypervisorError::OutOfMemory)?;
            let addr = frame.base();

            // Zero the table
            let virt = crate::memory::phys_to_virt(addr.data());
            unsafe {
                core::ptr::write_bytes(virt as *mut u8, 0, PAGE_SIZE);
            }

            // Set entry to point to new table with present, writable, user flags
            // (intermediate tables need full permissions for page walk to continue)
            let table_flags = NptFlags::read_write();
            entry.set_address(addr, table_flags);

            Ok(addr)
        }
    }

    /// Unmap a guest physical address
    pub fn unmap(&mut self, gpa: PhysicalAddress) -> Result<()> {
        let gpa_val = gpa.data();
        let pml4_index = (gpa_val >> 39) & 0x1FF;
        let pdpt_index = (gpa_val >> 30) & 0x1FF;
        let pd_index = (gpa_val >> 21) & 0x1FF;
        let pt_index = (gpa_val >> 12) & 0x1FF;

        // Walk to the PT entry
        let pml4 = unsafe { &mut *(crate::memory::phys_to_virt(self.pml4_addr.data()) as *mut NptPageTable) };

        if !pml4.entries[pml4_index].is_present() {
            return Ok(()); // Already unmapped
        }

        let pdpt_addr = pml4.entries[pml4_index].address();
        let pdpt = unsafe { &mut *(crate::memory::phys_to_virt(pdpt_addr.data()) as *mut NptPageTable) };

        if !pdpt.entries[pdpt_index].is_present() {
            return Ok(());
        }

        let pd_addr = pdpt.entries[pdpt_index].address();
        let pd = unsafe { &mut *(crate::memory::phys_to_virt(pd_addr.data()) as *mut NptPageTable) };

        if !pd.entries[pd_index].is_present() {
            return Ok(());
        }

        let pt_addr = pd.entries[pd_index].address();
        let pt = unsafe { &mut *(crate::memory::phys_to_virt(pt_addr.data()) as *mut NptPageTable) };

        // Clear the entry
        pt.entries[pt_index].0 = 0;

        log::trace!("NPT: Unmapped GPA {:#x}", gpa_val);

        // TODO: TLB invalidation (INVLPGA instruction)
        // TODO: Deallocate empty page tables

        Ok(())
    }

    /// Map a range of guest physical addresses to host physical addresses
    pub fn map_range(
        &mut self,
        gpa_start: PhysicalAddress,
        hpa_start: PhysicalAddress,
        size: usize,
        flags: NptFlags,
    ) -> Result<()> {
        let page_count = (size + PAGE_SIZE - 1) / PAGE_SIZE;

        for i in 0..page_count {
            let gpa = PhysicalAddress::new(gpa_start.data() + i * PAGE_SIZE);
            let hpa = PhysicalAddress::new(hpa_start.data() + i * PAGE_SIZE);
            self.map(gpa, hpa, flags)?;
        }

        log::debug!(
            "NPT: Mapped range GPA {:#x}-{:#x} -> HPA {:#x}-{:#x} ({} pages)",
            gpa_start.data(),
            gpa_start.data() + size,
            hpa_start.data(),
            hpa_start.data() + size,
            page_count
        );

        Ok(())
    }
}

impl Drop for NptMapper {
    fn drop(&mut self) {
        // TODO: Walk and deallocate all page table frames
        // For now, just log
        log::debug!("NPT: Dropping NPT mapper at {:#x}", self.pml4_addr.data());
    }
}

/// NPT violations (page faults in guest)
#[derive(Debug, Clone, Copy)]
pub struct NptViolation {
    /// Guest physical address that caused the fault
    pub gpa: PhysicalAddress,
    /// Was the access a write?
    pub write: bool,
    /// Was the access an instruction fetch?
    pub fetch: bool,
    /// Was the access from user mode?
    pub user: bool,
    /// Was the page present?
    pub present: bool,
}

impl NptViolation {
    /// Parse NPT violation from VMCB EXITINFO fields
    pub fn from_exitinfo(gpa: u64, exitinfo1: u64) -> Self {
        Self {
            gpa: PhysicalAddress::new(gpa),
            present: (exitinfo1 & (1 << 0)) != 0,
            write: (exitinfo1 & (1 << 1)) != 0,
            user: (exitinfo1 & (1 << 2)) != 0,
            fetch: (exitinfo1 & (1 << 4)) != 0,
        }
    }
}
