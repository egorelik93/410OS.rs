//! Handlers for console syscalls.

use core::ffi::{c_char, c_int};

use crate::registers::{self, Registers};
use crate::sync::disable_interrupts::{disableInterrupts, enteredDisabledInterrupts};


/// Argument packet for set_cursor_pos.
#[repr(C)]
#[derive(Debug)]
struct SetCursorPosArgs {
    row: c_int,
    col: c_int
}

/// Argument packet for get_cursor_pos.
#[repr(C)]
#[derive(Debug)]
struct GetCursorPosArgs {
    row: *mut c_int,
    col: *mut c_int
}

/// Argument packet for print.
#[repr(C)]
#[derive(Debug)]
struct PrintArgs {
    len: c_int,
    buf: *mut c_char
}


/// Entry point for set_term_color syscall.
#[unsafe(no_mangle)]
pub unsafe extern "cdecl" fn setTermColorHandler(reg: *mut Registers) {
    unsafe {
        let disabledInterrupts = enteredDisabledInterrupts();
        let reg = &mut *reg;
        let arg = reg.esi as c_int;
        reg.eax = super::set_term_color(arg) as u32;
    }
}

/// Entry point for set_cursor_pos syscall.
#[unsafe(no_mangle)]
pub unsafe extern "cdecl" fn setCursorPosHandler(reg: *mut Registers) {
    unsafe {
        let disabledInterrupts = enteredDisabledInterrupts();
        let reg = &mut *reg;
        let args = &*(reg.esi as *const SetCursorPosArgs);
        reg.eax = super::set_cursor_pos(args.row, args.col) as u32;
    }
}

/// Entry point for get_cursor_pos syscall.
#[unsafe(no_mangle)]
pub unsafe extern "cdecl" fn getCursorPosHandler(reg: *mut Registers) {
    unsafe {
        let disabledInterrupts = enteredDisabledInterrupts();
        let reg = &mut *reg;
        let args = &*(reg.esi as *const GetCursorPosArgs );
        reg.eax = super::get_cursor_pos(args.row, args.col) as u32;
    }
}

/// Entry point for print syscall.
#[unsafe(no_mangle)]
pub unsafe extern "cdecl" fn printHandler(reg: *mut Registers) {
    unsafe {
        let disabledInterrupts = enteredDisabledInterrupts();
        let reg = &mut *reg;
        let args = &*(reg.esi as *const PrintArgs);
        reg.eax = super::print(args.len, args.buf) as u32;
    }
}
