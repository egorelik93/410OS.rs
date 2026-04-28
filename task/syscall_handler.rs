//! C-level entrypoints for task syscalls.

use core::ffi::*;

use crate::registers::{Registers, SuspendedState};
use crate::thread::setSuspendedUserState;


/// Arguments to exec syscall
#[repr(C)]
#[derive(Debug)]
struct ExecArgs {
    execname: *const c_char,
    argvec: *mut *mut c_char
}


/// arguments to new_pages syscall
#[repr(C)]
#[derive(Debug)]
struct NewPagesArgs {
    base: *mut c_void,
    len: c_int
}



/// Entry point for fork syscall.
#[unsafe(no_mangle)]
pub unsafe extern "cdecl" fn forkHandler(state: *mut SuspendedState) {
    setSuspendedUserState(state);
    unsafe {
        (&mut *state).reg.eax = super::fork() as u32;
    }
}

/// Entry point for exec syscall.
#[unsafe(no_mangle)]
pub unsafe extern "cdecl" fn execHandler(reg: *mut Registers) {
    unsafe {
        let reg = &mut *reg;
        let args = &*(reg.esi as *const ExecArgs);
        reg.eax = super::exec(args.execname, args.argvec) as u32;
    }
}

/// Entry point for set_status syscall.
#[unsafe(no_mangle)]
pub unsafe extern "cdecl" fn setStatusHandler(reg: *mut Registers) {
    unsafe {
        let reg = &mut *reg;
        let status = reg.esi as c_int;
        super::set_status(status);
    }
}

/// Entry point for fork syscall.
#[unsafe(no_mangle)]
pub unsafe extern "cdecl" fn waitHandler(reg: *mut Registers) {
    unsafe {
        let reg = &mut *reg;
        let status_ptr = reg.esi as *mut c_int;
        reg.eax = super::wait(status_ptr) as u32;
    }
}

/// Entry point for new_pages syscall.
#[unsafe(no_mangle)]
pub unsafe extern "cdecl" fn newPagesHandler(reg: *mut Registers) {
    unsafe {
        let reg = &mut *reg;
        let args = &*(reg.esi as *const NewPagesArgs);
        reg.eax = super::new_pages(args.base, args.len) as u32;
    }
}

/// Entry point for remove_pages syscall.
#[unsafe(no_mangle)]
pub unsafe extern "cdecl" fn removePagesHandler(reg: *mut Registers) {
    unsafe {
        let reg = &mut *reg;
        let base = reg.esi as *mut c_void;
        reg.eax = super::remove_pages(base) as u32;
    }
}
