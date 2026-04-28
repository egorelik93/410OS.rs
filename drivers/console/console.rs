/// Functions for reading and writing data on the console.
///
/// This file contains the API for writing and reading
/// characters and colors displayed on the console.
/// However, it does not contain any cursor-specific functionality.


/* Console Types */

use core::ffi::{CStr, c_char, c_int};
use core::ptr;

use _410kern::seg::SEGSEL_KERNEL_CS;
use _410kern::video_defines::*;
use _410kern::page::PAGE_SIZE;

use crate::sync::disable_interrupts::disableInterrupts;
use crate::syscall_int::*;
use crate::idt_entry::*;
use crate::sync::mutex::Mutex;
use crate::task::lockReadFromMemory;
use crate::virtual_memory::{LogicalAddress, isUserReadableAddr};
use super::cursor::cursorState;
use super::{BACKSPACE, cursor, get_cursor};

/// The representation of a video display color description.
///
/// Color is represented by 7 bits denoting
/// foreground, background, and blink.
type Color = u8;

/// The data at each index of the video display.
///
/// Each index into the video memory has the pair of the character
/// to display at that location and the color to display it with.
#[repr(C)]
#[derive(Debug)]
struct ConsoleChar {
    ch: c_char,    /* The character to display. */
    color: Color   /* The color to display the character with. */
}

/// The format of the entire video display memory.
type Console = [[ConsoleChar; CONSOLE_HEIGHT]; CONSOLE_WIDTH];


/// Location of the 2D array of ConsoleChars forming the display.
const CONSOLE: *mut Console = ptr::with_exposed_provenance_mut(CONSOLE_MEM_BASE);

const BLANK: c_char = ' ' as c_char;


/* Console State */

#[derive(Debug)]
struct ConsoleHandle;

#[derive(Debug)]
struct State {
    /// The color to print future characters with.
    currColor: Color,
    console: ConsoleHandle
}

impl ConsoleHandle {
    const fn get(&self) -> &Console {
        unsafe { &*CONSOLE }
    }

    const fn get_mut(&mut self) -> &mut Console {
        unsafe { &mut *CONSOLE }
    }
}

/// Atomicity of the print syscall.
static printState: Mutex<State> = Mutex::new(State { currColor: FGND_LGRAY, console: ConsoleHandle });


/* Helper Functions */

/// Checks whether a given position is a valid location
/// in the console.
///
/// A position is valid iff it is entirely within
/// the boundaries of the console.
pub fn isValidPos(row: usize, col: usize) -> bool {
    0 <= row && row <= CONSOLE_HEIGHT && 0 <= col && col < CONSOLE_WIDTH
}

/// Checks for a valid color.
///
/// Checks to see if an integer code corresponds to a valid
/// color, returning the corresponding color if it is.
/// Colors that are valid may be used by the console display.
pub fn isValidColor(colorRep: c_int) -> bool {
    (FGND_BLACK as c_int) <= colorRep && colorRep <= ((BLINK | BGND_LGRAY | FGND_WHITE) as c_int)
}


impl State {
    /// Scroll the screen down by one line.
    ///
    /// Moves all text on the screen up by one line,
    /// erasing the top line of text and creating a new empty line
    /// on the bottom.
    fn scroll(&mut self) {
        /* Ignoring the first line which gets lost,
         * goes through every line and copies its
         * text with its coloring to the line above it.
         */
        for row in 1usize .. CONSOLE_HEIGHT {
            for col in 0usize .. CONSOLE_WIDTH {
                let oldChar: &mut ConsoleChar = &mut self.console.get_mut()[row][col];
                draw_char(row - 1, col, oldChar.ch, oldChar.color);
            }
        }

        /* Erases the last line. */
        for col in 0usize .. CONSOLE_WIDTH {
            draw_char(CONSOLE_HEIGHT - 1, col, BLANK, self.currColor)
        }
    }

    /// Moves the cursor to the next line,
    /// scrolling if necessary.
    ///
    /// This is a wrapper around just updating the
    /// cursor to ensure scrolling is accounted for.
    fn moveToNextLineFrom(&mut self, cursor: &mut cursor::State, row: usize) {
        if row + 1 >= CONSOLE_HEIGHT {
            self.scroll();
            cursor.set_cursor(CONSOLE_HEIGHT - 1, 0);
        } else {
            cursor.set_cursor(row + 1, 0);
        }
    }

    /// Moves the cursor forward a character,
    /// going to the next line if necessary.
    ///
    /// This is a wrapper around just updating the
    /// cursor to ensure wrap-around is accounted for.
    fn moveForwardFrom(&mut self, cursor: &mut cursor::State, row: usize, col: usize) {
        if col + 1 >= CONSOLE_WIDTH {
            self.moveToNextLineFrom(cursor, row);
        } else {
            cursor.set_cursor(row, col + 1);
        }
    }
}


/* Console API */

/// Writes a character to the display.
///
/// For most characters, this will write the given character
/// to the display where the cursor is currently located
/// and move the cursor forward by 1 character, wrapping around
/// and scrolling if necessary. However, for the following
/// characters there are special behaviors:
///
/// '\\n' - Move cursor to the next line, scrolling if necessary.
/// '\\r' - Move cursor to the start of the current line.
/// '\\b' - Move the cursor back a space and replace the character
///        at that location with a blank, up to the (0, 0) position.
///        Attempting to backspace from (0, 0) will do nothing.
#[unsafe(no_mangle)]
pub extern "cdecl" fn putbyte(c: c_char) -> c_char {
    printState.lock().putbyte(&mut cursorState.lock(), c)
}

