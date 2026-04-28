//! Implementation of Tasks

mod task_internal;
mod task_collection;
mod loader;
mod memory;
mod task;
mod manager;
pub mod syscall_handler;

/// Data structure containing information for a task
pub use task_internal::TaskBlock;

/* Individual Task Functions */
pub use loader::loadProgram;


/* Task Management API */
pub use manager::{
    installTaskManager,
    startTask,
    switchToTask
};

/* Memory Management API */
pub use memory::{
    lockReadFromMemory,
    lockWriteFromMemory
};

/* Syscalls */

pub use task::{
    exec,
    set_status,
};
pub use manager::{
    fork,
    task_vanish,
    wait
};

pub use memory::{
    new_pages,
    remove_pages
};
