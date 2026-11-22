//! No branch loses a citing test no approved list names (`land/deleted-tests-approved`,
//! `removal-needs-a-yes`): a land or a `--check-only` run refuses, before the lock, a head
//! that, against its merge base with `base`, loses a citing test as
//! `clm_coverage::deleted_tests` counts one gone (a test moved to another file, or renamed in its
//! own file still citing all it cited, is not), unless a change's `deleted-tests.txt` in `base`'s
//! tree names each. Whatever the branch's name: only a replayed rename, which `rename/replay`
//! vouches for, isn't judged. It reads the files of both trees under the CLM root that can hold
//! a test (`check.coverage/test-holding-paths`), the rest holding none on either side, so it
//! gives the verdict `clm coverage --deleted-tests` gives on a committed branch.

use std::path::{Path, PathBuf};

use clm_gate::ListLine;

use crate::{Error, Repo};

const RULE: &str = "land/deleted-tests-approved";

/// Refuses when `head`, against its merge base with `base`, loses a citing test no approved
/// list in `base`'s tree names; one line for each. A head `rename/replay` accepts from the
/// merge base isn't judged, and asking it fails as git does.
pub(crate) fn check(repo: &Repo, base: &str, head: &str) -> Result<(), Error> {
    // With no history in common there is nothing the branch removed from it; a land is
    // refused at its merge anyway.
    let Ok(merge_base) = repo.git(&["merge-base", base, head]) else {
        return Ok(());
    };
    // A branch that touches no file under the CLM root that can hold a test loses nothing,
    // and reads nothing.
    let changed = repo.git(&[
        "diff",
        "--name-only",
        "--no-renames",
        "--relative",
        "-z",
        &merge_base,
        head,
    ])?;
    if !changed
        .split('\0')
        .any(|p| !p.is_empty() && clm_coverage::holds_tests(Path::new(p)))
    {
        return Ok(());
    }
    // A replayed rename's rewritten markers are what the replay vouches for, wherever they sit.
    if clm_rename::replays(&repo.root, &merge_base, head).map_err(|e| Error::Git(e.to_string()))? {
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
                // A file that isn't UTF-8 holds no test, as `clm coverage` reads it.
                Some((PathBuf::from(p), String::from_utf8(bytes).ok()?))
            })
            .collect())
    };
    // Every file of both trees that can hold a test: an untouched file makes no test gone, but
    // its tests' names decide whether a touched file's is new. The head is the branch's
    // commit, never a working tree, so an untracked file counts for nothing here.
    let tests = |p: &str| clm_coverage::holds_tests(Path::new(p));
    let gone = clm_coverage::deleted_tests(&texts(&merge_base, &tests)?, &texts(head, &tests)?);
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
            // A test always reads `<file>::<name>`; one that didn't would still be named.
            let (file, name) = t.test.split_once("::").unwrap_or(("", &t.test));
            let bare = clm_coverage::bare_name(name);
            let named = approved
                .iter()
                .any(|l| l.file == file && clm_coverage::bare_name(&l.name) == bare);
            (!named).then(|| {
                format!(
                    "{file}: {RULE}: test {name}, which cited {}, is gone and no approved list \
                     names it. Keep the test under its name, or under a name no citing test \
                     carried, in its file, citing every requirement it cited, or leave the \
                     requirement unbuilt and add a question item",
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
        // req:land/deleted-tests-approved@90e1578b
        assert!(is_list("changes/c/deleted-tests.txt"));
        assert!(is_list("changes/archive/c/deleted-tests.txt"));
        assert!(!is_list("changes/c/deltas/deleted-tests.txt"));
        assert!(!is_list("changes/archive/deleted-tests.txt"));
        assert!(!is_list("docs/c/deleted-tests.txt"));
    }
}
