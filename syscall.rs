//! Syscall wrappers.

use crate::handler_wrapper::*;
use crate::thread::syscall_handler::*;
use crate::misc_syscall_handler::*;
use crate::virtual_memory::pageFaultHandler;
use crate::task::syscall_handler::*;
use crate::console::syscall_handler::*;
use crate::keyboard::syscall_handler::*;
use crate::timer::syscall_handler::*;


HANDLER!(exec);

HANDLER!(gettid);

HANDLER!(fork);

HANDLER!(threadFork);

HANDLER!(setStatus);

HANDLER!(vanish);

HANDLER!(wait);

HANDLER!(yield);

HANDLER!(halt);

HANDLER!(deschedule);

HANDLER!(makeRunnable);

HANDLER!(swexn);

HANDLER!(setTermColor);

HANDLER!(setCursorPos);

HANDLER!(getCursorPos);

HANDLER!(print);

HANDLER!(readline);

HANDLER!(getTicks);

HANDLER!(newPages);

HANDLER!(removePages);

HANDLER!(sleep);

HANDLER!(readfile);

HANDLER!(misbehave);
