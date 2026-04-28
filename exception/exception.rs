//! Exception routine wrappers

use crate::handler_wrapper::*;
use super::exn_handler::generalExnHandler;

/// Wrapper for handler to alignment check exceptions.
EXCEPTION!(alignExn, 0x11, generalExnHandler);
/// Wrapper for handler to segment not present exceptions.
EXCEPTION!(segExn, 0x0B, generalExnHandler);
/// Wrapper for handler to stack-segment exceptions.
EXCEPTION!(stackExn, 0x0C, generalExnHandler);
///  Wrapper for handler to general protection exceptions.
EXCEPTION!(protExn, 0x0D, generalExnHandler);

/// Wrapper for handler to divide error exceptions.
EXCEPTION_PUSH_ERROR!(divideExn, 0x00, generalExnHandler);
/// Wrapper for handler for breakpoint exceptions.
EXCEPTION_PUSH_ERROR!(debugExn, 0x01, generalExnHandler);
/// Wrapper for handler to breakpoint exceptions.
EXCEPTION_PUSH_ERROR!(breakpointExn, 0x03, generalExnHandler);
/// Wrapper for handler for overflow exceptions.
EXCEPTION_PUSH_ERROR!(overflowExn, 0x04, generalExnHandler);
/// Wrapper for handler for BOUND range exceeded exceptions.
EXCEPTION_PUSH_ERROR!(boundcheckExn, 0x05, generalExnHandler);
/// Wrapper for handler for invalid opcode exceptions.
EXCEPTION_PUSH_ERROR!(opcodeExn, 0x06, generalExnHandler);
/// Wrapper for handler for FPU not present exceptions.
EXCEPTION_PUSH_ERROR!(nofpuExn, 0x07, generalExnHandler);
/// Wrapper for handler to for x87 FPU exceptions.
EXCEPTION_PUSH_ERROR!(fpufaultExn, 0x10, generalExnHandler);
/// Wrapper for handler for SIMD FP exceptions.
EXCEPTION_PUSH_ERROR!(simdfaultExn, 0x13, generalExnHandler);
