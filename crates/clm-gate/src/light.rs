//! The light route (`light`): a branch whose spec part only adds scenarios to requirements
//! of the one component whose code it changes, in one change it archives `done`, and which
//! leaves those requirements proven.

use std::collections::BTreeSet;
use std::path::Path;

use clm_model::{ChangeStatus, Graph, Node, Requirement};

use crate::{ARCHIVE, At, Graphs, ReadPart, foreign_in, load_graph_at, renames_of, show};

/// `Ok(Ok(c))` when `light` takes the branch, `c` its component; `Ok(Err(why))` naming the
/// first condition it fails and what is at fault; `Err` when git fails. `spec` holds the
/// branch's spec changes and `code` its code files, each with its status; `read` its spec
/// part as a `spec` branch's.
pub(crate) fn judge(
    at: &At,
    spec: &[(char, &str)],
    code: &[(char, &str)],
    new_archived: &BTreeSet<&str>,
    read: &ReadPart,
) -> Result<Result<String, String>, String> {
    let loaded;
    let graphs = match &read.graphs {
        Some(graphs) => graphs,
        None => {
            loaded = Graphs {
                base: load_graph_at(at, at.merge_base, true)?,
                head: load_graph_at(at, at.head, true)?,
            };
            &loaded
        }
    };
    let c = match change_shape(at, spec, new_archived, &graphs.head.0)? {
        Ok(c) => c,
        Err(why) => return Ok(Err(why)),
    };
    for (rev, (_, violations)) in [(at.merge_base, &graphs.base), (at.head, &graphs.head)] {
        if !violations.is_empty() {
            return Ok(Err(format!("the specs at {rev} don't load")));
        }
    }
    // A cut makes a spec part need review too, so it is named first, as the more telling.
    if let Some(cut) = read.cuts.first() {
        return Ok(Err(format!("its spec part cuts: {cut}")));
    }
    if read.review {
        return Ok(Err(
            "its spec part would need review as a `spec` branch".to_string()
        ));
    }
    let added = match scenarios_added(&c, &graphs.base.0, &graphs.head.0) {
        Ok(added) => added,
        Err(why) => return Ok(Err(why)),
    };
    let base = at.base_specs()?;
    let wiring = base.wiring().unwrap_or_default();
    if let Some(f) = foreign_in(at, base, wiring, &c, code)
        .into_iter()
        .find(|f| !f.owner.is_empty())
    {
        return Ok(Err(format!(
            "`{}` is owned by `{}`, not `{c}`",
            f.path, f.owner
        )));
    }
    for r in added {
        if let Some(why) = not_current(at, r)? {
            return Ok(Err(why));
        }
    }
    Ok(Ok(c))
}

/// The component of the one new archived change the spec changes are the files of, or why
/// they aren't (the first condition of `light`).
fn change_shape(
    at: &At,
    spec: &[(char, &str)],
    new_archived: &BTreeSet<&str>,
    head: &Graph,
) -> Result<Result<String, String>, String> {
    let not_its = |file: &str| {
        Err(format!(
            "`{file}` isn't a file of one change archived `done` with its component's spec"
        ))
    };
    let [id] = new_archived.iter().copied().collect::<Vec<_>>()[..] else {
        let file = spec
            .iter()
            .map(|(_, f)| *f)
            .find(|f| !f.starts_with(ARCHIVE))
            .or(spec.first().map(|(_, f)| *f))
            .unwrap_or_default();
        return Ok(not_its(file));
    };
    let dir = format!("{ARCHIVE}{id}/");
    let change = format!("{dir}change.md");
    let deltas = format!("{dir}deltas/");
    let Some(c) = spec.iter().find_map(|(_, f)| {
        f.strip_prefix(&deltas)?
            .strip_suffix(".md")
            .filter(|c| !c.contains('/'))
    }) else {
        return Ok(Err(format!("change `{id}` names no component")));
    };
    let text = show(at.repo, &at.object(at.head, &change)).ok();
    let fm = text
        .as_deref()
        .and_then(clm_model::parse_change_frontmatter);
    let resolves = fm
        .as_ref()
        .map(|fm| fm.resolves.clone())
        .unwrap_or_default();
    let delta = format!("{deltas}{c}.md");
    let spec_file = head
        .nodes
        .get(c)
        .map(|n| n.path.to_string_lossy().replace('\\', "/"));
    for &(status, file) in spec {
        let resolved = status == 'D'
            && file
                .strip_prefix("backlog/")
                .and_then(|f| f.strip_suffix(".md"))
                .is_some_and(|slug| !slug.contains('/') && resolves.iter().any(|r| r == slug));
        if file != change && file != delta && spec_file.as_deref() != Some(file) && !resolved {
            return Ok(not_its(file));
        }
    }
    let Some(fm) = fm else {
        return Ok(Err(format!("`{change}` doesn't parse")));
    };
    if fm.status != ChangeStatus::Done {
        return Ok(Err(format!("`{change}` isn't `status: done`")));
    }
    if !fm.burdens.is_empty() {
        return Ok(Err(format!("`{change}` declares `human_burdens`")));
    }
    if text
        .as_deref()
        .and_then(renames_of)
        .is_some_and(|r| !r.is_empty())
    {
        return Ok(Err(format!("`{change}` records `renames`")));
    }
    Ok(Ok(c.to_string()))
}

