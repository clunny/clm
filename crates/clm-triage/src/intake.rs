//! `clm intake`: the host's open issues, pull requests and dependency alerts, and the red
//! `verified` results on `main` (`verify-after-land`), matched to the
//! backlog (`issues-enter-through-the-backlog`, `dependency-alerts-through-intake`). Every
//! line carries numbers, slugs, change IDs, commits, logins and an alert's one-word fields
//! only, never a title or body (`untrusted-text-read-only-verifier`).

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use clm_git::{fetch, git, tree_blobs};
use clm_host::Host;
use clm_model::{BacklogFrontmatter, parse_backlog_item, parse_change_frontmatter};

use crate::{Error, REMOTE, main_graph};

/// The numbers an item took in from the host: its `issues` or its `alerts`.
type Taken = fn(&BacklogFrontmatter) -> &[u64];

fn issues_of(f: &BacklogFrontmatter) -> &[u64] {
    &f.issues
}

fn alerts_of(f: &BacklogFrontmatter) -> &[u64] {
    &f.alerts
}

/// How the backlog stands to a set of open numbers: the items that name each, and for the
/// rest the item a later deletion removed, with the deleting commit.
struct Matches {
    imported: BTreeMap<u64, Vec<String>>,
    resolved: BTreeMap<u64, (String, String)>,
}

impl Matches {
    /// The line's tail for `n`: `imported <slug>...` or `resolved <slug> <by>`, or `None` for
    /// a number nothing took in.
    fn status(
        &self,
        root: &Path,
        changes: &BTreeMap<String, String>,
        n: u64,
    ) -> Result<Option<String>, Error> {
        if let Some(slugs) = self.imported.get(&n) {
            return Ok(Some(format!("imported {}", slugs.join(" "))));
        }
        let Some((slug, commit)) = self.resolved.get(&n) else {
            return Ok(None);
        };
        let by = match changes.get(slug) {
            Some(id) => id.clone(),
            None => short(root, commit)?,
        };
        Ok(Some(format!("resolved {slug} {by}")))
    }
}

/// `clm intake`: one line per open issue or pull request of `host`, sorted by number, then
/// one per open dependency alert, sorted by number, or `alerts unavailable` in their place,
/// then one per red `verified` result on `main` no green one has followed.
pub fn intake(root: &Path, host: &dyn Host) -> Result<Vec<String>, Error> {
    fetch(root)?;
    let mut open = host.list(root).map_err(|e| match e {
        clm_host::Error::Refused(why)
        | clm_host::Error::Git(why)
        | clm_host::Error::Nowhere(why) => Error::Refused(why),
    })?;
    open.sort_by_key(|o| o.number);
    // A login that may read issues may still be refused the alerts.
    let alerts = host.alerts(root).ok().map(|mut a| {
        a.sort_by_key(|a| a.number);
        a
    });
    let reds = unresolved_reds(root)?;
    if open.is_empty() && alerts.as_ref().is_some_and(Vec::is_empty) && reds.is_empty() {
        return Ok(Vec::new());
    }
    let graph = main_graph(root)?;
    let items: Vec<(&str, &BacklogFrontmatter)> = graph
        .backlog
        .values()
        .map(|i| (i.slug.as_str(), &i.frontmatter))
        .collect();
    let issue_numbers: Vec<u64> = open
        .iter()
        .filter(|o| !o.pull_request)
        .map(|o| o.number)
        .collect();
    let alert_numbers: Vec<u64> = alerts.iter().flatten().map(|a| a.number).collect();
    let issues = matches(root, &items, &issue_numbers, issues_of)?;
    let alert_matches = matches(root, &items, &alert_numbers, alerts_of)?;
    let changes = if issues.resolved.is_empty() && alert_matches.resolved.is_empty() {
        BTreeMap::new()
    } else {
        resolving_changes(root)?
    };
    let mut lines = Vec::new();
    for o in &open {
        if o.pull_request {
            lines.push(format!("pull {} {}", o.number, o.author));
        } else {
            let status = issues.status(root, &changes, o.number)?;
            let status = status.as_deref().unwrap_or("new");
            lines.push(format!("issue {} {status}", o.number));
        }
    }
    match &alerts {
        None => lines.push("alerts unavailable".to_string()),
        Some(alerts) => {
            for a in alerts {
                let tail = match alert_matches.status(root, &changes, a.number)? {
                    Some(status) => status,
                    None => format!(
                        "new {} {} {} {}",
                        a.severity, a.ecosystem, a.package, a.manifest
                    ),
                };
                lines.push(format!("alert {} {tail}", a.number));
            }
        }
    }
    for red in &reds {
        let slugs: BTreeSet<&str> = graph
            .backlog
            .values()
            .filter(|i| i.frontmatter.verify_failures.contains(&red.commit))
            .map(|i| i.slug.as_str())
            .collect();
        let tail = if slugs.is_empty() {
            let culprit = match &red.culprit {
                Some(c) => short(root, c)?,
                None => "unknown".to_string(),
            };
            let components: BTreeSet<&str> = red.components.iter().map(String::as_str).collect();
            let mut words = vec!["new", culprit.as_str()];
            words.extend(components);
            words.join(" ")
        } else {
            format!("imported {}", Vec::from_iter(slugs).join(" "))
        };
        lines.push(format!("verify {} {tail}", short(root, &red.commit)?));
    }
    Ok(lines)
}

