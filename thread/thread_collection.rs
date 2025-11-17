//! Manage the various thread collections on link.

use core::pin::Pin;

use crate::sync::rwlock::RWLock;
use crate::variable_queue::*;

use super::{Thread, ThreadBlock, ThreadQueue};

pub struct ThreadCollection {
    queue: RWLock<ThreadQueue>
}

impl ThreadCollection {
    /// Initialize a thread collection.
    pub const fn new() -> ThreadCollection {
        ThreadCollection {
            queue: RWLock::new(Head::new())
        }
    }

    /// Insert a thread into a collection.
    pub fn insertThread<'a>(&'a self, thread: Pin<Thread>) -> Pin<&'a ThreadBlock> {
        let mut guard = self.queue.lockWrite();
        unsafe { insert_tail!(&mut guard, thread.as_ref(), link) }
    }

    /// Remove a thread from a collection.
    pub fn removeThread<'a>(&self, thread: Pin<&mut ThreadBlock>) -> Pin<Thread> {
        let mut guard = self.queue.lock();
        remove!(&mut guard, thread.as_ref(), link);
    }
}
