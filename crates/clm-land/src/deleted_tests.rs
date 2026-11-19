//! A code branch loses no citing test no approved list names (`land/deleted-tests-approved`,
//! `removal-needs-a-yes`): a land or a `--check-only` run refuses, before anything else, a
//! branch not named `spec/*` whose head holds fewer citing tests of a name than its merge base
//! with `base`, unless a change's `deleted-tests.txt` in `base`'s tree names each.

use std::collections::BTreeSet;
use std::path::PathBuf;

use clm_gate::ListLine;

use crate::{Error, Repo};

const RULE: &str = "land/deleted-tests-approved";

/// Refuses when `head`, against its merge base with `base`, loses a citing test no approved
/// list in `base`'s tree names; one line for each. A branch named `spec/*` isn't judged; a
/// `None` branch, a detached `HEAD`, is.
pub(crate) fn check(
    repo: &Repo,
    branch: Option<&str>,
    base: &str,
    head: &str,
) -> Result<(), Error> {
    if branch.is_some_and(|b| b.starts_with("spec/")) {
        return Ok(());
    }
    let merge_base = repo.git(&["merge-base", base, head])?;
    // The files the branch adds, changes or deletes, relative to the CLM root.
    let changed = repo.git(&[
        "diff",
        "--name-only",
        "--no-renames",
        "--relative",
        "-z",
        &merge_base,
        head,
    ])?;
    let changed: BTreeSet<&str> = changed.split('\0').filter(|p| !p.is_empty()).collect();
    if changed.is_empty() {
        return Ok(());
    }
    let prefix = repo.git(&["rev-parse", "--show-prefix"])?;
    let texts = |rev: &str, keep: &dyn Fn(&str) -> bool| -> Result<Vec<(PathBuf, String)>, Error> {
        let blobs = clm_git::tree_blobs(&repo.root, rev, &|p| {
            p.strip_prefix(prefix.as_str()).is_some_and(keep)
        })?;
        Ok(blobs
            .into_iter()
            .filter_map(|(p, bytes)| {
                let p = p.strip_prefix(prefix.as_str())?.to_string();
                Some((PathBuf::from(p), String::from_utf8(bytes).ok()?))
            })
            .collect())
    };
    let touched = |p: &str| changed.contains(p);
    let gone = clm_coverage::deleted_tests(&texts(&merge_base, &touched)?, &texts(head, &touched)?);
    if gone.is_empty() {
        return Ok(());
    }
    let approved: Vec<ListLine> = texts(base, &is_list)?
        .iter()
        .flat_map(|(_, text)| clm_gate::list_lines(text))
        .collect();
    let lines: Vec<String> = gone
        .iter()
        .filter_map(|t| {
            let (file, name) = t.test.split_once("::")?;
            let bare = clm_coverage::bare_name(name);
            let named = approved
                .iter()
                .any(|l| l.file == file && clm_coverage::bare_name(&l.name) == bare);
            (!named).then(|| {
                format!(
                    "{file}: {RULE}: test {name}, which cited {}, is gone and no approved list \
                     names it. Keep the test under its name, or leave the requirement unbuilt \
                     and add a question item",
                    t.ids.join(", ")
                )
            })
        })
        .collect();
    if lines.is_empty() {
        Ok(())
    } else {
        Err(Error::Refused(lines.join("\n")))
    }
}

/// Whether a path, relative to the CLM root, is a change's list, live or archived:
/// `changes/<id>/deleted-tests.txt` or `changes/archive/<id>/deleted-tests.txt`.
fn is_list(path: &str) -> bool {
    let Some(rest) = path.strip_prefix("changes/") else {
        return false;
    };
    let rest = rest.strip_prefix("archive/").unwrap_or(rest);
    rest.split_once('/')
        .is_some_and(|(id, name)| id != "archive" && name == "deleted-tests.txt")
}

#[cfg(test)]
mod tests {
    use super::is_list;

    #[test]
    fn lists_live_and_archived() {
        // req:land/deleted-tests-approved@f6971347
        assert!(is_list("changes/c/deleted-tests.txt"));
        assert!(is_list("changes/archive/c/deleted-tests.txt"));
        assert!(!is_list("changes/c/deltas/deleted-tests.txt"));
        assert!(!is_list("changes/archive/deleted-tests.txt"));
        assert!(!is_list("docs/c/deleted-tests.txt"));
    }
}