/// A red `verified` result that no green one on `main` has followed.
struct Red {
    commit: String,
    culprit: Option<String>,
    components: Vec<String>,
}

/// The fold's red results over `origin/main`'s first-parent chain, in chain order
/// (`ledger/verified-record`).
fn unresolved_reds(root: &Path) -> Result<Vec<Red>, Error> {
    let fold = clm_ledger::fold(root).map_err(|e| Error::Git(e.to_string()))?;
    if fold.verifications.is_empty() {
        return Ok(Vec::new());
    }
    let chain = git(
        root,
        &[
            "rev-list",
            "--first-parent",
            "--reverse",
            &format!("{REMOTE}/main"),
        ],
    )?;
    let chain: Vec<String> = chain.lines().map(str::to_string).collect();
    Ok(fold
        .unresolved_reds(&chain)
        .into_iter()
        .filter_map(|r| match r.body {
            clm_ledger::Body::Verified {
                commit,
                culprit,
                components,
                ..
            } => Some(Red {
                commit,
                culprit,
                components,
            }),
            _ => None,
        })
        .collect())
}

/// `commit` abbreviated as `git rev-parse --short` abbreviates it.
fn short(root: &Path, commit: &str) -> Result<String, Error> {
    Ok(git(root, &["rev-parse", "--short", commit])?)
}

/// The items that took in each of `numbers` (by `taken`), and for the numbers no item takes
/// in, the item deleted from `origin/main` that did.
fn matches(
    root: &Path,
    items: &[(&str, &BacklogFrontmatter)],
    numbers: &[u64],
    taken: Taken,
) -> Result<Matches, Error> {
    let mut imported: BTreeMap<u64, Vec<String>> = BTreeMap::new();
    for (slug, frontmatter) in items {
        for n in taken(frontmatter).iter().filter(|n| numbers.contains(n)) {
            imported.entry(*n).or_default().push((*slug).to_string());
        }
    }
    let unmatched: BTreeSet<u64> = numbers
        .iter()
        .copied()
        .filter(|n| !imported.contains_key(n))
        .collect();
    let resolved = if unmatched.is_empty() {
        BTreeMap::new()
    } else {
        deleted_items(root, &unmatched, taken)?
    };
    Ok(Matches { imported, resolved })
}

/// For each of `wanted` the most recent item deleted from `origin/main`'s `backlog/` that
/// `taken` says took it in, as its slug and the deleting commit. The item is read at the
/// deleting commit's first parent, checking nothing against a graph; a file that no longer
/// parses is skipped.
fn deleted_items(
    root: &Path,
    wanted: &BTreeSet<u64>,
    taken: Taken,
) -> Result<BTreeMap<u64, (String, String)>, Error> {
    let main = format!("{REMOTE}/main");
    let log = git(
        root,
        &[
            "-c",
            "core.quotePath=false",
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
        for n in taken(&item.frontmatter)
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

/// For each slug the changes on `origin/main`, live or archived, name in `resolves` or
/// `from`, the greatest such change ID. The change files are read once.
fn resolving_changes(root: &Path) -> Result<BTreeMap<String, String>, Error> {
    let is_change =
        |p: &str| p.starts_with("changes/") && p.ends_with("/change.md") && !p.contains("/deltas/");
    let blobs = tree_blobs(root, &format!("{REMOTE}/main"), &is_change)?;
    let mut by: BTreeMap<String, String> = BTreeMap::new();
    for (path, bytes) in blobs {
        let Some(fm) = parse_change_frontmatter(&String::from_utf8_lossy(&bytes)) else {
            continue;
        };
        let Some(id) = path
            .strip_suffix("/change.md")
            .and_then(|p| p.rsplit('/').next())
        else {
            continue;
        };
        let from = fm.from.as_deref().and_then(|f| f.strip_prefix("backlog/"));
        for slug in fm.resolves.iter().map(String::as_str).chain(from) {
            let greatest = by.entry(slug.to_string()).or_default();
            if id > greatest.as_str() {
                *greatest = id.to_string();
            }
        }
    }
    Ok(by)
}
