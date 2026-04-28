//! Manage a task list.
//!
//! Note that the link field of a Task
//! should never be manipulated unless
//! the entire collection is locked.

use super::TaskBlock;
use super::task_internal::TaskCollection;

impl TaskCollection {
    /// Insert a task block into the task collection
    pub unsafe fn insertTask(&self, task: *const TaskBlock) -> &TaskBlock {
        let mut guard = self.queue.lock();
        unsafe { insert_tail!(&mut guard, task, link) }
    }

    /// Remove a task block from the task collection
    pub fn removeThread(&self, task: &TaskBlock) -> Option<*const TaskBlock> {
        let mut guard = self.queue.lock();
        remove!(&mut guard, &task, link)
    }
}
