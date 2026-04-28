//! Functions for allocating and
//! freeing mapped memory.

use core::ptr;

use _410kern::cr::get_cr3;

use crate::lprintf;
use crate::virtual_memory::*;

use super::address_mapping::AddressMapping;
use super::vm_internal::invalidatePage;


/// Check whether it is safe to follow and mutate within the page directory.
///
/// Not in the original implementation.
pub(super) fn isSafe(dir: &PageDirectory) -> bool {
    (unsafe { (get_cr3() as usize) == from_direct_mapping(kernelDirectory().cast_mut()) })
        && (ptr::from_ref(dir) != kernelDirectory())
}

/* Allocation */

/// Allocates a page and maps it.
///
/// This function is safe as long we are in the kernelDirectory and not trying to modify it.
/// This did not exist in the original implementation.
#[inline(always)]
pub fn mapPageSafe<M: AddressMapping>(dir: &mut PageDirectory, addr: LogicalAddress, flags: u32) -> Option<*mut Page> {
    assert!(isSafe(dir));
    unsafe {
        mapPage::<M>(dir, addr, flags)
    }
}

/// Allocates a page and maps it.
#[inline(always)]
pub unsafe fn mapPage<M: AddressMapping>(dir: &mut PageDirectory, addr: LogicalAddress, flags: u32) -> Option<*mut Page> {
    let pageAddr = LogicalAddress(PAGE_ALIGN(addr.0));
    let frame = M::allocAddressMapping(pageAddr)?;

    if !isPageAligned(ptr::without_provenance_mut::<u8>(frame)) {
        lprintf!("Can't map page.\n");
        return None;
    }

    unsafe {
        let page = unsafe { assume_direct_mapping(frame) };
        dir.insertPage(page, addr, flags).ok()?;

        Some(page)
    }
}

/// Allocates and maps a range of pages.
///
/// Returns Page-aligned physical address to the mapped
/// region if successful,
/// and the last address successfully mapped
/// if something failed, which must not
/// be page aligned.
/// Note that a return corresponding to
/// -1 indicates no allocations succeeded.
///
/// This function is safe as long we are in the kernelDirectory and not trying to modify it.
/// This did not exist in the original implementation.
#[inline(always)]
pub fn mapMemoryRangeSafe<M: AddressMapping>(
    dir: &mut PageDirectory,
    start: LogicalAddress,
    end: LogicalAddress,
    flags: u32)
-> Result<PhysicalAddress, PhysicalAddress> {
    assert!(isSafe(dir));
    unsafe {
        mapMemoryRange::<M>(dir, start, end, flags)
    }
}

/// Allocates and maps a range of pages.
///
/// Returns Page-aligned physical address to the mapped
/// region if successful,
/// and the last address successfully mapped
/// if something failed, which must not
/// be page aligned.
/// Note that a return corresponding to
/// usize::MAX indicates no allocations succeeded.
pub unsafe fn mapMemoryRange<M: AddressMapping>(
    dir: &mut PageDirectory,
    start: LogicalAddress,
    end: LogicalAddress,
    flags: u32)
-> Result<PhysicalAddress, PhysicalAddress> {
    for addr in foreach_page_in(start, end) {
        match unsafe { mapPage::<M>(dir, addr, flags) } {
            None => return Err(addr.0 - 1),
            Some(page) => if !isPageAligned(page) {
                return Err(addr.0 - 1);
            }
        }
    }

    match unsafe { dir.tryGetPage(start) } {
        None => Err(usize::MAX),
        Some(startPage) => unsafe {
            Ok(from_direct_mapping(core::ptr::from_ref(startPage).cast_mut()) + start.get_page_offset() as usize)
        }
    }
}


/* Freeing */

/// Free the page corresponding to an address.
///
/// This function is safe as long we are in the kernelDirectory and not trying to modify it.
/// This did not exist in the original implementation.
#[inline(always)]
pub fn freeMappedPageSafe<M: AddressMapping>(dir: &mut PageDirectory, addr: LogicalAddress) {
    assert!(isSafe(dir));
    unsafe {
        freeMappedPage::<M>(dir, addr);
    }
}

/// Free the page corresponding to an address.
pub unsafe fn freeMappedPage<M: AddressMapping>(dir: &mut PageDirectory, addr: LogicalAddress) {
    let dir_ptr = ptr::from_mut(dir);

    let Some(entry) = (unsafe { dir.tryGetPageEntryMut(addr) })
        else { return };

    if entry.page_is_present() {
        if entry.page_is_copy_on_write() {
            M::unreserveAddressMapping(1);
        } else {
            let page = entry.page_address();
            M::freeAddressMapping(page);
        }

        if unsafe { get_cr3() as usize == from_direct_mapping(dir_ptr) } {
            invalidatePage(addr);
        }

        *entry = PageEntry::no_page();
    }
}

/// Free an entire range of pages.
///
/// This function is safe as long we are in the kernelDirectory and not trying to modify it.
/// This did not exist in the original implementation.
#[inline(always)]
pub fn freeMemoryRangeSafe<M: AddressMapping>(dir: &mut PageDirectory, start: LogicalAddress, end: LogicalAddress) {
    assert!(isSafe(dir));
    unsafe {
        freeMemoryRange::<M>(dir, start, end);
    }
}

/// Free an entire range of pages.
pub unsafe fn freeMemoryRange<M: AddressMapping>(dir: &mut PageDirectory, start: LogicalAddress, end: LogicalAddress) {
    let mut iter = foreach_entry_in(start, end);
    while let Some(addr) = iter.next(dir) {
        unsafe { freeMappedPage::<M>(dir, addr); }
    }
}
