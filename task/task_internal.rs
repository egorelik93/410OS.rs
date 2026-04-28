//! Definition of the task type

use core::cell::{Cell, RefCell};
use core::ffi::c_void;
use core::sync::atomic::AtomicBool;

use _410kern::page::PAGE_SIZE;
use alloc::boxed::Box;
use crate::registers::SuspendedState;
use crate::sync::cond::Cond;
use crate::sync::mutex::Mutex;
use crate::sync::rwlock::RWLock;
use crate::thread::ThreadCollection;
use crate::variable_queue::*;
use crate::virtual_memory::PageDirectory;

/// Information about an allocated page
#[derive(Debug)]
pub struct PageAllocation {
    pub base: *mut c_void,
    pub len: usize,
    pub link: Link<PageAllocation>
}



pub type PageAllocationQueue = Head<PageAllocation>;


/// Structure with info about a task (TCB)
#[derive(Debug)]
pub struct TaskBlock {
    /// Page Directory for the task.
    pub(super) directory: RWLock<Box<PageDirectory>>,

    /// Tid of the first thread in the task.
    pub(super) originalTid: i32,

    /// Exit status of the task.
    pub(super) exitStatus: Mutex<i32>,

    /// Task that created this task.
    ///
    /// Note on the new implementation: this cell may only be read or written to
    /// when the overall task collection is locked.
    pub(super) parent: Cell<*const TaskBlock>,

    /// Collection of threads running on this task
    pub(super) threads: ThreadCollection,

    /// Link for the global task collection.
    pub(super) link: Link<TaskBlock>,

    /**
    Initial State for a new thread
    */
    pub(super) initState: Cell<SuspendedState>,

    /// Has this task exited.
    pub(super) exitFlag: AtomicBool,

    /// Is any task already waiting for this task.
    pub(super) claimed: AtomicBool,

    /// Cond var for exiting.
    pub(super) exited: Cond,

    /// Tracks user-requested allocations and their lengths.
    pub(super) allocations: RWLock<PageAllocationQueue>
}

unsafe impl Send for TaskBlock {}

pub const STACK_HIGH: usize = 0xffffffff;
pub const STACK_LOW: usize = STACK_HIGH.overflowing_add(1).0.overflowing_sub(PAGE_SIZE).0;


pub type TaskQueue = Head<TaskBlock>;

/// Collection of task blocks
#[derive(Debug)]
pub struct TaskCollection {
    pub queue: Mutex<TaskQueue>
}
