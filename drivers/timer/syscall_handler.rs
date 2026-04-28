//! Entry points for timer related syscalls.

use core::ffi::c_int;

use crate::registers::Registers;

#[unsafe(no_mangle)]
pub unsafe extern "cdecl" fn sleepHandler(reg: *mut Registers) {
    unsafe {
        let reg = &mut *reg;
        let ticks = reg.esi as c_int;
        reg.eax = super::sleep(ticks) as u32;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "cdecl" fn getTicksHandler(reg: *mut Registers) {
    unsafe {
        let reg = &mut *reg;
        reg.eax = super::get_ticks();
    }
}
