//! Functions for mapping virtual memory.

use core::alloc::Layout;
use core::cell::OnceCell;
use core::mem::MaybeUninit;
use core::ptr::null_mut;

use _410kern::cr::{get_cr0, set_cr0, get_cr2, get_cr3, set_cr3, CR0_PG};
use _410kern::idt::IDT_PF;
use _410kern::page::PAGE_SIZE;
use _410kern::seg::SEGSEL_KERNEL_CS;
use alloc::alloc::alloc;
use alloc::boxed::Box;

use crate::idt_entry::*;
use crate::registers::ExceptionState;
use crate::swexn::generalExnHandler;
use crate::sync::mutex::Mutex;
use crate::task::{lockReadFromMemory, lockWriteFromMemory};
use crate::virtual_memory::vm::frame_alloc::initFrameAllocator;
use crate::virtual_memory::*;

use super::common_kern::{machine_phys_frames, USER_MEM_START};
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
    unsafe { &*_zeroedPage }
}


/// C handler for page fault exceptions
#[unsafe(no_mangle)]
pub unsafe extern "cdecl" fn pageFaultHandler(state: *const ExceptionState) {
    let faultAddress = unsafe { get_cr2() };
    let old_dir = unsafe { get_cr3() };

    unsafe {
        if !isUserReadableAddr(LogicalAddress(faultAddress as usize), 1) {
            generalExnHandler(state, IDT_PF as u32)
        }
    }

    let mut dir = lockWriteFromMemory();
    let mut entry = unsafe { dir.tryGetPageEntry(LogicalAddress(faultAddress as usize)) };

    if let Some(entry) = entry && entry.page_is_copy_on_write() {
        unsafe { set_cr3(kernelDirectory().addr() as u32) };

        let page = AllocMapping::fulfillAddressMapping(LogicalAddress(PAGE_ALIGN(faultAddress as usize)));
        if page.is_none() {
            AllocMapping::unreserveAddressMapping(1);
            unsafe { set_cr3(kernelDirectory().addr() as u32) };
            drop(dir);
            unsafe { generalExnHandler(state, IDT_PF as u32) }
        }
        let page = unsafe { assume_direct_mapping(page.unwrap()) };
        let result = unsafe { dir.insertPage(
            page,
            LogicalAddress(PAGE_ALIGN(faultAddress as usize)),
            PAGE_WRITABLE | PAGE_USER_ACCESS)
        };

        if result.is_err() {
            AllocMapping::freeAddressMapping(page.addr());
            unsafe { set_cr3(kernelDirectory().addr() as u32) };
            drop(dir);
            unsafe { generalExnHandler(state, IDT_PF as u32) }
        }
        unsafe { &mut *page }.zero();
        unsafe { set_cr3(old_dir) }
    } else {
        drop(dir);

        /* Does not return. */
        generalExnHandler(state, IDT_PF as u32)
    }
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
            let isGlobal = if addr.0 < USER_MEM_START { PAGE_GLOBAL } else { 0 };

            if addr.0 < memSize {
                unsafe {
                    mapPage::<DirectMapping>(&mut kernelDirectory, addr, PAGE_WRITABLE | isGlobal).unwrap();
                }
            }
        }
    }

    unsafe {
        _kernelDirectory = initFrameAllocator(kernelDirectory, USER_MEM_START, memSize);

        *IDT().add(IDT_PF) = trapGate(HARDWARE_PRIVILEGE,
                                      crate::pagefault_handler_wrapper::pageFaultHandlerWrapper,
                                      SEGSEL_KERNEL_CS);

        set_cr3(_kernelDirectory.addr() as u32);

        let cr0 = get_cr0();
        set_cr0(cr0 | CR0_PG);

        let zeroedPage = assume_direct_mapping::<Page>(allocFrame().unwrap());
        (&mut *zeroedPage).zero();
        _zeroedPage = zeroedPage;
    }
}