/// Each run of white space read as one space, none at either end.
fn norm(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A requirement's statement, its text above its first `####` heading, and its `####`
/// blocks in order, headings inside a fence left out.
fn blocks(r: &Requirement) -> (String, Vec<String>) {
    let lines: Vec<&str> = r.text.lines().collect();
    let mask = clm_model::outline::fence_mask(&lines);
    let headings: Vec<usize> = (1..lines.len())
        .filter(|&i| !mask[i] && lines[i].starts_with("####"))
        .collect();
    let first = headings.first().copied().unwrap_or(lines.len());
    let statement = norm(&lines[1.min(first)..first].join("\n"));
    let blocks = headings
        .iter()
        .enumerate()
        .map(|(k, &i)| {
            let end = headings.get(k + 1).copied().unwrap_or(lines.len());
            norm(&lines[i..end].join("\n"))
        })
        .collect();
    (statement, blocks)
}

/// A node's frontmatter without its `status`, parsed.
fn frontmatter_but_status(node: &Node) -> Option<serde_yaml::Mapping> {
    let (fm, _) = clm_model::split_frontmatter(&node.source)?;
    let mut fm: serde_yaml::Mapping = serde_yaml::from_str(fm).ok()?;
    fm.remove("status");
    Some(fm)
}

/// The requirements of `c` at the head to which the head adds a scenario, when `c` changes
/// in nothing else (the third condition of `light`); why not otherwise.
fn scenarios_added<'g>(
    c: &str,
    base: &Graph,
    head: &'g Graph,
) -> Result<Vec<&'g Requirement>, String> {
    let (Some(before), Some(after)) = (base.nodes.get(c), head.nodes.get(c)) else {
        return Err(format!(
            "component `{c}` isn't at both the merge base and HEAD"
        ));
    };
    let changed = |what: &str| Err(format!("component `{c}`'s {what} changes"));
    if frontmatter_but_status(before) != frontmatter_but_status(after) {
        return changed("frontmatter");
    }
    if norm(&before.contract) != norm(&after.contract) {
        return changed("Contract");
    }
    if norm(&before.design) != norm(&after.design) {
        return changed("Design");
    }
    // The same IDs, as a set: a reorder alone is no change of them.
    let ids = |n: &Node| {
        n.requirements
            .iter()
            .map(|r| r.id.clone())
            .collect::<BTreeSet<_>>()
    };
    if ids(before) != ids(after) {
        return changed("set of requirements");
    }
    let mut added = Vec::new();
    for now in &after.requirements {
        let Some(was) = before.requirements.iter().find(|r| r.id == now.id) else {
            return changed("set of requirements");
        };
        let (was_statement, was_blocks) = blocks(was);
        let (now_statement, now_blocks) = blocks(now);
        if was_statement != now_statement {
            return Err(format!("the statement of `{}` changes", now.id));
        }
        let mut kept = was_blocks.iter().peekable();
        let mut scenarios = 0;
        for block in &now_blocks {
            if kept.peek() == Some(&block) {
                kept.next();
            } else if block.starts_with("#### Scenario:") {
                scenarios += 1;
            } else {
                return Err(format!(
                    "`{}` changes more than scenarios added to it",
                    now.id
                ));
            }
        }
        if kept.next().is_some() {
            return Err(format!(
                "`{}` changes more than scenarios added to it",
                now.id
            ));
        }
        if scenarios > 0 {
            added.push(now);
        }
    }
    if added.is_empty() {
        return Err(format!(
            "no scenario is added to component `{c}`'s requirements"
        ));
    }
    Ok(added)
}

/// Why `r` isn't current at the head, if it isn't: a marker citing it in a file under the
/// CLM root cites another version or none, or no marker cites it (the fifth condition of
/// `light`). Markers are read as `check.coverage` reads them, a file that isn't UTF-8
/// holding none.
fn not_current(at: &At, r: &Requirement) -> Result<Option<String>, String> {
    let needle = format!("req:{}", r.id);
    let found = match clm_git::run(
        at.repo,
        "git",
        &["grep", "-z", "-l", "-F", "-e", &needle, at.head, "--", "."],
        Default::default(),
    ) {
        Ok(out) => out,
        Err(clm_git::Error::Failed { status, .. }) if status.code() == Some(1) => Vec::new(),
        Err(e) => return Err(e.to_string()),
    };
    let prefix = format!("{}:", at.head);
    let mut files: Vec<String> = found
        .split(|&b| b == 0)
        .filter_map(|f| std::str::from_utf8(f).ok())
        .filter_map(|f| f.strip_prefix(&prefix))
        .map(str::to_string)
        .collect();
    files.sort();
    let mut cited = false;
    for file in files {
        let Ok(text) = show(at.repo, &at.object(at.head, &file)) else {
            continue;
        };
        for marker in clm_coverage::markers_in(Path::new(&file), &text) {
            if marker.id != r.id {
                continue;
            }
            if marker.version.as_deref() != Some(r.version()) {
                return Ok(Some(format!(
                    "`{}` isn't current: `{file}` cites it at {}",
                    r.id,
                    marker.version.as_deref().unwrap_or("no version")
                )));
            }
            cited = true;
        }
    }
    Ok((!cited).then(|| format!("no test cites `{}` at HEAD", r.id)))
}
