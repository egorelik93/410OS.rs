//! This is apparently necessary for linking against libsmp but is otherwise unused.

use core::ffi::*;

#[unsafe(no_mangle)]
unsafe extern "cdecl" fn tss_desc_create(tss: *mut c_void, tss_size: usize) -> u64 {
    let seg = 0u64;
    panic!();
    return seg;
}
