//! `obsolete-tests-elsewhere`: once the root names its `wiring`, a test in one component's
//! file whose every marker cites another component's removed requirement is named in an
//! approved `deleted-tests.txt`, since its owner's build can neither keep nor delete it
//! (see specs/check/SPEC.md).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use clm_coverage::{Marker, bare_name, matches, moved_to, obsolete_tests, owner};
use clm_gate::ListLine;
use clm_model::{Graph, Layer, Violation};

/// Each such test among the files holding `markers` (the markers in tests), read from `root`.
/// `removed` is `clm-coverage`'s `removed`.
pub fn violations(
    root: &Path,
    graph: &Graph,
    markers: &[Marker],
    removed: &BTreeMap<String, Vec<String>>,
) -> Vec<Violation> {
    let Some(wiring) = graph.wiring() else {
        return Vec::new();
    };
    // Only a file holding a marker for a removed ID can hold such a test.
    let paths: BTreeSet<&Path> = markers
        .iter()
        .filter(|m| removed.contains_key(&m.id))
        .map(|m| m.path.as_path())
        .collect();
    let files: Vec<(PathBuf, String)> = paths
        .into_iter()
        .filter(|p| !wiring.iter().any(|g| matches(g, &p.to_string_lossy())))
        .filter_map(|p| Some((p.to_path_buf(), std::fs::read_to_string(root.join(p)).ok()?)))
        .collect();
    let listed = approved_lists(root);
    let mut out = Vec::new();
    for test in obsolete_tests(graph, &files) {
        // Every marker cites a removed ID; none of them a capability.
        if !test.every || test.ids.iter().any(|id| Layer::of(id).is_some()) {
            continue;
        }
        let (file, name) = test.test.split_once("::").unwrap_or((&test.test, ""));
        let Some(mine) = owner(graph, Path::new(file)) else {
            continue;
        };
        let component = |id: &str| id.split_once('/').map_or("", |(c, _)| c).to_string();
        // A removed requirement of the owner's own is its build's to take out.
        if test.ids.iter().any(|id| component(id) == mine) {
            continue;
        }
        // One moved to the owner is the owner's build's to re-cite.
        if test
            .ids
            .iter()
            .any(|id| moved_to(graph, id).iter().any(|to| component(to) == mine))
        {
            continue;
        }
        let bare = bare_name(name);
        if listed
            .iter()
            .any(|l| l.file == file && bare_name(&l.name) == bare)
        {
            continue;
        }
        let first = &test.ids[0];
        let change = removed
            .get(first)
            .and_then(|c| c.first())
            .map_or("", String::as_str);
        out.push(Violation {
            path: PathBuf::from(format!("{file}:{}", test.line)),
            rule: "check/obsolete-tests-elsewhere",
            message: format!(
                "test {name} cites {first}, which {change} removed, from `{mine}`'s file; list it in that change's deleted-tests.txt, and prove what stays in the files of the component that states it"
            ),
        });
    }
    out
}

/// The list lines of every `changes/<id>/deleted-tests.txt` and
/// `changes/archive/<id>/deleted-tests.txt` under `root`.
fn approved_lists(root: &Path) -> Vec<ListLine> {
    let changes = root.join("changes");
    let dirs = |dir: PathBuf| {
        std::fs::read_dir(dir)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
    };
    dirs(changes.clone())
        .chain(dirs(changes.join("archive")))
        .filter_map(|d| std::fs::read_to_string(d.join("deleted-tests.txt")).ok())
        .flat_map(|text| clm_gate::list_lines(&text))
        .collect()
}
