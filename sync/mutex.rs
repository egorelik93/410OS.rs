//! Implementation of mutexes
//!
//! This implementation is primarly based around
//! requesters for a mutex lock being added to a queue,
//! and spin-waiting on a flag for whether they
//! have the lock until they can continue.
//! The queue itself uses xchg to guarantee
//! atomicity.

use core::mem::ManuallyDrop;
use core::ops::{Deref, DerefMut};
use core::pin::pin;
use core::ptr::{self, NonNull};
use core::sync::atomic::{AtomicBool, Ordering};

use crate::lprintf;
use crate::variable_queue::*;
use crate::thread::*;

use super::owned_lock::{self, OwnedLock, OwnedLockGuard};

#[derive(Debug)]
pub struct WaitListNode {
    link: Link<WaitListNode>,
    hasLock: AtomicBool,
    thread: Option<NonNull<ThreadBlock>>
}

pub type MutexWaitList = Head<WaitListNode>;

/// Mutex structure
///
/// Contains:
/// waitlistLock: a lock on the waitlist to acquire the mutex.
/// mutexLock: The main lock to try to acquire.
/// waitlist: the waitlist to acquire the mutex.
#[derive(Debug)]
pub struct Mutex<T> {
    waitList: OwnedLock<MutexWaitList>,
    mutexLock: OwnedLock<T>
}

unsafe impl<T> Sync for Mutex<T> where T: Send {}

impl<T> Mutex<T> {
    /// Create a mutex.
    pub const fn new(data: T) -> Self {
        Mutex {
            waitList: OwnedLock::new(Head::new()),
            mutexLock: OwnedLock::new(data),
        }
    }

    pub const fn get_mut(&mut self) -> &mut T {
        self.mutexLock.get_mut()
    }
}

impl<T> Drop for Mutex<T> {
    /// Destroys the mutex.
    ///
    /// Since our mutex type has nothing to deallocate,
    /// it only empties it and conveniently
    /// checks that no one is waiting
    /// so that an illegal operation can be noticed quickly.
    fn drop(&mut self) {
        if !self.waitList.get_mut().front().is_none() {
            lprintf!("ILLEGAL:
                Attempt to destroy mutex while being waited for: {}.\n",
                self
            );
        }

        *self.waitList.get_mut() = Head::new();
    }
}

impl<T> Mutex<T> {
    /// Wait until the calling thread owns the mutex.
    ///
    /// Our implementation uses a queue called
    /// the waitlist to track requesters of the lock.
    /// Be cause everyone who is waiting
    /// must currently be within mutex_lock,
    /// the waitlist is in fact formed from
    /// nodes on the thread-local stack,
    /// so we do not have to allocate heap memory.
    ///
    /// For efficiency, if no one is waiting for the lock
    /// and it is currently unlocked, the next requester
    /// may immediately take the lock without waiting.
    ///
    /// A waiter can receive the lock in one of two
    /// ways. The first is that whoever previously
    /// held the lock and unlocked it may hand the lock off.
    /// This previous holder is then responsible
    /// for removing the receiver off the waitlist
    /// and signaling through the receiver's hasLock flag
    /// that they may proceed. The second is that
    /// if somehow the mutex is fully unlocked without
    /// being handed off, a waiter may steal the lock.
    /// This is possible if the previous holder
    /// frees while a new wave of requesters
    /// joins the waitlist.
    ///
    /// We claim that a requesting thread cannot go through
    /// more than 2 xchg races in a row without
    /// being added to the waitlist, where it must then wait
    /// its turn.
    ///
    /// This lock is not re-entrant; if this thread already owns the lock
    /// this function will deadlock.
    pub fn lock(&self) -> MutexGuard<T> {
        let thisThread = getCurrentThread();

        // Initialize the Waiter information
        let thisThreadWaitInfo = WaitListNode {
            hasLock: AtomicBool::new(false),
            link: Link::new(),
            thread: thisThread
        };

        // Update Waitlist

        let mut waitList = self.waitList.waitForLock();

        // If the waitlist is empty and the mutex is unlocked,
        // no need to wait; you get the lock!
        if waitList.tail().is_none() && let Ok(guard) = self.mutexLock.tryLock() {
            // Release access to the waitlist
            drop(waitList);

            MutexGuard(self, ManuallyDrop::new(guard))
        } else {
            // Otherwise, you need to to register yourself on the waitlist
            // and wait.

            let thisThreadWaitInfo = pin!(thisThreadWaitInfo);

            let thisThreadWaitInfo = unsafe {
                // Register yourself on the waitlist.
                insert_tail!(&mut waitList, thisThreadWaitInfo.as_ref(), link)
            };

            // Release access to the waitlist
            drop(waitList);


            // Wait for lock.

            // If the lock was freed while you were registering,
            // you have a chance to steal the lock.
            let mut mutexResult = self.mutexLock.tryLock();

            // Wait to receive or steal the main lock
            let guard = loop {
                match mutexResult {
                    Ok(guard) => break guard,
                    Err(mutexHolder) => {
                        yieldThread(mutexHolder);

                        // Attempt to steal the lock.
                        mutexResult = self.mutexLock.tryLock();
                    }
                }
            };

            // Lock received, cleanup.

            // If you stole the lock, you have to remove yourself
            // from the waitlist.
            if !thisThreadWaitInfo.hasLock.load(Ordering::Acquire) {
                let mut waitList = self.waitList.waitForLock();

                remove!(&mut waitList, thisThreadWaitInfo, link);

                // Release access to the waitlist
                drop(waitList);
            }

            // Otherwise, whoever unlocked us was responsible for removing us
            // from the waitlist, so we're ready to go!

            drop(thisThreadWaitInfo);

            MutexGuard(self, ManuallyDrop::new(guard))
        }
    }

