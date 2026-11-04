//! The layer the Worker's routes stand on (`web.worker.platform`): the platform traits,
//! HTTP types, access check, GitHub client, file fetches, tree keys, installation tokens,
//! live state and the ledger's reading and web-ledger append. It imports neither
//! `clm-web` nor `clm-web-review`, and has no `worker` dependency: `clm-web`'s adapter
//! implements [`platform`]'s traits.

pub mod access;
#[cfg(feature = "fake")]
pub mod fake;
pub mod files;
pub mod gh;
pub mod http;
pub mod install;
pub mod keys;
pub mod ledger;
pub mod live;
pub mod platform;
