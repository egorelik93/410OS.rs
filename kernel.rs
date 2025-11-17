//! Functions related to kernel entrypoint

#![no_std]
#![no_main]

// Temporary while I fill in the pieces.
#![allow(warnings)]

extern crate alloc;

// #![feature(unsafe_pinned)]

#[macro_use]
mod variable_queue;

mod sync;
mod thread;
mod registers;
mod virtual_memory;
mod byte_utils;
mod malloc_wrappers;


mod task {
    pub struct TaskBlock;
}

#[macro_export]
macro_rules! lprintf {
    ($($arg:tt)*) => {()}
}
