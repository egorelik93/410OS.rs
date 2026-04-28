//! Assembly component of the timer handler.
//!
//! Saves registers before calling the timerHandler(),
//! restoring them later.

use crate::handler_wrapper::*;
use super::timer::timerHandler;

HANDLER!(timer);
