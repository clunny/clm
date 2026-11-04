//! `clm workaround`: a build's flag that a requirement was hard to build
//! (`workaround-command`, `workaround-flags`).

use std::collections::BTreeSet;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use clm_git::Failure;

/// Writes `backlog/workaround-<component>-<slug>-<n>.md` for `requirement` (`<component>/<slug>`) at
/// its current version, `<n>` the smallest whose file doesn't exist and whose slug no
/// `decided` record settles, and returns its repo-relative path.
/// `Refused`, writing nothing, when the graph at `root` doesn't hold the requirement.
pub fn workaround(
    root: &Path,
    requirement: &str,
    title: &str,
    body: &str,
) -> Result<PathBuf, Failure> {
    let (graph, _) = clm_model::load(root);
    let found = graph
        .requirement(requirement)
        .zip(requirement.split_once('/'));
    let Some((r, (node, slug))) = found else {
        return Err(Failure::Refused(format!(
            "`{requirement}` isn't a requirement"
        )));
    };
    let title = serde_json::to_string(title).expect("a string serializes");
    let mut text = format!(
        "---\ntitle: {title}\nkind: workaround\ncomponents: [{node}]\nrequirement: {requirement}@{}\n---\n\n{body}",
        r.version()
    );
    if !text.ends_with('\n') {
        text.push('\n');
    }
    let refused = |path: &Path, e: io::Error| Failure::Refused(format!("{}: {e}", path.display()));
    let dir = Path::new("backlog");
    fs::create_dir_all(root.join(dir)).map_err(|e| refused(dir, e))?;
    // A settled slug stays settled, so a new flag never takes one (`effort/unsettled-flags`).
    // A ledger that can't be read settles nothing here; `crowded` reports it.
    let settled: BTreeSet<String> = clm_ledger::fold(root)
        .map(|fold| {
            let decided = fold.decisions.values().flatten();
            decided
                .filter_map(|r| match &r.body {
                    clm_ledger::Body::Decided { settles, .. } => Some(settles.clone()),
                    _ => None,
                })
                .flatten()
                .collect()
        })
        .unwrap_or_default();
    // Created only when absent, so two builds flagging at once never overwrite a flag.
    for n in 1.. {
        if settled.contains(&format!("workaround-{node}-{slug}-{n}")) {
            continue;
        }
        let path = PathBuf::from(format!("backlog/workaround-{node}-{slug}-{n}.md"));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(root.join(&path))
        {
            Ok(mut file) => {
                file.write_all(text.as_bytes())
                    .map_err(|e| refused(&path, e))?;
                return Ok(path);
            }
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(refused(&path, e)),
        }
    }
    unreachable!("some number is free")
}

/// Each capability `requirement` serves (and `CAPABILITIES.md` holds) whose unsettled workaround
/// flags number `threshold` or more, with the count, in serves order (`workaround-command`).
/// `Git` when the ledger can't be read.
pub fn crowded(
    root: &Path,
    requirement: &str,
    threshold: u64,
) -> Result<Vec<(String, usize)>, Failure> {
    let (graph, _) = clm_model::load(root);
    let capabilities = clm_effort::capabilities_of(&graph, requirement);
    if capabilities.is_empty() {
        return Ok(Vec::new());
    }
    let fold = clm_ledger::fold(root).map_err(|e| Failure::Git(e.to_string()))?;
    Ok(capabilities
        .into_iter()
        .map(|c| (c.to_string(), clm_effort::unsettled(&graph, &fold, c).len()))
        .filter(|(_, n)| *n as u64 >= threshold)
        .collect())
}
