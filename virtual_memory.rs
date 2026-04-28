//! Header for virtual memory.

use elain::Align;

#[path = "vm/mod.rs"]
mod vm;
use vm::*;

use crate::common_kern;

use core::ptr::{self, without_provenance_mut};

use _410kern::page::PAGE_SIZE;

use crate::byte_utils::FILTER_BIT_RANGE;

/// Number of page entries in a table.
pub const NUM_PAGE_ENTRIES: usize = PAGE_SIZE / size_of::<PageEntry>();

/// Number of pages in kernel memory
pub const NUM_KERNEL_PAGES: usize = common_kern::USER_MEM_START / PAGE_SIZE;

/// Total size of all pages in a table
pub const TABLE_SIZE: usize = NUM_PAGE_ENTRIES * PAGE_SIZE;

/// Round down an address to a page boundary.
#[inline(always)]
pub fn PAGE_ALIGN(address: usize) -> usize {
    FILTER_BIT_RANGE(address, 12, 32)
}

/// Round down an address to a table boundary.
#[inline(always)]
pub fn TABLE_ALIGN(address: usize) -> usize {
    FILTER_BIT_RANGE(address, 22, 32)
}

/// Convention for error values of type Page *
///
/// Since NULL is sometimes a valid Page,
/// we instead adopt the convention for Page *
/// that non-page aligned addresses are errors.
/// In particular, this should represent an error
/// with address as meaningful information.
#[inline(always)]
pub const fn ERROR_AT(address: usize) -> usize {
    address - 1
}


pub const PAGE_PRESENT_BIT: usize = 0;
pub const PAGE_WRITABLE_BIT: usize = 0;
pub const PAGE_USER_ACCESS_BIT: usize = 0;
pub const PAGE_GLOBAL_BIT: usize = 8;
pub const PAGE_COPY_ON_WRITE_BIT: usize = 9;
pub const PAGE_FREE_BIT: usize = 10;

pub const PAGE_PRESENT: u32 = 1 << PAGE_PRESENT_BIT;
pub const PAGE_WRITABLE: u32 = 1 << PAGE_WRITABLE_BIT;
pub const PAGE_USER_ACCESS: u32 = 1 << PAGE_USER_ACCESS_BIT;
pub const PAGE_GLOBAL: u32 = 1 << PAGE_GLOBAL_BIT;
pub const PAGE_COPY_ON_WRITE: u32 = 1 << PAGE_COPY_ON_WRITE_BIT;
pub const PAGE_FREE: u32 = 1 << PAGE_FREE_BIT;


impl LogicalAddress {
    /// Construct a logical address.
    pub fn new(tableIndex: u16, pageIndex: u16, offset: u16) -> Self {
        LogicalAddress(
            (tableIndex as usize) << 22 | (pageIndex as usize) << 12 | offset as usize)
    }

    /// Takes the offset LogicalAddress
    ///
    /// The original port had an OFFSET macro, but
    /// that has been replaced by Rust's byte_offset method.
    pub fn offset(self, bytes: usize) -> LogicalAddress {
        LogicalAddress(self.0 + bytes)
    }

    /// Return a LogicalAddress aligned on a page boundary.
    ///
    /// Replaces some direct uses of PAGE_ALIGN in the original implementation
    #[inline(always)]
    pub fn page_align(self) -> Self {
        LogicalAddress(PAGE_ALIGN(self.0))
    }

    /// Checks if an address if page-aligned.
    ///
    /// Replaced direct version of isPageAligned in the original implementations
    pub fn is_page_aligned(self) -> bool {
        isPageAligned(without_provenance_mut::<u8>(self.0))
    }
}


pub const PHYS_NULL: PhysicalAddress = 0;
pub const LOGIC_NULL: LogicalAddress = LogicalAddress(0);
pub const NOT_A_FRAME: PhysicalAddress = usize::MAX;
pub const NOT_A_PAGE: *mut Page = NOT_A_FRAME as *mut Page;


#[derive(Debug)]
struct PageIter {
    next: usize,
    start: usize,
    end: usize
}

impl Iterator for PageIter {
    type Item = LogicalAddress;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start <= self.next && self.next < self.end {
            let current = LogicalAddress(self.next);
            self.next += PAGE_SIZE;
            Some(current)
        } else { None }
    }
}

/// Iterate over all pages in a range.
///
/// This serves as the head of a loop
/// that iterates over all page-aligned addresses
/// whose pages contain addresses in the range.
pub fn foreach_page_in(start: LogicalAddress, end: LogicalAddress) -> impl Iterator<Item = LogicalAddress> {
    let start = PAGE_ALIGN(start.0);
    PageIter {
        start: start,
        end: if end.0 == 0 { usize::MAX } else { end.0 },
        next: start
    }
}

