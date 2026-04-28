//! Functions for manipulating individual tasks.

use core::cell::Cell;
use core::ffi::{CStr, c_char, c_int};
use core::ptr::null;
use core::sync::atomic::AtomicBool;

use _410kern::cr::set_cr3;
use alloc::boxed::Box;

use crate::common_kern::USER_MEM_START;
use crate::malloc_wrappers::sfree;
use crate::registers::SuspendedState;
use crate::sync::cond::Cond;
use crate::sync::mutex::Mutex;
use crate::sync::rwlock::RWLock;
use crate::task::task_internal::PageAllocation;
use crate::thread::{ThreadCollection, exitKernelMode, getCurrentTask, isLastThreadInTask};
use crate::variable_queue::{Head, Link};
use crate::virtual_memory::{AllocMapping, DirectMapping, LogicalAddress, PAGE_GLOBAL, PAGE_WRITABLE, PageDirectory, copyMemoryRange, freeMemoryRangeSafe, isUserReadableAddr, kernelDirectory, mapMemoryRangeSafe, readableStringLen};

use super::{TaskBlock, loadProgram, switchToTask};

impl TaskBlock {
    /// Allocates and inits a new task.
    pub fn new() -> Option<Box<Self>> {
        let mut task = Box::try_new(TaskBlock {
            directory: RWLock::new(PageDirectory::new()?),
            originalTid: 0,
            exitStatus: Mutex::new(0),
            parent: Cell::new(null()),
            threads: ThreadCollection::new(),
            link: Link::new(),
            initState: Cell::new(SuspendedState::default()),
            exitFlag: AtomicBool::new(false),
            claimed: AtomicBool::new(false),
            exited: Cond::new(),
            allocations: RWLock::new(Head::new()),
        }).ok()?;

        mapMemoryRangeSafe::<DirectMapping>(
            task.directory.get_mut(),
            LogicalAddress(0),
            LogicalAddress(USER_MEM_START),
            PAGE_GLOBAL | PAGE_WRITABLE).ok()?;

        Some(task)
    }
}

impl Drop for TaskBlock {
    /// Free a task.
    fn drop(&mut self) {
        let allocations = self.allocations.get_mut();
        while let Some(alloc) = unsafe { allocations.front_unchecked() } {
            let alloc = remove!(allocations, alloc, link).unwrap();

            sfree(alloc.cast::<u8>().cast_mut(), size_of::<PageAllocation>());
        }

        freeMemoryRangeSafe::<AllocMapping>(&mut self.directory.lockWrite(), LogicalAddress(USER_MEM_START), LogicalAddress(0));
    }
}

impl TaskBlock {
    /// Copies the memory of one task to another.
    pub fn copyTask(&mut self, from: &TaskBlock) -> Result<(), ()> {
        if copyMemoryRange::<AllocMapping>(
            &mut self.directory.get_mut(),
            &from.directory.lockRead(),
            LogicalAddress(USER_MEM_START),
            LogicalAddress(0))
            .is_err() {
                freeMemoryRangeSafe::<AllocMapping>(&mut self.directory.get_mut(), LogicalAddress(USER_MEM_START), LogicalAddress(0));
                return Err(())
            }

        Ok(())
    }

    /// Get thread collection of a task.
    pub fn threadsOfTask(&self) -> &ThreadCollection {
        &self.threads
    }
}


/* Syscalls */

/// Switches the currently running task to run the given program.
///
/// Cannot exec if this task has multiple threads.
pub fn exec(execname: *const c_char, argvec: *mut *mut c_char) -> c_int {
    let task = getCurrentTask().unwrap();

    if !isLastThreadInTask() {
        return -1;
    }

    if unsafe { readableStringLen(execname).is_none() } {
        return -1;
    }

    if unsafe { !isUserReadableAddr(LogicalAddress(argvec.addr()), size_of::<*mut *mut c_char>()) } {
        return -1;
    }

    let mut i = 0;
    let mut arg;
    while unsafe { arg = *(argvec.add(i)); arg.is_null() } {
        if unsafe { readableStringLen(arg).is_none() } {
            return -1;
        }

        if unsafe { !isUserReadableAddr(
            LogicalAddress(argvec.add(i + 1).read().addr()),
            size_of::<*mut *mut c_char>()) } {
                return -1;
            }

        i += 1;
    }

    unsafe {
        set_cr3(kernelDirectory().addr() as u32);
    }

    let dir = task.directory.lockWrite();
    let result = loadProgram(&task, dir, LogicalAddress(execname.addr()), LogicalAddress(argvec.addr()));

    unsafe { switchToTask(task); }

    if result.is_err() {
        return -1;
    }

    unsafe {
        let initState = task.initState.get();
        exitKernelMode(initState)
    }

    /* Should not be here */
    -1
}

/// Sets the exit status.
pub fn set_status(status: c_int) {
    let task = getCurrentTask().unwrap();

    *task.exitStatus.lock() = status;
}
