//! The lists `clm coverage` prints beside its report (`coverage-lists`): the markers no
//! test counts, the tests a removal leaves with nothing to cite, and the tests a branch
//! deleted. The computing is `clm-coverage`'s; this reads a checkout's files and formats.

use std::fs;
use std::path::{Path, PathBuf};

use clm_model::{Graph, Violation};

use clm_coverage::{deleted_tests, obsolete_tests, scan, uncounted};

const RULE: &str = "check/coverage-lists";

/// `clm coverage`'s lines: each requirement's coverage, then one
/// `<id> uncounted <file>:<line>[ ...]` line per entry whose markers stand outside every
/// test. `Err` carries spec violations that make the report unreliable.
pub(crate) fn coverage_lines(root: &Path) -> Result<Vec<String>, Vec<Violation>> {
    let mut lines: Vec<String> = crate::coverage(root)?
        .iter()
        .map(ToString::to_string)
        .collect();
    let graph = load(root)?;
    for entry in uncounted(&graph, &scan(root).outside) {
        let at: Vec<String> = entry.markers.iter().map(ToString::to_string).collect();
        lines.push(format!("{} uncounted {}", entry.id, at.join(" ")));
    }
    Ok(lines)
}

/// `clm coverage --obsolete-tests`: a count line, then a `DELETE` or `UNCITE` line per test
/// holding a marker that cites what a live change removed.
pub(crate) fn obsolete_tests_lines(root: &Path) -> Result<Vec<String>, Vec<Violation>> {
    let graph = load(root)?;
    let tests = obsolete_tests(&graph, &read_files(root));
    let delete = tests.iter().filter(|t| t.every).count();
    let mut lines = vec![format!(
        "# tests to delete: {delete}; tests that only lose a marker: {}",
        tests.len() - delete
    )];
    lines.extend(tests.iter().map(|t| {
        if t.every {
            format!(
                "DELETE {}\tcites only removed: {}",
                t.test,
                t.ids.join(", ")
            )
        } else {
            format!("UNCITE {}\tloses: {}", t.test, t.ids.join(", "))
        }
    }));
    Ok(lines)
}

/// `clm coverage --deleted-tests <base>`: a count line, then a `DELETED` line per citing test
/// of the revision `base` that `clm_coverage::deleted_tests` finds gone from the working tree,
/// counted by bare name across files, both sides read under the CLM root and named from it.
/// `Err` names `base` when it resolves to no commit.
pub(crate) fn deleted_tests_lines(root: &Path, base: &str) -> Result<Vec<String>, Vec<Violation>> {
    load(root)?;
    let fail = |why: String| {
        vec![Violation {
            path: PathBuf::from("."),
            rule: RULE,
            message: why,
        }]
    };
    let commit = clm_git::git(
        root,
        &[
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("{base}^{{commit}}"),
        ],
    )
    .map_err(|_| fail(format!("`{base}` is no commit")))?;
    // The revision's files under the CLM root, named from it as the working tree's are and as
    // `clm land` names them (`land/deleted-tests-approved`).
    let prefix = clm_git::git(root, &["rev-parse", "--show-prefix"])
        .map_err(|e| fail(format!("can't place the CLM root in its repository: {e}")))?;
    let blobs = clm_git::tree_blobs(root, &commit, &|p| {
        p.strip_prefix(prefix.as_str())
            .is_some_and(|p| !p.ends_with(".md"))
    })
    .map_err(|e| fail(format!("can't read `{base}`: {e}")))?;
    let before: Vec<(PathBuf, String)> = blobs
        .into_iter()
        .filter_map(|(p, bytes)| {
            let p = p.strip_prefix(prefix.as_str())?;
            Some((PathBuf::from(p), String::from_utf8(bytes).ok()?))
        })
        .collect();
    let gone = deleted_tests(&before, &read_files(root));
    let mut lines = vec![format!("# tests deleted since {base}: {}", gone.len())];
    lines.extend(
        gone.iter()
            .map(|t| format!("DELETED {}\t{}", t.test, t.ids.join(", "))),
    );
    Ok(lines)
}

fn load(root: &Path) -> Result<Graph, Vec<Violation>> {
    let (graph, violations) = clm_model::load(root);
    if violations.is_empty() {
        Ok(graph)
    } else {
        Err(violations)
    }
}

/// The text of every file `scan` reads, by repository-relative path.
fn read_files(root: &Path) -> Vec<(PathBuf, String)> {
    let mut paths = clm_git::run(
        root,
        "git",
        &[
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ],
        clm_git::Options::default(),
    )
    .map(|out| {
        out.split(|b| *b == 0)
            .filter(|s| !s.is_empty())
            .map(|s| PathBuf::from(String::from_utf8_lossy(s).into_owned()))
            .collect()
    })
    .unwrap_or_else(|_| {
        let mut all = Vec::new();
        walk(root, root, &mut all);
        all
    });
    paths.sort();
    paths
        .into_iter()
        .filter_map(|p| Some((p.clone(), fs::read_to_string(root.join(&p)).ok()?)))
        .collect()
}

fn walk(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if path.is_dir() {
            if !name.starts_with('.') && name != "target" {
                walk(root, &path, out);
            }
        } else if let Ok(rel) = path.strip_prefix(root) {
            out.push(rel.to_path_buf());
        }
    }
}
