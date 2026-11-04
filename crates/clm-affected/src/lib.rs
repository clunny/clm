//! `clm affected`: the nodes and files a change can break (specs/check/affected/SPEC.md).

mod select;

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use clm_git::{Error, Failure, Options};

pub use select::{Changed, Selection, select};

/// The selection for the working tree of `root` against the merge base of `base` and `HEAD`,
/// with `all` the `[affected] all` globs. Refused when the graph can't be loaded, `base`
/// doesn't resolve to a commit or has no merge base with `HEAD`; `Git` when git fails
/// otherwise.
pub fn affected(root: &Path, base: &str, all: &[String]) -> Result<Selection, Failure> {
    let (graph, violations) = clm_model::load(root);
    if !violations.is_empty() {
        let lines: Vec<String> = violations.iter().map(clm_check::format).collect();
        return Err(Failure::Refused(lines.join("\n")));
    }
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
    let changed: Vec<Changed> = paths
        .into_iter()
        .map(|path| Changed {
            // A submodule or a nested repository is no file a runner can take.
            exists: root.join(&path).is_file(),
            path,
        })
        .collect();

    Ok(select(&graph, &changed, &scanned(root)?, all))
}

/// The files `check`'s coverage scan reads, tracked or untracked and not ignored, UTF-8 and
/// not Markdown, each with the requirement IDs its `req:` markers cite.
fn scanned(root: &Path) -> Result<BTreeMap<String, Vec<String>>, Failure> {
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
    let mut files = BTreeMap::new();
    for path in names(&listed) {
        let rel = Path::new(&path);
        if rel.extension().is_some_and(|e| e == "md") || !root.join(rel).is_file() {
            continue;
        }
        let Ok(text) = fs::read_to_string(root.join(rel)) else {
            continue; // not UTF-8: not a test source
        };
        let cited = clm_check::markers_in(rel, &text)
            .into_iter()
            .map(|m| m.id)
            .collect();
        files.insert(path, cited);
    }
    Ok(files)
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
