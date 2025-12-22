//! Functions inserting or requesting a memory mapping.

use core::alloc::Layout;
use core::ptr;
use _410kern::cr::get_cr3;
use _410kern::page::PAGE_SIZE;

use alloc::alloc::alloc;
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
        if !GET_BIT(self.0, PAGE_WRITABLE_BIT)
            && GET_BIT(flags, PAGE_WRITABLE_BIT) {
                *self = *self | PAGE_WRITABLE
            }
    }
}

impl PageDirectory {
    /// Whether the current directory the currently set one.
    ///
    /// Not in the original implementation as a separate function,
    /// due to being more verbose in Rust.
    fn is_cr3(&self) -> bool {
        unsafe { get_cr3() as usize == from_direct_mapping(ptr::from_ref(self).cast_mut()) }
    }

    /* Insertion */

    /// Inserts a previously set up
    /// page table into a directory.
    #[inline(always)]
    pub(super) unsafe fn insertPageTable(&mut self, table: *mut PageTable, index: usize, flags: u32) {
        let mut entry = &mut self.0[index];

        unsafe {
            *entry = PageEntry::new(from_direct_mapping(table), flags | PAGE_PRESENT);
        }
    }

    /// Inserts a previously allocated page
    /// into a directory.
    #[inline(always)]
    pub unsafe fn insertPage(&mut self, page: *mut Page, addr: LogicalAddress, flags: u32) -> Result<(), ()> {
        let is_cr3 = self.is_cr3();

        unsafe {
            if self.tryGetPageEntryMut(addr).is_some() && is_cr3 {
                invalidatePage(addr);
            }

            let entry = self.getPageTable(addr, flags).ok_or(())?.getPageEntryMut(addr);

            *entry = PageEntry::new(from_direct_mapping(page), flags | PAGE_PRESENT);
            Ok(())
        }
    }


    /* Request a mapping */

    /// Get a page table for the address.
    ///
    /// Creates a new page table if one does not yet exist
    pub(super) unsafe fn getPageTable(&mut self, addr: LogicalAddress, flags: u32) -> Option<&mut PageTable> {
        let is_cr3 = self.is_cr3();

        let mut entry = self.getPageTableEntryMut(addr);

        if !entry.page_is_present() {
            unsafe {
                let table = alloc(Layout::from_size_align_unchecked(PAGE_SIZE, PAGE_SIZE)).cast::<PageTable>();

                table.cast::<Page>().as_mut()?.zero();

                self.insertPageTable(table, addr.get_page_table() as usize, flags);
                Some(&mut *table)
            }
        } else {
            entry.upgradeFlags(flags);
            if is_cr3 {
                invalidatePage(addr);
            }

            unsafe {
                assume_direct_mapping::<PageTable>(entry.page_address()).as_mut()
            }
        }
    }

    /// Get a page for the address.
    ///
    /// Creates a new page if one does not yet exist.
    pub(super) unsafe fn getPage<M: AddressMapping>(&mut self, addr: LogicalAddress, flags: u32) -> Option<*mut Page> {
        let is_cr3 = self.is_cr3();

        let table = unsafe { self.getPageTable(addr, flags)? };
        let entry = table.getPageEntryMut(addr);

        if !entry.page_is_present() && !entry.page_is_free() {
            let mut page = mapPage::<M>(self, addr, flags)?;

            if !isPageAligned(page) {
                return None;
            }

            if is_cr3 {
                unsafe { &mut *page }.zero();
            }

            return Some(page)
        } else if !entry.page_is_present() && entry.page_is_free() {
            *entry = *entry | PAGE_PRESENT;
        }

        Some(unsafe { assume_direct_mapping(entry.page_address()) })
    }

    /// Get the address for an entire range of memory.
    pub unsafe fn getMemoryRange<M: AddressMapping>(&mut self, start: LogicalAddress, end: LogicalAddress, flags: u32) -> Option<PhysicalAddress> {
        for addr in foreach_page_in(start, end) {
            let page = unsafe { self.getPage::<M>(addr, flags)? };

            if !isPageAligned(page) {
                return None;
            }
        }

        let startPage = self.tryGetPage(start)?;
        Some(from_direct_mapping(ptr::from_ref(startPage).cast_mut()) + start.get_page_offset() as usize)
    }

    /// Gets the physical address corresponding to a logical address.
    ///
    /// Creates a mapping if one does not exist for the address.
    pub unsafe fn getPhysicalAddress<M: AddressMapping>(&mut self, addr: LogicalAddress, flags: u32) -> Option<PhysicalAddress> {
        let page = unsafe { self.getPage::<M>(addr, flags)? };

        if !isPageAligned(page) {
            return None;
        }

        let offset = addr.get_page_offset() as usize;
        Some(from_direct_mapping(page) + offset)
    }

    /// Set flags on page entries covering a range of addresses
    ///
    /// This preserves the present flag and copy-on-write flag.
    pub unsafe fn setRangeFlags(&mut self, start: LogicalAddress, end: LogicalAddress, flags: u32) {
        let mut iter = foreach_entry_in(start, end);
        while let Some(addr) = iter.next(self) {
            let Some(entry) = self.tryGetPageEntryMut(addr)
                else { continue; };

            if entry.page_is_present() {
                if entry.page_is_copy_on_write() {
                    *entry = PageEntry::new(
                        entry.page_address(),
                        ((PAGE_PRESENT | flags) & !PAGE_WRITABLE) | PAGE_COPY_ON_WRITE);
                } else {
                    *entry = PageEntry::new(entry.page_address(), PAGE_PRESENT | flags)
                }
            }
        }
    }
}
