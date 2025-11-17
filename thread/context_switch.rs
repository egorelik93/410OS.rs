//! Switch the currently running thread.

use core::ffi::c_void;
use core::ptr::{self, NonNull, null_mut};
use super::continuation::{callWithCurrentContinuation, continueFromContinuation};
use super::thread_internal::{KERNEL_STACK_SIZE, getNextThread};
use super::{continuation::Continuation, *};
use crate::sync::disable_interrupts::{DisabledInterruptsGuard, disableInterrupts};
use crate::task::TaskBlock;

static mut _currentThread: *mut ThreadBlock = null_mut();

/// Obtain the currently running thread.
pub fn getCurrentThread<'a>() -> Option<&'a ThreadBlock> {
    unsafe { NonNull::new(_currentThread).map(|p| p.as_ref()) }
}

/// Obtain the currently set task.
pub fn getCurrentTask() -> Option<NonNull<TaskBlock>> {
    let thread = getCurrentThread()?;
    NonNull::new(unsafe { thread.as_ref().task() })
}

/// Update a thread to store the given continuation.
fn saveContinuationTo(thread: &mut ThreadBlock, cont: Continuation) {
    thread.kernelStackOffset = unsafe { cont.byte_offset_from_unsigned(thread) };
    unimplemented!()
    // thread.inKernelDirectory = (get_cr3() == kernelDirectory())
}

/// Save the given continuation and switch to the given
/// thread.
///
/// This function is meant to be passed into
/// callWithCurrentContinuation.
/// Context switches must not occur during this function.
///
/// Does not return. All resources will be leaked if not manually dropped before calling.
unsafe extern "cdecl" fn saveAndContinue(cont: Continuation, args: *mut c_void) -> ! {
    unsafe {
        let mut curr = getCurrentThread().unwrap_unchecked();

        saveContinuationTo(curr.as_mut(), cont);

        let next: *const ThreadBlock = args.cast();
        continueThread(next)
    }
}

/// Context switch to a thread.
///
/// If thread is None, will switch to the next
/// thread in the schedule.
/// Otherwise, will try to switch to the given
/// thread. If it is not scheduled,
/// return -1.
/// Interrupts are disabled so that context
/// switches cannot be nested.
pub fn yieldThread(thread: Option<&ThreadHandle>) -> Result<(), ()> {
    let disabledInterrupts = disableInterrupts();
    yieldThreadWithoutInterrupts(&disabledInterrupts, thread)
}

/// This function was not part of the original C implementation
pub fn yieldThreadWithoutInterrupts(disabledInterrupts: &DisabledInterruptsGuard, thread: Option<&ThreadHandle>)
                                   -> Result<(), ()> {
                                       let thread = thread.or_else(|| getNextThread(disabledInterrupts));
    let Some(thread) = thread
    else {
        drop(disabledInterrupts);
        return Err(());
    };

    yieldThreadTo(&disabledInterrupts, thread)
}

/// This function was not part of the original C implementation
///
/// While this function requires interrupts disabled to do its work and promises to
/// return with them disabled, interrupts will be re-enabled while other threads run.
pub fn yieldThreadTo(disabledInterrupts: &DisabledInterruptsGuard, thread: &ThreadBlock) -> Result<(), ()> {
    if !thread.scheduled.get() {
        Err(())
    } else {
        unsafe {
            let ptr = ptr::from_ref(thread).cast_mut().cast();
            callWithCurrentContinuation(saveAndContinue, thread);
        }
        Ok(())
    }
}

/// Resume a saved thread.
///
/// Must be called while interrupts are disabled.
///
/// Does not return. All resources will be leaked if not manually dropped before calling.
pub unsafe fn continueThread(thread: &ThreadBlock) -> ! {
    unsafe {
        let oldThread = _currentThread;
        _currentThread = thread;

        set_esp0(thread.byte_add(KERNEL_STACK_SIZE).addr().get());
        let cont: Continuation = thread.byte_add(thread.as_mut().kernelStackOffset).as_ptr().cast();

        //

        continueFromContinuation(cont)
    }
}
