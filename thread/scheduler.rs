//! Scheduler Implementation
//!
//! Because the timer interrupt handler needs
//! access to the scheduler.
//! the scheduler itself
//! cannot use mutexes when trying to
//! obtain the next thread.
//! We thus use disable_interrupts
//! to prevent the timer from running.

use core::ptr::NonNull;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::sync::disable_interrupts::{self, DisabledInterruptsGuard, disableInterrupts};
use crate::sync::owned_lock::{OwnedLock, OwnedLockGuard};
use crate::sync::mutex::Mutex;
use crate::variable_queue::Head;

use super::context_switch::yieldThreadWithoutInterrupts;
use super::{ThreadBlock, ThreadHandle, getCurrentThread};
use super::thread_internal::getActiveThreadByTid;

type ScheduledThreads = Head<ThreadBlock>;


/// Holds scheduling data, and a lock for synchronization
struct Schedule(OwnedLock<ScheduleInner>);

struct ScheduleInner {
    next: Option<ThreadHandle>,
    queue: ScheduledThreads
}

static sched: Schedule = Schedule::new();


impl Schedule {
    /// Create a schedule.
    const fn new() -> Schedule {
        Schedule(Mutex::new(ScheduleInner {
            next: None,
            queue: Head::new()
        }))
    }
}

fn getSchedule(_: &DisabledInterruptsGuard) -> OwnedLockGuard<ScheduleInner> {
    sched.0.waitForLockWith(|t| {})
}

/// Move forward to the next thread in the schedule.
///
/// This function only serves to retrieve the next thread
/// and update the scheduler; it does NOT context switch.
///
/// Should only be run while interrupts are disabled.
pub fn getNextThread(disabledInterrupts : &DisabledInterruptsGuard) -> Option<ThreadHandle> {
    let sched_ = getSchedule(disabledInterrupts);

    let curr = sched_.next;
    let Some(curr) = curr
    else {
        sched_.next = sched_.queue.front().map(|t| t.handle());
        return None;
    };

    let next = curr.scheduleLink.next();
    match next {
        None => { sched_.next = sched_.queue.front().map(|t| t.handle()); },
        Some(next) => { sched_.next = Some(next); }
    };

    Some(curr)
}

/// Add a thread to the schedule.
///
/// This will become the next scheduled thread
/// to run.
pub fn scheduleThread(disabledInterrupts : &DisabledInterruptsGuard, thread: &ThreadHandle) -> Result<(), ()> {
    let sched_ = getSchedule(&disableInterrupts());

    if !thread.scheduled.swap(true, Ordering::AcqRel) {
        match sched_.next {
            None => {
                unsafe {
                    let thread = insert_tail!(&mut sched_.queue, thread.deref_pin(), scheduleLink);
                }
                sched_.next = Some(thread.handle());
            },
            Some(next) => {
                unsafe {
                    insert_after!(&mut sched_.queue, &next, thread.deref_pin(), scheduleLink);
                }
            }
        }

        Ok(())
    } else { Err(()) }
}

/// Remove a thread from the schedule.
///
/// From the old C implementation, no longer relevant:
/// If in kernel_main or descheduling yourself,
/// this leaves interrupts disabled, regardless
/// of whether it succeeded or not!!
/// This is so that we can reliably
/// free resources and trigger a
/// context switch immediately afterwards,
/// which reenables interrupts.
/// Otherwise, we could
/// have redundant context switches following
/// descheduling.
pub fn descheduleThread(disabledInterrupts: &DisabledInterruptsGuard, thread: &ThreadHandle) -> Result<(), ()> {
    if thread.scheduled.swap(false, Ordering::AcqRel) {
        let sched_ = getSchedule(disabledInterrupts);
        remove!(&mut sched_.queue, &thread, scheduleLink);

        if Some(thread) == sched_.next {
            getNextThread(&disabledInterrupts);
        }

        Ok(())
    } else {
        Err(())
    }
}

/// Obtain a scheduled thread block corresponding to a tid.
pub fn getScheduledThreadByTid(tid: i32) -> Option<ThreadHandle> {
    let disabledInterrupts = disableInterrupts();

    for curr in getSchedule(disabledInterrupts).queue.iter(|t| t.scheduleLink) {
        if curr.tid == tid {
            if curr.scheduled.get() {
                Some(curr.handle())
            } else {
                None
            }
        }
    }
}

/// Blocks the thread until a condition is met.
///
/// If given NULL, will only deschedule until being rescheduled.
///
/// This function will only ever return with interrupts enabled.
pub fn blockUntil(disabledInterrupts: &DisabledInterruptsGuard, cond: &AtomicBool) {
    let thread = getCurrentThread()?;

    while !cond.load(Ordering::Release) {
        descheduleThread(&disabledInterrupts, thread);
        yieldThreadWithoutInterrupts(&disabledInterrupts, None)
    }
}


// Syscalls


/// Schedule a thread.
///
/// # Parameters
/// 1. tid: Tid of the thread to schedule.
///        Must have been descheduled by deschedule().
///
/// # Returns
///
/// 0 if successfully scheduled,
/// -1 otherwise.
fn make_runnable(tid: i32) -> i32 {
    if tid < 0 {
        return -1;
    }

    let Some(thread) = getActiveThreadByTid(tid)
    else { return -1; };
}
