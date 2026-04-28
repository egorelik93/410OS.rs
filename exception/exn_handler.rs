//! Exception handler

use core::ffi::c_uint;
use core::ptr::null_mut;

use _410kern::cr::get_cr2;

use crate::lprintf;
use crate::task::set_status;
use crate::thread::{exitKernelMode, getCurrentThread, vanish};
use crate::ureg::UReg;
use crate::registers::{ExceptionState, SuspendedState};


/// Copy values from exception state to uregs
fn copyExceptionStateToUreg(vec: c_uint, exnState: &ExceptionState) -> UReg {
    UReg {
        cause: vec,
        cr2: unsafe { get_cr2() },

        ds: exnState.ds,
        es: exnState.es,
        fs: exnState.fs,
        gs: exnState.gs,

        edi: exnState.reg.edi,
        esi: exnState.reg.esi,
        ebp: exnState.reg.ebp,
        zero: 0, /* Dummy %esp, set to zero */
        ebx: exnState.reg.ebx,
        edx: exnState.reg.edx,
        ecx: exnState.reg.ecx,
        eax: exnState.reg.eax,

        error_code: exnState.err,
        eip: exnState.eip,
        cs: exnState.cs,
        eflags: exnState.eflags,
        esp: exnState.esp,
        ss: exnState.ss
    }
}


/// Generalized exception handler
///
/// If there is a swexn handler installed, it de-registers the handler and
/// runs it in user mode.
#[unsafe(no_mangle)]
pub unsafe extern "cdecl" fn generalExnHandler(exnState: *const ExceptionState, vec: c_uint) -> ! {
    let myThreadBlock = getCurrentThread().unwrap();

    // If there is no swexn handler, ruthlessly kill the thread.
    // TODO: Try to repair the thread first!!
    if myThreadBlock.esp3.get().is_null() || myThreadBlock.swexnHandler.get().is_none() {
        lprintf!("Killing TID %d due to exception %u.\n", myThreadBlock->tid, vec);

        set_status(-2);
        vanish()
    } else {
        // If there is a swexn handler installed

        // Design decision: Explicitly return to user-mode, never "return"
        // There is a performance hit passing an object to exitKernelMode()
        let mut exitState = SuspendedState::copyExceptionState(unsafe { &*exnState });

        // Set up user-mode stack
        let uregs = myThreadBlock.exnUreg.get();
        *unsafe { &mut *uregs } = copyExceptionStateToUreg(vec, unsafe { &*exnState });

        // Set iret parameters so we run the swexn handler
        exitState.esp = myThreadBlock.esp3.get().addr() as u32;
        exitState.eip = myThreadBlock.swexnHandler.get().map(|h| h as u32).unwrap_or(0);

        // De-register swexn handler
        myThreadBlock.esp3.set(null_mut());
        myThreadBlock.swexnHandler.set(None);

        unsafe { exitKernelMode(exitState) }
    }
}
