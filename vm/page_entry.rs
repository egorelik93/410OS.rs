//! Utilities for working with page entries.

use crate::byte_utils::{FILTER_BIT_RANGE, GET_BIT};
use crate::virtual_memory::*;

impl PageEntry {
    /// Checks if this page has been accessed
    #[inline(always)]
    pub(super) const fn page_accessed(self) -> bool {
        GET_BIT(self.0, 5) != 0
    }

    /// Checks if a page has been written to
    #[inline(always)]
    pub(super) const fn page_written(self) -> bool {
        GET_BIT(self.0, 6) != 0
    }

    /// Return address of page
    #[inline(always)]
    pub(super) const fn page_address(self) -> usize {
        FILTER_BIT_RANGE(self.0, 12, 32)
    }

    /// Return flags
    #[inline(always)]
    pub(super) const fn page_flags(self) -> u16 {
        FILTER_BIT_RANGE(self.0, 0, 12)
    }

    /// Checks if a page is writable
    #[inline(always)]
    pub(super) const fn page_is_writable(self) -> bool {
        GET_BIT(self.0, PAGE_WRITABLE_BIT) != 0
    }

    /// Checks if a page is COW
    #[inline(always)]
    pub(super) const fn page_is_copy_on_write(self) -> bool {
        GET_BIT(self.0, PAGE_COPY_ON_WRITE_BIT) != 0
    }

    /// Checks if a page is free
    #[inline(always)]
    pub(super) const fn page_is_free(self) -> bool {
        GET_BIT(self.0, PAGE_FREE_BIT) != 0
    }

    /// Checks if a page is present
    #[inline(always)]
    pub(super) const fn page_is_present(self) -> bool {
        GET_BIT(self.0, PAGE_PRESENT_BIT) != 0
    }

    /// Returns a null page
    #[inline(always)]
    pub(super) const fn no_page() -> Self {
        PageEntry(0)
    }

    /// Returns a page entry for an address and flags
    #[inline(always)]
    pub(super) const fn new(addr: PhysicalAddress, flags: u16) -> Self {
        PageEntry(PageEntry(addr).page_address() | flags)
    }
}
