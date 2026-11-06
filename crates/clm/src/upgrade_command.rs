//! `clm upgrade`: which `clm-upgrade` call each flag makes (`upgrade-command`).

use std::path::Path;

/// What `clm upgrade` was asked to do: at most one of its flags is set.
pub(crate) struct Request {
    pub apply: bool,
    pub note: Option<String>,
    pub decline: Option<String>,
    pub lock: Option<String>,
}

/// The text to print, or the refusal to print with exit status 1.
pub(crate) fn run(root: &Path, request: Request) -> Result<String, String> {
    if let Some(id) = request.note {
        clm_upgrade::note(root, &id)
    } else if let Some(id) = request.decline {
        clm_upgrade::decline(root, &id)
    } else if let Some(version) = request.lock {
        lock(root, &version)
    } else if request.apply {
        clm_upgrade::apply(root)
    } else {
        clm_upgrade::list(root)
    }
}

/// `--lock`: refuses a project too new for this binary, as the other flags do, then locks the
/// migrations of this repository's `clm-upgrade` crate.
fn lock(root: &Path, version: &str) -> Result<String, String> {
    clm_upgrade::list(root)?;
    let ids: Vec<&str> = clm_upgrade::migrations().iter().map(|m| m.id).collect();
    let added = clm_upgrade::append_lock(&root.join("crates/clm-upgrade"), version, &ids)?;
    Ok(added.iter().map(|id| format!("locked {id}\n")).collect())
}
