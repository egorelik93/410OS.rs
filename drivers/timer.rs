//! Header for the timer installer.

mod timer;
mod timer_handler_wrapper;
pub mod syscall_handler;

/* Syscalls */

pub use timer::{
    get_ticks,
    sleep
};

pub use timer::installTimerDriver;
