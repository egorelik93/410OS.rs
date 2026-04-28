//! Keyboard API and Handler.

use core::cell::Cell;
use core::clone::CloneToUninit;
use core::ffi::c_char;
use core::ffi::c_int;
use core::pin::pin;
use core::ptr;
use core::ptr::null_mut;
use core::sync::atomic::AtomicBool;
use core::sync::atomic::AtomicPtr;
use core::sync::atomic::{Ordering, AtomicUsize};

use _410kern::asm::*;
use _410kern::keyhelp::*;
use _410kern::interrupt_defines::*;
use _410kern::page::PAGE_SIZE;
use _410kern::seg::SEGSEL_KERNEL_CS;
use alloc::boxed::Box;
use crate::sync::disable_interrupts::enteredDisabledInterrupts;
use crate::syscall_int::*;
use crate::drivers::console::{BACKSPACE, putbyte};
use crate::idt_entry::*;
use crate::registers::Registers;
use crate::sync::cond::Cond;
use crate::sync::disable_interrupts::disableInterrupts;
use crate::sync::mutex::Mutex;
use crate::thread::blockUntil;
use crate::thread::getCurrentThread;
use crate::thread::{ThreadHandle, scheduleThread};
use crate::variable_queue::{Head, Link};
use crate::virtual_memory::LogicalAddress;
use crate::virtual_memory::isUserWritableAddr;

use super::buffer::CharBuffer;
///
/// This file contains functions for
/// communicating with and handling the keyboard.
///
/// For readline, we maintain a queue of waiters,
/// with temporary buffer for each where new input
/// gets stored until we are ready to
/// reschedule them.
/// When for any reason we can't place input
/// in someone's buffer, we have a fixed
/// size circular buffer where extra input
/// gets stored and offloaded when
/// new waiters come in.
/// Input that comes in to the extra buffer
/// will not be echoed until it is moved
/// into someone's temporary buffer.


type Scancode = u8;

struct ReadLineWaitInfo {
    buffer: Box<[Cell<c_char>]>,
    currSize: AtomicUsize,
    nodeReady: AtomicBool,
    inputReady: AtomicBool,
    thread: ThreadHandle,
    link: Link<ReadLineWaitInfo>
}

unsafe impl Send for ReadLineWaitInfo {}

type ReadLineQueue = Head<ReadLineWaitInfo>;

struct ReadLineWaitList {
    queue: Mutex<ReadLineQueue>
}

#[repr(C)]
struct ReadLineArgs {
    len: c_int,
    buf: *mut c_char
}

/* Saved state */

// Locks must be held only with interrupts disabled and for as little as possible.

/// Extra storage for chars when no awaiting request.
static extraBuffer: Mutex<CharBuffer> = Mutex::new(CharBuffer::new());
static readLineWaitList: ReadLineWaitList = ReadLineWaitList { queue: Mutex::new(Head::new()) };
static inputReady: Cond = Cond::new();


/// Insert a character into an info node.
fn insertCharInto(c: c_char, info: Option<&ReadLineWaitInfo>) {
    if info.as_ref().is_none_or(|info| !info.nodeReady.load(Ordering::Acquire)) {
        extraBuffer.tryLock().map(|mut b| b.insertChar(c));
    } else {
        let info = info.unwrap();

        if !info.inputReady.load(Ordering::Acquire) {
            if c == BACKSPACE as c_char {
                let result = info.currSize.try_update(Ordering::Release, Ordering::Relaxed, |i| {
                    if i > 0 { Some(i - 1) } else { None }
                });

                if result.is_ok() {
                    putbyte(c);
                }
            } else {
                let currSize = info.currSize.fetch_add(1, Ordering::AcqRel);
                if let Some(slot) = info.buffer.get(currSize) {
                    slot.set(c);
                    putbyte(c);
                }
            }

            if info.currSize.load(Ordering::Acquire) >= info.buffer.len() || c == b'\n' as c_char {
                info.inputReady.store(true, Ordering::Release);
            }
        } else {
            let nextInfo = info.link.next();
            insertCharInto(c, nextInfo);
        }

        if info.inputReady.load(Ordering::Acquire) {
            scheduleThread(&disableInterrupts(), &info.thread);
        }
    }
}

/// Interrupt handler for the keyboard.
///
/// The handler that will be called
/// during a keyboard interrupt (via keyboardHandlerWrapper()).
/// The primary function of the handler is to store the scancode
#[unsafe(no_mangle)]
pub extern "cdecl" fn keyboardHandler(_: *mut Registers) {
    let scancode = unsafe { inb(KEYBOARD_PORT) };

    unsafe { outb(INT_CTL_PORT, INT_ACK_CURRENT); }

    let key = unsafe { process_scancode(scancode as c_int) };
    if KH_HASDATA(key) != 0 && KH_ISMAKE(key) != 0 {
        let c = KH_GETCHAR(key) as c_char;

        let Some(queue) = readLineWaitList.queue.tryLock() else { return; };
        let info = unsafe { queue.front_unchecked() };
        insertCharInto(c, info);
    }
}


/* Syscalls */

