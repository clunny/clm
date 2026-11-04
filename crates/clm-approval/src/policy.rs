//! The `approval` policy a land counts approvals under (`approval-from-base`): the one
//! committed at the base it lands on, never the checkout's, the head's or the merged tree's,
//! unless the clone's own `clm.local.toml` sets it.

use clm_config::Layer;

use crate::{Error, Repo};

/// The policy and where it came from.
pub struct Policy {
    /// `approval = "other"`: only an approval by someone other than the authors counts.
    pub other: bool,
    /// `origin/main:clm.toml`, `main:clm.toml` or `clm.local.toml`.
    pub source: String,
}

const LOCAL: &str = "clm.local.toml";

impl Repo<'_> {
    /// The policy for a land onto `base`, the commit of the `main` fetched: the clone's
    /// `clm.local.toml` when it sets `approval`, else the top-level `clm.toml` committed at
    /// `base`, `any` when the file or the key is missing. A file that doesn't parse, or a
    /// value other than `any` and `other`, is a settings error (exit 2).
    pub fn policy(&self, base: &str) -> Result<Policy, Error> {
        let setting = &self.approval;
        if matches!(&setting.layer, Layer::File(p) if p.file_name() == Some(LOCAL.as_ref())) {
            return Ok(Policy {
                other: setting.value == clm_config::Approval::Other,
                source: LOCAL.into(),
            });
        }
        let source = format!("{}:clm.toml", self.base());
        let listed = self.git(&[
            "ls-tree",
            "--full-tree",
            "--name-only",
            base,
            "--",
            "clm.toml",
        ])?;
        let value = match listed.is_empty() {
            true => None,
            false => {
                let text = self.git(&["cat-file", "blob", &format!("{base}:clm.toml")])?;
                let table: toml::Table = text
                    .parse()
                    .map_err(|e| Error::Git(format!("approval from {source}: {e}")))?;
                table.get("approval").cloned()
            }
        };
        let other = match &value {
            None => false,
            Some(toml::Value::String(v)) if v == "any" => false,
            Some(toml::Value::String(v)) if v == "other" => true,
            Some(v) => {
                return Err(Error::Git(format!(
                    "approval from {source}: `approval = {v}` is neither \"any\" nor \"other\""
                )));
            }
        };
        Ok(Policy { other, source })
    }
}