    pub fn tryLock(&self) -> Option<MutexGuard<T>> {
        if let Ok(waitList) = self.waitList.tryLock()
            && waitList.tail().is_none()
            && let Ok(guard) = self.mutexLock.tryLock()
        {
            drop(waitList);
            Some(MutexGuard(self, ManuallyDrop::new(guard)))
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct MutexGuard<'a, T>(&'a Mutex<T>, ManuallyDrop<OwnedLockGuard<'a, T>>);

impl<'a, T> MutexGuard<'a, T> {
    pub(super) fn mutex(&self) -> &'a Mutex<T> {
        self.0
    }
}

impl<T> Deref for MutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &*self.1
    }
}

impl<T> DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.1
    }
}

impl<T> Drop for MutexGuard<'_, T> {
    /// Make the mutex available to other threads.
    ///
    /// If there is anyone in the waitlist, the unlocker's job
    /// is to choose the first waiter and hand them the lock
    /// immediately, both removing them from the
    /// waitlist and signaling through hasLock.
    ///
    /// If no one is on the waitlist, the mutex status is set to
    /// unlocked so the next requester can take it.
    fn drop(&mut self) {
        let mut waitList = self.0.waitList.waitForLock();

        // If the waitlist is empty, we indicate the mutex is now unlocked
        // without passing it on to anyone.
        match waitList.front_ptr() {
            None => {
                // Since the mutex is locked, no one is removing themself off the waitlist
                // until the moment we've unlocked the status, at which point we're done.
                // Anyone who tries to wait for the waitlist will add themselves onto it.
                // Ideally that means that if we are here,
                // there was no contention on the waitlist.
                unsafe { ManuallyDrop::drop(&mut self.1) };
            },
            Some(nextRunner) => {
                // Otherwise, we pass the lock on to someone else.
                // For now, that is the first thread in the list.

                // Update Waitlist

                let nextRunner = unsafe { &*nextRunner };
                remove!(&mut waitList, nextRunner, link);

                // Release access to the waitlist. Since the next runner
                // is off the waitlist, no one else can affect it.
                drop(waitList);

                // Triggers to the next runner they are ready to go.
                unsafe { ManuallyDrop::take(&mut self.1).transferLockTo(nextRunner.thread.unwrap()) };
                nextRunner.hasLock.store(true, Ordering::Release);
            }
        }
    }
}
