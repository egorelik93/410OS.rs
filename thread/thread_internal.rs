//! Definition of thread related types.

use core::cell::{Cell, UnsafeCell};
use core::ffi::c_void;
use core::pin::Pin;
use core::ptr::NonNull;
use core::sync::atomic::{AtomicBool, AtomicU32};
use crate::variable_queue::Link;
use crate::task::TaskBlock;
use crate::registers::*;

pub(super) const KERNEL_STACK_SIZE: usize = 2048;

pub(super) const TID_NOT_A_THREAD: i32 = -1;

pub type ThreadBlockLink = Link<ThreadBlock>;

/// TCB structure, containing info about a thread
///
/// The thread block is in fact always the bottom portion of a
/// kernel stack; anytime we allocate a thread block,
/// we actually want to allocate the entire stack.
#[derive(Debug)]
pub struct ThreadBlock {
    /// Unique Thread Identifier
    pub(super) tid: i32,

    /// Task the thread is running on.
    pub(super) task: *mut TaskBlock,

    /// Whether the thread was in the kernel directory before being suspended
    pub(super) inKernelDirectory: Cell<bool>,

    /// Current bottom of the kernel stack
    /// relative to the thread block address
    pub(super) kernelStackOffset: Cell<usize>,

    /// General purpose queue link.
    pub(super) link: ThreadBlockLink,

    /// Flag for whether the thread is free
    pub(super) free: Cell<bool>,

    /// Flag for whether the thread is scheduled
    pub(super) scheduled: AtomicBool,

    /// Was the thread descheduled by the user.
    pub(super) userDescheduled: AtomicBool,

    /// Scheduling Queue link.
    pub(super) scheduleLink: ThreadBlockLink,

    /// Task's Thread Queue link
    pub(super) taskLink: ThreadBlockLink,

    /// Pointer to the saved user state from mode switch.
    pub(super) suspendedUserState: Cell<*mut SuspendedState>,

    /// Registered swexn
    pub(super) swexnHandler: Cell<*mut c_void>, // ...,

    /// Exception stack
    pub(super) esp3: *mut c_void,

    /// Space for ureg_t object on exception stack
    pub(super) exnUreg: *mut c_void, // ureg_t

    /// A count of handles and other references to this object.
    ///
    /// Was not part of the original C implmentation, but necessary to avoid use
    /// after free issues.
    pub(super) refCount: AtomicU32,

    /// The number of active DisabledInterruptsGuards on this thread
    ///
    /// Was not part of the original C implementation.
    pub(crate) disabledInterruptsRefCount: Cell<u32>
}

unsafe impl Send for ThreadBlock {}

impl ThreadBlock {
    pub(super) fn link(&self) -> &ThreadBlockLink {
        unsafe { &self.0.get().link }
    }

    pub(super) fn scheduleLink(&self) -> &ThreadBlockLink {
        unsafe { &self.0.get().scheduleLink }
    }

    pub(super) fn taskLink(&self) -> &ThreadBlockLink {
        unsafe { &self.0.get().taskLink }
    }
}


pub use super::scheduler::getNextThread;
pub use super::manager::getActiveThreadByTid;
