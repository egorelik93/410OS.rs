//! Operations for setting large portions of memory.

use core::ptr::without_provenance_mut;

use _410kern::page::PAGE_SIZE;

use crate::virtual_memory::manager::zeroedPage;
use crate::virtual_memory::{AddressMapping, LogicalAddress, PAGE_ALIGN, PAGE_COPY_ON_WRITE, PAGE_WRITABLE, Page, PageDirectory, TABLE_SIZE, assume_direct_mapping, foreach_page_in, foreach_table_in, isPageAligned};

use super::memory_alloc;


/// Return larger of two values
#[inline(always)]
fn MAX<T: Ord>(x: T, y : T) -> T {
    if x > y { x } else { y }
}

/// Return smaller of two values
fn MIN<T: Ord>(x: T, y: T) -> T {
    if x < y { x } else { y }
}

/// Copies a range of memory from one directory to another.
///
/// Can only copy on table-aligned boundaries.
/// Must be called from within the kernel directory.
pub fn copyMemoryRange<M: AddressMapping>(
    to: &mut PageDirectory,
    from: &PageDirectory,
    start: LogicalAddress,
    end: LogicalAddress)
-> Result<(), LogicalAddress> {
    assert!(memory_alloc::isSafe(to));

    for tableAddr in foreach_table_in(start, end) {
        let tableEntry = from.getPageTableEntry(tableAddr);

        if !tableEntry.page_is_present() {
            continue;
        }

        let tableFlags = tableEntry.page_flags();

        let table = unsafe { to.getPageTable(tableAddr, tableFlags as u32).ok_or(tableAddr)? };

        for addr in foreach_page_in(tableAddr, tableAddr.offset(TABLE_SIZE)) {
            let Some(oldPageEntry) = (unsafe { from.tryGetPageEntry(addr) }) else { continue; };

            if !oldPageEntry.page_is_present() { continue; }

            let flags = oldPageEntry.page_flags();

            if oldPageEntry.page_is_copy_on_write() {
                M::reserveAddressMapping(1).map_err(|_| addr)?;

                unsafe { to.insertPage(zeroedPage() as *const Page as *mut Page, addr, flags as u32) };
            } else {
                let oldPage = unsafe { &*assume_direct_mapping::<Page>(oldPageEntry.page_address()) };
                let page = unsafe { &mut *to.getPage::<M>(addr, flags as u32).ok_or(addr)? };

                oldPage.copyPage(page);
            }
        }
    }

    Ok(())
}


/// Sets a range of memory in the given directory to zero.
///
/// This version differs from the original implementation in that it passes
/// in a directory and expects to run in the kernelDirectory, rather than
/// applying to the current directory.
///
/// When possible this function
/// uses zero-fill on demand.
pub fn zeroedMemoryRange<M: AddressMapping>(dir: &mut PageDirectory, start: LogicalAddress, end: LogicalAddress, flags: u32) -> Result<(), LogicalAddress> {
    assert!(memory_alloc::isSafe(dir));

    let mut zfodStart = start;
    let mut zfodEnd = end;

    /* Dangling part of page before ZFOD region */
    if zfodStart.0 < zfodEnd.0 && !zfodStart.is_page_aligned() {
        let zfodStart = zfodStart.page_align().offset(PAGE_SIZE);
        let endSection = MIN(zfodStart, zfodEnd);

        let regionStart = unsafe { dir.getPhysicalAddress::<M>(start, flags).ok_or(start)? };

        for i in 0 .. (endSection.0 - start.0) {
            unsafe {
                *assume_direct_mapping::<u8>(regionStart + i) = 0
            }
        }
    }

    /* Dangling part of page after ZFOD region */
    if zfodStart <= zfodEnd.page_align() && !zfodStart.is_page_aligned() {
        zfodEnd = zfodStart.page_align();

        let regionEnd = unsafe { dir.getPhysicalAddress::<M>(end, flags).ok_or(end)? };

        for i in 0 .. (end.0 - zfodEnd.0) {
            unsafe {
                *assume_direct_mapping::<u8>(regionEnd + i) = 0
            }
        }
    }

    /* ZFOD region - entirely zeroed pages */
    if zfodStart < zfodEnd && zfodStart.is_page_aligned() && zfodEnd.is_page_aligned() {
        let count = (zfodEnd.0 - zfodStart.0) / PAGE_SIZE;
        M::reserveAddressMapping(count).map_err(|_| zfodStart)?;

        for addr in foreach_page_in(zfodStart, zfodEnd) {
            unsafe { dir.insertPage(
                zeroedPage() as *const Page as *mut Page,
                addr,
                (flags | PAGE_COPY_ON_WRITE) & !PAGE_WRITABLE) }
            .map_err(|_| addr)?;
        }
    }

    Ok(())
}
