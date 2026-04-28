//! Checks whether a given address is valid.

use core::ffi::{CStr, c_char};
use core::u8;

use _410kern::cr::get_cr3;
use _410kern::page::PAGE_SIZE;

use crate::byte_utils::GET_BIT;
use crate::virtual_memory::*;

/// Checks if an address if page-aligned.
#[inline(always)]
pub fn isPageAligned<T>(addr: *mut T) -> bool {
    addr.addr() % PAGE_SIZE == 0
}

/// Return a page entry in the current directory.
#[inline(always)]
unsafe fn getPageFlags(addr: LogicalAddress) -> Option<PageEntry> {
    unsafe {
        let dir = assume_direct_mapping::<PageDirectory>(get_cr3() as usize).as_ref()?;
        Some(*dir.tryGetPageEntry(addr)?)
    }
}

/// Checks whether a given range is unmapped.
#[inline(always)]
pub unsafe fn isUnmappedAddr(addr: LogicalAddress, len: usize) -> bool {
    foreach_page_in(addr, addr.offset(len)).all(|curr| {
        match unsafe { getPageFlags(curr) } {
            None => false,
            Some(entry) => !entry.page_accessed()
        }
    })
}

/// Checks if a sequence of addresses is user-readable.
#[inline(always)]
pub unsafe fn isUserReadableAddr(addr: LogicalAddress, len: usize) -> bool {
    foreach_page_in(addr, addr.offset(len)).all(|curr| {
        match unsafe { getPageFlags(curr) } {
            None => false,
            Some(entry) => entry.page_is_present() && GET_BIT(entry.0, PAGE_USER_ACCESS_BIT)
        }
    })
}

/// Checks if a sequence of addresses is user-writable.
#[inline(always)]
pub unsafe fn isUserWritableAddr(addr: LogicalAddress, len: usize) -> bool {
    foreach_page_in(addr, addr.offset(len)).all(|curr| {
        match unsafe { getPageFlags(curr) } {
            None => false,
            Some(entry) => entry.page_is_present()
                && GET_BIT(entry.0, PAGE_USER_ACCESS_BIT)
                && GET_BIT(entry.0, PAGE_WRITABLE_BIT)
        }
    })
}

/// Return the readable length of a string.
pub unsafe fn readableStringLen(str: *const c_char) -> Option<usize> {
    let mut len = 0;
    let mut c = str;

    unsafe {
        while isUserReadableAddr(LogicalAddress(c.expose_provenance()), 1) {
            if *c == b'\0' as i8 {
                return Some(len);
            }

            len += 1;
            c = c.offset(1);
        }

        None
    }
}
