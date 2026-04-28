//! API for managing the memory of a task.

use core::ffi::{c_int, c_void};

use alloc::boxed::Box;

use crate::sync::rwlock::{ReadGuard, WriteGuard};
use crate::thread::getCurrentTask;
use crate::variable_queue::Link;
use crate::virtual_memory::{AddressMapping, AllocMapping, LogicalAddress, PAGE_USER_ACCESS, PageDirectory, freeMemoryRangeSafe, isPageAligned, isUnmappedAddr, zeroedMemoryRange};

use super::task_internal::PageAllocation;


/// Locks modifications to memory allocations.
///
/// Stops other threads from
/// removing user memory that we need access
/// to.
/// Does not affect allocation of new memory.
pub fn lockReadFromMemory<'a>() -> ReadGuard<'a, Box<PageDirectory>> {
    let task = getCurrentTask().unwrap();
    task.directory.lockRead()
}

/// Locks modifications to memory allocations.
///
/// This function was mentioned in the original implementaton
/// but seemingly not implemented or used.
pub fn lockWriteFromMemory<'a>() -> WriteGuard<'a, Box<PageDirectory>> {
    let task = getCurrentTask().unwrap();
    task.directory.lockWrite()
}


/* Syscalls */

/// Allocates len bytes at base.
///
/// The region must not yet be allocated.
/// This call will record the allocation.
pub fn new_pages(base: *mut c_void, len: c_int) -> c_int {
    let Ok(alloc) = Box::try_new(PageAllocation {
        base,
        len: len as usize,
        link: Link::new()
    }) else { return -1; };


    let task = getCurrentTask().unwrap();

    let mut allocations = task.allocations.lockWrite();
    let mut dir = task.directory.lockWrite();

    if !isPageAligned(base) ||
        !isPageAligned(unsafe { base.byte_add(len as usize) }) ||
        unsafe { !isUnmappedAddr(LogicalAddress(base.addr()), len as usize) } {
            return -1;
        }

    let result = zeroedMemoryRange::<AllocMapping>(
        &mut dir,
        LogicalAddress(base.addr()),
        LogicalAddress(base.addr() + len as usize),
        PAGE_USER_ACCESS);

    match result {
        Ok(()) => {
            unsafe { insert_tail!(&mut allocations, Box::into_raw(alloc), link) };
            0
        },
        Err(result) => {
            freeMemoryRangeSafe::<AllocMapping>(
                &mut dir,
                LogicalAddress(base.addr()),
                result);
            -1
        }
    }
}

/// Removes a memory region allocated at base.
///
/// The region must have been allocated by new_pages.
pub fn remove_pages(base: *mut c_void) -> c_int {
    let task = getCurrentTask().unwrap();

    let mut allocations = task.allocations.lockWrite();

    for alloc_ptr in unsafe { allocations.iter_ptr(|a| &a.link) } {
        if unsafe { &*alloc_ptr }.base == base {
            {
                let alloc = unsafe { &*alloc_ptr };

                let mut dir = task.directory.lockWrite();

                remove!(&mut allocations, alloc, link);

                freeMemoryRangeSafe::<AllocMapping>(
                    &mut dir,
                    LogicalAddress(base.addr()),
                    LogicalAddress(base.addr() + alloc.len));
            }

            unsafe { Box::from_raw(alloc_ptr.cast_mut()) };
            return 0;
        }
    }

    -1
}
