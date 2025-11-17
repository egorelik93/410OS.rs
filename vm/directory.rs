//! Operations on the page directory itself.

use core::alloc::Layout;
use core::ptr::NonNull;

use _410kern::cr::get_cr3;
use _410kern::page::PAGE_SIZE;
use alloc::alloc::alloc;
use alloc::boxed::Box;

use crate::virtual_memory::{Page, PageDirectory, assume_direct_mapping, from_direct_mapping, kernelDirectory};
use super::vm_internal::PageTable;

impl PageDirectory {
    /// Allocates a new page directory.
    #[inline(always)]
    pub fn new() -> Option<Box<PageDirectory>> {
        unsafe {
            let dir = alloc(Layout::from_size_align_unchecked(PAGE_SIZE, PAGE_SIZE)).cast::<PageDirectory>();

            dir.cast::<Page>().as_mut()?.zero();

            Some(Box::from_raw(dir))
        }
    }
}

impl Drop for PageDirectory {
    /// Free a page directory.
    ///
    /// This function frees the tables and directory,
    /// but not pages associated with them.
    ///
    /// This function is safe as long as we are in the kernelDirectory
    /// and not trying to drop it.
    fn drop(&mut self) {
        assert!(unsafe { get_cr3() == from_direct_mapping(kernelDirectory()) }
            && self as *const _ != kernelDirectory());

        for tableEntry in self.0 {
            if tableEntry.page_is_present() {
                drop(
                    unsafe {
                        Box::from_raw(assume_direct_mapping::<PageTable>(tableEntry.page_address()))
                    })
            }
        }
    }
}
