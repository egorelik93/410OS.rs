//! Functions related to kernel entrypoint

#![no_std]
#![no_main]

// #![feature(unsafe_pinned)]
#![feature(allocator_api)]
#![feature(arbitrary_self_types)]

// Temporary while I fill in the pieces.
#![allow(warnings)]

extern crate alloc;


use core::panic::PanicInfo;

#[macro_use]
mod variable_queue;

mod sync;
mod thread;
mod registers;
mod virtual_memory;
mod byte_utils;
mod malloc_wrappers;
mod idgen;


mod task {
    pub struct TaskBlock;
}

#[macro_export]
macro_rules! lprintf {
    ($($arg:tt)*) => {()}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    lprintf!("Panic {}", info);
    loop {}
}
