//! `clm intake`: the host's open issues and pull requests matched to the backlog
//! (`issues-enter-through-the-backlog`). Every line carries numbers, slugs, change IDs,
//! commits and logins only, never a title or body (`untrusted-text-read-only-verifier`).

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use clm_git::{fetch, git, tree_blobs};
use clm_host::Host;
use clm_model::{parse_backlog_item, parse_change_frontmatter};

use crate::{Error, REMOTE, main_graph};

/// `clm intake`: one line per open issue or pull request of `host`, sorted by number.
pub fn intake(root: &Path, host: &dyn Host) -> Result<Vec<String>, Error> {
    fetch(root)?;
    let mut open = host.list(root).map_err(|e| match e {
        clm_host::Error::Refused(why)
        | clm_host::Error::Git(why)
        | clm_host::Error::Nowhere(why) => Error::Refused(why),
    })?;
    if open.is_empty() {
        return Ok(Vec::new());
    }
    open.sort_by_key(|o| o.number);
    let graph = main_graph(root)?;
    let mut imported: BTreeMap<u64, Vec<&str>> = BTreeMap::new();
    for item in graph.backlog.values() {
        for n in &item.frontmatter.issues {
            imported.entry(*n).or_default().push(&item.slug);
        }
    }
    let unmatched: BTreeSet<u64> = open
        .iter()
        .filter(|o| !o.pull_request && !imported.contains_key(&o.number))
        .map(|o| o.number)
        .collect();
    let resolved = if unmatched.is_empty() {
        BTreeMap::new()
    } else {
        deleted_items(root, &unmatched)?
    };
    let mut lines = Vec::new();
    for o in &open {
        if o.pull_request {
            lines.push(format!("pull {} {}", o.number, o.author));
        } else if let Some(slugs) = imported.get(&o.number) {
            lines.push(format!("issue {} imported {}", o.number, slugs.join(" ")));
        } else if let Some((slug, commit)) = resolved.get(&o.number) {
            let by = match resolving_change(root, slug)? {
                Some(id) => id,
                None => git(root, &["rev-parse", "--short", commit])?,
            };
            lines.push(format!("issue {} resolved {slug} {by}", o.number));
        } else {
            lines.push(format!("issue {} new", o.number));
        }
    }
    Ok(lines)
}

/// For each of `wanted` the most recent item deleted from `origin/main`'s `backlog/` whose
/// `issues` named it, as its slug and the deleting commit. The item is read at the deleting
/// commit's first parent, checking nothing against a graph; a file that no longer parses is
/// skipped.
fn deleted_items(
    root: &Path,
    wanted: &BTreeSet<u64>,
) -> Result<BTreeMap<u64, (String, String)>, Error> {
    let main = format!("{REMOTE}/main");
    let log = git(
        root,
        &[
            "log",
            "--diff-filter=D",
            "--no-renames",
            "--name-only",
            "--format=commit %H",
            &main,
            "--",
            "backlog/",
        ],
    )?;
    let mut found: BTreeMap<u64, (String, String)> = BTreeMap::new();
    let mut commit = "";
    for line in log.lines().filter(|l| !l.is_empty()) {
        if let Some(hash) = line.strip_prefix("commit ") {
            commit = hash;
            continue;
        }
        let Some(slug) = line
            .strip_prefix("backlog/")
            .and_then(|f| f.strip_suffix(".md"))
            .filter(|s| !s.contains('/'))
        else {
            continue;
        };
        let Ok(text) = git(root, &["show", &format!("{commit}^:{line}")]) else {
            continue;
        };
        let Ok(item) = parse_backlog_item(slug, &text) else {
            continue;
        };
        for n in item
            .frontmatter
            .issues
            .iter()
            .filter(|n| wanted.contains(n))
        {
            found
                .entry(*n)
                .or_insert_with(|| (slug.to_string(), commit.to_string()));
        }
        if found.len() == wanted.len() {
            break;
        }
    }
    Ok(found)
}

/// The greatest ID among the changes on `origin/main`, live or archived, whose `resolves`
/// or `from` names `slug`.
fn resolving_change(root: &Path, slug: &str) -> Result<Option<String>, Error> {
    let is_change =
        |p: &str| p.starts_with("changes/") && p.ends_with("/change.md") && !p.contains("/deltas/");
    let blobs = tree_blobs(root, &format!("{REMOTE}/main"), &is_change)?;
    let from = format!("backlog/{slug}");
    Ok(blobs
        .into_iter()
        .filter_map(|(path, bytes)| {
            let fm = parse_change_frontmatter(&String::from_utf8_lossy(&bytes))?;
            let named = fm.resolves.iter().any(|r| r == slug) || fm.from.as_deref() == Some(&from);
            let id = path.strip_suffix("/change.md")?.rsplit('/').next()?;
            named.then(|| id.to_string())
        })
        .max())
}
