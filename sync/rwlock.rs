//! Implementation of reader/writer locks.
//!
//! We rely on underlying mutexes and cond vars,
//! and simply keep track of the current mode and
//! how many are reading and how many are waiting for writes.
//! This code primarily just implements the mode switching.

use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};

use super::cond::Cond;
use super::mutex::Mutex;
use super::owned_lock::OwnedLock;

#[derive(Debug, PartialEq, Eq)]
enum RWLockMode {
    Read = 0,
    Write = 1
}

/// Structure for a readers-writers lock
///
/// Contains:
/// statusMutex: A mutex on the rwlock itself
/// canWrite: Cond var that a writer waits on until it can write
/// canRead: Cond var that readers wait on before they can read
/// readerCount: Number of readers currently reading
/// writerWaitlistSize: Number of writers currently waiting
/// mode: Whether we are in RWLOCK_READ or RWLOCK_WRITE mode. (See rwlock.h)
#[derive(Debug)]
pub struct RWLock<T> {
    status: Mutex<RWLockStatus>,
    canWrite: Cond,
    canRead: Cond,
    data: UnsafeCell<T>
}

#[derive(Debug)]
struct RWLockStatus {
    readerCount: u32,
    writerWaitlistSize: u32,
    mode: RWLockMode
}


impl<T> RWLock<T> {
    /// Initializes a rwlock.
    pub const fn new(data: T) -> RWLock<T> {
        RWLock {
            status: Mutex::new(RWLockStatus {
                readerCount: 0,
                writerWaitlistSize: 0,
                mode: RWLockMode::Read
            }),
            canWrite: Cond::new(),
            canRead: Cond::new(),
            data: Unsafe::new(data)
        }
    }
}

impl<T> Drop for RWLock<T> {
    /// Destroys the rwlock.
    ///
    /// If anyone is still waiting on the rwlock,
    /// an illegal operation,
    /// the cond_destroys will trigger an error
    /// and terminate the program.
    fn drop(&mut self) {
        let status = self.status.get_mut();
        status.writerWaitlistSize = 0;
        status.readerCount = 0;
        status.mode = RWLockMode::Write;
    }
}

impl<T> RWLock<T> {
    /// Wait for read access to the rwlock.
    pub fn lockRead(&self) -> ReadGuard<T> {
        let mut status = self.status.lock();

        // If anyone currently has write access,
        // or in accordance with the spec
        // anyone is waiting for write access,
        // we must wait until they have obtained
        // access before we get to read.
        while status.mode == RWLockMode::Write || status.writerWaitlistSize > 0 {
            status = self.canRead.waitForCond(status)
        }

        // If we are ready to read, we notify that there
        // is an additional reader and set the mode to READ.
        status.readerCount += 1;
        status.mode = RWLockMode::Read;

        ReadGuard(self, unsafe { &*self.data.get() })
    }

    /// Wait for write access to the rwlock.
    pub fn lockWrite(&self) -> WriteGuard<T> {
        let mut status = self.status.lock();

        // If we wish to get write access,
        // and either someone already has it or
        // people are reading,
        // we must join a queue of those waiting
        // for write access
        status.writerWaitlistSize += 1;

        while status.mode == RWLockMode::Write || status.readerCount > 0 {
            status = self.canWrite.waitForCond(status);
        }

        // Once we get write access, we are no longer on the waitlist,
        // and the mutex is in WRITE mode.
        status.writerWaitlistSize -= 1;
        status.mode = RWLockMode::Write;

        WriteGuard(self, unsafe { &mut *self.data.get() })
    }
}

#[derive(Debug)]
pub struct ReadGuard<'a, T>(&'a RWLock<T>, &'a T);

#[derive(Debug)]
pub struct WriteGuard<'a, T>(&'a RWLock<T>, &'a mut T);


impl<T> Deref for ReadGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.1
    }
}

impl<T> Deref for WriteGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.1
    }
}

impl<T> DerefMut for WriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.1
    }
}

impl<T> Drop for ReadGuard<'_, T> {
    /// Unlock access to the rwlock.
    fn drop(&mut self) {
        let mut status = self.0.status.lock();

        // If no longer reading, one less person is reading.
        status.readerCount -= 1;

        // If no one else is reading, someone may write.
        if status.readerCount == 0 {
            self.0.canWrite.signalCond();
        }
    }
}

impl<T> Drop for WriteGuard<'_, T> {
    /// Unlock access to the rwlock.
    fn drop(&mut self) {
        let mut status = self.0.status.lock();

        // If no longer writing,
        // we are temporarily in read mode.
        // This is ok, since we still have the status locked.
        status.mode = RWLockMode::Read;

        // If anyone is waiting for write access, pass off to them,
        // otherwise everyone waiting for read access can.
        if status.writerWaitlistSize > 0 {
            self.0.canWrite.signalCond();
        } else {
            self.0.canRead.broadcastCond();
        }
    }
}

impl<'a, T> WriteGuard<'a, T> {
    /// Downgrades access to read from write.
    ///
    /// This simply switches the state between modes
    /// without releasing the lock.
    pub fn downgradeRWLock(self) -> ReadGuard<'a, T> {
        let mut status = self.0.status.lock();

        status.readerCount += 1;
        status.mode = RWLockMode::Read;

        ReadGuard(self.0, &self.1)
    }
}
