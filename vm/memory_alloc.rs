//! Functions for allocating and
//! freeing mapped memory.

use core::ptr;

use _410kern::cr::get_cr3;

use crate::lprintf;
use crate::virtual_memory::*;

use super::address_mapping::AddressMapping;
use super::vm_internal::invalidatePage;


/* Allocation */

/// Allocates a page and maps it.
///
/// This function is safe as long we are in the kernelDirectory and not trying to modify it.
/// This did not exist in the original implementation.
#[inline(always)]
pub fn mapPageSafe<M: AddressMapping>(dir: &mut PageDirectory, addr: LogicalAddress, flags: u32) -> Option<*mut Page> {
    assert!(unsafe { get_cr3() == from_direct_mapping(kernelDirectory()) } && dir != kernelDirectory());
    unsafe {
        mapPage::<M>(dir, addr, flags)
    }
}

/// Allocates a page and maps it.
#[inline(always)]
pub unsafe fn mapPage<M: AddressMapping>(dir: &mut PageDirectory, addr: LogicalAddress, flags: u32) -> Option<*mut Page> {
    let pageAddr = LogicalAddress(PAGE_ALIGN(addr.0));
    let frame = M::allocAddressMapping(pageAddr)?;

    if !isPageAligned(ptr::without_provenance(frame)) {
        lprintf!("Can't map page.\n");
        return None;
    }

    unsafe {
        let page = unsafe { assume_direct_mapping(frame) };
        dir.insertPage(page, addr, flags)?;

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
    assert!(unsafe { get_cr3() == from_direct_mapping(kernelDirectory()) } && dir != kernelDirectory());
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
/// -1 indicates no allocations succeeded.
pub unsafe fn mapMemoryRange<M: AddressMapping>(
    dir: &mut PageDirectory,
    start: LogicalAddress,
    end: LogicalAddress,
    flags: u32)
-> Result<PhysicalAddress, PhysicalAddress> {
    for addr in foreach_page_in(start, end) {
        match unsafe { mapPage::<M>(dir, addr, flags) } {
            None => Err(addr - 1),
            Some(page) => if !isPageAligned(page) {
                return Err(addr - 1);
            }
        }
    }

    match unsafe { dir.tryGetPage(start) } {
        None => Err(-1),
        Some(startPage) => unsafe {
            Ok(from_direct_mapping(startPage) + start.get_page_offset())
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
    assert!(unsafe { get_cr3() == from_direct_mapping(kernelDirectory()) } && dir != kernelDirectory());
    unsafe {
        freeMappedPage::<M>(dir, addr);
    }
}

/// Free the page corresponding to an address.
pub unsafe fn freeMappedPage<M: AddressMapping>(dir: &mut PageDirectory, addr: LogicalAddress) {
    let entry = unsafe { dir.tryGetPageEntryMut(addr)? };

    if entry.page_is_present() {
        if entry.page_is_copy_on_write() {
            M::unreserveAddressMapping(1);
        } else {
            let page = entry.page_address();
            M::freeAddressMapping(page);
        }

        if unsafe { get_cr3() == from_direct_mapping(dir) } {
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
pub fn freeMemoryRangeSafe<M: AddressMapping>(dir: &PageDirectory, start: LogicalAddress, end: LogicalAddress) {
    assert!(unsafe { get_cr3() == from_direct_mapping(kernelDirectory()) } && dir != kernelDirectory());
    unsafe {
        freeMemoryRange::<M>(dir, start, end);
    }
}

/// Free an entire range of pages.
pub unsafe fn freeMemoryRange<M: AddressMapping>(dir: &PageDirectory, start: LogicalAddress, end: LogicalAddress) {
    for (dir, addr) in foreach_entry_in(dir, start, end) {
        unsafe { freeMappedPage::<M>(dir, addr); }
    }
}
