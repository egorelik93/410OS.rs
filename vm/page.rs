//! Functions for manipulating pages.

use core::ops::{Index, IndexMut};
use core::ptr::NonNull;

use _410kern::cr::get_cr3;
use _410kern::page::PAGE_SIZE;

use crate::virtual_memory::*;
use super::frame_alloc::{allocFrame, freeFrame};

impl Page {
    /// Set all contents of a page to 0.
    pub fn zero(&mut self) {
        for i in 0..PAGE_SIZE {
            self.0[i] = 0;
        }
    }

    /// Allocates a new page.
    ///
    /// Should only be run while in the kernel directory.
    #[inline(always)]
    pub fn new() -> Option<NonNull<Page>> {
        assert!(unsafe { get_cr3() == from_direct_mapping(kernelDirectory) });

        let page = NonNull::new(unsafe { assume_direct_mapping::<Page>(allocFrame()?) })?;
        unsafe { page.as_mut().zero() };
        Some(page)
    }

    /// Frees a page allocated by newPage.
    ///
    /// Should only be run while in the kernel directory.
    #[inline(always)]
    pub fn freePage(self: NonNull<Page>) {
        assert!(unsafe { get_cr3() == from_direct_mapping(kernelDirectory()) });

        freeFrame(unsafe { from_direct_mapping(self.as_ptr()) });
    }

    /// Copies all contents of one page to another.
    pub fn copyPage(&self, to: &mut Page) {
        for i in 0..PAGE_SIZE {
            to.0[i] = self.0[i];
        }
    }
}
