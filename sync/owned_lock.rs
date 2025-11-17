//! A lightweight locking mechanism that tracks its owner.
//!
//! This wraps a very small lock based on xchg,
//! which stores which thread owns the lock.

use core::cell::{Cell, UnsafeCell};
use core::mem;
use core::ops::{Deref, DerefMut};
use core::ptr::{NonNull, null_mut};
use core::sync::atomic::{AtomicBool, AtomicPtr, Ordering};

use crate::lprintf;
use crate::thread::*;

pub use crate::thread::{ThreadBlock, ThreadHandle};

use super::disable_interrupts::disableInterrupts;

pub type LockedStatus = bool;

pub const LOCKED: LockedStatus = true;
pub const UNLOCKED: LockedStatus = false;

/// An owned lock contains a lock
/// flag and the current owner.
#[derive(Debug)]
pub struct OwnedLock<T> {
    status: AtomicBool,
    owner: AtomicPtr<ThreadBlock>,
    guardCreated: Cell<bool>,
    data: UnsafeCell<T>
}

unsafe impl<T> Sync for OwnedLock<T> where T: Send {}

/// Attempt to lock a flag.
///
/// Atomically sets the provided flag to LOCKED, attempting
/// to do so while it is set to UNLOCKED, thus "obtaining the lock."
#[inline(always)]
fn try_lock(flag: &AtomicBool) -> bool {
    flag.swap(LOCKED, Ordering::AcqRel) == LOCKED
}

/// Atomically unlock a flag.
#[inline(always)]
fn unlock(flag: &AtomicBool) {
    flag.store(UNLOCKED, Ordering::Release);
}

impl<T> OwnedLock<T> {
    /// Create an owned lock
    pub const fn new(data: T) -> OwnedLock<T> {
        OwnedLock {
            status: AtomicBool::new(UNLOCKED),
            owner: AtomicPtr::new(null_mut()),
            guardCreated: Cell::new(false),
            data: UnsafeCell::new(data)
        }
    }

    pub const fn get_mut(&mut self) -> &mut T {
        self.data.get_mut()
    }
}

impl<T> Drop for OwnedLock<T> {
    /// Destroy an owned lock
    fn drop(&mut self) {
        self.owner = AtomicPtr::default();
        self.status = AtomicBool::new(LOCKED);
    }
}

impl<T> OwnedLock<T> {
    /// Attempt to lock an OwnedLock object
    pub fn tryLock(&self) -> Result<OwnedLockGuard<T>, Option<ThreadHandle>> {
        if !try_lock(&self.status) {
            self.owner.store(getCurrentThread().map_or(null_mut(), |p: &ThreadBlock| p.handle().get()), Ordering::Release);
        }

        let owner = ThreadHandle::new(self.owner.load(Ordering::Acquire));
        if owner == getCurrentThread().handle() && !self.guardCreated.get() {
            self.guardCreated.set(true);
            Ok(OwnedLockGuard(self))
        } else {
            Err(owner)
        }
    }

    fn owner(&self) -> Option<ThreadHandle> {
        ThreadHandle::new(self.owner.load(Ordering::Acquire))
    }

    /// Waits until we own the lock.
    ///
    /// The current thread will yield to the owner
    /// of the lock until then.
    ///
    /// This lock is not re-entrant; if this thread already owns the lock
    /// this function will deadlock.
    pub fn waitForLock(&self) -> OwnedLockGuard<T> {
        self.waitForLockWith(|owner| {
            if let None = owner {
                yieldThread(None);
            } else {
                let guard = disableInterrupts();
                let owner = self.owner();
                yieldThreadWithoutInterrupts(&guard, owner);
            }
        })
    }

    /// Loops and calls the wait function until the lock can be obtained.
    ///
    /// This lock is not re-entrant; if this thread already owns the lock
    /// this function will deadlock.
    ///
    /// This function was not part of the original C implemenation,
    /// but assists us with locking the scheduler itself.
    pub fn waitForLockWith<F>(&self, wait: F) -> OwnedLockGuard<T>
    where F: Fn(Option<ThreadHandle>) {
        let thread = getCurrentThread();

        loop {
            match self.tryLock() {
                Ok(guard) => return guard,
                Err(owner) => {
                    if owner == thread {
                        lprintf!("Warning: Guard for lock {} was already created", self);

                        while self.guardCreated.get() {
                            wait(None);
                        }
                    } else {
                        wait(owner);
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct OwnedLockGuard<'a, T>(&'a OwnedLock<T>);

impl<T> OwnedLockGuard<'_, T> {
    /// Transfer an owned lock to another thread
    pub fn transferLockTo(self, thread: NonNull<ThreadBlock>) {
        self.0.owner.store(thread.as_ptr(), Ordering::Release);
        mem::forget(self);
    }
}

impl<T> Deref for OwnedLockGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe {
            &*self.0.data.get()
        }
    }
}

impl<T> DerefMut for OwnedLockGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe {
            &mut *self.0.data.get()
        }
    }
}

impl<T> Drop for OwnedLockGuard<'_, T> {
    /// Unlock an OwnedLock
    fn drop(&mut self) {
        self.0.owner.store(null_mut(), Ordering::Release);
        unlock(&self.0.status);
    }
}
