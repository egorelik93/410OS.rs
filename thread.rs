//! Thread Implementation

mod thread;
mod thread_internal;
mod continuation;
mod context_switch;
mod scheduler;
mod thread_collection;
mod manager;
mod swexn_handler;
pub mod syscall_handler;

use core::ops::{Deref, DerefMut};
use core::pin::Pin;
use core::ptr::{NonNull, null_mut};
use core::sync::atomic::Ordering;

/// Data structure containing information about a thread
pub use thread_internal::ThreadBlock;

/// Thread Management API
pub use manager::{
    Thread,
    installThreadManager,
    freeThread,
    startThread,
    forkThreadToTask,
    setSuspendedUserState
};

/// Thread Collection API
pub use thread_collection::{
    ThreadCollection,
    removeThreadFromTask,
    isLastThreadInTask
};
pub use manager::clearFreeThreadCollection;

/// Scheduling API
pub use scheduler::{
    scheduleThread,
    descheduleThread,
    blockUntil,
    blockUntilRescheduled
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

/// Syscalls
pub use manager::{
    gettid,
    thread_fork,
    vanish
};
pub use scheduler::{
    deschedule,
    make_runnable,
    _yield
};

use crate::sync::mutex::Mutex;
use crate::variable_queue::Head;

pub type ThreadQueue = Head<ThreadBlock>;

pub use thread_internal::ThreadHandle;
