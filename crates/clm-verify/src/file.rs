//! `files-unproven`: what the mutation pass found unproven reaches `main` as a `bug` backlog
//! item per component, landed through `clm land` from a temporary branch, so the requirement
//! reads as not built until a build resolves the item (`check.coverage/coverage-report`).

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use clm_ledger::Body;

use crate::chain::State;
use crate::mutate::{Cite, cites, scratch};
use crate::{Ctx, Error};

/// The entries of one `mutated` record's commit for one component, `<id>@<version>`.
struct Candidate {
    /// The commit the record names, a full ID.
    found: String,
    component: String,
    entries: Vec<String>,
    /// The item's path in the git repository.
    path: String,
}

/// Files what is still to file and returns the notes for stderr; a failure is a note, never
/// an error, and changes nothing about the verification.
pub(crate) fn file(ctx: &Ctx) -> Vec<String> {
    match try_file(ctx) {
        Ok(None) => Vec::new(),
        Ok(Some((items, requirements))) => {
            vec![format!(
                "unproven: filed {items} items for {requirements} requirements"
            )]
        }
        Err(why) => vec![format!("unproven: not filed: {why}")],
    }
}

fn try_file(ctx: &Ctx) -> Result<Option<(usize, usize)>, Error> {
    // The ledger as it is now, which holds the record the pass just appended.
    let state = State::load(ctx)?;
    let candidates = candidates(ctx, &state)?;
    if candidates.is_empty() {
        return Ok(None);
    }
    let scratch = scratch(ctx)?;
    let top = scratch.path().join("tree");
    let branch = format!("unproven/{}", ctx.short(&ctx.base)?);
    let done = attempt(ctx, &candidates, &branch, &top);
    // Whether or not it landed, nothing is left behind.
    let _ = clm_git::remove_worktree(&ctx.root, &top);
    let _ = ctx.git(&["worktree", "prune"]);
    let _ = ctx.git(&["branch", "-D", &branch]);
    let _ = ctx.git(&["push", "--quiet", clm_git::REMOTE, "--delete", &branch]);
    done
}

/// The entries each `mutated` record on the chain holds, by commit and component, left out
/// when a commit after it already added the item's path.
fn candidates(ctx: &Ctx, state: &State) -> Result<Vec<Candidate>, Error> {
    let mut found: BTreeMap<(String, String), Vec<String>> = BTreeMap::new();
    for (commit, records) in &state.fold.mutations {
        if state.index(commit).is_none() {
            continue;
        }
        for record in records {
            let Body::Mutated { unproven, .. } = &record.body else {
                continue;
            };
            for entry in unproven {
                let component = entry.split('/').next().unwrap_or_default().to_string();
                let all = found.entry((commit.clone(), component)).or_default();
                if !all.contains(entry) {
                    all.push(entry.clone());
                }
            }
        }
    }
    let mut out = Vec::new();
    for ((commit, component), entries) in found {
        let path = format!(
            "{}backlog/unproven-{component}-{}.md",
            ctx.prefix,
            ctx.short(&commit)?
        );
        let range = format!("{commit}..{}", ctx.base);
        let added = ctx.git(&[
            "log",
            "--first-parent",
            "--diff-filter=A",
            "--name-only",
            "--format=",
            &range,
            "--",
            &format!(":(top){path}"),
        ])?;
        if added.trim().is_empty() {
            out.push(Candidate {
                found: commit,
                component,
                entries,
                path,
            });
        }
    }
    Ok(out)
}

/// Writes the items in a worktree of `origin/main` on `branch`, pushes it and lands it, and
/// returns how many items and requirements it filed, or none when no entry is current.
fn attempt(
    ctx: &Ctx,
    candidates: &[Candidate],
    branch: &str,
    top: &Path,
) -> Result<Option<(usize, usize)>, Error> {
    let at = top.to_string_lossy();
    ctx.git(&["worktree", "add", "--quiet", "-B", branch, &at, &ctx.base])?;
    let dir = top.join(&ctx.prefix);
    let (graph, _) = clm_model::load(&dir);
    let cites = cites(&dir, &graph)?;
    let (mut items, mut requirements) = (0, 0);
    for c in candidates {
        let current: Vec<&String> = c
            .entries
            .iter()
            .filter(|e| {
                e.split_once('@').is_some_and(|(id, version)| {
                    graph
                        .requirement(id)
                        .is_some_and(|r| r.version() == version)
                })
            })
            .collect();
        if current.is_empty() {
            continue;
        }
        let item = top.join(&c.path);
        if let Some(parent) = item.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&item, text(ctx, c, &current, &cites)?)?;
        items += 1;
        requirements += current.len();
    }
    if items == 0 {
        return Ok(None);
    }
    clm_git::git(top, &["add", "--all"])?;
    clm_git::git(
        top,
        &[
            "commit",
            "--quiet",
            "--message",
            "File the requirements the mutation pass found unproven",
        ],
    )?;
    ctx.git(&[
        "push",
        "--quiet",
        "--force",
        clm_git::REMOTE,
        &format!("{branch}:refs/heads/{branch}"),
    ])?;
    // The branch is committed and pushed: the worktree has done its work.
    clm_git::remove_worktree(&ctx.root, top)?;
    clm_land::land(&ctx.root, Some(branch), ctx.env)?;
    Ok(Some((items, requirements)))
}

/// The item for one component: the entries and the tests that cite each.
fn text(ctx: &Ctx, c: &Candidate, current: &[&String], cites: &[Cite]) -> Result<String, Error> {
    let short = ctx.short(&c.found)?;
    let component = &c.component;
    let mut out = format!(
        "---\ntitle: Tests of {component} notice no change to its code\nkind: bug\n\
         components: [{component}]\nunproven: [{}]\n---\n\n",
        current
            .iter()
            .map(|e| e.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );
    out.push_str(&format!(
        "After the land `{short}` the mutation pass changed the code of `{component}`, and the \
         tests that cite these requirements passed with every change it tried:\n\n"
    ));
    for entry in current {
        let id = entry.split('@').next().unwrap_or_default();
        out.push_str(&format!("- `{id}`:"));
        let tests: Vec<String> = cites
            .iter()
            .filter(|t| t.id == id)
            .map(|t| format!(" `{}` `{}`", t.path, t.name))
            .collect();
        out.push_str(&tests.join(","));
        out.push('\n');
    }
    out.push_str(
        "\nA test that proves the requirement fails when its code is wrong: strengthen these \
         so one does, or, when they are already sound, delete this item.\n",
    );
    Ok(out)
}