impl State {
    fn putbyte(&mut self, cursor: &mut cursor::State, c: c_char) -> c_char {
        let (cursorRow, cursorCol) = get_cursor();

        match c as u8 {
            b'\n' => {
                self.moveToNextLineFrom(cursor, cursorRow);
                return c;
            },
            b'\r' => {
                cursor.set_cursor(cursorRow, 0);
                return c;
            },
            BACKSPACE => {
                if cursorCol > 0 {
                    /* If in the middle of a line, go back a space */
                    self.draw_char(cursorRow, cursorCol - 1, BLANK, self.currColor);
                    cursor.set_cursor(cursorRow, cursorCol - 1);
                } else if cursorRow > 0 {
                    /* If at the start of a line but not the first line,
                     * go to the end of the previous line.
                     */
                    self.draw_char(cursorRow - 1, CONSOLE_WIDTH - 1, BLANK, self.currColor);
                    cursor.set_cursor(cursorRow - 1, CONSOLE_WIDTH - 1);
                }
                /* If at (0, 0), do nothing. */

                return c;
            }
            _ => {
                self.draw_char(cursorRow, cursorCol, c, self.currColor);
                self.moveForwardFrom(cursor, cursorRow, cursorCol);
                return c;
            }
        }
    }
}

/// Writes a character string of the given length to the display.
///
/// Writes len many characters to the display, and moves the
/// cursor forward accordingly (depending on special characters).
/// This simply calls putbyte() repeatedly, so all behaviors
/// there will work here.
pub unsafe fn putbytes(s: *const c_char, len: usize) {
    let ptr = s;

    let disabledInterrupts = disableInterrupts();

    let mut state = printState.lock();
    let mut cursor = cursorState.lock();

    for i in 0usize .. len {
        state.putbyte(&mut cursor, unsafe { *ptr.add(i) });
    }
}

/// Writes a character with a particular color
/// to a particular place on the screen.
///
/// Writes the character to the given location
/// with the given color, ignoring the
/// current cursor position and color and not performing
/// any special behavior for special characters.
pub fn draw_char(row: usize, col: usize, ch: c_char, color: u8) {
    printState.lock().draw_char(row, col, ch, color);
}

impl State {
    fn draw_char(&mut self, row: usize, col: usize, ch: c_char, color: u8) {
        if isValidColor(color as i32) && isValidPos(row, col) {
            self.console.get_mut()[row][col] = ConsoleChar {
                ch: ch as i8,
                color: color
            }
        }
    }
}

/// Gets the character currently displayed at a position.
pub fn get_char(row: usize, col: usize) -> c_char {
    printState.lock().get_char(row, col)
}

impl State {
    fn get_char(&self, row: usize, col: usize) -> c_char {
        self.console.get()[row][col].ch
    }
}

/// Resets to the console to be entirely blank,
/// moving the cursor back to (0, 0).
/// The set color remains the same.
pub fn clear_console() {
    printState.lock().clear_console();
}

impl State {
    fn clear_console(&mut self) {
        for row in 0usize .. CONSOLE_HEIGHT {
            for col in 0usize .. CONSOLE_WIDTH {
                self.draw_char(row, col, BLANK, self.currColor);
            }
        }
    }
}

/// Gets the current color future characters will be printed with.
///
/// Sets the passed pointer with the current color.
pub fn get_term_color() -> u8 {
    printState.lock().get_term_color()
}

impl State {
    fn get_term_color(&self) -> u8 {
        self.currColor
    }
}


/** Syscalls */

/// Sets the color future characters will be printed with.
pub fn set_term_color(color: c_int) -> c_int {
    if isValidColor(color) {
        printState.lock().currColor = color as Color;
        return 0;
    } else {
        return -1;
    }
}


/// Prints the contents of a buffer to the console.
pub unsafe fn print(len: c_int, buf: *mut c_char) -> c_int {
    if len as usize > PAGE_SIZE {
        return -1;
    }

    let dir = lockReadFromMemory();

    if unsafe { !isUserReadableAddr(LogicalAddress(buf.addr()), len as usize) } {
        return -1;
    }

    unsafe { putbytes(buf, len as usize) };

    return 0;
}


/// Initialize console related syscalls and state.
pub unsafe fn installConsoleDriver() {
    unsafe {
        *IDT().add(SET_TERM_COLOR_INT) = interruptGate(
            USER_PRIVILEGE,
            crate::syscall::setTermColorHandlerWrapper,
            SEGSEL_KERNEL_CS);

        *IDT().add(SET_CURSOR_POS_INT) = interruptGate(
            USER_PRIVILEGE,
            crate::syscall::setCursorPosHandlerWrapper,
            SEGSEL_KERNEL_CS);

        *IDT().add(GET_CURSOR_POS_INT) = interruptGate(
            USER_PRIVILEGE,
            crate::syscall::getCursorPosHandlerWrapper,
            SEGSEL_KERNEL_CS);

        *IDT().add(PRINT_INT) = interruptGate(
            USER_PRIVILEGE,
            crate::syscall::printHandlerWrapper,
            SEGSEL_KERNEL_CS);

        clear_console();
    }
}
