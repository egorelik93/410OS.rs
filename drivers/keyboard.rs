//! Header for the keyboard API and installer.

mod buffer;
mod keyboard;
mod keyboard_handler_wrapper;

pub use keyboard::{
    installKeyboardDriver,
    readline
};

pub mod syscall_handler {
    pub use super::keyboard::readlineHandler;
}
