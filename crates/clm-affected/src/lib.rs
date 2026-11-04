//! `clm affected`: the nodes and files a change can break (specs/check/affected/SPEC.md).

mod readers;
mod select;

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use clm_git::{Error, Failure, Options};

pub use readers::{readers, reads_in};
pub use select::{Changed, Selection, select};

/// The selection for the working tree of `root` against the merge base of `base` and `HEAD`,
/// with `all` the `[affected] all` globs. Refused when the graph can't be loaded, `base`
/// doesn't resolve to a commit or has no merge base with `HEAD`; `Git` when git fails
/// otherwise. The readers (`readers`) join the files.
pub fn affected(root: &Path, base: &str, all: &[String]) -> Result<Selection, Failure> {
    let (graph, violations) = clm_model::load(root);
    if !violations.is_empty() {
        let lines: Vec<String> = violations.iter().map(clm_check::format).collect();
        return Err(Failure::Refused(lines.join("\n")));
    }
    let differing = differing(root, base)?;
    let scan = scanned(root)?;
    Ok(match select(&graph, &differing, &scan.cited, all) {
        Selection::Nodes { nodes, mut files } => {
            files.extend(readers(&differing, &scan.reads));
            files.sort();
            files.dedup();
            Selection::Nodes { nodes, files }
        }
        everything => everything,
    })
}

/// The readers alone: the files whose reads markers match a path that differs between the
/// merge base of `base` and `HEAD` and the working tree. It loads neither the graph nor
/// `[affected] all`.
pub fn affected_readers(root: &Path, base: &str) -> Result<Vec<String>, Failure> {
    Ok(readers(&differing(root, base)?, &scanned(root)?.reads))
}

/// Every path that differs between the merge base of `base` and `HEAD` and the working tree,
/// untracked files and doc-only paths included, sorted.
fn differing(root: &Path, base: &str) -> Result<Vec<Changed>, Failure> {
    let commit = format!("{base}^{{commit}}");
    match git(root, &["rev-parse", "--verify", "--quiet", &commit]) {
        // `--quiet` makes a ref that resolves to no commit exit 1, and nothing else does.
        Err(Error::Failed { status, .. }) if status.code() == Some(1) => {
            return Err(Failure::Refused(format!("unknown base {base}")));
        }
        other => other?,
    };
    let merge_base = match git(root, &["merge-base", base, "HEAD"]) {
        Ok(out) => String::from_utf8_lossy(&out).trim().to_string(),
        Err(Error::Failed { status, .. }) if status.code() == Some(1) => String::new(),
        Err(e) => return Err(e.into()),
    };
    if merge_base.is_empty() {
        return Err(Failure::Refused(format!(
            "no merge base between {base} and HEAD"
        )));
    }

    let mut paths = names(&git(
        root,
        &["diff", "--name-only", "--no-renames", "-z", &merge_base],
    )?);
    paths.extend(names(&git(
        root,
        &["ls-files", "--others", "--exclude-standard", "-z"],
    )?));
    paths.sort();
    paths.dedup();
    Ok(paths
        .into_iter()
        .map(|path| Changed {
            // A submodule or a nested repository is no file a runner can take.
            exists: root.join(&path).is_file(),
            path,
        })
        .collect())
}

/// What the scan of the files reads.
struct Scan {
    /// Each file with the requirement IDs its `req:` markers cite.
    cited: BTreeMap<String, Vec<String>>,
    /// Each file holding a reads marker, with its globs.
    reads: BTreeMap<String, Vec<String>>,
}

/// The files `check`'s coverage scan reads, tracked or untracked and not ignored, UTF-8 and
/// not Markdown.
fn scanned(root: &Path) -> Result<Scan, Failure> {
    let listed = git(
        root,
        &[
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ],
    )?;
    let mut cited = BTreeMap::new();
    let mut reads = BTreeMap::new();
    for path in names(&listed) {
        let rel = Path::new(&path);
        if rel.extension().is_some_and(|e| e == "md") || !root.join(rel).is_file() {
            continue;
        }
        let Ok(text) = fs::read_to_string(root.join(rel)) else {
            continue; // not UTF-8: not a test source
        };
        let ids = clm_check::markers_in(rel, &text)
            .into_iter()
            .map(|m| m.id)
            .collect();
        cited.insert(path.clone(), ids);
        let globs = reads_in(&text);
        if !globs.is_empty() {
            reads.insert(path, globs);
        }
    }
    Ok(Scan { cited, reads })
}

fn git(root: &Path, args: &[&str]) -> Result<Vec<u8>, Error> {
    clm_git::run(root, "git", args, Options::default())
}

/// The NUL-separated names git printed.
fn names(out: &[u8]) -> Vec<String> {
    out.split(|&b| b == 0)
        .filter(|name| !name.is_empty())
        .map(|name| String::from_utf8_lossy(name).into_owned())
        .collect()
}
