//! `land/verify-stop`: a code land that touches a component whose slow tests the human took in
//! as broken waits for the fix, and one that touches the components of a red result nobody
//! has taken in yet is warned (`capability/slow-tests-after-land`).

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use clm_ledger::{Body, Fold};
use clm_model::{BacklogKind, load_files};

use crate::{Error, Repo};

/// An item that holds components: a `kind: bug` item whose `verify_failures` isn't empty.
struct Held {
    slug: String,
    components: Vec<String>,
}

/// Refuses `head` when its code changes a file under the `paths` of a component a `kind: bug`
/// item on `base`'s backlog holds with `verify_failures`, unless a commit it adds carries
/// `Resolves: backlog/<slug>` for one item holding that component. Otherwise gives the warnings for the red
/// `verified` results on `base`'s first-parent chain that no item names and whose components
/// `head` touches. A spec land, which changes no code, and a code land touching neither,
/// give none.
pub(crate) fn check(
    repo: &Repo,
    base: &str,
    head: &str,
    fold: &Fold,
) -> Result<Vec<String>, Error> {
    let prefix = repo.git(&["rev-parse", "--show-prefix"])?;
    let items = backlog(repo, base, &prefix)?;
    let named: BTreeSet<&str> = items
        .iter()
        .flat_map(|(_, fails)| fails.iter().map(String::as_str))
        .collect();
    let held: Vec<Held> = items
        .iter()
        .filter(|(item, fails)| item.frontmatter.kind == BacklogKind::Bug && !fails.is_empty())
        .map(|(item, _)| Held {
            slug: item.slug.clone(),
            components: item.frontmatter.nodes.clone(),
        })
        .collect();
    let reds = if fold.verifications.is_empty() {
        Vec::new()
    } else {
        let chain = repo.git(&["rev-list", "--first-parent", "--reverse", base])?;
        let chain: Vec<String> = chain.lines().map(str::to_string).collect();
        fold.unresolved_reds(&chain)
    };
    let reds: Vec<_> = reds
        .iter()
        .filter_map(|r| match &r.body {
            Body::Verified {
                commit,
                culprit,
                components,
                ..
            } if !named.contains(commit.as_str()) => Some((commit, culprit, components)),
            _ => None,
        })
        .collect();
    if held.is_empty() && reds.is_empty() {
        return Ok(Vec::new());
    }
    let files = code_files(repo, base, head, &prefix)?;
    if files.is_empty() {
        return Ok(Vec::new());
    }
    let paths = component_paths(repo, base, &prefix)?;
    let touches = |component: &str| {
        paths.get(component).is_some_and(|globs| {
            files
                .iter()
                .any(|f| globs.iter().any(|g| clm_check::matches(g, f)))
        })
    };

    let resolves = resolved(repo, base, head)?;
    // A held component the branch touches is fixed by a resolved item holding it; the items
    // not yet resolved go on holding it.
    let mut holders: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for h in &held {
        for c in &h.components {
            holders.entry(c).or_default().push(&h.slug);
        }
    }
    let blocking: Vec<(&str, &Vec<&str>)> = holders
        .iter()
        .filter(|(c, slugs)| touches(c) && !slugs.iter().any(|s| resolves.contains(*s)))
        .map(|(c, slugs)| (*c, slugs))
        .collect();
    if !blocking.is_empty() {
        let mut why = String::from(
            "land/verify-stop: slow tests failed after a land, and this branch changes code of",
        );
        for (component, slugs) in &blocking {
            let items: Vec<String> = slugs.iter().map(|s| format!("backlog/{s}")).collect();
            why.push_str(&format!("\n  `{component}`, held by {}", items.join(", ")));
        }
        why.push_str(
            "\nit lands once a commit it adds carries the trailer `Resolves: backlog/<slug>` \
             for one item holding each component named",
        );
        return Err(Error::Refused(why));
    }

    Ok(reds
        .into_iter()
        .filter_map(|(commit, culprit, components)| {
            let touched: Vec<&str> = components
                .iter()
                .map(String::as_str)
                .filter(|c| touches(c))
                .collect();
            (!touched.is_empty()).then(|| {
                format!(
                    "slow tests failed after {commit}, culprit {}, on {}; run /clm:triage to take it in",
                    culprit.as_deref().unwrap_or("unknown"),
                    touched.join(", ")
                )
            })
        })
        .collect())
}

/// Every item of `base`'s backlog that loads, with its `verify_failures` (all kinds: an item
/// of any kind that names a red commit has taken it in).
fn backlog(
    repo: &Repo,
    base: &str,
    prefix: &str,
) -> Result<Vec<(clm_model::BacklogItem, Vec<String>)>, Error> {
    let dir = format!("{prefix}backlog/");
    let blobs = clm_git::tree_blobs(&repo.root, base, &|p: &str| {
        p.starts_with(&dir) && p.ends_with(".md") && !p[dir.len()..].contains('/')
    })?;
    Ok(blobs
        .into_iter()
        .filter_map(|(path, bytes)| {
            let slug = path.strip_prefix(&dir)?.strip_suffix(".md")?;
            let item =
                clm_model::parse_backlog_item(slug, &String::from_utf8_lossy(&bytes)).ok()?;
            let fails = item.frontmatter.verify_failures.clone();
            Some((item, fails))
        })
        .collect())
}

/// The files `head` changes against `base` from their merge base that are code as the gate
/// judges it (not spec paths), relative to the CLM root.
fn code_files(repo: &Repo, base: &str, head: &str, prefix: &str) -> Result<Vec<String>, Error> {
    let range = format!("{base}...{head}");
    let out = repo.git(&["diff", "--name-only", "-z", "--no-renames", &range])?;
    Ok(out
        .split('\0')
        .filter(|p| !p.is_empty())
        .filter_map(|p| p.strip_prefix(prefix))
        .filter(|p| !clm_gate::is_spec_path(p))
        .map(str::to_string)
        .collect())
}

/// Each component's `paths` at `base`.
fn component_paths(
    repo: &Repo,
    base: &str,
    prefix: &str,
) -> Result<BTreeMap<String, Vec<String>>, Error> {
    let dir = format!("{prefix}specs/");
    let blobs = clm_git::tree_blobs(&repo.root, base, &|p: &str| p.starts_with(&dir))?;
    let files: clm_model::Files = blobs
        .into_iter()
        .filter_map(|(path, bytes)| Some((PathBuf::from(path.strip_prefix(prefix)?), bytes)))
        .collect();
    let (graph, _) = load_files(&files);
    Ok(graph
        .nodes
        .into_iter()
        .map(|(id, node)| (id, node.frontmatter.paths))
        .collect())
}

/// The slugs the commits `head` adds to `base` resolve with a `Resolves: backlog/<slug>`
/// trailer.
fn resolved(repo: &Repo, base: &str, head: &str) -> Result<BTreeSet<String>, Error> {
    let range = format!("{base}..{head}");
    let out = repo.git(&[
        "log",
        "--format=%(trailers:key=Resolves,valueonly,separator=%x0a)",
        &range,
    ])?;
    Ok(out
        .lines()
        .filter_map(|l| l.trim().strip_prefix("backlog/"))
        .map(str::to_string)
        .collect())
}
