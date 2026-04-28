//! Program loading.
//!
//! Functions for the loading
//! of user programs from binary
//! files should be written in
//! this file. The function
//! elf_load_helper() is provided
//! for your use.

use core::borrow::BorrowMut;
use core::clone::CloneToUninit;
use core::ffi::{CStr, c_char, c_int, c_void};
use core::mem::MaybeUninit;
use core::ops::{Deref, DerefMut};
use core::ptr::{self, null, null_mut};

use _410kern::eflags::*;
use _410kern::elf::elf_410::*;
use _410kern::cr::set_cr3;
use _410kern::seg::{self, SEGSEL_KERNEL_DS};
use alloc::boxed::Box;
use num_traits::PrimInt;

use crate::common_kern::USER_MEM_START;
use crate::readfile::getbytes;
use crate::registers::{Registers, SuspendedState};
use crate::task::task_internal::{STACK_HIGH, STACK_LOW};
use crate::virtual_memory::{
    AllocMapping, LogicalAddress, PAGE_USER_ACCESS, PAGE_WRITABLE, Page, PageDirectory, assume_direct_mapping, from_direct_mapping, inKernelDirectory, kernelDirectory, zeroedMemoryRange};

use super::TaskBlock;

#[inline(always)]
fn ALIGN<T: PrimInt>(x: T, y: T) -> T {
    ((x) + (y - ((x) % y)) % y)
}

/// Stack of arguments to task
#[repr(C)]
struct MainCallStack {
    returnAddress: *mut c_void,
    argc: c_int,
    argvec: *mut *mut c_char,
    stackHigh: *mut c_void,
    stackLow: *mut c_void
}


/* --- Local function prototypes --- */

/// Sets up a task with the given program
///
/// Loads the given program into the task.
/// This may only be called while cr3
/// is set to the kernel directory.
///
/// The program name and its arguments
/// must not take more than PAGE_SIZE bytes
/// total when copied into memory.
///
/// TODO: This translation of the API is weird but necessary to maintain
/// the existing pattern. We probably will want to come back and change the pattern though.
pub fn loadProgram<T: BorrowMut<PageDirectory>, D: DerefMut<Target=T>>(
    task: &TaskBlock,
    mut dir: D,
    logicalExecName: LogicalAddress,
    logicalArgVec: LogicalAddress)
