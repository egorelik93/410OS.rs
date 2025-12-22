//! Functions for obtaining page entries.

use crate::virtual_memory::{LogicalAddress, Page, PageDirectory, PageEntry};

use super::vm_internal::PageTable;

impl PageDirectory {
    /// Get a page table entry.
    #[inline(always)]
    pub(super) fn getPageTableEntry(&self, addr: LogicalAddress) -> &PageEntry {
        let directoryIndex = addr.get_page_table();
        &self.0[directoryIndex as usize]
    }

    /// Get a page table entry.
    #[inline(always)]
    pub(super) fn getPageTableEntryMut(&mut self, addr: LogicalAddress) -> &mut PageEntry {
        let directoryIndex = addr.get_page_table();
        &mut self.0[directoryIndex as usize]
    }
}


impl PageTable {
    /// Get a page entry.
    #[inline(always)]
    pub(super) fn getPageEntry(&self, addr: LogicalAddress) -> &PageEntry {
        let tableIndex = addr.get_page();
        &self.0[tableIndex as usize]
    }

    /// Get a page entry.
    #[inline(always)]
    pub(super) fn getPageEntryMut(&mut self, addr: LogicalAddress) -> &mut PageEntry {
        let tableIndex = addr.get_page();
        &mut self.0[tableIndex as usize]
    }
}


impl PageDirectory {
    /// Get a page table.
    #[inline(always)]
    pub(super) unsafe fn tryGetPageTable(&self, addr: LogicalAddress) -> Option<&PageTable> {
        let entry = self.getPageTableEntry(addr);
        unsafe { super::assume_direct_mapping::<PageTable>(entry.page_address()).as_ref() }
    }

    /// Get a page table.
    #[inline(always)]
    pub(super) unsafe fn tryGetPageTableMut(&mut self, addr: LogicalAddress) -> Option<&mut PageTable> {
        let entry = self.getPageTableEntry(addr);
        unsafe { super::assume_direct_mapping::<PageTable>(entry.page_address()).as_mut() }
    }

    /// Get a page table entry.
    ///
    /// If the table exists, this will get the entry for
    /// an addr.
    /// If the table does not exist, returns NULL.
    #[inline(always)]
    pub unsafe fn tryGetPageEntry(&self, addr: LogicalAddress) -> Option<&PageEntry> {
        let table = self.tryGetPageTable(addr)?;
        Some(table.getPageEntry(addr))
    }

    /// Get a page table entry.
    ///
    /// If the table exists, this will get the entry for
    /// an addr.
    /// If the table does not exist, returns NULL.
    #[inline(always)]
    pub unsafe fn tryGetPageEntryMut(&mut self, addr: LogicalAddress) -> Option<&mut PageEntry> {
        let table = self.tryGetPageTableMut(addr)?;
        Some(table.getPageEntryMut(addr))
    }

    /// Get a page.
    #[inline(always)]
    pub(super) unsafe fn tryGetPage(&self, addr: LogicalAddress) -> Option<&Page> {
        let entry = self.tryGetPageEntry(addr)?;
        unsafe { super::assume_direct_mapping::<Page>(entry.page_address()).as_ref() }
    }

    /// Get a page.
    #[inline(always)]
    pub(super) unsafe fn tryGetPageMut(&mut self, addr: LogicalAddress) -> Option<&mut Page> {
        let entry = self.tryGetPageEntryMut(addr)?;
        unsafe { super::assume_direct_mapping::<Page>(entry.page_address()).as_mut() }
    }
}
