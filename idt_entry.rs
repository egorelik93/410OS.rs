//! Construct interrupt handler entries.
//!
//! Utilities for constriction gates to be stored in the
//! interrupt table.

use _410kern::asm::*;

use crate::byte_utils::{FILTER_BIT_RANGE, UPDATE_BIT, UPDATE_BIT_RANGE};

/// Location of the interrupt table as a Gate pointer.
#[inline(always)]
pub fn IDT() -> *mut Gate {
    unsafe { idt_base() as *mut Gate }
}

/// Privilege Levels
pub const HARDWARE_PRIVILEGE: u8 = 0;
pub const USER_PRIVILEGE: u8 = 3;


pub const UPPER_WORD_START: u8 = 32;

/// Construct a gate.
///
/// General format of IDT entries.
/// Since the format is primarily controlled by
/// individual bits, I just expose it as 2 32-bit words
/// so we can separate the bit manipulation into 2 parts.
#[inline(always)]
pub const fn GATE(word1: u32, word2: u32) -> u64 {
    (word2 as u64) << UPPER_WORD_START | (word1 as u64)
}

/// IDT Gate.
pub type Gate = u64;


/* Trap Gate specification */
pub const TRAP_GATE_SEGMENT_END: usize = 32;
pub const TRAP_GATE_SEGMENT_START: usize = 16;
pub const TRAP_GATE_LOWER_OFFSET_END: usize = 16;
pub const TRAP_GATE_LOWER_OFFSET_START: usize = 0;
pub const TRAP_GATE_UPPER_OFFSET_END: usize = 32;
pub const TRAP_GATE_UPPER_OFFSET_START: usize = 16;
pub const TRAP_GATE_SEGMENT_PRESENT_POS: usize = 15;
pub const TRAP_GATE_DPL_END: usize = 15;
pub const TRAP_GATE_DPL_START: usize = 13;
pub const TRAP_GATE_SIZE_POS: usize = 11;
pub const TRAP_GATE_RESERVED_END: usize = 5;
pub const TRAP_GATE_RESERVED_START: usize = 0;

pub const TRAP_GATE_SEGMENT_PRESENT: u32 = 1;
pub const TRAP_GATE_SIZE_BIT: u32 = 1;

/// Bits always set in a Trap gate.
pub const TRAP_GATE_CONSTANT: u32 = 0x700;


/* Interrupt Gate specification */
pub const INTERRUPT_GATE_SEGMENT_END: usize = 32;
pub const INTERRUPT_GATE_SEGMENT_START: usize = 16;
pub const INTERRUPT_GATE_LOWER_OFFSET_END: usize = 16;
pub const INTERRUPT_GATE_LOWER_OFFSET_START: usize = 0;
pub const INTERRUPT_GATE_UPPER_OFFSET_END: usize = 32;
pub const INTERRUPT_GATE_UPPER_OFFSET_START: usize = 16;
pub const INTERRUPT_GATE_SEGMENT_PRESENT_POS: usize = 15;
pub const INTERRUPT_GATE_DPL_END: usize = 15;
pub const INTERRUPT_GATE_DPL_START: usize = 13;
pub const INTERRUPT_GATE_SIZE_POS: usize = 11;
pub const INTERRUPT_GATE_RESERVED_END: usize = 5;
pub const INTERRUPT_GATE_RESERVED_START: usize = 0;

pub const INTERRUPT_GATE_SEGMENT_PRESENT: u32 = 1;
pub const INTERRUPT_GATE_SIZE_BIT: u32 = 1;

/// Bits always set in a Interrupt gate.
pub const INTERRUPT_GATE_CONSTANT: u32 = 0x600;


/// Creates a Trap Gate.
pub fn trapGate(privilege: u8, handler: unsafe extern "cdecl" fn(), segment: u16) -> Gate {
    /* The handler address is split into 2 parts.
     * We do not need to shift them when we place
     * them back into the gate, however.
     */
    let lowerOffset = FILTER_BIT_RANGE(
        handler as usize,
        TRAP_GATE_LOWER_OFFSET_START,
        TRAP_GATE_LOWER_OFFSET_END) as u32;
    let upperOffset = FILTER_BIT_RANGE(
        handler as usize,
        TRAP_GATE_UPPER_OFFSET_START,
        TRAP_GATE_UPPER_OFFSET_END) as u32;

    let flags = UPDATE_BIT_RANGE(
        UPDATE_BIT(
            UPDATE_BIT(
                TRAP_GATE_CONSTANT,
                TRAP_GATE_SEGMENT_PRESENT_POS,
                TRAP_GATE_SEGMENT_PRESENT),
            TRAP_GATE_SIZE_POS,
            TRAP_GATE_SIZE_BIT),
        TRAP_GATE_DPL_START,
        TRAP_GATE_DPL_END,
        privilege as u32);

    let word1 = ((segment as u32) << TRAP_GATE_SEGMENT_START) | lowerOffset;
    let word2 = upperOffset | flags;

    GATE(word1, word2)
}


/// Creates an Interrupt Gate.
///
/// Unlike Trap gates, interrupt gates disable interrupts
/// before running the handler.
pub fn interruptGate(privilege: u8, handler: unsafe extern "cdecl" fn() -> (), segment: u16) -> Gate {
    /* The handler address is split into 2 parts.
     * We do not need to shift them when we place
     * them back into the gate, however.
     */
    let lowerOffset = FILTER_BIT_RANGE(
        handler as usize,
        INTERRUPT_GATE_LOWER_OFFSET_START,
        INTERRUPT_GATE_LOWER_OFFSET_END) as u32;
    let upperOffset = FILTER_BIT_RANGE(
        handler as usize,
        INTERRUPT_GATE_UPPER_OFFSET_START,
        INTERRUPT_GATE_UPPER_OFFSET_END) as u32;

    let flags = UPDATE_BIT_RANGE(
        UPDATE_BIT(
            UPDATE_BIT(
                INTERRUPT_GATE_CONSTANT,
                INTERRUPT_GATE_SEGMENT_PRESENT_POS,
                INTERRUPT_GATE_SEGMENT_PRESENT),
            INTERRUPT_GATE_SIZE_POS,
            INTERRUPT_GATE_SIZE_BIT),
        INTERRUPT_GATE_DPL_START,
        INTERRUPT_GATE_DPL_END,
        privilege as u32);

    let word1 = ((segment as u32) << INTERRUPT_GATE_SEGMENT_START) | lowerOffset;
    let word2 = upperOffset | flags;

    GATE(word1, word2)
}
