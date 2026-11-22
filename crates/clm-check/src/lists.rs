//! The lists `clm coverage` prints beside its report (`coverage-lists`): the markers no
//! test counts, the tests a removal leaves with nothing to cite, and the tests a branch
//! deleted. The computing and the reading of the working tree's files are `clm-coverage`'s;
//! this reads the base revision's files and formats.

use std::fs;
use std::path::{Path, PathBuf};

use clm_model::{Graph, Violation};

use clm_coverage::{deleted_tests, obsolete_tests, read_files, scan, uncounted};

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
        // Markdown, the extension `md` exactly, holds no test (`markers-in-tests`).
        p.strip_prefix(prefix.as_str())
            .is_some_and(|p| Path::new(p).extension().is_none_or(|e| e != "md"))
    })
    .map_err(|e| fail(format!("can't read `{base}`: {e}")))?;
    let before: Vec<(PathBuf, String)> = blobs
        .into_iter()
        .filter_map(|(p, bytes)| {
            let p = p.strip_prefix(prefix.as_str())?;
            Some((PathBuf::from(p), String::from_utf8(bytes).ok()?))
        })
        .collect();
    // `tree_blobs` skips the revision's links (`vcs/tree-blobs`), so the working tree's are
    // left out too: a link's tests count on neither side, as `clm land` counts them.
    let mut after = read_files(root);
    after.retain(|(p, _)| {
        !fs::symlink_metadata(root.join(p)).is_ok_and(|m| m.file_type().is_symlink())
    });
    let gone = deleted_tests(&before, &after);
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
