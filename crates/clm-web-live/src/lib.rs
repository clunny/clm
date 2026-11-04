//! The two ways `web` reaches a browser without being asked (specs/web/live/SPEC.md): the
//! `Live` Durable Object, which tells a repo's open pages that it changed and sends Web
//! Push messages, and the payloads the Worker's webhook sends.

#[cfg(all(feature = "durable", target_arch = "wasm32"))]
mod durable;
mod hub;
mod notices;
mod push;
mod send;

#[cfg(all(feature = "durable", target_arch = "wasm32"))]
pub use durable::Live;
pub use hub::{Answer, Host, Request, fetch, on_close, on_message};
pub use notices::{Item, Notice, review_notice, waiting_notice};
pub use push::{PushError, PushRequest, Subscription, Vapid, web_push};
