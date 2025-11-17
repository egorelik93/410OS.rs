//! Saved general purpose registers.

/// General-purpose registers
#[derive(Debug, Copy, Clone, Default)]
#[repr(C)]
pub struct Registers {
    pub edi: u32,
    pub esi: u32,
    pub ebp: u32,
    pub esp: u32,
    pub ebx: u32,
    pub edx: u32,
    pub ecx: u32,
    pub eax: u32
}

/// Items pushed on the stack by x86 after an interrupt
#[derive(Debug, Copy, Clone, Default)]
#[repr(C)]
pub struct SuspendedState {
    pub reg: Registers,
    pub gs: u32,
    pub fs: u32,
    pub es: u32,
    pub ds: u32,
    pub eip: u32,
    pub cs: u32,
    pub eflags: u32,
    pub esp: u32,
    pub ss: u32
}

/// Items pushed on the stack by x86 after an exception
#[derive(Debug, Copy, Clone, Default)]
#[repr(C)]
pub struct ExceptionState {
    pub reg: Registers,
    pub gs: u32,
    pub fs: u32,
    pub es: u32,
    pub ds: u32,
    pub err: u32,
    pub eip: u32,
    pub cs: u32,
    pub eflags: u32,
    pub esp: u32,
    pub ss: u32
}
