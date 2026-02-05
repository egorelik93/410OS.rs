//! Utility for generating ids in a thread-safe manner.

use crate::sync::mutex::Mutex;

/// Used to generate IDs.
pub struct IDGenerator {
    counter: Mutex<i32>,
    start: i32,
}

impl IDGenerator {
    /// Initializes the IDGenerator structure
    pub const fn new(start: i32) -> Self {
        IDGenerator {
            counter: Mutex::new(start),
            start: start
        }
    }

    /// Generate a new ID, using the given IDGenerator structure
    pub fn generateID(&self) -> i32 {
        let mut guard = self.counter.lock();

        let id = *guard;
        *guard = guard.wrapping_add(1);

        if *guard < self.start {
            *guard = self.start;
        }

        id
    }
}
