//! Approvals, checks and lands as records in per-writer ledger branches
//! (specs/ledger/SPEC.md, `per-writer-ledgers`).

mod doc_only;
mod fold;
mod record;

pub use doc_only::{doc_only, legacy_doc_only};
pub use fold::{Fold, fold_records, person, show, slug};
pub use record::{Body, Decision, LandKind, Rating, Record, Usage, VerifyMode, signed_message};

#[cfg(feature = "git")]
mod append;
#[cfg(feature = "git")]
mod branch;
#[cfg(feature = "git")]
mod code_key;
#[cfg(feature = "git")]
mod compact;
#[cfg(feature = "git")]
mod read;
#[cfg(feature = "git")]
mod sign;
#[cfg(feature = "git")]
mod verify;
#[cfg(feature = "git")]
mod writer;

#[cfg(feature = "git")]
pub use append::{Appended, Entry, append, append_all, effort_id};
#[cfg(feature = "git")]
pub use code_key::{code_key, code_key_with};
#[cfg(feature = "git")]
pub use compact::{Compacted, compact};
#[cfg(feature = "git")]
pub use read::fold;
#[cfg(feature = "git")]
pub use sign::{Signed, sign, signed, signed_by};
#[cfg(feature = "git")]
pub use verify::{Finding, has_ledger, verify};
#[cfg(feature = "git")]
pub use writer::writer;

#[cfg(feature = "git")]
pub use git::*;

#[cfg(feature = "git")]
mod git {
    use std::fmt;

    #[derive(Debug)]
    pub enum Error {
        Git(clm_git::Error),
        Config(clm_config::Error),
        Io(std::io::Error),
        /// The writer identity can't be made.
        Writer(String),
        /// A record can't be signed, or a signature checked: no key, or `ssh-keygen`
        /// failed or can't be run.
        Signing(String),
        /// A committed setting the audit reads can't be parsed.
        Settings(String),
    }

    impl fmt::Display for Error {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Error::Git(e) => e.fmt(f),
                Error::Config(e) => e.fmt(f),
                Error::Io(e) => e.fmt(f),
                Error::Writer(why) | Error::Signing(why) | Error::Settings(why) => f.write_str(why),
            }
        }
    }

    impl std::error::Error for Error {}

    impl From<clm_git::Error> for Error {
        fn from(e: clm_git::Error) -> Self {
            Error::Git(e)
        }
    }

    impl From<clm_config::Error> for Error {
        fn from(e: clm_config::Error) -> Self {
            Error::Config(e)
        }
    }

    impl From<std::io::Error> for Error {
        fn from(e: std::io::Error) -> Self {
            Error::Io(e)
        }
    }

    /// `bytes` random bytes as lowercase hex.
    pub(crate) fn random_hex(bytes: usize) -> Result<String, Error> {
        let mut buf = vec![0u8; bytes];
        getrandom::fill(&mut buf).map_err(|e| Error::Io(std::io::Error::other(e.to_string())))?;
        Ok(buf.iter().map(|b| format!("{b:02x}")).collect())
    }
}
