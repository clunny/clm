//! A build holds its own files (`land/build-holds-own-files`): `impl/<component>` changes
//! the files `clm-gate`'s `foreign_files` says its component's build may, and a land or a
//! `--check-only` run refuses before anything else when it changes another's.

use crate::{Error, Repo};

/// The component an `impl/<component>` branch builds: `None` for any other name, and for
/// `impl/archive/<id>`.
fn component_of(branch: &str) -> Option<&str> {
    let component = branch.strip_prefix("impl/")?;
    (!component.is_empty() && !component.starts_with("archive/")).then_some(component)
}

/// Refuses when `branch` is `impl/<component>` and `head`, against `base`, changes a file
/// another component owns; one line for each file.
pub(crate) fn check(repo: &Repo, branch: &str, base: &str, head: &str) -> Result<(), Error> {
    let Some(component) = component_of(branch) else {
        return Ok(());
    };
    let foreign = clm_gate::foreign_files(&repo.root, base, head, component)
        .map_err(|e| Error::Refused(format!("land/build-holds-own-files: {e}")))?;
    if foreign.is_empty() {
        return Ok(());
    }
    let lines: Vec<String> = foreign
        .iter()
        .map(|f| match f.owner.as_str() {
            // No component's: outside the CLM root (`gate/paths-under-the-root`).
            "" => format!(
                "{}: land/build-holds-own-files: outside the CLM root, which no build of the \
                 root's components may change. Leave the requirement unbuilt and add a question \
                 item that blocks it",
                f.path
            ),
            _ => format!(
                "{}: land/build-holds-own-files: owned by `{}`; impl/{component} changes its own \
                 files, the wiring and a capability test it serves. Build this in `{}`, or leave \
                 the requirement unbuilt and add a question item that blocks it",
                f.path, f.owner, f.owner
            ),
        })
        .collect();
    Err(Error::Refused(lines.join("\n")))
}
