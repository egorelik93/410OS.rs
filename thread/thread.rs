//! Functions that manipulate individual threads.

use core::cell::Cell;
use core::ffi::c_void;
use core::ptr::{self, null_mut};
use crate::registers::*;
use crate::task::TaskBlock;
use crate::variable_queue::Link;

use super::thread_internal::*;
use super::continuation::*;

/// Structure to hold initial state of thread
#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct InitialThreadState {
    ignoredRegisters: Registers,
    ignoredEax: *mut c_void,
    returnAddress: unsafe extern "cdecl" fn(SuspendedState) -> !,
    ignoredEbp: *mut c_void,
    state: SuspendedState
}

impl ThreadBlock {
    /// Create a thread block.
    pub(super) fn new() -> ThreadBlock {
        ThreadBlock {
            tid: 0,
            task: null_mut(),
            inKernelDirectory: Cell::new(false),
            kernelStackOffset: Cell::new(KERNEL_STACK_SIZE),
            link: Link::new(),
            free: Cell::new(false),
            scheduled: Cell::new(false),
            userDescheduledMutex: null_mut(), //(),
            userDescheduled: false,
            scheduleLink: Link::new(),
            taskLink: Link::new(),
            suspendedUserState: null_mut(),
            swexnHandler: null_mut(),
            esp3: null_mut(),
            exnUreg: null_mut()
        }
    }

    /// Get Tid of a thread block.
    pub fn tid(&self) -> i32 {
        self.tid
    }

    /// Get task associated with a thread.
    pub fn task(&self) -> *mut TaskBlock {
        self.task
    }

    /// Load an initial state for the kernel stack.
    pub fn load(&mut self, task: *mut TaskBlock, state: *mut SuspendedState) {
        self.task = task;
        self.kernelStackOffset.set(KERNEL_STACK_SIZE - size_of::<InitialThreadState>());

        unsafe {
            let threadState: &mut InitialThreadState = &mut *ptr::from_mut(self).byte_add(self.kernelStackOffset.get()).cast();

            threadState.state = *state;
            self.suspendedUserState = &raw mut threadState.state;

            threadState.returnAddress = exitKernelMode;
        }
    }
}
