//! Functions for marking hidden memory.
//!
//! "Hidden memory" is a term we use to describe
//! addresses in task directories whose
//! page entries are not marked present
//! but who have their free bit set.
//! Our convention is that the
//! entry actually represents a real mapping,
//! but is hidden from the corresponding
//! task.
//! The purpose of this is that we can
//! pretend to clear a page directory,
//! and undo any new mappings
//! until we choose
//! to free hidden memory.
//!
//! We currently only use this during exec,
//! so that we can
//! recover if we cannot
//! map enough memory to load a new program.

use crate::virtual_memory::AddressMapping;

use super::memory_alloc::freeMappedPageSafe;
use super::{PAGE_FREE, PAGE_PRESENT, LogicalAddress, PageDirectory, foreach_entry_in};
use super::vm_internal::invalidatePage;

impl PageDirectory {
    /// Hide a page if it is currently mapped.
    pub fn hideAddress(&mut self, addr: LogicalAddress) {
        let is_cr3 = self.is_cr3();

        let Some(entry) = (unsafe { self.tryGetPageEntryMut(addr) }) else { return; };

        if entry.page_is_present() {
            if is_cr3 {
                invalidatePage(addr);
            }

            *entry = (*entry & !PAGE_PRESENT) | PAGE_FREE;
        }
    }

    /// Hide all present pages in a range.
    pub fn hideMemoryRange(&mut self, start: LogicalAddress, end: LogicalAddress) {
        let mut iter = foreach_entry_in(start, end);
        while let Some(addr) = iter.next(self) {
            self.hideAddress(addr);
        }
    }

    /// Commit hidden memory to be freed.
    ///
    /// Goes through all memory in a range,
    /// and if it is hidden, free it.
    /// If a present page is marked free,
    /// remove that marking.
    pub fn freeHiddenMemory<M: AddressMapping>(&mut self, start: LogicalAddress, end: LogicalAddress) {
        let mut iter = foreach_entry_in(start, end);
        while let Some(addr) = iter.next(self) {
            let Some(entry) = (unsafe { self.tryGetPageEntryMut(addr) }) else { continue; };

            if !entry.page_is_present() && entry.page_is_free() {
                freeMappedPageSafe::<M>(self, addr);
            } else if entry.page_is_present() && entry.page_is_free() {
                *entry = *entry & !PAGE_FREE;
            }
        }
    }

    /// Undoes mappings since hiding memory.
    ///
    /// Goes through all memory in a range,
    /// and if it is present without
    /// being marked free, frees the memory.
    /// Present pages that are marked free
    /// turn that bit off.
    pub fn restoreHiddenMemory<M: AddressMapping>(&mut self, start: LogicalAddress, end: LogicalAddress) {
        let mut iter = foreach_entry_in(start, end);
        while let Some(addr) = iter.next(self) {
            let Some(entry) = (unsafe { self.tryGetPageEntryMut(addr) }) else { continue; };

            if entry.page_is_present() && !entry.page_is_free() {
                freeMappedPageSafe::<M>(self, addr);
            } else if entry.page_is_present() && entry.page_is_free() {
                *entry = *entry & !PAGE_FREE;
            }
        }
    }
}
