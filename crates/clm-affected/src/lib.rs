//! `clm affected`: the components and files a change can break (specs/check/affected/SPEC.md).

mod command;
mod readers;
mod select;
mod why;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use clm_git::{Error, Failure, Options};
use clm_model::Graph;

pub use command::{Args, Settings, command};
pub use readers::{readers, reads_in};
use select::overlap;
pub use select::{Changed, Selection, select};
pub use why::why_chain;

/// The selection for the working tree of `root` against the merge base of `base` and `HEAD`,
/// with `all` the `[affected] all` globs. Refused when the graph can't be loaded, `base`
/// doesn't resolve to a commit or has no merge base with `HEAD`; `Git` when git fails
/// otherwise. The readers (`readers`) join the files.
pub fn affected(root: &Path, base: &str, all: &[String]) -> Result<Selection, Failure> {
    let (graph, differing, scan) = inputs(root, base)?;
    Ok(combined(
        &graph,
        &differing,
        &scan.cited,
        &scan.unread,
        &scan.reads,
        all,
    ))
}

/// The lines saying why the change selects `path` (`why_chain`), with `all` the
/// `[affected] all` globs as for `affected`. Refused as `affected` is.
pub(crate) fn why(
    root: &Path,
    base: &str,
    path: &str,
    all: &[String],
) -> Result<Vec<String>, Failure> {
    let (graph, differing, scan) = inputs(root, base)?;
    Ok(why_chain(
        &graph,
        &differing,
        &scan.cited,
        &scan.unread,
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
    unread: &[String],
    reads: &BTreeMap<String, Vec<String>>,
    all: &[String],
) -> Selection {
    match select(graph, differing, cited, unread, all) {
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
pub(crate) fn affected_readers(root: &Path, base: &str) -> Result<Vec<String>, Failure> {
    Ok(readers(&differing(root, base)?, &scanned(root)?.reads))
}

/// Every path under `root` that differs between the merge base of `base` and `HEAD` and the
/// working tree, untracked files and doc-only paths included, named from `root` and sorted
/// (`clm-root-paths`).
fn differing(root: &Path, base: &str) -> Result<Vec<Changed>, Failure> {
    resolve(root, base, &format!("unknown base {base}"))?;
    let merge_base = merge_base(root, base, "HEAD", &format!("{base} and HEAD"))?;

    let mut paths = names(&git(
        root,
        &[
            "diff",
            "--name-only",
            "--no-renames",
            "--relative",
            "-z",
            &merge_base,
        ],
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
    if rev.starts_with('-') {
        return Err(Failure::Refused(unknown.to_string()));
    }
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
    /// The listed files, Markdown left out, that can't be read as UTF-8 text: only an
    /// exemption selects them (`affected-files`).
    unread: Vec<String>,
}

impl Scan {
    /// Records what the scan of one file's `text` finds.
    fn add(&mut self, path: String, text: &str) {
        let ids = cited_ids(text);
        self.cited.insert(path.clone(), ids);
        let globs = reads_in(text);
        if !globs.is_empty() {
            self.reads.insert(path, globs);
        }
    }
}

/// The requirement IDs every `req:` marker of a file's text cites, inside a test or outside one
/// (`affected-files`): a file is selected for the component it cites wherever its marker sits.
fn cited_ids(text: &str) -> Vec<String> {
    clm_coverage::marker_spans(text)
        .into_iter()
        .map(|span| span.id)
        .collect()
}

/// The files `check.coverage`'s scan reads, tracked or untracked and not ignored, UTF-8 and
/// not Markdown, with the listed non-Markdown files it can't read as UTF-8 kept apart.
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
        match fs::read_to_string(root.join(rel)) {
            Ok(text) => scan.add(path, &text),
            Err(_) => scan.unread.push(path),
        }
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
/// files that still exist are those of `to`'s tree, each read at the CLM root's place in it,
/// and every path is under `root` and named from it (`clm-root-paths`). A symbolic link of
/// `to` counts as a file when it leads to one as a checkout's link would, judged in the tree
/// `to` extracts to (`leads_to_file`). Refused as `affected` is when the graph can't be
/// loaded.
pub(crate) fn affected_between(
    root: &Path,
    from: &str,
    to: &str,
    all: &[String],
) -> Result<Selection, Failure> {
    let paths = names(&git(
        root,
        &[
            "diff",
            "--name-only",
            "--no-renames",
            "--relative",
            "-z",
            from,
            to,
        ],
    )?);
    let prefix = show_prefix(root)?;
    let wanted: BTreeSet<&str> = paths.iter().map(String::as_str).collect();
    let keep = |p: &str| !p.ends_with(".md") || wanted.contains(p);
    // The whole tree, since the graph also reads `changes/`, `backlog/` and the human layer,
    // and a link is judged where it leads in it.
    let tree = clm_git::extract_tree(root, to)?;
    let at_root = tree.path().join(&prefix);
    let mut scan = Scan::default();
    let mut present = BTreeSet::new();
    let blobs = clm_git::tree_blobs(root, to, &|p| {
        p.strip_prefix(prefix.as_str()).is_some_and(keep)
    })?;
    for (path, bytes) in blobs {
        let Some(path) = path.strip_prefix(prefix.as_str()).map(str::to_string) else {
            continue;
        };
        present.insert(path.clone());
        if path.ends_with(".md") {
            continue;
        }
        match String::from_utf8(bytes) {
            Ok(text) => scan.add(path, &text),
            Err(_) => scan.unread.push(path),
        }
    }
    for link in links(root, to, &prefix)? {
        let target = at_root.join(&link);
        if !keep(&link) || !leads_to_file(tree.path(), &target) {
            continue;
        }
        present.insert(link.clone());
        if link.ends_with(".md") {
            continue;
        }
        match fs::read_to_string(&target) {
            Ok(text) => scan.add(link, &text),
            Err(_) => scan.unread.push(link),
        }
    }
    let differing: Vec<Changed> = paths
        .iter()
        .map(|path| Changed {
            exists: present.contains(path),
            path: path.clone(),
        })
        .collect();
    let graph = load_graph(&at_root)?;
    Ok(combined(
        &graph,
        &differing,
        &scan.cited,
        &scan.unread,
        &scan.reads,
        all,
    ))
}

/// Whether the link at `path`, in the tree extracted at `top`, leads to a file as a checkout's
/// link would: links to links and absolute targets followed, except that a relative target
/// leading out of `top`, the repository's top, leads to nothing (`overlap-selection`).
fn leads_to_file(top: &Path, path: &Path) -> bool {
    let mut current = path.to_path_buf();
    // As many hops as the kernel follows before it gives up on a loop.
    for _ in 0..40 {
        let Ok(meta) = fs::symlink_metadata(&current) else {
            return false;
        };
        if !meta.file_type().is_symlink() {
            return meta.is_file();
        }
        let Ok(target) = fs::read_link(&current) else {
            return false;
        };
        if target.is_absolute() {
            current = target;
            continue;
        }
        let inside = current.starts_with(top);
        let mut next = current.parent().map(Path::to_path_buf).unwrap_or_default();
        for part in target.components() {
            match part {
                std::path::Component::ParentDir => {
                    next.pop();
                }
                std::path::Component::CurDir => {}
                other => next.push(other),
            }
        }
        if inside && !next.starts_with(top) {
            return false;
        }
        current = next;
    }
    false
}

/// The CLM root's place in its git repository: `git rev-parse --show-prefix`, empty at the
/// top and ending in `/` below it.
fn show_prefix(root: &Path) -> Result<String, Failure> {
    let out = git(root, &["rev-parse", "--show-prefix"])?;
    Ok(String::from_utf8_lossy(&out)
        .trim_end_matches('\n')
        .to_string())
}

/// The symbolic links (mode `120000`) of `commit` under `prefix`, named from it.
fn links(root: &Path, commit: &str, prefix: &str) -> Result<Vec<String>, Failure> {
    let listing = git(root, &["ls-tree", "-r", "-z", "--full-tree", commit])?;
    Ok(names(&listing)
        .into_iter()
        .filter_map(|entry| {
            // "<mode> SP <type> SP <oid> TAB <path>"
            let (meta, path) = entry.split_once('\t')?;
            if !meta.starts_with("120000 ") {
                return None;
            }
            path.strip_prefix(prefix).map(str::to_string)
        })
        .collect())
}

/// The `[affected] all` globs of the `clm.toml` at the CLM root's place in `rev`'s tree, or
/// of none when it holds no such file, loaded as `config` loads a committed `clm.toml`
/// (`config/committed-settings`), the clone's own layers read as any loading reads them.
/// Refused with `<rev>:<path>: <reason>` when that file can't be loaded, and with the error
/// loading gives when another layer can't.
fn committed_globs(
    root: &Path,
    rev: &str,
    prefix: &str,
    env: &dyn Fn(&str) -> Option<String>,
) -> Result<Vec<String>, Failure> {
    let path = format!("{prefix}clm.toml");
    let source = format!("{rev}:{path}");
    let text = match clm_git::tree_blobs(root, rev, &|p| p == path)?.pop() {
        Some((_, bytes)) => Some(
            String::from_utf8(bytes)
                .map_err(|_| Failure::Refused(format!("{source}: not UTF-8 text")))?,
        ),
        None => None,
    };
    clm_config::load_committed(root, text.as_deref(), &source, env)
        .map(|settings| settings.affected.value.map(|a| a.all).unwrap_or_default())
        .map_err(|e| Failure::Refused(e.to_string()))
}

/// What changing both `a` and `b` can break (`overlap`): each side's selection against
/// their merge base, with the `[affected] all` globs its own commit holds. Refused with
/// `unknown revision <rev>` for a revision that is no commit, `no merge base between <a> and
/// <b>` for commits with none, and then as each side's settings and graph are, side `a`'s
/// first (`overlap-selection`).
pub(crate) fn affected_overlap(
    root: &Path,
    a: &str,
    b: &str,
    env: &dyn Fn(&str) -> Option<String>,
) -> Result<Selection, Failure> {
    for rev in [a, b] {
        resolve(root, rev, &format!("unknown revision {rev}"))?;
    }
    let base = merge_base(root, a, b, &format!("{a} and {b}"))?;
    let prefix = show_prefix(root)?;
    let side = |rev: &str| {
        let all = committed_globs(root, rev, &prefix, env)?;
        affected_between(root, &base, rev, &all)
    };
    let first = side(a)?;
    Ok(overlap(first, side(b)?))
}

#[cfg(test)]
mod cited_ids_tests {
    use super::cited_ids;

    #[test]
    fn a_marker_outside_a_test_is_cited_like_one_inside() {
        // req:check.affected/affected-files@72f3baff
        let marker = |id: &str| format!("// {}{id}", concat!("req", ":"));
        let text = format!(
            "{}\nfn helper() {{}}\n\n#[test]\nfn t() {{\n    {}\n}}\n",
            marker("a/outside"),
            marker("a/inside@0123abcd")
        );
        assert_eq!(cited_ids(&text), ["a/outside", "a/inside"]);
    }
}

/// `overlap`: what two selections can both break, and `affected_between`, the selection
/// between two commits it is applied to.
#[cfg(test)]
mod overlap_tests {
    use std::fs;
    use std::path::Path;
    use std::process::Command;

    use tempfile::TempDir;

    use super::{Selection, affected_between, overlap};

    fn nodes(nodes: &[&str], files: &[&str]) -> Selection {
        Selection::Nodes {
            nodes: nodes.iter().map(|s| s.to_string()).collect(),
            files: files.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn all(path: &str) -> Selection {
        Selection::All(path.to_string())
    }

    #[test]
    fn both_everything_gives_the_first() {
        // req:check.affected/overlap-selection@bc565b43
        assert_eq!(overlap(all("a"), all("b")), all("a"));
    }

    #[test]
    fn one_everything_gives_the_other_unchanged() {
        // req:check.affected/overlap-selection@bc565b43
        let other = nodes(&["c"], &["f"]);
        assert_eq!(overlap(all("a"), other.clone()), other);
        assert_eq!(overlap(other.clone(), all("a")), other);
    }

    #[test]
    fn otherwise_the_components_and_files_both_hold() {
        // req:check.affected/overlap-selection@bc565b43
        let a = nodes(&["a", "c", "d"], &["f1", "f2", "f4"]);
        let b = nodes(&["b", "c", "d"], &["f2", "f3", "f4"]);
        assert_eq!(overlap(a, b), nodes(&["c", "d"], &["f2", "f4"]));
        assert_eq!(
            overlap(nodes(&["a"], &["x"]), nodes(&["b"], &[])),
            nodes(&[], &[])
        );
    }

    fn write(root: &Path, rel: &str, content: &str) {
        let path = root.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    fn git(root: &Path, args: &[&str]) {
        let out = Command::new("git")
            .args(["-c", "user.name=t", "-c", "user.email=t@t"])
            .args(["-c", "commit.gpgsign=false"])
            .args(args)
            .current_dir(root)
            .output()
            .unwrap();
        assert!(out.status.success(), "git {args:?}: {out:?}");
    }

    fn commit_all(root: &Path, message: &str) {
        git(root, &["add", "-A"]);
        git(root, &["commit", "-q", "-m", message]);
    }

    fn spec(paths: &str, depends_on: &str) -> String {
        format!(
            "---\ntitle: N\nstatus: active\nowners: []\npaths: {paths}\ndepends_on: {depends_on}\n---\n"
        )
    }

    #[test]
    fn affected_between_gives_the_selection_between_two_commits() {
        // req:check.affected/overlap-selection@bc565b43
        // `a`, `b` on `a`, and `c` on nothing; `main`, then `x` changing `a`'s file and `y`
        // changing `b`'s, each from `main`.
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        git(root, &["init", "-q", "-b", "main"]);
        write(root, "clm.toml", "");
        write(root, "specs/SPEC.md", &spec("[\"clm.toml\"]", "[]"));
        write(root, "specs/a/SPEC.md", &spec("[\"crates/a/**\"]", "[]"));
        write(root, "specs/b/SPEC.md", &spec("[\"crates/b/**\"]", "[a]"));
        write(root, "specs/c/SPEC.md", &spec("[\"crates/c/**\"]", "[]"));
        for id in ["a", "b", "c"] {
            write(root, &format!("crates/{id}/src/lib.rs"), "fn f() {}\n");
        }
        commit_all(root, "start");
        git(root, &["switch", "-q", "-c", "x"]);
        write(root, "crates/a/src/lib.rs", "fn changed() {}\n");
        commit_all(root, "x");
        git(root, &["switch", "-q", "-c", "y", "main"]);
        write(root, "crates/b/src/lib.rs", "fn changed() {}\n");
        commit_all(root, "y");
        // The working tree, `y` checked out with `c` changed and a file untracked, is not read.
        write(root, "crates/c/src/lib.rs", "fn uncommitted() {}\n");
        write(root, "crates/c/src/new.rs", "fn untracked() {}\n");

        let x = affected_between(root, "main", "x", &[]).unwrap();
        // `a` changed, and `b` depends on it; each owns its file.
        assert_eq!(
            x,
            nodes(&["a", "b"], &["crates/a/src/lib.rs", "crates/b/src/lib.rs"])
        );
        let y = affected_between(root, "main", "y", &[]).unwrap();
        assert_eq!(y, nodes(&["b"], &["crates/b/src/lib.rs"]));
        // What both can break.
        assert_eq!(overlap(x, y), nodes(&["b"], &["crates/b/src/lib.rs"]));
        // The `[affected] all` globs given select everything for a change they match.
        assert_eq!(
            affected_between(root, "main", "x", &["crates/a/**".to_string()]).unwrap(),
            all("crates/a/src/lib.rs")
        );
    }
}
