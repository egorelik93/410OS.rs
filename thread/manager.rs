//! Global collections of threads.

use core::alloc::Layout;
use core::ffi::c_int;
use core::mem::{self, MaybeUninit};
use core::ops::{Deref, DerefMut};
use core::pin::Pin;
use core::sync::atomic::Ordering;

use _410kern::seg::SEGSEL_KERNEL_CS;
use alloc::alloc::{alloc, dealloc};

use super::thread_internal::KERNEL_STACK_SIZE;
use super::{ThreadBlock, ThreadCollection, ThreadHandle, getCurrentThread, getCurrentTask, scheduleThread};
use crate::idt_entry::*;
use crate::registers::SuspendedState;
use crate::sync::disable_interrupts::disableInterrupts;
use crate::idgen::IDGenerator;
use crate::task::{TaskBlock, task_vanish};
use crate::thread::blockUntilRescheduled;
use crate::syscall_int::*;


static tidGen: IDGenerator = IDGenerator::new(1);

/// Threads that are in use.
static activeColl: ThreadCollection = ThreadCollection::new();

/// Thread blocks that have been allocated but
/// do not correspond to a thread.
static freeColl: ThreadCollection = ThreadCollection::new();


/* Kernel functions */

/// Initialize the threading system.
pub unsafe fn installThreadManager() {
    unsafe {
        *IDT().add(GETTID_INT) = trapGate(
            USER_PRIVILEGE,
            crate::syscall::gettidHandlerWrapper,
            SEGSEL_KERNEL_CS);

        *IDT().add(THREAD_FORK_INT) = trapGate(
            USER_PRIVILEGE,
            crate::syscall::threadForkHandlerWrapper,
            SEGSEL_KERNEL_CS);

        *IDT().add(VANISH_INT) = trapGate(
            USER_PRIVILEGE,
            crate::syscall::threadForkHandlerWrapper,
            SEGSEL_KERNEL_CS);
        *IDT().add(YIELD_INT) = trapGate(
            USER_PRIVILEGE,
            crate::syscall::yieldHandlerWrapper,
            SEGSEL_KERNEL_CS);

        *IDT().add(DESCHEDULE_INT) = trapGate(
            USER_PRIVILEGE,
            crate::syscall::descheduleHandlerWrapper,
            SEGSEL_KERNEL_CS);

        *IDT().add(MAKE_RUNNABLE_INT) = trapGate(
            USER_PRIVILEGE,
            crate::syscall::descheduleHandlerWrapper,
            SEGSEL_KERNEL_CS);

        *IDT().add(SWEXN_INT) = trapGate(
            USER_PRIVILEGE,
            crate::syscall::swexnHandlerWrapper,
            SEGSEL_KERNEL_CS);
    }
}

/// An (owned) pointer to the full thread allocation, including the kernel stack.
///
/// Not in the original C implementation, which directly used pointers to ThreadBlocks
/// for this.
#[derive(Debug)]
pub struct Thread(*mut ThreadBlock);

impl Thread {
    unsafe fn from_raw(thread: *mut ThreadBlock) -> Self {
        Thread(thread)
    }

    unsafe fn into_raw(self: Thread) -> *mut ThreadBlock {
        let ptr = self.0;
        mem::forget(self);
        ptr
    }

    fn into_pin(self) -> Pin<Thread> {
        unsafe { Pin::new_unchecked(self) }
    }
}

impl Deref for Thread {
    type Target = ThreadBlock;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0 }
    }
}

impl DerefMut for Thread {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.0 }
    }
}


// Kernel functions

impl Drop for Thread {
    /// Deallocates a ThreadBlock
    ///
    /// This was not a separate function in the original implementation,
    /// but the logic was contained in clearFreeThreadCollection
    fn drop(&mut self) {
        unsafe {
            dealloc(self.0.cast(), Layout::from_size_align_unchecked(KERNEL_STACK_SIZE, KERNEL_STACK_SIZE));
        }
    }
}