-> Result<u32, ()> {
    assert!(inKernelDirectory());

    /* Check for a valid program */

    /* Should be checked before calling loadProgram. */
    let physExecName = unsafe {
        if let Some(addr) = (*dir).borrow_mut().getPhysicalAddress::<AllocMapping>(logicalExecName, 0) {
            assume_direct_mapping::<c_char>(addr)
        } else {
            null()
        }
    };

    let physArgVec = unsafe {
        if let Some(addr) = (*dir).borrow_mut().getPhysicalAddress::<AllocMapping>(logicalArgVec, 0) {
            assume_direct_mapping::<*mut c_char>(addr)
        } else {
            null_mut()
        }
    };

    let mut se_hdr: MaybeUninit<SimpleElf> = MaybeUninit::zeroed();

    if unsafe { elf_check_header(physExecName) } < 0 {
        return Err(());
    }

    if unsafe { elf_load_helper(se_hdr.as_mut_ptr(), physExecName) < 0 } {
        return Err(());
    }

    let se_hdr: SimpleElf = unsafe { se_hdr.assume_init() };


    /* Loading arguments onto the stack */

    let stack = Page::new().ok_or(())?;

    let mut argc = 0;
    if !physArgVec.is_null() {
        let mut curr = unsafe { *physArgVec };
        while !curr.is_null() {
            argc += 1;
            curr = unsafe { *physArgVec.add(argc) };
        }
    }

    let mut esp = STACK_HIGH.wrapping_add(1);
    let mut argAddress = unsafe { from_direct_mapping(stack.byte_add(STACK_HIGH - STACK_LOW + 1).as_ptr()) };

    let execName = unsafe { CStr::from_ptr(physExecName) };
    let execNameLen = execName.count_bytes();
    let execNameAligned = ALIGN(execNameLen + 1, size_of::<*mut c_char>());

    esp = esp.wrapping_sub(execNameAligned);
    argAddress -= execNameAligned;
    let execNameDest = unsafe { assume_direct_mapping::<c_char>(argAddress) };
    let stackExecName = LogicalAddress(esp);

    unsafe { execName.clone_to_uninit(execNameDest.cast()) };

    esp = esp.wrapping_sub(argc * size_of::<*mut *mut c_char>());
    argAddress -= argc * size_of::<*mut *mut c_char>();

    let copiedArgVec = LogicalAddress(esp);
    let newArgVec = unsafe { assume_direct_mapping::<*mut c_char>(argAddress) };

    for i in 0..argc {
        let arg = unsafe {
            let physArg = LogicalAddress((*physArgVec.add(i)).addr());
            if let Some(addr) = (*dir).borrow_mut().getPhysicalAddress::<AllocMapping>(physArg, 0) {
                assume_direct_mapping::<c_char>(addr)
            } else {
                null_mut()
            }
        };

        if !arg.is_null() {
            let arg = unsafe { CStr::from_ptr(arg) };

            let len = arg.count_bytes();
            let aligned = ALIGN(len + 1, size_of::<*mut *mut c_char>());

            esp = esp.wrapping_sub(aligned);
            argAddress -= aligned;

            unsafe { arg.clone_to_uninit(stack.as_ptr().cast::<u8>().with_addr(argAddress)) };
            unsafe { *newArgVec.add(i) = ptr::without_provenance_mut(esp) }
        } else {
            unsafe { *newArgVec.add(i) = null_mut() }
        }
    }

    esp = esp.wrapping_sub(size_of::<MainCallStack>());
    argAddress -= size_of::<MainCallStack>();

    if esp < STACK_LOW {
        return Err(())
    }

    let mainArgs = unsafe { &mut *stack.as_ptr().with_addr(argAddress).cast::<MaybeUninit<MainCallStack>>() };
    mainArgs.write(MainCallStack {
        argc: argc as i32,
        argvec: ptr::without_provenance_mut(copiedArgVec.0),
        stackHigh: ptr::without_provenance_mut(STACK_HIGH),
        stackLow: ptr::without_provenance_mut(STACK_LOW),
        returnAddress: null_mut()
    });


    /* Obtain memory for the program regions. */

    drop(dir);
    let task_dir = &mut task.directory.lockWrite();

    task_dir.hideMemoryRange(LogicalAddress(USER_MEM_START), LogicalAddress(0));
    let allocFailed = |dir: &mut PageDirectory| {
        dir.restoreHiddenMemory::<AllocMapping>(LogicalAddress(USER_MEM_START), LogicalAddress(0));
    };

    let _text = unsafe {
        task_dir.getMemoryRange::<AllocMapping>(
            LogicalAddress(se_hdr.e_txtstart as usize),
            LogicalAddress((se_hdr.e_txtstart + se_hdr.e_txtlen) as usize),
            PAGE_WRITABLE | PAGE_USER_ACCESS).ok_or_else(|| allocFailed(task_dir))?
    };

    let _data = unsafe {
        task_dir.getMemoryRange::<AllocMapping>(
            LogicalAddress(se_hdr.e_datstart as usize),
            LogicalAddress((se_hdr.e_datstart + se_hdr.e_datlen) as usize),
            PAGE_WRITABLE | PAGE_USER_ACCESS).ok_or_else(|| allocFailed(task_dir))?
    };

    let _rodata = unsafe {
        task_dir.getMemoryRange::<AllocMapping>(
            LogicalAddress(se_hdr.e_rodatstart as usize),
            LogicalAddress((se_hdr.e_rodatstart + se_hdr.e_rodatlen) as usize),
            PAGE_WRITABLE | PAGE_USER_ACCESS).ok_or_else(|| allocFailed(task_dir))?
    };


    /* Place data into memory */

    unsafe {
        task_dir.insertPage(
            stack.as_ptr(),
            LogicalAddress(STACK_LOW),
            PAGE_WRITABLE | PAGE_USER_ACCESS).map_err(|_| allocFailed(task_dir))?;
    }


    zeroedMemoryRange::<AllocMapping>(
        task_dir,
        LogicalAddress(se_hdr.e_bssstart as usize),
        LogicalAddress((se_hdr.e_bssstart + se_hdr.e_bsslen) as usize),
        PAGE_USER_ACCESS | PAGE_WRITABLE).map_err(|_| allocFailed(task_dir));

    /* Need to switch into task directory
     * so that bytes are loaded into the proper
     * addresses in memory.
     * We need to switch back
     * to ensure that loadProgram
     * returns in the kernel directory.
     */

    unsafe {
        set_cr3((&raw const **task_dir).addr() as u32);

        let execName = CStr::from_ptr(ptr::with_exposed_provenance(stackExecName.0));

        getbytes(
            execName,
            se_hdr.e_txtoff as usize,
            &mut *ptr::slice_from_raw_parts_mut(
                ptr::with_exposed_provenance_mut(se_hdr.e_txtstart as usize),
                se_hdr.e_txtlen as usize));
        getbytes(
            execName,
            se_hdr.e_datoff as usize,
            &mut *ptr::slice_from_raw_parts_mut(
                ptr::with_exposed_provenance_mut(se_hdr.e_datstart as usize),
                se_hdr.e_datlen as usize));
        getbytes(
            execName,
            se_hdr.e_rodatoff as usize,
            &mut *ptr::slice_from_raw_parts_mut(
                ptr::with_exposed_provenance_mut(se_hdr.e_rodatstart as usize),
                se_hdr.e_rodatlen as usize));

        set_cr3(kernelDirectory().addr() as u32);
    }

    task_dir.freeHiddenMemory::<AllocMapping>(LogicalAddress(USER_MEM_START), LogicalAddress(0));

    /* Need to update flags to lower permissions.
     * We need all of them so that overlapping pages
     * get the highest permissions.
     */

    unsafe {
        task_dir.setRangeFlags(
            LogicalAddress(se_hdr.e_txtstart as usize),
            LogicalAddress((se_hdr.e_txtstart + se_hdr.e_txtlen) as usize),
            PAGE_USER_ACCESS);

        task_dir.setRangeFlags(
            LogicalAddress(se_hdr.e_rodatstart as usize),
            LogicalAddress((se_hdr.e_rodatstart + se_hdr.e_rodatlen) as usize),
            PAGE_USER_ACCESS);

        task_dir.setRangeFlags(
            LogicalAddress(se_hdr.e_datstart as usize),
            LogicalAddress((se_hdr.e_datstart + se_hdr.e_datlen) as usize),
            PAGE_USER_ACCESS | PAGE_WRITABLE);

        task_dir.setRangeFlags(
            LogicalAddress(se_hdr.e_bssstart as usize),
            LogicalAddress((se_hdr.e_bssstart + se_hdr.e_bsslen) as usize),
            PAGE_USER_ACCESS | PAGE_WRITABLE);

        task_dir.setRangeFlags(LogicalAddress(STACK_LOW), LogicalAddress(STACK_HIGH), PAGE_USER_ACCESS | PAGE_WRITABLE);
    }


    /* Set the initial state of new threads */

    task.initState.set(SuspendedState {
        gs: seg::SEGSEL_USER_DS as u32,
        fs: seg::SEGSEL_USER_DS as u32,
        es: seg::SEGSEL_USER_DS as u32,
        ds: seg::SEGSEL_USER_DS as u32,

        eip: se_hdr.e_entry as u32,
        cs: seg::SEGSEL_USER_DS as u32,

        eflags: ((unsafe { get_eflags() as u64 } | EFL_RESV1 | EFL_IF | EFL_IOPL_RING0) & !EFL_AC) as u32,

        esp: esp as u32,
        ss: SEGSEL_KERNEL_DS as u32,

        reg: Registers::default()
    });

    Ok(0)
}
