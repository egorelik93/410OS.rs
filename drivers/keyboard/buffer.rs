use core::default;
/// Implementation of the keyboard buffer.
///
/// The buffer is implemented as a circular buffer.
/// If the buffer fills up, no new entries will be written.
/// The buffer is not thread safe, but it should
/// be safe to remove and insert at the same time.

use core::ffi::c_char;

use _410kern::page::PAGE_SIZE;

pub const CHAR_BUFFER_SIZE: usize = PAGE_SIZE;

/// A fixed length circular buffer.
///
/// The oldest entry in the buffer is tracked
/// by readIndex, while the next entry will
/// be written at writeIndex.
pub struct CharBuffer {
    readIndex: usize,
    writeIndex: usize,
    buffer: [c_char; CHAR_BUFFER_SIZE]
}


/// Checks if two indices refer to the same location.
///
/// Checks for equality mod the size of the buffer.
#[inline(always)]
fn EQUAL_INDEX(v1: usize, v2: usize) -> bool {
    v1 % CHAR_BUFFER_SIZE == v2 % CHAR_BUFFER_SIZE
}

/// Adds to the index going around the circle.
///
/// Addition modulo the size of the buffer.
#[inline(always)]
fn ADD_TO_INDEX(i: usize, v: usize) -> usize {
    (i + v) % CHAR_BUFFER_SIZE
}

impl CharBuffer {
    /// Create a buffer.
    ///
    /// Sets all indices to 0.
    pub const fn new() -> Self {
        CharBuffer {
            readIndex: 0,
            writeIndex: 0,
            buffer: [0 as c_char; CHAR_BUFFER_SIZE] }
    }

    /// Add a char to the buffer.
    ///
    /// Will write the char to the next writeable
    /// index in the buffer. If the buffer is full,
    /// nothing new will be written.
    pub fn insertChar(&mut self, c: c_char) {
        let writeIndex = self.writeIndex;
        let readIndex = self.readIndex;

        if EQUAL_INDEX(writeIndex + 1, readIndex) {
            return;
        }
        self.buffer[writeIndex] = c;
        self.writeIndex = ADD_TO_INDEX(writeIndex, 1);
    }

    /// Attempt to read and remove the oldest char.
    pub fn removeChar(&mut self) -> Option<c_char> {
        let readIndex = self.readIndex;
        let writeIndex = self.writeIndex;

        if EQUAL_INDEX(readIndex, writeIndex) {
            None
        } else {
            let c = self.buffer[readIndex];
            self.readIndex = ADD_TO_INDEX(readIndex, 1);

            Some(c)
        }
    }
}
