//! Invalidate a TLB entry.

use core::arch::naked_asm;

use crate::virtual_memory::LogicalAddress;

/// Flushes an address from the TLB.
#[unsafe(naked)]
pub extern "cdecl" fn invalidatePage(addr: LogicalAddress) {
    naked_asm!(
        "mov 4(%esp), %eax",
        "invlpg (%eax)",
        "ret",
        options(att_syntax)
    );
}
