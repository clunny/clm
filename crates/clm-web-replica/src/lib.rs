//! The repo's `Replica` Durable Object (specs/web/worker/replica/SPEC.md): the class that
//! runs `replica`'s core on Cloudflare (wasm32 only), the core over a platform of its own,
//! the Web Push notifications a recompute or a `pull_request` event calls for, and the
//! storage an uninstall deletes.

#[cfg(target_arch = "wasm32")]
mod durable;
mod notify;
mod object;
mod removal;
mod source;

#[cfg(target_arch = "wasm32")]
pub use durable::Replica;
pub use notify::{Permissions, review, waiting};
pub use object::{Alarm, Answer, Object};
pub use removal::{Storage, on_removal};

use clm_web_platform::platform::{Clock, GitHub, Live, Log, Secrets};

/// What the object needs of its platform, and no more: GitHub, the clock, the secrets, the
/// `Live` objects and the log (`object-class`).
pub trait Host: GitHub + Clock + Secrets + Live + Log {}

impl<T: GitHub + Clock + Secrets + Live + Log> Host for T {}
