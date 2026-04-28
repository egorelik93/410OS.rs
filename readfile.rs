//! The readfile system call.

use core::ffi::{CStr, c_char, c_int, c_uint};
use core::ptr;

use _410kern::exec2obj::*;
use _410kern::seg::SEGSEL_KERNEL_CS;

use crate::idt_entry::*;
use crate::task::lockReadFromMemory;
use crate::virtual_memory::{LogicalAddress, isUserWritableAddr};
use crate::syscall_int::*;


#[unsafe(export_name = "getbytes")]
unsafe extern "cdecl" fn getbytes_c(filename: *const c_char, offset: c_int, size: c_int, buf: *mut c_char) -> c_int {
    unsafe {
        getbytes(CStr::from_ptr(filename), offset as usize, &mut *ptr::slice_from_raw_parts_mut(buf.cast(), size as usize))
            .map_or(-1, |_| 0)
    }
}

/// Copies data from a file into a buffer.
pub fn getbytes(filename: &CStr, offset: usize, buf: &mut [u8]) -> Result<usize, ()> {
    /* Find file */

    let mut entry = None;

    for i in 0 .. MAX_NUM_APP_ENTRIES {
        let currEntry = unsafe { &exec2obj_userapp_TOC[i] };
        let execname = unsafe { &*ptr::slice_from_raw_parts(currEntry.execname.as_ptr().cast(), MAX_EXECNAME_LEN) };

        if Ok(filename) == CStr::from_bytes_until_nul(execname) {
            entry = Some(currEntry);
            break;
        }
    }

    let Some(entry) = entry else { return Err(()) };

    if offset > entry.execlen as usize {
        return Err(());
    }

    for i in 0 .. buf.len() {
        if offset + i < entry.execlen as usize {
            buf[i] = unsafe { *entry.execbytes.add(offset + i) };
        } else {
            return Ok(i);
        }
    }

    Ok(buf.len())
}


/* Syscalls */

/// Read the contents of a file.
///
/// Input is validated before calling getbytes.
pub unsafe fn readfile(filename: *const c_char, buf: *mut c_char, count: c_int, offset: c_int) -> c_int {
    if count < 0 || offset < 0 {
        return -1;
    }

    let dir = lockReadFromMemory();

    if unsafe { !isUserWritableAddr(LogicalAddress(buf.addr()), count as usize) } {
        return -1;
    }

    let filename = unsafe { CStr::from_ptr(filename) };

    getbytes(filename, offset as usize, unsafe { &mut *ptr::slice_from_raw_parts_mut(buf.cast(), count as usize) })
        .map_or(-1, |_| 0)
}


/// Install the readfile syscall.
pub unsafe fn installFileSystem() {
    unsafe {
        *IDT().add(READFILE_INT) = trapGate(USER_PRIVILEGE,
                                            crate::syscall::readfileHandlerWrapper,
                                            SEGSEL_KERNEL_CS);
    }
}
