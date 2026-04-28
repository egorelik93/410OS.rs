//! Assembly component of the keyboard handler.
//!
//! Saves registers before calling the keyboardHandler(),
//! restoring them later.

use crate::handler_wrapper::*;
use super::keyboard::keyboardHandler;

HANDLER!(keyboard);
