//! Functions for mapping virtual memory.

use core::alloc::Layout;
use core::cell::OnceCell;
use core::mem::MaybeUninit;
use core::ptr::null_mut;

use _410kern::cr::get_cr3;
use _410kern::page::PAGE_SIZE;
use alloc::alloc::alloc;
use alloc::boxed::Box;

use crate::sync::mutex::Mutex;
use crate::virtual_memory::*;

use super::common_kern::machine_phys_frames;
use super::vm_internal::{PageTable, mapPage};
use super::frame_alloc::allocFrame;

static mut _kernelDirectory: *const PageDirectory = null_mut();
static mut _zeroedPage: *const Page = null_mut();

/// Return the kernel page directory
#[inline(always)]
pub fn kernelDirectory() -> *const PageDirectory {
    unsafe { _kernelDirectory }
}

/// Return whether we are currently in the kernel directory.
///
/// This was not a separate function in the original implementation,
/// due to being more verbose in Rust.
pub fn inKernelDirectory() -> bool {
    unsafe { get_cr3() as usize == from_direct_mapping(kernelDirectory().cast_mut()) }
}

/// Return a zeroed page
#[inline(always)]
pub fn zeroedPage() -> &'static Page {
    unsafe { &*_zeroedPage.get().unwrap() }
}

/// Return address of start of the next page to the input address
#[inline(always)]
pub fn nextAddress(dir: &PageDirectory, curr: LogicalAddress) -> LogicalAddress {
    if curr.0 == TABLE_ALIGN(curr.0) {
        let entry = dir.getPageTableEntry(curr);
        if !entry.page_is_present() {
            return curr.offset(TABLE_SIZE);
        }
    }
    return curr.offset(PAGE_SIZE);
}

/// Initialize the kernel's virtual memory system
pub unsafe fn initVirtualMemory() {
    let mut kernelDirectory: Box<PageDirectory> = PageDirectory::new().unwrap();

    let numFrames = machine_phys_frames() as usize;
    let numTables = numFrames / PAGE_SIZE;
    let memSize = numFrames * PAGE_SIZE;

    for i in 0..numTables {
        let mut table = unsafe { alloc(Layout::from_size_align_unchecked(PAGE_SIZE, PAGE_SIZE)).cast::<PageTable>() };

        unsafe {
            kernelDirectory.insertPageTable(table, i, PAGE_WRITABLE);
        }

        for j in 0..NUM_PAGE_ENTRIES {
            let addr = LogicalAddress::new(i as u16, j as u16, 0);
            let isGlobal = if addr.0 < super::common_kern::USER_MEM_START { PAGE_GLOBAL } else { 0 };

            if addr.0 < memSize {
                unsafe {
                    mapPage::<DirectMapping>(&mut kernelDirectory, addr, PAGE_WRITABLE | isGlobal).unwrap();
                }
            }
        }
    }

    unsafe {
        *(&mut *_kernelDirectory) = kernelDirectory;
    }

    todo!();

    unsafe {
        let zeroedPage = unsafe { assume_direct_mapping::<Page>(allocFrame().unwrap()) };
        (&mut *zeroedPage).zero();
        _zeroedPage = zeroedPage;
    }
}
