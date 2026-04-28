//! Functions for manipulating the global task collection.

use core::cell::{LazyCell, OnceCell};
use core::ffi::{CStr, c_int};
use core::mem::{ManuallyDrop, MaybeUninit};
use core::ptr::null;
use core::sync::atomic::Ordering;

use _410kern::seg::SEGSEL_KERNEL_CS;
use alloc::boxed::Box;

use _410kern::cr::{set_cr3};
use crate::idt_entry::*;
use crate::sync::disable_interrupts::{self, disableInterrupts};
use crate::sync::mutex::Mutex;
use crate::thread::{Thread, blockUntil, blockUntilRescheduled, clearFreeThreadCollection, forkThreadToTask, getCurrentTask, startThread};
use crate::variable_queue::Head;
use crate::syscall_int::*;
use crate::virtual_memory::{LogicalAddress, isUserWritableAddr, kernelDirectory};

use super::{TaskBlock, loadProgram, lockReadFromMemory};
use super::task_internal::TaskCollection;


const INIT_PROGRAM: &CStr = c"init";
const IDLE_PROGRAM: &CStr = c"idle";

static coll: TaskCollection = TaskCollection { queue: Mutex::new(Head::new()) };
static mut _init: *const TaskBlock = null();
static mut _idle: *const TaskBlock = null();


/// Install IDT gates for task-related syscalls
pub unsafe fn installTaskManager() {
    unsafe {
        *IDT().add(EXEC_INT) = trapGate(
            USER_PRIVILEGE,
            crate::syscall::execHandlerWrapper,
            SEGSEL_KERNEL_CS);

        *IDT().add(FORK_INT) = trapGate(
            USER_PRIVILEGE,
            crate::syscall::forkHandlerWrapper,
            SEGSEL_KERNEL_CS);

        *IDT().add(SET_STATUS_INT) = trapGate(
            USER_PRIVILEGE,
            crate::syscall::setStatusHandlerWrapper,
            SEGSEL_KERNEL_CS);

        *IDT().add(WAIT_INT) = trapGate(
            USER_PRIVILEGE,
            crate::syscall::waitHandlerWrapper,
            SEGSEL_KERNEL_CS);

        *IDT().add(NEW_PAGES_INT) = trapGate(
            USER_PRIVILEGE,
            crate::syscall::newPagesHandlerWrapper,
            SEGSEL_KERNEL_CS);

        *IDT().add(REMOVE_PAGES_INT) = trapGate(
            USER_PRIVILEGE,
            crate::syscall::removePagesHandlerWrapper,
            SEGSEL_KERNEL_CS);
    }

    let init = TaskBlock::new().unwrap();
    let argsInit = [ INIT_PROGRAM.as_ptr(), null() ];
    loadProgram(
        &init,
        unsafe { &mut *kernelDirectory().cast_mut() },
        LogicalAddress(INIT_PROGRAM.as_ptr().addr()),
        LogicalAddress((&raw const  argsInit).addr()));

    let idle = TaskBlock::new().unwrap();
    let argsIdle = [ IDLE_PROGRAM.as_ptr(), null() ];
    loadProgram(
        &idle,
        unsafe { &mut *kernelDirectory().cast_mut() },
        LogicalAddress(IDLE_PROGRAM.as_ptr().addr()),
        LogicalAddress((&raw const argsIdle).addr()));

    unsafe {
        _init = startTask(init).unwrap();
        _idle = startTask(idle).unwrap();
    }
}


/// Activates a loaded task.
pub fn startTask(mut task: Box<TaskBlock>) -> Result<*const TaskBlock, Box<TaskBlock>> {
    let Some(mut thread) = Thread::new() else { return Err(task); };
    thread.load((&raw const *task).cast_mut(), &task.initState.get());
    task.originalTid = thread.tid();

    let task_raw = Box::into_raw(task);
    unsafe { coll.insertTask(task_raw) };
    startThread(thread);

    Ok(task_raw)
}


/// Switch to the task's page directory.
pub unsafe fn switchToTask(task: &TaskBlock) {
    unsafe { set_cr3((&raw const **task.directory.lockRead()).addr() as u32); }
}


/// Obtains the init task.
pub fn initialTask() -> &'static TaskBlock {
    unsafe { &*_init }
}


/// Obtains the idle task.
pub fn idleTask() -> &'static TaskBlock {
    unsafe { &*_idle }
}


/* Syscalls */

/// Fork the current task into a new task.
///
/// New task will be a copy of the old
/// and resume in user mode where the old one
/// called fork.
pub fn fork() -> c_int {
    let Some(mut task) = TaskBlock::new() else { return -1 };

    let curr = getCurrentTask().unwrap();

    let err = task.copyTask(curr);
    if err.is_err() {
        return -1;
    }

    task.parent.set(curr);

    let Some(newThread) = forkThreadToTask(&*task as *const TaskBlock)
        else { return -1 };

    let tid = newThread.tid();
    task.originalTid = tid;

    unsafe { coll.insertTask(Box::into_raw(task)) };

    startThread(newThread);
    tid
}


/// Wait for a child to exit.
///
/// Claims a child task and will
/// clean it up and obtain its status
/// upon exiting.
pub unsafe fn wait(status_ptr: *mut c_int) -> c_int {
    if !status_ptr.is_null() &&
        unsafe { !isUserWritableAddr(LogicalAddress(status_ptr.addr()), size_of::<c_int>()) } {
            return -1;
        }

    let parent = getCurrentTask().unwrap();

    let mut queue = coll.queue.lock();

    for curr in queue.iter_ptr(|t| &t.link) {
        if unsafe { (&*curr).parent.get() == parent && !(&*curr).claimed.swap(true, Ordering::AcqRel) } {

            let tid;
            {
                let curr = unsafe { &*curr };

                while !curr.exitFlag.load(Ordering::Acquire) {
                    queue = curr.exited.waitForCond(queue);
                }

                let dir = lockReadFromMemory();

                if !status_ptr.is_null() && unsafe { !isUserWritableAddr(LogicalAddress(status_ptr.addr()), size_of::<c_int>()) } {
                    curr.claimed.store(false, Ordering::Release);
                    return -1;
                }

                if !status_ptr.is_null() {
                    unsafe { *status_ptr = *curr.exitStatus.lock() };
                }

                remove!(queue, curr, link);
                drop(queue);
                drop(dir);

                tid = curr.originalTid;
            }

            unsafe { Box::from_raw(curr.cast_mut()) };
            return tid;
        }
    }

    -1
}


/// Exits the current task.
pub fn task_vanish() -> ! {
    let task = getCurrentTask().unwrap();

    let queue = coll.queue.lock();
    for curr in queue.iter(|t| &t.link) {
        if curr.parent.get() == task {
            curr.parent.set(initialTask());
            curr.claimed.store(false, Ordering::Release);
        }
    }

    drop(queue);

    task.exitFlag.store(true, Ordering::Release);

    task.exited.signalCond();

    let disabledInterrupts = disableInterrupts();
    blockUntilRescheduled(&disabledInterrupts);
    unreachable!()
}
