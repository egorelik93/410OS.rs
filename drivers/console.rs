//! Function prototypes for the console driver.
//!
//! This contains the prototypes and global variables for the console
//! driver.

mod console;
mod cursor;
pub mod syscall_handler;


/// 'Backspace character code'
///
/// Not in the original implmentation, since C supports '\b' (and Rust doesn't)
pub const BACKSPACE: u8 = b'\x08';


/* Position */

pub use console::{
    isValidPos,
    isValidColor
};


/* Console text */

pub use console::{
    putbyte,
    putbytes,
    set_term_color,
    get_term_color,
    clear_console,
    draw_char,
    get_char
};


/* Cursor */

pub use cursor::{
    set_cursor,
    get_cursor,
    hide_cursor,
    show_cursor
};

/* Syscalls */
pub use console::{
    print,
    // set_term_color
};
pub use cursor::{
    get_cursor_pos,
    set_cursor_pos
};

pub use console::installConsoleDriver;
