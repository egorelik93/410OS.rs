//! Handler for swexn syscall

use core::ffi::{c_int, c_void};
use core::ptr::{null, null_mut};

use _410kern::eflags::{EFL_IOPL_RING0, EFL_IOPL_RING3};

use crate::registers::{Registers, SuspendedState};
use crate::swexn::SwexnHandler;
use crate::ureg::UReg;
use crate::virtual_memory::{LogicalAddress, isUserWritableAddr};

use super::getCurrentThread;


/// Checks if a ureg_t is a valid parameter to the swexn() syscall
unsafe fn is_ureg_valid(regs: *mut UReg) -> bool {
    if regs.is_null() { return true; }

    // Check if each byte of regs is user-writable
    // The register values should have legitimately come from the user
    if unsafe { !isUserWritableAddr(LogicalAddress(regs.addr()), size_of_val(&*regs)) } {
        return false;
    }

    let regs = unsafe { &*regs };

    // Check IOPL flags in EFLAGS register. We are using rings 0 and 3.
    // Make sure I/O can only be accessed at Ring 0
    let iopl = regs.eflags as u64 & EFL_IOPL_RING3;

    if iopl != EFL_IOPL_RING0 {
        return false;
    }

    true
}


pub fn copyURegToSuspendedState(src: &UReg) -> SuspendedState {
    SuspendedState {
        // Note: Not copying eax!
        reg: Registers {
            edi: src.edi,
            esi: src.esi,
            ebp: src.ebp,
            esp: src.esp,
            ebx: src.ebx,
            edx: src.edx,
            ecx: src.ecx,
            // Note: Not copying eax!
            eax: 0
        },
        gs: src.gs,
        fs: src.fs,
        es: src.es,
        ds: src.ds,
        eip: src.eip,
        cs: src.cs,
        eflags: src.eflags,
        esp: src.esp,
        ss: src.ss
    }
}


/// Handle the swexn syscall
///
/// Note this might be called from a user's swexn handler
pub fn swexn(esp3: *mut c_void, eip: Option<SwexnHandler>, arg: *mut c_void, newreg: *mut UReg) -> c_int {
    // Check for invalid invocation (newreg is invalid)
    if unsafe { !is_ureg_valid(newreg) } {
        return -1;
    }

    let currentThreadBlock = getCurrentThread().unwrap();

    // If we have to de-register the handler
    if esp3.is_null() || eip.is_none() {
        currentThreadBlock.swexnHandler.set(None);
        currentThreadBlock.esp3.set(null_mut());
    } else {  // If we have to register the handler
        currentThreadBlock.swexnHandler.set(eip);
        let mut swexnEsp = esp3;

        // esp3 is one word higher than first address to push onto
        // We must first reserve space for a ureg_t object
        // Then, push the args in reverse, and then a canary return address
        swexnEsp = unsafe { swexnEsp.byte_sub(size_of::<UReg>()) };
        let uregAddr = swexnEsp;
        currentThreadBlock.exnUreg.set(uregAddr.cast());

        swexnEsp = unsafe { swexnEsp.byte_sub(size_of::<*mut UReg>()) };
        unsafe { *swexnEsp.cast() = uregAddr };

        swexnEsp = unsafe { swexnEsp.byte_sub(size_of::<*mut c_void>()) };
        unsafe { *swexnEsp.cast() = arg };

        swexnEsp = unsafe { swexnEsp.byte_sub(size_of::<*mut c_void>()) };
        unsafe { *swexnEsp.cast() = 0xdeadd00d as *mut c_void };

        currentThreadBlock.esp3.set(swexnEsp);
    }

    // Write new registers if necessary
    if !newreg.is_null() {
        unsafe {
            *currentThreadBlock.suspendedUserState.get() = copyURegToSuspendedState(&*newreg);
        }
    }

    0
}