impl Thread {
    /// Allocates and initialize a thread.
    ///
    /// This includes the kernel stack.
    pub fn new() -> Option<Thread> {
        /* Allocation */

        let mut guard = freeColl.queue.lockWrite();

        let thread = match unsafe { guard.tail_ptr() } {
            None => {
                drop(guard);

                unsafe {
                    let thread = alloc(Layout::from_size_align_unchecked(KERNEL_STACK_SIZE, KERNEL_STACK_SIZE))
                        .cast::<ThreadBlock>();

                    if thread.is_null() {
                        return None;
                    }

                    thread
                }
            },
            Some(thread) => {
                unsafe {
                    remove!(&mut guard, &*thread, link);
                    drop(guard);
                    thread.cast::<ThreadBlock>().cast_mut()
                }
            }
        };

        /* Initialization */
        unsafe { thread.write(ThreadBlock::new()) };
        let thread = unsafe { &mut *thread };
        thread.tid = tidGen.generateID();

        Some(unsafe { Thread::from_raw(thread) })
    }
}

/// Frees an active thread.
pub fn freeThread(thread: &ThreadBlock) {
    let Some(thread) = activeColl.removeThread(thread) else { return };
    unsafe { freeColl.insertThread(thread); }
}

/// Empties the queue of freed threads.
pub fn clearFreeThreadCollection() {
    let mut queue = freeColl.queue.lockWrite();

    for thread in unsafe { queue.iter_ptr(|t| t.link()) } {
        if unsafe { &*thread }.refCount.load(Ordering::Acquire) > 0 {
            continue;
        }

        let thread = unsafe { Thread::from_raw(thread.cast_mut()) };

        remove!(&mut queue, &thread, link);
        drop(Thread);
    }
}

/// Activate thread.
///
/// thread must be loaded.
pub fn startThread(thread: Thread) {
    /* Attach to task */
    // ...

    let thread = unsafe { activeColl.insertThread(Thread::into_raw(thread).cast()) };

    let guard = disableInterrupts();
    scheduleThread(&guard, &thread.handle());
}

/// Obtain an active thread block corresponding to a tid.
pub fn getActiveThreadByTid(tid: i32) -> Option<ThreadHandle> {
    let coll = activeColl.queue.lockWrite();

    for curr in coll.iter(|t| &t.link) {
        if curr.tid == tid {
            return Some(curr.handle())
        }
    }

    None
}

/// Fork the current thread into an arbitrary task.
///
/// The new thread will be set to return to the same point
/// as in the original in user mode,
/// but returning a value of 0.
pub fn forkThreadToTask(task: *const TaskBlock) -> Option<Thread> {
    let mut new = Thread::new()?;

    let curr = getCurrentThread().unwrap();

    new.load(task, unsafe { &*curr.suspendedUserState.get() });
    unsafe { &mut *new.suspendedUserState.get() }.reg.eax = 0;
    new.inKernelDirectory.set(false);

    new.swexnHandler.set(curr.swexnHandler.get());
    new.esp3.set(curr.esp3.get());
    new.exnUreg.set(curr.exnUreg.get());

    Some(new)
}

/// Sets the suspended user state pointer
/// of the current thread.
pub fn setSuspendedUserState(state: *mut SuspendedState) {
    getCurrentThread().map(|t| t.suspendedUserState.set(state));
}


/* Syscalls */

/// Obtain tid of the currently running thread.
pub fn gettid() -> c_int {
    getCurrentThread().map_or(-1, |t| t.tid)
}

/// Kill the currently running thread.
pub fn vanish() -> ! {
    let thread = getCurrentThread().unwrap();
    let task = getCurrentTask().unwrap();
    let taskThreads = task.threadsOfTask();

    let mut queue = taskThreads.queue.lockWrite();

    let first = queue.front();
    if first == Some(thread) && first.unwrap().taskLink().next().is_none() {
        drop(queue);
        task_vanish()
    } else {
        remove!(&mut queue, thread, taskLink);
        drop(queue);

        freeThread(thread);
        blockUntilRescheduled(&disableInterrupts());
        unreachable!()
    }
}

/// Fork the current thread.
pub fn thread_fork() -> c_int {
    let old = getCurrentThread().unwrap();
    let Some(new) = forkThreadToTask(old.task) else { return -1; };

    let tid = new.tid;
    startThread(new);
    tid
}
