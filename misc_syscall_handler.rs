//! C-level entrypoints for misc. syscalls.

use core::ffi::{c_char, c_int};

use crate::halt;
use crate::readfile::readfile;
use crate::registers::Registers;
use crate::sync::disable_interrupts::enteredDisabledInterrupts;


/// Arguments to readfile
#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct ReadFileArgs {
    filename: *const c_char,
    buf: *mut c_char,
    count: c_int,
    offset: c_int
}


/// Entry point into readfile.
#[unsafe(no_mangle)]
pub unsafe extern "cdecl" fn readfileHandler(reg: *mut Registers) {
    unsafe {
        let reg = &mut *reg;
        let args = &*(reg.esi as *const ReadFileArgs);
        reg.eax = readfile(args.filename, args.buf, args.count, args.offset) as u32;
    }
}

/// Entry point into halt.
#[unsafe(no_mangle)]
pub unsafe extern "cdecl" fn haltHandler(reg: *mut Registers) {
    let disabledInterrupts = unsafe { enteredDisabledInterrupts() };
    halt();
    unreachable!();
}


/// Misbehave handler. Does nothing.
#[unsafe(no_mangle)]
pub unsafe extern "cdecl" fn misbehaveHandler(reg: *mut Registers) {}
