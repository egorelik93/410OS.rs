//! Macros to generate assembly wrapper for a handler.
//!
//! Saves registers before calling C handlers, and restores them after.

pub use paste::paste;


/// Save data segment and general purpose registers on stack
macro_rules! SAVE_REGISTERS {
    () => { concat!(
        /* Save the data segment registers */
        "push %ds\n",
        "push %es\n",
        "push %fs\n",
        "push %gs\n",
        /* Save general registers on stack */
        "pusha\n"
    )}
}

pub(crate) use SAVE_REGISTERS;

/// Restore general and data segment registers from stack
macro_rules! RESTORE_REGISTERS {
    () => { concat!(
        /* Load saved registers */
        "popa\n",

        /* Load the data segment registers */
        "pop %gs\n",
        "pop %fs\n",
        "pop %es\n",
        "pop %ds\n"
    )}
}

pub(crate) use RESTORE_REGISTERS;

// Generates an assembly wrapper for an interrupt handler.
//
// This wrapper generates all assembly boilerplate
// for a handler. In particular,
// the resulting assembly will be a global assembly function called
// (NAME)HandlerWrapper, which saves registers
// and calls the function (NAME)Handler.
// NAME is the argument provided.
// The handler is passed
// a pointer to the saved registers,
// which may then be read from, updated,
// or ignored as appropriate.
macro_rules! HANDLER {
    ($name:ident) => { paste! {
        #[unsafe(no_mangle)]
        #[unsafe(naked)]
        pub unsafe extern "cdecl" fn [<$name HandlerWrapper>]() {
            ::core::arch::naked_asm!(concat!(
                crate::handler_wrapper::SAVE_REGISTERS!(),

                /* Call the C handler */
                "movl %esp, %eax\n",
                "push %eax\n",
                "call {}\n",
                "pop %eax\n",

                crate::handler_wrapper::RESTORE_REGISTERS!(),

                /* Return from interrupt */
                "iret\n"
            ),
                sym [<$name Handler>],
                options(att_syntax))
        }
    }}
}

pub(crate) use HANDLER;



/// Generates an assembly wrapper for an exception handler
///
/// This pushes the exception vector number and a pointer to the iret state
/// as arguments to the subsequent function (FN).
/// Hardware has already pushed an error code on this stack.
macro_rules! EXCEPTION {
    ($name:ident, $vec:expr, $fn:ident) => { paste! {
        #[unsafe(no_mangle)]
        #[unsafe(naked)]
        pub unsafe extern "cdecl" fn [<$name HandlerWrapper>]() {
            ::core::arch::naked_asm!(concat!(
                crate::handler_wrapper::SAVE_REGISTERS!(),

                /* Call the C handler */
                "movl %esp, %eax\n",
                "push {}\n",
                "push %eax\n",
                "call {}\n",
                "pop %eax\n",
                "pop %eax\n",

                crate::handler_wrapper::RESTORE_REGISTERS!(),

                /* Return from interrupt */
                "add $4, %esp\n",
                "iret\n"
            ),
                const $vec,
                sym $fn,
                options(att_syntax))
        }
    }}
}

pub(crate) use EXCEPTION;


/// Generates an assembly wrapper for an exception handler
///
/// This pushes the exception vector number and a pointer to the iret state
/// as arguments to the subsequent function (FN).
/// Hardware has not pushed an error code, so we push one for consistency
macro_rules! EXCEPTION_PUSH_ERROR {
    ($name:ident, $vec:expr, $fn:ident) => { paste! {
        #[unsafe(no_mangle)]
        #[unsafe(naked)]
        pub unsafe extern "cdecl" fn [<$name HandlerWrapper>]() {
            ::core::arch::naked_asm!(concat!(
                "push $0xdead00d\n",
                crate::handler_wrapper::SAVE_REGISTERS!(),

                /* Call the C handler */
                "movl %esp, %eax\n",
                "push {}\n",
                "push %eax\n",
                "call {}\n",
                "pop %eax\n",
                "pop %eax\n",

                crate::handler_wrapper::RESTORE_REGISTERS!(),

                /* Return from interrupt */
                "add $4, %esp\n",
                "iret\n"
            ),
                const $vec,
                sym $fn,
                options(att_syntax))
        }
    }}
}

pub(crate) use EXCEPTION_PUSH_ERROR;
