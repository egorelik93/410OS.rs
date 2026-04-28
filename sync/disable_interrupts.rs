//! A guard for disabled interrupts.
//!
//! While the concept of disabling interrupts
//! was central to the original C implementation,
//! this file is new to the Rust port.

use core::mem::ManuallyDrop;

use crate::thread::getCurrentThread;

static mut noThreadRefCount: u32 = 0;

/// Use to mark that we have entered a section where interrupst were already disabled
/// by some external mechanism.
///
/// This function is extremely unsafe.
/// Callers must be especially careful to properly handle this in functions that do not return.
pub unsafe fn enteredDisabledInterrupts() -> DisabledInterruptsEntry {
    match getCurrentThread() {
        None => unsafe {
            noThreadRefCount += 1;
        },
        Some(thread) => {
            thread.disabledInterruptsRefCount.update(|i| i + 1);
        }
    }

    DisabledInterruptsEntry(())
}

/// Calling this function while interrupts are already disabled through a separate
/// mechanism makes it undefined whether interrupts are currently disabled or not.
/// Users of DisabledInterruptsGuard must not not rely soley on this for
/// memory safety.
pub fn disableInterrupts() -> DisabledInterruptsGuard {
    unsafe { _410kern::asm::disable_interrupts(); }

    DisabledInterruptsGuard(unsafe { ManuallyDrop::new(enteredDisabledInterrupts()) })
}

#[derive(Debug)]
pub struct DisabledInterruptsEntry(());

#[derive(Debug)]
pub struct DisabledInterruptsGuard(ManuallyDrop<DisabledInterruptsEntry>);

impl DisabledInterruptsEntry {
    /// Converts the assumption of disabled interrupts into a guard that will re-enable interrupts
    /// when dropped.
    ///
    /// This function can be extremely dangerous if re-enabling interrupts is not what you want.
    /// That being said, in all known use cases in this project this is the desired behavior.
    pub unsafe fn into_guard(self) -> DisabledInterruptsGuard {
        DisabledInterruptsGuard(ManuallyDrop::new(self))
    }

    unsafe fn release(&mut self) -> u32 {
        match getCurrentThread() {
            None => unsafe {
                noThreadRefCount -= 1;
                noThreadRefCount
            },
            Some(thread) => {
                let i = thread.disabledInterruptsRefCount.get() - 1;
                thread.disabledInterruptsRefCount.set(i);
                i
            }
        }
    }
}

impl Drop for DisabledInterruptsEntry {
    fn drop(&mut self) {
        unsafe { self.release(); }
    }
}

impl Drop for DisabledInterruptsGuard {
    fn drop(&mut self) {
        let refCount = unsafe { self.0.release() };

        if refCount == 0 {
            unsafe { _410kern::asm::enable_interrupts(); }
        }
    }
}
