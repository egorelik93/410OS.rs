//! C-level entrypoints for thread syscalls.

use core::ffi::{c_int, c_void};

use crate::registers::{Registers, SuspendedState};
use crate::ureg::UReg;
use crate::swexn::SwexnHandler;


/// Arguments to swexn system call
#[repr(C)]
#[derive(Debug)]
struct SwexnArgs {
    esp3: *mut c_void,
    eip: Option<SwexnHandler>,
    arg: *mut c_void,
    newreg: *mut UReg
}


/// Entry point into gettid.
#[unsafe(no_mangle)]
pub unsafe extern "cdecl" fn gettidHandler(reg: *mut Registers) {
    unsafe {
        let reg = &mut *reg;
        reg.eax = super::gettid() as u32;
    }
}

/// Entry point into vanish.
#[unsafe(no_mangle)]
pub unsafe extern "cdecl" fn vanishHandler(reg: *mut Registers) {
    super::vanish()
}

/// Entry point into make_runnable.
#[unsafe(no_mangle)]
pub unsafe extern "cdecl" fn makeRunnableHandler(reg: *mut Registers) {
    unsafe {
        let reg = &mut *reg;
        let tid = reg.esi as c_int;
        reg.eax = super::make_runnable(tid) as u32;
    }
}

/// Entry point into deschedule.
#[unsafe(no_mangle)]
pub unsafe extern "cdecl" fn descheduleHandler(reg: *mut Registers) {
    unsafe {
        let reg = &mut *reg;
        let reject = reg.esi as *mut c_int;
        reg.eax = super::deschedule(reject) as u32;
    }
}

/// Entry point into yield.
#[unsafe(no_mangle)]
pub unsafe extern "cdecl" fn yieldHandler(reg: *mut Registers) {
    unsafe {
        let reg = &mut *reg;
        let tid = reg.esi as c_int;
        reg.eax = super::_yield(tid) as u32;
    }
}

/// Entry point into thread_fork.
#[unsafe(no_mangle)]
pub unsafe extern "cdecl" fn threadForkHandler(state: *mut SuspendedState) {
    super::setSuspendedUserState(state);
    unsafe {
        (&mut *state).reg.eax = super::thread_fork() as u32;
    }
}

/// Extract arguments from syscall packet and install the swexn
#[unsafe(no_mangle)]
pub unsafe extern "cdecl" fn swexnHandler(userState: *mut SuspendedState) {
    super::setSuspendedUserState(userState);

    unsafe {
        // Extract arguments and perform swexn call
        let args = &*((&*userState).reg.esi as *const SwexnArgs);

        // Write return value to user eax
        (&mut *userState).reg.eax = super::swexn_handler::swexn(args.esp3, args.eip, args.arg, args.newreg) as u32;
    }
}
