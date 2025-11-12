//! Extended Page Tables (EPT) for Intel VMX
//!
//! EPT provides second-level address translation (Guest Physical → Host Physical).
//! EPT operates in parallel with normal paging and has different permission/memory-type bits.
//!
//! ## Architecture
//! ```
//! Guest Virtual  --[Guest PT]--> Guest Physical --[EPT]--> Host Physical
//!      (GVA)                           (GPA)                    (HPA)
//! ```
//!
//! ## Integration with Redox
//! - Uses Redox's `PhysicalAddress` types
//! - Leverages Redox's frame allocator
//! - Extends Redox's `PageFlags` with EPT-specific bits
//! - Follows Redox's `PageMapper` pattern

use crate::hypervisor::{HypervisorError, Result};
use crate::memory::{self, Frame};
use crate::paging::{PhysicalAddress, PageFlags, PAGE_SIZE};

/// EPT memory types (from Intel SDM Table 28-6)
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EptMemoryType {
    /// Uncacheable
    Uncacheable = 0,
    /// Write Combining
    WriteCombining = 1,
    /// Write Through
    WriteThrough = 4,
    /// Write Protected
    WriteProtected = 5,
    /// Write Back (most common)
    WriteBack = 6,
}

/// EPT-specific flags extending Redox's PageFlags
#[derive(Debug, Clone, Copy)]
pub struct EptFlags {
    /// Base Redox page flags (for compatibility)
    base_flags: PageFlags,
    /// EPT Read permission
    read: bool,
    /// EPT Write permission
    write: bool,
    /// EPT Execute permission
    execute: bool,
    /// Memory type
    memory_type: EptMemoryType,
    /// Ignore PAT (Page Attribute Table)
    ignore_pat: bool,
}

impl EptFlags {
    /// Create new EPT flags with standard permissions
    pub fn new(read: bool, write: bool, execute: bool) -> Self {
        Self {
            base_flags: PageFlags::empty(),
            read,
            write,
            execute,
            memory_type: EptMemoryType::WriteBack,
            ignore_pat: false,
        }
    }

    /// Create read-write-execute flags (common for guest RAM)
    pub fn read_write_execute() -> Self {
        Self::new(true, true, true)
    }

    /// Create read-execute flags (common for guest code)
    pub fn read_execute() -> Self {
        Self::new(true, false, true)
    }

    /// Create read-write flags (common for guest data)
    pub fn read_write() -> Self {
        Self::new(true, true, false)
    }

    /// Set memory type
    pub fn with_memory_type(mut self, memory_type: EptMemoryType) -> Self {
        self.memory_type = memory_type;
        self
    }

    /// Convert to raw EPT PTE bits
    pub fn to_ept_entry(&self) -> u64 {
        let mut entry = 0u64;

        // Bits 0-2: Read, Write, Execute
        if self.read {
            entry |= 1 << 0;
        }
        if self.write {
            entry |= 1 << 1;
        }
        if self.execute {
            entry |= 1 << 2;
        }

        // Bits 3-5: Memory type
        entry |= (self.memory_type as u64) << 3;

        // Bit 6: Ignore PAT
        if self.ignore_pat {
            entry |= 1 << 6;
        }

        entry
    }

    /// Parse EPT entry bits
    pub fn from_ept_entry(entry: u64) -> Self {
        Self {
            base_flags: PageFlags::empty(),
            read: (entry & (1 << 0)) != 0,
            write: (entry & (1 << 1)) != 0,
            execute: (entry & (1 << 2)) != 0,
            memory_type: match (entry >> 3) & 0b111 {
                0 => EptMemoryType::Uncacheable,
                1 => EptMemoryType::WriteCombining,
                4 => EptMemoryType::WriteThrough,
                5 => EptMemoryType::WriteProtected,
                6 => EptMemoryType::WriteBack,
                _ => EptMemoryType::WriteBack, // Default to most common
            },
            ignore_pat: (entry & (1 << 6)) != 0,
        }
    }
}

/// EPT Page Table Entry
#[derive(Clone, Copy)]
#[repr(transparent)]
struct EptEntry(u64);

impl EptEntry {
    /// Create empty entry
    fn new() -> Self {
        Self(0)
    }

    /// Check if entry is present (has any R/W/X bits set)
    fn is_present(&self) -> bool {
        (self.0 & 0b111) != 0
    }

    /// Get physical address from entry
    fn address(&self) -> PhysicalAddress {
        // Bits 12-51 contain the physical address
        PhysicalAddress::new(self.0 & 0x000F_FFFF_FFFF_F000)
    }

    /// Set physical address
    fn set_address(&mut self, addr: PhysicalAddress, flags: EptFlags) {
        // Clear old address and flags, keep reserved bits
        self.0 &= !0x000F_FFFF_FFFF_FFFF;

        // Set new address (bits 12-51)
        self.0 |= addr.data() & 0x000F_FFFF_FFFF_F000;

        // Set flags (bits 0-11)
        self.0 |= flags.to_ept_entry();
    }

    /// Check if this is a huge page (2MB or 1GB)
    fn is_huge_page(&self) -> bool {
        // Bit 7 indicates a huge page in EPT
        (self.0 & (1 << 7)) != 0
    }
}

