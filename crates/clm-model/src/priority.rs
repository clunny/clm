//! `priority` of backlog items and changes, `P1` to `P5` (`priority-caps`).

use std::fmt;

use serde::{Deserialize, Serialize};

/// Declared highest first, so the derived `Ord` sorts `P1` before `P5`.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize,
)]
pub enum Priority {
    P1,
    P2,
    P3,
    P4,
    #[default]
    P5,
}

impl Priority {
    /// How many items and live changes may hold this priority; `None` is unlimited.
    pub fn cap(self) -> Option<usize> {
        match self {
            Priority::P1 => Some(1),
            Priority::P2 => Some(2),
            Priority::P3 => Some(3),
            Priority::P4 => Some(4),
            Priority::P5 => None,
        }
    }
}

impl fmt::Display for Priority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Priority::P1 => "P1",
            Priority::P2 => "P2",
            Priority::P3 => "P3",
            Priority::P4 => "P4",
            Priority::P5 => "P5",
        })
    }
}
