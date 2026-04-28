//! Wrapper for pagefault handler.

use _410kern::idt::*;
use crate::handler_wrapper::*;
use crate::virtual_memory::pageFaultHandler;

EXCEPTION!(pageFault, IDT_PF, pageFaultHandler);
