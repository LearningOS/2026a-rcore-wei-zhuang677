//! Per-task state for stride scheduling.

use core::cmp::Ordering;

/// A large constant used to turn priority into a stride increment.
pub const BIG_STRIDE: u128 = 1 << 64;
/// The priority assigned to a newly created task.
pub const DEFAULT_PRIORITY: usize = 16;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// The stride scheduling state for a task.
pub struct Stride {
    /// Accumulated virtual runtime of this task.
    pub stride: u128,
    /// Increment added whenever this task is scheduled.
    pub pass: u128,
    /// Scheduling priority; priorities must be greater than one.
    pub priority: usize,
}

impl Stride {
    /// Create stride state for a valid priority.
    ///
    /// Returns `None` for priorities that are invalid for stride scheduling.
    pub fn new(priority: usize) -> Option<Self> {
        if priority as isize <= 1 {
            return None;
        }
        Some(Self {
            stride: 0,
            pass: BIG_STRIDE / priority as u128,
            priority,
        })
    }

    /// Change priority without resetting the accumulated stride.
    ///
    /// Returns `false` and leaves the state unchanged for invalid priorities.
    pub fn set_priority(&mut self, priority: usize) -> bool {
        if priority as isize <= 1 {
            return false;
        }
        self.priority = priority;
        self.pass = BIG_STRIDE / priority as u128;
        true
    }

    /// Account for one scheduling quantum.
    pub fn advance(&mut self) {
        self.stride += self.pass;
    }

    /// Compare tasks by accumulated stride.
    pub fn cmp(&self, other: &Self) -> Ordering {
        self.stride.cmp(&other.stride)
    }
}

impl Default for Stride {
    fn default() -> Self {
        Self::new(DEFAULT_PRIORITY).expect("default stride priority must be valid")
    }
}
