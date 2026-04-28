//! IDT entries for software exceptions

use paste::paste;

use _410kern::seg::SEGSEL_KERNEL_CS;

use crate::ureg::*;
use crate::idt_entry::{HARDWARE_PRIVILEGE, IDT, trapGate};
use super::exception::*;


macro_rules! EXN_GATE {
    ($vec:expr, $name:ident) => { paste! {
        *unsafe { &mut *IDT().add($vec) } = trapGate(
            HARDWARE_PRIVILEGE,
            [< $name ExnHandlerWrapper >],
            SEGSEL_KERNEL_CS);
    }};
}


macro_rules! GATE_PUSH_ERROR {
    ($vec:expr, $name:ident) => { paste! {
        *unsafe { &mut *IDT().add($vec) } = trapGate(
            HARDWARE_PRIVILEGE,
            [< $name ExnHandlerWrapper >],
            SEGSEL_KERNEL_CS);
    }};
}



/// Install gates for exception error codes (except pagefault)
pub unsafe fn installExceptionGates() {
    // Exception gates for when error code is saved on the stack
    EXN_GATE!(SWEXN_CAUSE_ALIGNFAULT, align);    /* 0x11   Fault, yes */
    EXN_GATE!(SWEXN_CAUSE_SEGFAULT, seg);        /* 0x0B   Fault, yes */
    EXN_GATE!(SWEXN_CAUSE_STACKFAULT, stack);    /* 0x0C   Fault, yes */
    EXN_GATE!(SWEXN_CAUSE_PROTFAULT, prot);      /* 0x0D   Fault, yes */

    // Exception gates for when error code is not saved on the stack
    GATE_PUSH_ERROR!(SWEXN_CAUSE_DIVIDE, divide);           /* 0x00   Fault, no */
    GATE_PUSH_ERROR!(SWEXN_CAUSE_DEBUG, debug);             /* 0x01   Fault, no */
    GATE_PUSH_ERROR!(SWEXN_CAUSE_BREAKPOINT, breakpoint);   /* 0x03   Trap, no */
    GATE_PUSH_ERROR!(SWEXN_CAUSE_OVERFLOW, overflow);       /* 0x04   Trap, no */
    GATE_PUSH_ERROR!(SWEXN_CAUSE_BOUNDCHECK, boundcheck);   /* 0x05   Fault, no */
    GATE_PUSH_ERROR!(SWEXN_CAUSE_OPCODE, opcode);           /* 0x06   Fault, no */
    GATE_PUSH_ERROR!(SWEXN_CAUSE_NOFPU, nofpu);             /* 0x07   Fault, no */
    GATE_PUSH_ERROR!(SWEXN_CAUSE_FPUFAULT, fpufault);       /* 0x10   Fault, no */
    GATE_PUSH_ERROR!(SWEXN_CAUSE_SIMDFAULT, simdfault);     /* 0x13   Fault, no */
}
