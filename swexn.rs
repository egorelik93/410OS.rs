use core::ffi::c_void;

use crate::ureg::UReg;

pub type SwexnHandler = unsafe extern "cdecl" fn(*mut c_void, *mut UReg);

pub use crate::exception::exn_gates::installExceptionGates;
pub use crate::exception::exn_handler::generalExnHandler;
