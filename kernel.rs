//! Functions related to kernel entrypoint

#![no_std]
#![no_main]

// #![feature(unsafe_pinned)]
#![feature(allocator_api)]
#![feature(arbitrary_self_types)]
#![feature(clone_to_uninit)]

// Temporary while I fill in the pieces.
#![allow(warnings)]

#![allow(non_snake_case)]
#![allow(clippy::similar_names)]

extern crate alloc;


use core::panic::PanicInfo;
use core::ffi::*;

use _410kern::asm::disable_interrupts;
use _410kern::multiboot::MBInfo;

use _410kern::seg::SEGSEL_KERNEL_CS;
use drivers::console::installConsoleDriver;
use drivers::keyboard::installKeyboardDriver;
use drivers::timer::installTimerDriver;
use idt_entry::{HARDWARE_PRIVILEGE, IDT, USER_PRIVILEGE, interruptGate, trapGate};
use readfile::installFileSystem;
use swexn::installExceptionGates;
use syscall::*;
use syscall_int::*;
use task::installTaskManager;
use thread::{installThreadManager, yieldThread};

#[macro_use]
mod variable_queue;

mod sync;
mod thread;
mod registers;
mod virtual_memory;
mod byte_utils;
mod malloc_wrappers;
mod idgen;
mod task;
mod readfile;
mod idt_entry;
mod handler_wrapper;
mod syscall;
mod swexn;
mod exception;
mod smp_glue;
mod pagefault_handler_wrapper;
mod misc_syscall_handler;

#[path = "../spec/common_kern.rs"]
mod common_kern;

#[path = "../spec/syscall_int.rs"]
mod syscall_int;

#[path = "../spec/ureg.rs"]
mod ureg;

mod drivers {
    pub mod console;
    pub mod keyboard;
    pub mod timer;
}
use drivers::{
    console,
    keyboard,
    timer
};
use virtual_memory::initVirtualMemory;


#[macro_export]
macro_rules! lprintf {
    ($($arg:tt)*) => {()}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    lprintf!("Panic {}", info);
    halt()
}


/// Ends kernel operation.
pub fn halt() -> ! {
    unsafe { disable_interrupts(); }

    loop {}
}


/// Kernel entrypoint.
#[unsafe(no_mangle)]
pub unsafe extern "cdecl" fn kernel_main(mbinfo: MBInfo, argc: c_int, argv: *const *const c_char, envp: *const *const c_char) -> ! {
    lprintf!( "Hello from a brand new kernel!" );

    unsafe {
        installConsoleDriver();
        installTimerDriver();
        installKeyboardDriver();

        initVirtualMemory();

        installThreadManager();
        installTaskManager();

        installExceptionGates();
        installFileSystem();

        *IDT().add(HALT_INT) = interruptGate(USER_PRIVILEGE,
                                             haltHandlerWrapper,
                                             SEGSEL_KERNEL_CS);

        *IDT().add(MISBEHAVE_INT) = trapGate(HARDWARE_PRIVILEGE,
                                             misbehaveHandlerWrapper,
                                             SEGSEL_KERNEL_CS);
    }

    /* Automatically enables interrupts
     * upon starting a thread.
     */
    yieldThread(None);

    halt()
}
