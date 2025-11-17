//! Internal utilities for working with virtual memory.

use _410kern::page::PAGE_SIZE;
use elain::Align;

use crate::byte_utils::GET_BIT_RANGE;

use super::{LogicalAddress, PageEntry, NUM_PAGE_ENTRIES};


impl LogicalAddress {
    /// Return page table id of an address.
    #[inline(always)]
    pub const fn get_page_table(self) -> u16 {
        GET_BIT_RANGE(self.0, 22, 32)
    }


    /// Return page id of an address.
    #[inline(always)]
    pub const fn get_page(self) -> u16 {
        GET_BIT_RANGE(self.0, 12, 22);
    }


    /// Return page offset of an address.
    #[inline(always)]
    pub const fn get_page_offset(self) -> u16 {
        GET_BIT_RANGE(self.0, 0, 12)
    }
}

#[repr(C)]
#[derive(Debug, Default)]
pub struct PageTable(pub [PageEntry; NUM_PAGE_ENTRIES], Align<PAGE_SIZE>);

pub use super::memory_alloc::mapPage;

pub use super::invalidate_page::invalidatePage;
