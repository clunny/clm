//! The migrations the binary holds, in the order they landed on `main`. Each one is a directory
//! `src/migrations/<id>/` holding `mod.rs` and any text it embeds, pulled in here with
//! `#[path]` since an ID such as `20261105-clm-upgrade` isn't a Rust identifier.

use crate::Migration;

/// The migrations, oldest first.
pub(crate) static LIST: &[Migration] = &[];
