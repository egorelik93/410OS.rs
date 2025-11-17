//! Header for loading in a saved context.

use core::ffi::c_void;
use crate::registers::*;

pub(super) type Continuation = *mut c_void;

unsafe extern "cdecl" {
    /// Returns to user mode.
    ///
    /// Does not return. All resources will be leaked if not manually dropped before calling.
    pub fn exitKernelMode(state: SuspendedState) -> !;

    /// Resumes a thread from a given continuation.
    ///
    /// Does not return. All resources will be leaked if not manually dropped before calling.
    pub fn continueFromContinuation(cont: Continuation) -> !;

    /// Call a function that takes the state of the current thread.
    ///
    /// This function saves the state of the current thread on the
    /// stack, takes the location of this state on the stack (the continuation),
    /// and calls the provided function with this
    /// and the generic arg you provided.
    /// Because it is called below where the
    /// state is stored,
    /// the provided function is free to use this continuation
    /// however it wants, including saving to be resumed from this point later.
    /// If the provided function returns, it will act as calling
    /// continueFromContinuation on the continuation.
    /// Otherwise, this function should only return when
    /// someone else calls continueFromContinuation on cont.
    pub fn callWithCurrentContinuation(next: unsafe extern "cdecl" fn(cont: Continuation, args: *mut c_void) -> !, args: *mut c_void);
}
    
core::arch::global_asm!(include_str!("continuation.S"), options(att_syntax));