#[derive(Debug)]
struct TableIter {
    next: usize,
    start: usize,
    end: usize
}

impl Iterator for TableIter {
    type Item = LogicalAddress;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start <= self.next && self.next < self.end {
            let current = LogicalAddress(self.next);
            self.next += TABLE_SIZE;
            Some(current)
        } else { None }
    }
}

/// Iterate over all tables in a range.
///
/// This serves as the head of a loop
/// that iterates over all table-aligned addresses
/// whose pages contain addresses in the range.
pub fn foreach_table_in(start: LogicalAddress, end: LogicalAddress) -> impl Iterator<Item = LogicalAddress> {
    let start = TABLE_ALIGN(start.0);
    TableIter {
        start: start,
        end: if end.0 == 0 { usize::MAX } else { end.0 },
        next: start
    }
}

#[derive(Debug)]
pub struct EntryIter {
    next: usize,
    start: usize,
    end: usize
}

impl EntryIter {
    fn next(&mut self, dir: &PageDirectory) -> Option<LogicalAddress> {
        if self.start <= self.next && self.next < self.end {
            let current = LogicalAddress(self.next);
            self.next = nextAddress(dir, current).0;
            Some(current)
        } else { None }
    }
}

/// Iterate over all pages with entries.
///
/// This serves as the head of a loop
/// that iterates over all page-aligned addresses
/// whose pages contain addresses in the range
/// and who have entries in some table in the directory.
///
/// Distinguished from the other loops in that
/// it skips over tables that do not have
/// entries in the directory.
///
/// Unfortunately, due to borrowing, we cannot pass in PageDirectory
/// here as in the original.
/// Instead, we need to pass in the PageDirectory on each call to next;
/// this prevents us from implementing the Rust Iterator trait.
pub fn foreach_entry_in(start: LogicalAddress, end: LogicalAddress) -> EntryIter {
    let start = PAGE_ALIGN(start.0);
    EntryIter {
        start: start,
        end: if end.0 == 0 { usize::MAX } else { end.0 },
        next: start
    }
}

pub type PhysicalAddress = usize;
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct LogicalAddress(pub usize);
#[repr(C)]
#[derive(Debug)]
pub struct Page([u8; PAGE_SIZE], Align<PAGE_SIZE>);

#[derive(Copy, Clone, Default, Debug)]
pub struct PageEntry(u32);

#[repr(C)]
#[derive(Debug)]
pub struct PageDirectory([PageEntry; NUM_PAGE_ENTRIES], Align<PAGE_SIZE>);


/// Mark where we are assuming that we have direct access to the given address.
///
/// This function did not exist in the original implementation,
/// reflecting cases that were only taken into account haphazardly.
/// When not properly accounted for, this function is extremely dangerous.
#[deprecated(note = "This function is dangerous and uses should be redesigned")]
pub unsafe fn assume_direct_mapping<T>(addr: PhysicalAddress) -> *mut T {
    ptr::with_exposed_provenance_mut(addr)
}

/// The reverse of assume_direct_mapping.
///
/// Not in the original implementation.
/// As with that function, marks a design flaw in the original implementation.
/// If I come back to this beyond just porting, should be the first issue addressed.
#[deprecated(note = "This function is dangerous and uses should be redesigned")]
pub unsafe fn from_direct_mapping<T>(ptr: *mut T) -> PhysicalAddress {
    ptr.expose_provenance()
}


pub use address_mapping::AddressMapping;


/* Allocation Strategies */

pub use alloc_mapping::AllocMapping;
pub use direct_mapping::DirectMapping;

/* Page API */

pub use manager::zeroedPage;


/* Page Directories */
pub use manager::{
    kernelDirectory,
    inKernelDirectory
};


/* Memory Allocation and Freeing */

pub use memory_alloc::{
    mapMemoryRangeSafe,
    freeMappedPageSafe,
    freeMemoryRangeSafe
};


/* Lookup Mappings */
pub use manager::nextAddress;


/* Mass Memory Writing */

pub use memory_write::{
    copyMemoryRange,
    zeroedMemoryRange
};

/* Memory Validation */

pub use validate_memory::{
    isPageAligned,
    isUserReadableAddr,
    isUserWritableAddr,
    isUnmappedAddr,
    readableStringLen
};


/* Installers */

pub use manager::initVirtualMemory;

pub use manager::pageFaultHandler;
