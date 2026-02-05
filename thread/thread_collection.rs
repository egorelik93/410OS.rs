//! Manage the various thread collections on link.

use core::pin::Pin;
use core::{mem, ptr};
use core::sync::atomic::Ordering;

use crate::sync::rwlock::{RWLock, WriteGuard};
use crate::variable_queue::*;

use super::{ThreadBlock, ThreadHandle, ThreadQueue};

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
