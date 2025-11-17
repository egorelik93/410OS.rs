//! A guard for disabled interrupts.
//!
//! While the concept of disabling interrupts
//! was central to the original C implementation,
//! this file is new to the Rust port.

use crate::thread::getCurrentThread;

static mut noThreadRefCount: u32 = 0;

/// Calling this function while interrupts are already disabled through a separate
/// mechanism makes it undefined whether interrupts are currently disabled or not.
/// Users of DisabledInterruptsGuard must not not rely soley on this for
/// memory safety.
pub fn disableInterrupts() -> DisabledInterruptsGuard {
    match getCurrentThread() {
        None => unsafe {
            noThreadRefCount += 1;
        }
        Some(thread) => {
            thread.disabledInterruptsRefCount.update(|i| i + 1);
        }
    }

    DisabledInterruptsGuard()
}

#[derive(Debug)]
pub struct DisabledInterruptsGuard();

impl Drop for DisabledInterruptsGuard {
    fn drop(&mut self) {
        let refCount = match getCurrentThread() {
            None => unsafe {
                noThreadRefCount -= 1;
                noThreadRefCount
            },
            Some(thread) => {
                let i = thread.disabledInterruptsRefCount.get() - 1;
                thread.disabledInterruptsRefCount.set(i);
                i
            }
        };

        if refCount == 0 {

        }
    }
}
