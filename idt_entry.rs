//! Header for constructing IDT entries.
//!
//! Header for utilities for constriction gates to be stored in the
//! interrupt table.

/// Location of the interrupt table as a Gate pointer.
pub const IDT: *mut Gate = idt_base() as *mut Gate;

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
    (word2 as u64) << UPPER_WORD_START | word1
}

/// IDT Gate.
pub type Gate = u64;
