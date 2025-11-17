//! Functions inserting or requesting a memory mapping.

use core::ptr;
use _410kern::cr::get_cr3;
use _410kern::page::PAGE_SIZE;
use alloc::boxed::Box;

use crate::byte_utils::GET_BIT;
use crate::virtual_memory::*;

use super::address_mapping::AddressMapping;
use super::vm_internal::{PageTable, invalidatePage, mapPage};

/* Helper */

impl PageEntry {
    /// Modifies a page entry to at least
    /// have the permissions in flags.
    pub(super) fn upgradeFlags(&mut self, flags: u32) {
        if !GET_BIT(*self, PAGE_WRITABLE_BIT)
            && GET_BIT(flags, PAGE_WRITABLE_BIT) {
                *self = *self | PAGE_WRITABLE
            }
    }
}

impl PageDirectory {
    /* Insertion */

    /// Inserts a previously set up
    /// page table into a directory.
    #[inline(always)]
    pub(super) unsafe fn insertPageTable(&mut self, table: *mut PageTable, index: u32, flags: u32) {
        let mut entry = &mut self.0[index];

        unsafe {
            *entry = PageEntry::new(from_direct_mapping(table), flags | PAGE_PRESENT);
        }
    }

    /// Inserts a previously allocated page
    /// into a directory.
    #[inline(always)]
    pub unsafe fn insertPage(&mut self, page: *mut Page, addr: LogicalAddress, flags: u32) -> Result<(), ()> {
        unsafe {
            let mut entry = self.tryGetPageEntryMut(addr);
            match entry {
                None => {
                    let table = self.getPageTable(addr, flags)?;
                    entry = table.getPageEntry(addr);
                }
                Some(mut entry) => {
                    if get_cr3() == from_direct_mapping(self) {
                        invalidatePage(addr);
                    }

                    self.getPageTable(addr, flags);
                }
            }

            *entry = PageEntry::new(from_direct_mapping(page), flags | PAGE_PRESENT);
            Ok(())
        }
    }


    /* Request a mapping */

    /// Get a page table for the address.
    ///
    /// Creates a new page table if one does not yet exist
    pub(super) unsafe fn getPageTable(&mut self, addr: LogicalAddress, flags: u32) -> Option<&mut PageTable> {
        const { assert_eq!(core::mem::align_of::<PageTable>(), PAGE_SIZE) };

        let mut entry = self.getPageTableEntryMut(addr);

        if !entry.page_is_present() {
            let table = Box::try_new(PageTable::default())?;

            unsafe {
                self.insertPageTable(Box::into_raw(table), addr.get_page_table(), flags);
            }
        } else {
            entry.upgradeFlags(flags);
            if unsafe { get_cr3() == from_direct_mapping(self) } {
                invalidatePage(addr);
            }
        }

        unsafe {
            Some(assume_direct_mapping(entry.page_address()).as_mut())
        }
    }

    /// Get a page for the address.
    ///
    /// Creates a new page if one does not yet exist.
    pub(super) unsafe fn getPage<M: AddressMapping>(&mut self, addr: LogicalAddress, flags: u32) -> Option<*mut Page> {
        let table = unsafe { self.getPageTable(addr, flags)? };
        let entry = table.getPageEntry(addr);

        if !entry.page_is_present() && !entry.page_is_free() {
            let mut page = mapPage::<M>(self, addr, flags)?;

            if !isPageAligned(page) {
                return None;
            }

            if unsafe { get_cr3() == from_direct_mapping(self) } {
                unsafe { &mut *page }.zero();
            }
        } else if !entry.page_is_present() && entry.page_is_free() {
            *entry = *entry | PAGE_PRESENT;
        }

        unsafe { assume_direct_mapping(entry.page_address()) }
    }

    /// Get the address for an entire range of memory.
    pub unsafe fn getMemoryRange<M: AddressMapping>(&mut self, start: LogicalAddress, end: LogicalAddress, flags: u32) -> Option<PhysicalAddress> {
        for addr in foreach_page_in(start, end) {
            let page = unsafe { self.getPage(addr, flags)? };

            if !isPageAligned(page) {
                return None;
            }
        }

        let startPage = self.tryGetPage(start)?;
        from_direct_mapping(startPage) + start.get_page_offset()
    }

    /// Gets the physical address corresponding to a logical address.
    ///
    /// Creates a mapping if one does not exist for the address.
    pub unsafe fn getPhysicalAddress<M: AddressMapping>(&mut self, addr: LogicalAddress, flags: u32) -> Option<PhysicalAddress> {
        let page = unsafe { self.getPage::<M>(addr, flags)? };

        if !isPageAligned(page) {
            return None;
        }

        let offset = addr.get_page_offset();
        from_direct_mapping(page) + offset
    }

    /// Set flags on page entries covering a range of addresses
    ///
    /// This preserves the present flag and copy-on-write flag.
    pub unsafe fn setRangeFlags(&mut self, start: LogicalAddress, end: LogicalAddress, flags: u32) {
        for (dir, addr) in foreach_entry_in(self, start, end) {
            let Some(entry) = dir.tryGetPageEntry(addr)
                else { continue; };

            if entry.page_is_present() {
                if entry.page_is_copy_on_write() {
                    *entry = PageEntry(
                        ((entry.page_address() | PAGE_PRESENT | flags)
                            & !PAGE_WRITABLE) | PAGE_COPY_ON_WRITE);
                } else {
                    *entry = PageEntry(entry.page_address() | PAGE_PRESENT | flags)
                }
            }
        }
    }
}
