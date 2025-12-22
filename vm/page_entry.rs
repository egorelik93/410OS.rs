//! Utilities for working with page entries.

use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign};

use crate::byte_utils::{FILTER_BIT_RANGE, GET_BIT};
use crate::virtual_memory::*;

impl PageEntry {
    /// Checks if this page has been accessed
    #[inline(always)]
    pub(super) fn page_accessed(self) -> bool {
        GET_BIT(self.0, 5)
    }

    /// Checks if a page has been written to
    #[inline(always)]
    pub(super) fn page_written(self) -> bool {
        GET_BIT(self.0, 6)
    }

    /// Return address of page
    #[inline(always)]
    pub(super) fn page_address(self) -> usize {
        FILTER_BIT_RANGE(self.0, 12, 32) as usize
    }

    /// Return flags
    #[inline(always)]
    pub(super) fn page_flags(self) -> u16 {
        FILTER_BIT_RANGE(self.0, 0, 12) as u16
    }

    /// Checks if a page is writable
    #[inline(always)]
    pub(super) fn page_is_writable(self) -> bool {
        GET_BIT(self.0, PAGE_WRITABLE_BIT)
    }

    /// Checks if a page is COW
    #[inline(always)]
    pub(super) fn page_is_copy_on_write(self) -> bool {
        GET_BIT(self.0, PAGE_COPY_ON_WRITE_BIT)
    }

    /// Checks if a page is free
    #[inline(always)]
    pub(super) fn page_is_free(self) -> bool {
        GET_BIT(self.0, PAGE_FREE_BIT)
    }

    /// Checks if a page is present
    #[inline(always)]
    pub(super) fn page_is_present(self) -> bool {
        GET_BIT(self.0, PAGE_PRESENT_BIT)
    }

    /// Returns a null page
    #[inline(always)]
    pub(super) const fn no_page() -> Self {
        PageEntry(0)
    }

    /// Returns a page entry for an address and flags
    #[inline(always)]
    pub(super) fn new(addr: PhysicalAddress, flags: u32) -> Self {
        PageEntry(PageEntry(addr as u32).page_address() as u32 | flags)
    }
}

impl BitOr<u32> for PageEntry {
    type Output = PageEntry;

    fn bitor(self, rhs: u32) -> Self::Output {
        PageEntry(self.0 | rhs)
    }
}

impl BitOrAssign<u32> for PageEntry {
    fn bitor_assign(&mut self, rhs: u32) {
        self.0 |= rhs
    }
}

impl BitAnd<u32> for PageEntry {
    type Output = PageEntry;

    fn bitand(self, rhs: u32) -> Self::Output {
        PageEntry(self.0 & rhs)
    }
}

impl BitAndAssign<u32> for PageEntry {
    fn bitand_assign(&mut self, rhs: u32) {
        self.0 &= rhs
    }
}
