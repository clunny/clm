//! `fold(root)`: the fold over the ledger branches in a clone.

use std::path::Path;

use crate::{Error, Fold, branch, fold_records};

/// The fold over every `origin/clm/ledger/*` ref and this clone's own local ledger branch,
/// which may be ahead. Doesn't fetch.
pub fn fold(root: &Path) -> Result<Fold, Error> {
    let mut refs = list(root, &remote_ledgers())?;
    // The local branches whose clone part is this clone's: the writer's, under whatever
    // person part it has had.
    if let Some(clone) = crate::writer::clone_id(root)? {
        let suffix = format!("/{clone}");
        refs.extend(
            list(root, "refs/heads/clm/ledger/")?
                .into_iter()
                .filter(|r| r.ends_with(&suffix)),
        );
    }
    let mut texts = Vec::new();
    let mut warnings = Vec::new();
    for name in &refs {
        let short = name
            .strip_prefix("refs/remotes/")
            .or_else(|| name.strip_prefix("refs/heads/"))
            .unwrap_or(name);
        match branch::read(root, name) {
            Ok(bytes) => texts.push((
                short.to_string(),
                String::from_utf8_lossy(&bytes).into_owned(),
            )),
            Err(e) => warnings.push(format!("{short}: skipped, no {}: {e}", branch::FILE)),
        }
    }
    let sources: Vec<(&str, &str)> = texts
        .iter()
        .map(|(r, t)| (r.as_str(), t.as_str()))
        .collect();
    let mut fold = fold_records(&sources);
    fold.warnings.splice(0..0, warnings);
    Ok(fold)
}

/// Where the remote's ledger branches are tracked: `refs/remotes/<REMOTE>/clm/ledger/`.
pub(crate) fn remote_ledgers() -> String {
    format!("refs/remotes/{}/clm/ledger/", clm_git::REMOTE)
}

/// The refs under `prefix`.
pub(crate) fn list(root: &Path, prefix: &str) -> Result<Vec<String>, Error> {
    let out = clm_git::git(root, &["for-each-ref", "--format=%(refname)", prefix])?;
    Ok(out.lines().map(str::to_string).collect())
}
