//! Manage the various thread collections on link.

use core::pin::Pin;
use core::{mem, ptr};
use core::sync::atomic::Ordering;

use crate::sync::rwlock::{RWLock, WriteGuard};
use crate::variable_queue::*;

use super::{ThreadBlock, ThreadHandle, ThreadQueue, getCurrentTask, getCurrentThread};

#[derive(Debug)]
pub struct ThreadCollection {
    pub queue: RWLock<ThreadQueue>
}

impl ThreadCollection {
    /// Initialize a thread collection.
    pub const fn new() -> ThreadCollection {
        ThreadCollection {
            queue: RWLock::new(Head::new())
        }
    }

    /// Insert a thread into a collection.
    pub unsafe fn insertThread(&self, thread: *const ThreadBlock) -> &ThreadBlock {
        let mut guard = self.queue.lockWrite();
        unsafe { insert_tail!(&mut guard, thread, link) }
    }

    /// Remove a thread from a collection.
    pub fn removeThread(&self, thread: &ThreadBlock) -> Option<*const ThreadBlock> {
        assert!(thread.refCount.load(Ordering::Acquire) == 1);
        
        let mut guard = self.queue.lockWrite();
        remove!(&mut guard, &thread, link)
    }
}

/// Disassociate a thread with a task.
pub fn removeThreadFromTask(thread: &ThreadBlock) {
    if !thread.task.is_null() {
        let mut taskThreads = unsafe { &*thread.task }.threadsOfTask().queue.lockWrite();
        remove!(&mut taskThreads, thread, taskLink);
    }
}

/// Checks if the current thread is the last one
/// in the task.
pub fn isLastThreadInTask() -> bool {
    let thread = getCurrentThread().unwrap();
    let task = getCurrentTask().unwrap();
    let taskThreads = task.threadsOfTask().queue.lockRead();

    let first = taskThreads.front();
    first == Some(thread) && first.unwrap().taskLink().next().is_none()
}
