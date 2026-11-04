//! Signed repositories (`signed-ledger-records`, `signed-approve`, `signed-land`):
//! whether a land or an approval signs, the land's check that a key is configured before any
//! work, and the signing itself, through `clm-ledger`.

use std::ffi::OsString;
use std::path::PathBuf;

use clm_ledger::{Body, Signed};

use crate::{Error, Repo};

/// The namespaces an `approved` and a `landed` record are signed under.
pub const APPROVE: &str = "clm-approve";
pub const LAND: &str = "clm-land";

/// Why a land or an approval can't sign: no usable key.
const NO_KEY: &str = "this repository signs its approvals and lands (docs/allowed_signers on \
                      main), but no SSH signing key is configured: set git's user.signingkey \
                      to your SSH key, and gpg.format to ssh (git config gpg.format ssh)";

impl Repo<'_> {
    /// Whether the repository is signed at `base`, the commit of the `main` fetched.
    pub fn signed(&self, base: &str) -> Result<bool, Error> {
        Ok(clm_ledger::signed(&self.root, base)?)
    }

    /// Refused, naming `user.signingkey`, unless a key is configured: `gpg.format` is `ssh`
    /// and `user.signingkey` is set, to a readable file when it is a path. Signs nothing.
    pub fn require_key(&self) -> Result<(), Error> {
        let refused = |detail: &str| Error::Refused(format!("{NO_KEY}{detail}"));
        if self.git_config(&["--get", "gpg.format"])?.as_deref() != Some("ssh") {
            return Err(refused(""));
        }
        let Some(key) = self
            .git_config(&["--get", "--type=path", "user.signingkey"])?
            .filter(|k| !k.is_empty())
        else {
            return Err(refused(""));
        };
        if key.starts_with("key::") {
            return Ok(());
        }
        // A relative path is taken from the top of the repository, as `sign` takes it.
        let top = PathBuf::from(self.git(&["rev-parse", "--show-toplevel"])?);
        let path = top.join(&key);
        if let Err(e) = std::fs::File::open(&path) {
            return Err(refused(&format!(
                "; user.signingkey names {}, which can't be read: {e}",
                path.display()
            )));
        }
        Ok(())
    }

    /// `body` signed under `namespace`, refused, naming `user.signingkey`, when it can't be.
    pub fn sign(&self, body: Body, namespace: &str) -> Result<Signed, Error> {
        clm_ledger::sign(&self.root, body, namespace, self.env).map_err(|e| match e {
            clm_ledger::Error::Signing(why) if why.contains("user.signingkey") => {
                Error::Refused(why)
            }
            clm_ledger::Error::Signing(why) => {
                Error::Refused(format!("{why} (the key is git's user.signingkey)"))
            }
            e => e.into(),
        })
    }

    /// `git config <args>`, `None` when unset, with git's config read as `env` says, as
    /// `clm-ledger`'s `sign` reads it.
    fn git_config(&self, args: &[&str]) -> Result<Option<String>, Error> {
        let vars: Vec<(&str, OsString)> = ["HOME", "XDG_CONFIG_HOME", "GIT_CONFIG_GLOBAL"]
            .into_iter()
            .filter_map(|name| (self.env)(name).map(|v| (name, OsString::from(v))))
            .collect();
        let vars: Vec<(&str, &std::ffi::OsStr)> =
            vars.iter().map(|(k, v)| (*k, v.as_os_str())).collect();
        let mut full = vec!["config"];
        full.extend_from_slice(args);
        let options = clm_git::Options {
            stdin: None,
            env: &vars,
        };
        match clm_git::run(&self.root, "git", &full, options) {
            Ok(out) => Ok(Some(String::from_utf8_lossy(&out).trim().to_string())),
            // Exit 1: unset.
            Err(clm_git::Error::Failed { status, .. }) if status.code() == Some(1) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}
