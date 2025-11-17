//! Thread Implementation

mod thread;
mod thread_internal;
mod continuation;
mod context_switch;
mod scheduler;
mod thread_collection;
mod manager;

use core::ops::{Deref, DerefMut};
use core::pin::Pin;
use core::ptr::{NonNull, null_mut};
use core::sync::atomic::Ordering;

/// Data structure containing information about a thread
pub use thread_internal::ThreadBlock;

/// Thread Management API
/*pub use manager::{
    installThreadManager,
}*/

/// Thread Collection API
pub use thread_collection::ThreadCollection;

/// Scheduling API
pub use scheduler::{
    scheduleThread,
    descheduleThread,
    blockUntil
};

/// Mode Switch
pub use continuation::exitKernelMode;

/// Context Switch API
pub use context_switch::{
    getCurrentThread,
    getCurrentTask,
    yieldThread,
    yieldThreadTo,
    yieldThreadWithoutInterrupts,
    continueThread
};

use crate::sync::mutex::Mutex;
use crate::variable_queue::Head;

/// An identifier for a thread.
///
/// Can be used to access the corresponding thread block, but as this handle is not bound
/// to a lifetime, unless it is the current thread, this is unsafe.
///
/// Not in the original C implementation, which directly used pointers to ThreadBlocks
/// for this.
#[derive(Debug, Eq, PartialEq)]
pub struct ThreadHandle(NonNull<ThreadBlock>);

unsafe impl Send for ThreadHandle {}

impl ThreadHandle {
    pub unsafe fn from_raw_parts(thread: *const ThreadBlock) -> ThreadHandle {
        unsafe { ThreadHandle(NonNull::new_unchecked(thread.cast_mut())) }
    }

    pub unsafe fn to_raw_parts(self) -> *const ThreadBlock {
        self.0.as_ptr().cast()
    }

    pub fn deref_pin(&self) -> Pin<&ThreadBlock> {
        unsafe { Pin::new_unchecked(&self) }
    }
}

impl ThreadBlock {
    pub fn handle(&self) -> ThreadHandle {
        self.refCount.fetch_add(1, Ordering::AcqRel);
        ThreadHandle(NonNull::from_ref(self))
    }
}

impl Deref for ThreadHandle {
    type Target = ThreadBlock;

    fn deref(&self) -> &Self::Target {
        unsafe { self.0.as_ref() }
    }
}

impl Clone for ThreadHandle {
    fn clone(&self) -> ThreadHandle {
        self.handle()
    }
}

impl Drop for ThreadHandle {
    fn drop(&mut self) {
        self.refCount.fetch_sub(1, Ordering::AcqRel)
    }
}

/// An (owned) pointer to the full thread allocation, including the kernel stack.
///
/// Not in the original C implementation, which directly used pointers to ThreadBlocks
/// for this.
struct Thread(*mut ThreadBlock);

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

pub type ThreadQueue = Head<ThreadBlock>;
