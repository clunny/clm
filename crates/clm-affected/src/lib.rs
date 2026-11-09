//! `clm affected`: the nodes and files a change can break (specs/check/affected/SPEC.md).

mod readers;
mod select;
mod why;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use clm_git::{Error, Failure, Options};
use clm_model::Graph;

pub use readers::{readers, reads_in};
pub use select::{Changed, Selection, overlap, select};
pub use why::why_chain;

/// The selection for the working tree of `root` against the merge base of `base` and `HEAD`,
/// with `all` the `[affected] all` globs. Refused when the graph can't be loaded, `base`
/// doesn't resolve to a commit or has no merge base with `HEAD`; `Git` when git fails
/// otherwise. The readers (`readers`) join the files.
pub fn affected(root: &Path, base: &str, all: &[String]) -> Result<Selection, Failure> {
    let (graph, differing, scan) = inputs(root, base)?;
    Ok(combined(&graph, &differing, &scan.cited, &scan.reads, all))
}

/// The lines saying why the change selects `path` (`why_chain`), with `all` the
/// `[affected] all` globs as for `affected`. Refused as `affected` is.
pub fn why(root: &Path, base: &str, path: &str, all: &[String]) -> Result<Vec<String>, Failure> {
    let (graph, differing, scan) = inputs(root, base)?;
    Ok(why_chain(
        &graph,
        &differing,
        &scan.cited,
        &scan.reads,
        all,
        path,
    ))
}

/// The selection with the readers (`readers`) joined to its files.
fn combined(
    graph: &Graph,
    differing: &[Changed],
    cited: &BTreeMap<String, Vec<String>>,
    reads: &BTreeMap<String, Vec<String>>,
    all: &[String],
) -> Selection {
    match select(graph, differing, cited, all) {
        Selection::Nodes { nodes, mut files } => {
            files.extend(readers(differing, reads));
            files.sort();
            files.dedup();
            Selection::Nodes { nodes, files }
        }
        everything => everything,
    }
}

/// The graph, the differing paths and the scan, all refused as `affected` is.
fn inputs(root: &Path, base: &str) -> Result<(Graph, Vec<Changed>, Scan), Failure> {
    Ok((load_graph(root)?, differing(root, base)?, scanned(root)?))
}

/// The graph of the specs under `root`, refused with its violations when it has any.
fn load_graph(root: &Path) -> Result<Graph, Failure> {
    let (graph, violations) = clm_model::load(root);
    if !violations.is_empty() {
        let lines: Vec<String> = violations.iter().map(clm_check::format).collect();
        return Err(Failure::Refused(lines.join("\n")));
    }
    Ok(graph)
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
    resolve(root, base, &format!("unknown base {base}"))?;
    let merge_base = merge_base(root, base, "HEAD", &format!("{base} and HEAD"))?;

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

/// `rev` as a commit, or `Refused(unknown)`.
fn resolve(root: &Path, rev: &str, unknown: &str) -> Result<(), Failure> {
    let commit = format!("{rev}^{{commit}}");
    match git(root, &["rev-parse", "--verify", "--quiet", &commit]) {
        // `--quiet` makes a ref that resolves to no commit exit 1, and nothing else does.
        Err(Error::Failed { status, .. }) if status.code() == Some(1) => {
            Err(Failure::Refused(unknown.to_string()))
        }
        other => other.map(drop).map_err(Failure::from),
    }
}

/// The merge base of `a` and `b`, or `Refused` naming `between` when they have none.
fn merge_base(root: &Path, a: &str, b: &str, between: &str) -> Result<String, Failure> {
    let base = match git(root, &["merge-base", a, b]) {
        Ok(out) => String::from_utf8_lossy(&out).trim().to_string(),
        Err(Error::Failed { status, .. }) if status.code() == Some(1) => String::new(),
        Err(e) => return Err(e.into()),
    };
    if base.is_empty() {
        return Err(Failure::Refused(format!("no merge base between {between}")));
    }
    Ok(base)
}

/// What the scan of the files reads.
#[derive(Default)]
struct Scan {
    /// Each file with the requirement IDs its `req:` markers cite.
    cited: BTreeMap<String, Vec<String>>,
    /// Each file holding a reads marker, with its globs.
    reads: BTreeMap<String, Vec<String>>,
}

impl Scan {
    /// Records what the scan of one file's `text` finds.
    fn add(&mut self, path: String, text: &str) {
        let ids = clm_check::markers_in(Path::new(&path), text)
            .into_iter()
            .map(|m| m.id)
            .collect();
        self.cited.insert(path.clone(), ids);
        let globs = reads_in(text);
        if !globs.is_empty() {
            self.reads.insert(path, globs);
        }
    }
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
    let mut scan = Scan::default();
    for path in names(&listed) {
        let rel = Path::new(&path);
        if rel.extension().is_some_and(|e| e == "md") || !root.join(rel).is_file() {
            continue;
        }
        let Ok(text) = fs::read_to_string(root.join(rel)) else {
            continue; // not UTF-8: not a test source
        };
        scan.add(path, &text);
    }
    Ok(scan)
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

/// The selection for the paths that differ between the commits `from` and `to`, read from
/// the two commits alone: no working tree, no untracked file. The graph, the scan and the
/// files that still exist are those of `to`'s tree. Refused as `affected` is when the graph
/// can't be loaded.
pub fn affected_between(
    root: &Path,
    from: &str,
    to: &str,
    all: &[String],
) -> Result<Selection, Failure> {
    let paths = names(&git(
        root,
        &["diff", "--name-only", "--no-renames", "-z", from, to],
    )?);
    let wanted: BTreeSet<&str> = paths.iter().map(String::as_str).collect();
    let blobs = clm_git::tree_blobs(root, to, &|p| !p.ends_with(".md") || wanted.contains(p))?;
    let mut scan = Scan::default();
    let mut present = BTreeSet::new();
    for (path, bytes) in blobs {
        present.insert(path.clone());
        if path.ends_with(".md") {
            continue;
        }
        if let Ok(text) = String::from_utf8(bytes) {
            scan.add(path, &text);
        }
    }
    let differing: Vec<Changed> = paths
        .iter()
        .map(|path| Changed {
            exists: present.contains(path),
            path: path.clone(),
        })
        .collect();
    let specs = clm_git::extract_paths(root, to, &["specs"])?;
    let graph = load_graph(specs.path())?;
    Ok(combined(&graph, &differing, &scan.cited, &scan.reads, all))
}

/// What changing both `a` and `b` can break (`overlap`): each side's selection against
/// their merge base. Refused with `unknown revision <rev>` for a revision that is no commit
/// and `no merge base between <a> and <b>` for commits with none.
pub fn affected_overlap(
    root: &Path,
    a: &str,
    b: &str,
    all: &[String],
) -> Result<Selection, Failure> {
    for rev in [a, b] {
        resolve(root, rev, &format!("unknown revision {rev}"))?;
    }
    let base = merge_base(root, a, b, &format!("{a} and {b}"))?;
    Ok(overlap(
        affected_between(root, &base, a, all)?,
        affected_between(root, &base, b, all)?,
    ))
}
