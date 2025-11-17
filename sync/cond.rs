//! Implementation of cond vars.
//!
//! Our implementation uses a queue of listeners
//! to track who is awaiting a signal,
//! to select a receiver for the signal,
//! and to notify them of the signal.
//! This queue is protected by a mutex.
//!
//! We achieve atomicity by giving each listener
//! a flag for whether they should be descheduled,
//! and having them deschedule themselves until the signaler
//! updates the flag and wakes them up,
//! which will stop them from descheduling further.

use core::pin::pin;
use core::ptr::NonNull;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::sync::disable_interrupts::disableInterrupts;
use crate::thread::{ThreadBlock, ThreadHandle, blockUntil, getCurrentThread, scheduleThread};
use crate::lprintf;
use crate::variable_queue::*;

use super::mutex::{Mutex, MutexGuard};

// Follows the spec for the blockUntil function.
pub const TRY_TO_DESCHEDULE: bool = false;
pub const DO_NOT_DESCHEDULE: bool = true;


/// Set a flag to DO_NOT_DESCHEDULE.
#[inline(always)]
fn send_signal(flag: &AtomicBool) -> bool {
    flag.swap(DO_NOT_DESCHEDULE, Ordering::AcqRel)
}

/// Set a flag to TRY_TO_DESCHEDULE.
///
/// Atomically sets the provided flag to TRY_TO_DESCHEDULE,
/// determining whether anyone had already set it to DO_NOT_DESCHEDULE.
#[inline(always)]
fn do_deschedule(flag: &AtomicBool) -> bool {
    flag.swap(TRY_TO_DESCHEDULE, Ordering::AcqRel)
}


/// A node in the condition listener queue.
///
/// Contains:
///   link: Queue link to use in variable queue macros.
///   tid:  Kernel-level thread id for the listening thread.
///   doNotDeschedule: Value whose pointer will be passed to deschedule
///                 for whether to deschedule or not.
///                 Follows that spec.
#[derive(Debug)]
pub struct QueueNode {
    link: Link<QueueNode>,
    thread: Option<ThreadHandle>,
    doNotDeschedule: AtomicBool
}

pub type CondQueue = Head<QueueNode>;


/// Condition variable structure
///
/// Contains:
/// queueMutex: A mutex on the wait queue
/// queue: The wait queue
#[derive(Debug)]
pub struct Cond {
    queue: Mutex<CondQueue>
}


impl Cond {
    /// Create a cond var.
    pub const fn new() -> Cond {
        Cond {
            queue: Mutex::new(Head::new())
        }
    }
}

impl Drop for Cond {
    /// Destroys the cond var.
    ///
    /// Our cond type checks that no one is waiting
    /// so that an illegal operation can be noticed quickly.
    fn drop(&mut self) {
        if self.queue.get_mut().front().is_some() {
            lprintf!("ILLEGAL:
              Attempt to destroy cond var while being listened to {}\n",
              self);
        }

        *self.queue.get_mut() = Head::new();
    }
}

impl Cond {
    /// Wait until our cond var been signaled.
    ///
    /// Our implementation uses a queue to track
    /// who is waiting on a condition.
    /// As with the mutex implementation,
    /// we take advantage of all waiters being in this
    /// function to store the queue nodes on the local
    /// thread stack, avoiding reliance on malloc.
    ///
    /// The majority of the logic here is simply adding
    /// oneself to the listener queue, which is protected
    /// by a second mutex.
    /// The only interesting code is how
    /// we guarantee atomicity between the time the
    /// queue is unlocked and the time and thread
    /// is descheduled by requiring the waiter
    /// to deschedule based on a flag that the
    /// eventual signaler will have to set atomically.
    /// This flag determines whether thread should
    /// not deschedule itself or stop descheduling itself,
    /// depending on when the signal was sent compared
    /// to when the queue was unlocked.
    pub fn waitForCond<T>(&self, guard: MutexGuard<T>) -> MutexGuard<T> {
        // Initialize the Queue information
        let thisThreadWaitInfo = QueueNode {
            doNotDeschedule: AtomicBool::new(TRY_TO_DESCHEDULE),
            link: Link::new(),
            thread: getCurrentThread()
        };

        let thisThreadWaitInfo = pin!(thisThreadWaitInfo);

        // Update Queue

        // Wait to obtain access to the queue
        let mut queue = self.queue.lock();

        // Register yourself on the queue.
        let thisThreadWaitInfo = unsafe { insert_tail!(&mut queue, thisThreadWaitInfo.as_ref(), link) };

        // Unlock the user mutex.
        let mutex = guard.mutex();
        drop(guard);

        // To give the appearance of atomicity, we will deschedule
        // the current thread only if the signal about
        // whether to deschedule was sent,
        // as determined by doNotDeschedule.
        // Wheoever sends us the signal is expected to
        // try to reschedule us, and if it turns out
        // we were never descheduled, set doNotDeschedule
        // so that we know not to try to deschedule ourselves
        // when we receive control again.


        // Unlocking must occur atomically with descheduling,
        // but before we are switched out.
        // We can't use locks because then they have the same
        // problem of unlocking before switching out.
        // Thus, we rely on descheduleThread disabling
        // interrupts in this instance until context switch.
        let disabledInterrupts = disableInterrupts();
        blockUntil(&disabledInterrupts, &thisThreadWaitInfo.doNotDeschedule);

        // At this point, we've been rescheduled because
        // a signal was sent. The signaler
        // was responsible for removing us from the queue,
        // so we can proceed.

        mutex.lock()
    }

    /// Signal and remove the given node from the queue.
    ///
    /// This is the underlying critical section code
    /// for sending a signal to a listener,
    /// responsible for removing them from the greater
    /// queue,
    /// setting their flag to give them permission to run,
    /// and making them runnable.
    ///
    /// This may only be called when you have
    /// the waitlist locked.
    fn signalAndRemoveQueueNode(queue: &mut CondQueue, node: &QueueNode) {
        // First, we need to remove the node from the queue.

        remove!(queue, node, link);

        /// Now we can try to reschedule the node's thread.
        /// If make_runnable returns a negative value,
        /// then the thread never got a chance to deschedule itself,
        /// so we need to send the signal manually.
        /// Actually, at this point setting doNotDeschedule
        /// doesn't do any harm anyway, so we send the signal
        /// in all cases.
        /// If it runs before we send the signal, then it will
        /// deschedule itself and not run until we call
        /// make_runnable.
        /// If the thread begins running after we send the
        /// signal,
        /// that's ok, because clearly it wasn't descheduled yet
        /// and sending the signal means it won't deschedule,
        /// but with the signal sent it's fine for it to proceed.
        /// Our call to make_runnable will simply do nothing.
        send_signal(&node.doNotDeschedule);

        let thread = node.thread?;
        scheduleThread(disableInterrupts(), thread);
    }

    /// Allow some thread on the queue to run.
    ///
    /// If the queue is empty, the signal is ignored.
    /// Otherwise, the first listener in the queue will
    /// be sent a signal and removed.
    ///
    /// Wraps the critical section of signalAndRemoveQueueNode
    /// with locks.
    pub fn signalCond(&self) {
        let queue = self.queue.lock();

        if let Some(front) = queue.front() {
            Cond::signalAndRemoveQueueNode(queue, front);
        }
    }

    /// Allow all threads on the queue to run.
    ///
    /// Locks the queue and sends everyone a signal.
    pub fn broadcastCond(&self) {
        let queue = self.queue.lock();

        for curr in queue.iter(|n| n.link) {
            Cond::signalAndRemoveQueueNode(queue, curr);
        }
    }
}
