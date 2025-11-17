use crate::registers::SuspendedState;

use super::{Thread, ThreadCollection, ThreadHandle, getCurrentThread};

/// Threads that are in use.
static activeColl: ThreadCollection = ThreadCollection::new();

/// Thread blocks that have been allocated but
/// do not correspond to a thread.
static freeColl: ThreadCollection = ThreadCollection::new();


// Kernel functions

impl Drop for Thread {
    /// Frees an active thread.
    fn drop(&mut self) {
        activeColl.removeThread(self)
    }
}

/// Obtain an active thread block corresponding to a tid.
pub fn getActiveThreadByTid(tid: i32) -> Option<ThreadHandle> {
    let coll = activeColl.queue.lock();

    for curr in coll.iter(|t| t.link) {
        if curr.tid == tid {
            return Some(curr.handle())
        }
    }

    None
}

/// Sets the suspended user state pointer
/// of the current thread.
pub(super) fn setSuspendedState(state: *mut SuspendedState) {
    getCurrentThread()?.suspendedUserState.set(state);
}
