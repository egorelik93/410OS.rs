//! Implementation of a logical cursor.
//!
//! This file implements functions that allow users
//! to access a "logical cursor" that tracks and
//! updates the physical cursor.


use core::ffi::c_int;

use _410kern::asm::*;
use _410kern::video_defines::*;

use crate::byte_utils::LSB;
use crate::byte_utils::MSB;
use crate::sync::mutex::Mutex;
use crate::virtual_memory::LogicalAddress;
use crate::virtual_memory::isUserWritableAddr;

use super::isValidPos;

/// Calculates the flat offset for a cursor position.
///
/// Given a row and column for a position, converts
/// that into a flat offset index.
#[inline(always)]
const fn CONSOLE_OFFSET(row: usize, col: usize) -> usize {
    row * CONSOLE_WIDTH + col
}


/* Logical Cursor State */

#[repr(C)]
#[derive(Debug)]
pub(super) struct State {
    /// Row of the logical cursor.
    row: usize,
    /// Column of the logical cursor.
    col: usize,
    /// Whether the physical cursor is visible.
    visible: bool
}

pub(super) static cursorState: Mutex<State> = Mutex::new(State {
    row: 0,
    col: 0,
    visible: true
});


/* Helper Functions */

impl State {
    /// Sets the position of the physical cursor.
    ///
    /// Moves the raw physical cursor without any checks
    /// or updates to the logical cursor.
    fn moveCursorTo(&mut self, row: usize, col: usize) {
        /* Flatten row and col into a single array offset */
        let offset = CONSOLE_OFFSET(row, col);

        unsafe {
            /* Send Most Significant Byte of the offset to the cursor device */
            outb(CRTC_IDX_REG, CRTC_CURSOR_MSB_IDX);
            outb(CRTC_DATA_REG as u16, MSB(offset as u16));

            /* Send Least Significant Byte of the offset to the cursor device */
            outb(CRTC_IDX_REG, CRTC_CURSOR_LSB_IDX);
            outb(CRTC_DATA_REG, LSB(offset as u16));
        }
    }
}


/* Cursor API */

/// Gets the current logical cursor position.
///
/// Writes the row and col of the position into the given pointers.
pub fn get_cursor() -> (usize, usize) {
    cursorState.lock().get_cursor()
}

impl State {
    pub(super) fn get_cursor(&self) -> (usize, usize) {
        (self.row, self.col)
    }
}

/// Moves the logical cursor to the given position.
///
/// The given position must be valid for the cursor to move.
/// The physical cursor will only move if it is currently
/// set to be visible; otherwise it will remain hidden,
/// and only the logical cursor will be updated.
pub fn set_cursor(row: usize, col: usize) -> Result<(), ()> {
    if !isValidPos(row, col) {
        return Err(())
    }

    cursorState.lock().set_cursor(row, col);
    Ok(())
}

impl State {
    pub(super) fn set_cursor(&mut self, row: usize, col: usize) {
        if self.visible {
            self.moveCursorTo(row, col);
        } else {
            self.hide_cursor();
        }

        self.row = row;
        self.col = col;
    }
}

/// Hides the physical cursor.
///
/// Moves the physical cursor off the screen
/// while keeping the logical cursor the same.
pub fn hide_cursor() {
    cursorState.lock().hide_cursor();
}

impl State {
    fn hide_cursor(&mut self) {
        self.moveCursorTo(CONSOLE_HEIGHT, CONSOLE_WIDTH);
        self.visible = false;
    }
}

/// Shows the physical cursor.
///
/// Moves the physical cursor to the logical cursor.
pub fn show_cursor() {
    cursorState.lock().show_cursor();
}

impl State {
    fn show_cursor(&mut self) {
        self.moveCursorTo(self.row, self.col);
        self.visible = true;
    }
}


/* Syscalls */

/// Gets the current logical cursor position.
///
/// Validates arguments before calling get_cursor.
pub unsafe fn get_cursor_pos(row: *mut c_int, col: *mut c_int) -> c_int {
    unsafe {
        if isUserWritableAddr(LogicalAddress(row.addr()), size_of::<c_int>()) &&
            isUserWritableAddr(LogicalAddress(col.addr()), size_of::<c_int>()) {
                let guard = cursorState.lock();
                *row = guard.row as c_int;
                *col = guard.col as c_int;
                0
            } else {
                -1
            }
    }
}

/// Sets the logical cursor position.
///
/// For compatibility with the syscall spec.
#[inline(always)]
pub fn set_cursor_pos(row: c_int, col: c_int) -> c_int {
    set_cursor(row as usize, col as usize).map_or(-1, |_| 0)
}