/// EPT Page Table (512 entries, 4KB)
#[repr(C, align(4096))]
struct EptPageTable {
    entries: [EptEntry; 512],
}

impl EptPageTable {
    /// Create a new empty page table
    fn new() -> Self {
        Self {
            entries: [EptEntry::new(); 512],
        }
    }
}

/// EPT Mapper - manages EPT page tables following Redox's PageMapper pattern
pub struct EptMapper {
    /// Root page table (PML4) physical address
    pml4_addr: PhysicalAddress,
}

impl EptMapper {
    /// Create new EPT mapper with allocated root table
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

        log::debug!("EPT: Created new EPT structure at {:#x}", pml4_addr.data());

        Ok(Self { pml4_addr })
    }

    /// Get the EPT pointer value for VMCS
    pub fn ept_pointer(&self) -> u64 {
        // EPT pointer format (Intel SDM 28.2.2):
        // Bits 0-2: EPT paging-structure memory type (6 = write-back)
        // Bit 3: Reserved (0)
        // Bits 4-5: EPT page-walk length minus 1 (3 = 4-level paging)
        // Bit 6: Enable accessed and dirty flags (0 for now)
        // Bits 7-11: Reserved (0)
        // Bits 12-51: Physical address of EPT PML4 table

        let memory_type = EptMemoryType::WriteBack as u64;
        let page_walk_length = 3u64; // 4-level paging (walk length 4, so 4-1=3)

        (self.pml4_addr.data() & 0x000F_FFFF_FFFF_F000)
            | (page_walk_length << 3)
            | memory_type
    }

    /// Map a guest physical address to a host physical address
    pub fn map(&mut self, gpa: PhysicalAddress, hpa: PhysicalAddress, flags: EptFlags) -> Result<()> {
        // EPT uses 4-level paging like x86_64
        // Calculate indices for each level
        let gpa_val = gpa.data();
        let pml4_index = (gpa_val >> 39) & 0x1FF;
        let pdpt_index = (gpa_val >> 30) & 0x1FF;
        let pd_index = (gpa_val >> 21) & 0x1FF;
        let pt_index = (gpa_val >> 12) & 0x1FF;

        log::trace!(
            "EPT: Mapping GPA {:#x} -> HPA {:#x} (indices: PML4={} PDPT={} PD={} PT={})",
            gpa_val, hpa.data(), pml4_index, pdpt_index, pd_index, pt_index
        );

        // Walk page tables, allocating as needed
        let pml4 = unsafe { &mut *(crate::memory::phys_to_virt(self.pml4_addr.data()) as *mut EptPageTable) };

        // PML4 -> PDPT
        let pdpt_addr = self.get_or_create_table(&mut pml4.entries[pml4_index])?;
        let pdpt = unsafe { &mut *(crate::memory::phys_to_virt(pdpt_addr.data()) as *mut EptPageTable) };

        // PDPT -> PD
        let pd_addr = self.get_or_create_table(&mut pdpt.entries[pdpt_index])?;
        let pd = unsafe { &mut *(crate::memory::phys_to_virt(pd_addr.data()) as *mut EptPageTable) };

        // PD -> PT
        let pt_addr = self.get_or_create_table(&mut pd.entries[pd_index])?;
        let pt = unsafe { &mut *(crate::memory::phys_to_virt(pt_addr.data()) as *mut EptPageTable) };

        // Set final mapping in PT
        pt.entries[pt_index].set_address(hpa, flags);

        Ok(())
    }

    /// Helper: Get existing table or create new one
    fn get_or_create_table(&mut self, entry: &mut EptEntry) -> Result<PhysicalAddress> {
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

            // Set entry to point to new table with RWX permissions
            // (intermediate tables need RWX for page walk to continue)
            let table_flags = EptFlags::read_write_execute();
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
        let pml4 = unsafe { &mut *(crate::memory::phys_to_virt(self.pml4_addr.data()) as *mut EptPageTable) };

        if !pml4.entries[pml4_index].is_present() {
            return Ok(()); // Already unmapped
        }

        let pdpt_addr = pml4.entries[pml4_index].address();
        let pdpt = unsafe { &mut *(crate::memory::phys_to_virt(pdpt_addr.data()) as *mut EptPageTable) };

        if !pdpt.entries[pdpt_index].is_present() {
            return Ok(());
        }

        let pd_addr = pdpt.entries[pdpt_index].address();
        let pd = unsafe { &mut *(crate::memory::phys_to_virt(pd_addr.data()) as *mut EptPageTable) };

        if !pd.entries[pd_index].is_present() {
            return Ok(());
        }

        let pt_addr = pd.entries[pd_index].address();
        let pt = unsafe { &mut *(crate::memory::phys_to_virt(pt_addr.data()) as *mut EptPageTable) };

        // Clear the entry
        pt.entries[pt_index].0 = 0;

        log::trace!("EPT: Unmapped GPA {:#x}", gpa_val);

        // TODO: TLB invalidation (INVEPT instruction)
        // TODO: Deallocate empty page tables

        Ok(())
    }
}

impl Drop for EptMapper {
    fn drop(&mut self) {
        // TODO: Walk and deallocate all page table frames
        // For now, just log
        log::debug!("EPT: Dropping EPT mapper at {:#x}", self.pml4_addr.data());
    }
}