/// Reads a line of input from the terminal.
pub unsafe fn readline(len: c_int, buf: *mut c_char) -> c_int {
    unsafe {
        if !isUserWritableAddr(LogicalAddress(buf.addr()), len as usize) {
            return -1;
        }
    }

    let info = &ReadLineWaitInfo {
        nodeReady: AtomicBool::new(false),
        inputReady: AtomicBool::new(false),
        thread: {
            let Some(thread) = getCurrentThread() else { return -1; };
            thread.handle()
        },
        link: Link::new(),
        buffer: {
            if len as usize > PAGE_SIZE {
                return -1;
            }

            let Ok(buffer) = Box::try_new_uninit_slice(len as usize) else { return -1 };
            unsafe { buffer.assume_init() }
        },
        currSize: AtomicUsize::new(0),
    };

    let disabledInterrupts = disableInterrupts();
    let mut queue = readLineWaitList.queue.lock();

    if queue.front().is_none() {
        unsafe { insert_tail!(&mut queue, info, link); }
        drop(queue);
        drop(disabledInterrupts);

        while info.currSize.load(Ordering::Acquire) < info.buffer.len() {
            let disabledInterrupts = disableInterrupts();
            let mut guard = extraBuffer.lock();

            let c = guard.removeChar();

            match c {
                Some(c) if c as u8 == BACKSPACE => {
                    drop(guard);
                    drop(disabledInterrupts);

                    info.currSize.try_update(Ordering::Release, Ordering::Relaxed, |i| {
                        if i > 0 { Some(i - 1) } else { None }
                    });
                    putbyte(c);
                    continue;
                },
                None => {
                    /* We believe the extra buffer is empty,
                     * so all new characters should by default
                     * redirect to this node's buffer.
                     * We ignore characters added between emptying
                     * the buffer and redirecting input;
                     * otherwise they'd be out of order.
                     */

                    info.nodeReady.store(true, Ordering::Release);
                    *guard = CharBuffer::new();

                    drop(guard);
                    drop(disabledInterrupts);

                    break;
                },
                Some(c) => {
                    let currSize = info.currSize.fetch_add(1, Ordering::AcqRel);
                    if (currSize >= info.buffer.len()) { break; }

                    info.buffer[currSize].set(c);
                        
                    drop(guard);
                    drop(disabledInterrupts);

                    putbyte(c);
                    if c as u8 == b'\n' { break; }
                }
            }
        }

        let currSize = info.currSize.load(Ordering::Acquire);
        if currSize > info.buffer.len() ||
            (currSize > 0 && info.buffer[currSize].get() as u8 == b'\n') {
                /* If this was the first node, we possibly got all of our input
                 * while inserting ourselves on the waitlist.
                 * Check that now, so we can remove ourselves
                 * before adding anyone else to the list.
                 * This means we can redirect input onto the extra
                 * buffer without worrying about order.
                 */

                let disabledInterrupts = disableInterrupts();
                let mut queue = readLineWaitList.queue.lock();

                remove!(&mut queue, info, link);

                drop(queue);
                drop(disabledInterrupts);

                let currSize = info.currSize.load(Ordering::Acquire).min(info.buffer.len());
                unsafe {
                    info.buffer[0..currSize].clone_to_uninit(buf as *mut u8);
                }

                return currSize as i32;
            }

        let disabledInterrupts = disableInterrupts();

        info.nodeReady.store(true, Ordering::Release);

        blockUntil(&disabledInterrupts, &info.inputReady);
    } else {
        unsafe { insert_tail!(&mut queue, info, link); }

        info.nodeReady.store(true, Ordering::Release);

        drop(queue);

        blockUntil(&disabledInterrupts, &info.inputReady);
    }

    let disabledInterrupts = disableInterrupts();
    let mut queue = readLineWaitList.queue.lock();

    remove!(&mut queue, info, link);

    drop(queue);
    drop(disabledInterrupts);

    let currSize = info.currSize.load(Ordering::Acquire).min(info.buffer.len());
    unsafe {
        info.buffer[0..currSize].clone_to_uninit(buf as *mut u8);
    }

    currSize as i32
}

#[unsafe(no_mangle)]
pub unsafe extern "cdecl" fn readlineHandler(reg: *mut Registers) {
    unsafe {
        let disabledInterrupts = enteredDisabledInterrupts();
        let reg = &mut *reg;
        let args: &ReadLineArgs = &*(reg.esi as *const ReadLineArgs);
        reg.eax = readline(args.len, args.buf) as u32;
    }
}

/// Set up the queue, the buffer, and the interrupt table.
pub unsafe fn installKeyboardDriver() {
    unsafe {
        *IDT().add(KEY_IDT_ENTRY) = trapGate(
            HARDWARE_PRIVILEGE,
            super::keyboard_handler_wrapper::keyboardHandlerWrapper,
            SEGSEL_KERNEL_CS);
        *IDT().add(READLINE_INT) = interruptGate(
            USER_PRIVILEGE,
            crate::syscall::readfileHandlerWrapper,
            SEGSEL_KERNEL_CS);
    }
}
