//! Timer Configuration and Handling.
//!
//!  This file contains functions for initializing
//!  and communicating with the timer.

use core::ffi::{c_int, c_uint};
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use _410kern::asm::outb;
use _410kern::interrupt_defines::*;
use _410kern::seg::SEGSEL_KERNEL_CS;
use _410kern::timer_defines::*;

use crate::syscall_int::*;
use crate::byte_utils::{LSB, MSB};
use crate::idt_entry::*;
use crate::sync::disable_interrupts::{disableInterrupts, enteredDisabledInterrupts};
use crate::sync::mutex::Mutex;
use crate::thread::{ThreadHandle, descheduleThread, getCurrentThread, scheduleThread, yieldThread, yieldThreadWithoutInterrupts};
use crate::variable_queue::{Head, Link};


/// Milliseconds per Second.
const MS_PER_S: u16 = 1000;

/// Milliseconds to pass before each timer interrupt.
const MS_PER_INTERRUPT: u16 = 10;

#[repr(C)]
#[derive(Debug)]
struct SleepNode {
    thread: ThreadHandle,
    sleepTo: u32,
    descheduleFlag: AtomicBool,
    link: Link<SleepNode>
}

type SleepQueue = Head<SleepNode>;


#[repr(C)]
#[derive(Debug)]
struct SleepCollection {
    queue: Mutex<SleepQueue>
}

/* Timer State */

/// Number of interrupts since the timer started.
static tickSinceStart: AtomicU32 = AtomicU32::new(0);

static sleepColl: SleepCollection = SleepCollection { queue: Mutex::new(Head::new()) };


/// Interrupt handler for the timer.
///
/// The initial handler that will be called during
/// a timer interrupt (via timerHandlerWrapper()).
/// This will wake any threads who are currently
/// sleeping and context switch.
///
/// Waking threads works similarly to a cond var,
/// but instead of locking
/// until we can send a signal, we simply don't
/// do anything until the next tick if the sleep queue
/// is currently locked.
pub unsafe fn timerHandler() {
    let disabledInterrupts = unsafe { enteredDisabledInterrupts().into_guard() };

    let time = tickSinceStart.fetch_add(1, Ordering::Relaxed) + 1;

    unsafe { outb(INT_CTL_PORT, INT_ACK_CURRENT); }

    let thread = getCurrentThread().unwrap();

    if let Some(mut queue) = sleepColl.queue.tryLock() {
        for curr in unsafe { queue.iter_unchecked(|s| &s.link) } {
            if time < curr.sleepTo {
                break;
            }

            curr.descheduleFlag.store(true, Ordering::Release);
            remove!(queue, curr, link);

            scheduleThread(&disabledInterrupts, &thread.handle());
        }
    }

    yieldThreadWithoutInterrupts(&disabledInterrupts, None);
}


/// Configure the interval interrupts are sent.
///
/// Sets the timer to send an interrupt
/// every ms milliseconds.
/// This is done by converting this period in milliseconds
/// to per second frequency.
pub fn setTimerInterruptPeriod(ms: u16) {
    let cycles = ((TIMER_RATE as u64) * (ms as u64) / (MS_PER_S as u64)) as u16;

    unsafe {
        outb(TIMER_MODE_IO_PORT, TIMER_SQUARE_WAVE);
        outb(TIMER_PERIOD_IO_PORT, LSB(cycles));
        outb(TIMER_PERIOD_IO_PORT, MSB(cycles))
    }
}

/* Syscalls */

/// Gets the time since the system started.
pub fn get_ticks() -> c_uint {
    tickSinceStart.load(Ordering::Relaxed)
}

/// Puts the current thread to sleep.
///
/// The current thread will be descheduled
/// for at least ticks interrupts from the timer.
pub fn sleep(ticks: c_int) -> c_int {
    if ticks == 0 { return 0; }
    if ticks < 0 { return -1; }

    let node = &SleepNode {
        thread: getCurrentThread().unwrap().handle(),
        sleepTo: get_ticks() + (ticks as c_uint),
        descheduleFlag: AtomicBool::new(false),
        link: Link::new()
    };

    let mut queue = sleepColl.queue.lock();

    'inserted: {
        for curr in unsafe { queue.iter_unchecked(|s| &s.link) } {
            if node.sleepTo <= curr.sleepTo {
                unsafe { insert_before!(queue, curr, node, link) };
                break 'inserted;
            }
        }

        unsafe { insert_tail!(queue, node, link) };
    }

    drop(queue);

    while get_ticks() < node.sleepTo {
        let disabledInterrupts = disableInterrupts();
        descheduleThread(&disabledInterrupts, &getCurrentThread().unwrap());
        yieldThread(None);
    }

    0
}



/// Sets up the timer..
///
/// Installs the user's timer handler and
/// sets up the interrupt table
/// and the timer device.
pub unsafe fn installTimerDriver() {
    unsafe {
        *IDT().add(TIMER_IDT_ENTRY) = interruptGate(
            HARDWARE_PRIVILEGE,
            super::timer_handler_wrapper::timerHandlerWrapper,
            SEGSEL_KERNEL_CS);

        *IDT().add(GET_TICKS_INT) = trapGate(
            USER_PRIVILEGE,
            crate::syscall::getTicksHandlerWrapper,
            SEGSEL_KERNEL_CS);

        *IDT().add(SLEEP_INT) = trapGate(
            USER_PRIVILEGE,
            crate::syscall::sleepHandlerWrapper,
            SEGSEL_KERNEL_CS);
    }

    setTimerInterruptPeriod(MS_PER_INTERRUPT);
}
