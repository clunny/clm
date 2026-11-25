//! `files-unproven`: what the mutation pass found unproven, and what two passes in a row left
//! unjudged, reaches `main` as a `bug` backlog item per component, landed through `clm land`
//! from a temporary branch, so the requirement reads as not built until a build resolves the
//! item (`check.coverage/coverage-report`).

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use clm_ledger::Body;

use crate::chain::{Left, State};
use crate::kept::MIN_NAME;
use crate::mutate::{Cite, cites, scratch};
use crate::{Ctx, Error};

/// A requirement ID with its run of records that counted it against it, newest first.
type Run = (String, Vec<Left>);

/// What one item is to name.
enum Found {
    /// `<id>@<version>` entries a pass found unproven.
    Unproven(Vec<String>),
    /// The requirements a run of two or more passes counted against them, with their runs.
    Unjudged(Vec<Run>),
}

/// One item to file, for one component and the commit of the record that found it.
struct Candidate {
    /// The commit the item is named after, a full ID.
    found: String,
    component: String,
    what: Found,
    /// The item's path in the git repository.
    path: String,
}

/// How many items, and requirements in them, a filing landed, of each kind.
#[derive(Default)]
struct Filed {
    unproven: (usize, usize),
    unjudged: (usize, usize),
}

/// Files what is still to file and returns the notes for stderr; a failure is a note, never
/// an error, and changes nothing about the verification. Once an entry to file is found, `stop`
/// is judged before each step that writes and commits, pushes or lands, and a step not yet
/// started then isn't, the note being `unproven: not filed: stopped`. Its `git` commands and
/// its land run in the caller's process group, so a Ctrl-C at a terminal ends the one running,
/// which fails its step.
pub(crate) fn file(ctx: &Ctx, stop: &AtomicBool) -> Vec<String> {
    match try_file(ctx, stop) {
        Ok(None) => Vec::new(),
        Ok(Some(filed)) => [("unproven", filed.unproven), ("unjudged", filed.unjudged)]
            .into_iter()
            .filter(|(_, (items, _))| *items > 0)
            .map(|(kind, (items, requirements))| {
                format!("{kind}: filed {items} items for {requirements} requirements")
            })
            .collect(),
        Err(why) => vec![format!("unproven: not filed: {why}")],
    }
}

fn try_file(ctx: &Ctx, stop: &AtomicBool) -> Result<Option<Filed>, Error> {
    // The ledger as it is now, which holds the record the pass just appended.
    let state = State::load(ctx)?;
    let candidates = candidates(ctx, &state)?;
    if candidates.is_empty() {
        return Ok(None);
    }
    let scratch = scratch(ctx)?;
    let top = scratch.path().join("tree");
    let branch = format!("unproven/{}", ctx.short(&ctx.base)?);
    let done = attempt(ctx, &candidates, &branch, &top, stop);
    // Whether or not it landed, nothing is left behind.
    let _ = clm_git::remove_worktree(&ctx.root, &top);
    let _ = ctx.git(&["worktree", "prune"]);
    let _ = ctx.git(&["branch", "-D", &branch]);
    let _ = ctx.git(&["push", "--quiet", clm_git::REMOTE, "--delete", &branch]);
    done
}

/// The items still to file: the entries each `mutated` record on the chain holds, by commit
/// and component, and the requirements whose run of records counting them against them
/// holds two or more, by the commit of the second oldest of the run and component; each
/// left out when a commit after the one it is named after already added an item of its kind
/// and component named by any prefix of that commit at least 7 characters long.
fn candidates(ctx: &Ctx, state: &State) -> Result<Vec<Candidate>, Error> {
    let component = |id: &str| id.split('/').next().unwrap_or_default().to_string();
    let mut unproven: BTreeMap<(String, String), Vec<String>> = BTreeMap::new();
    for (commit, records) in &state.fold.mutations {
        if state.index(commit).is_none() {
            continue;
        }
        for record in records {
            let Body::Mutated {
                unproven: found, ..
            } = &record.body
            else {
                continue;
            };
            for entry in found {
                let all = unproven
                    .entry((commit.clone(), component(entry)))
                    .or_default();
                if !all.contains(entry) {
                    all.push(entry.clone());
                }
            }
        }
    }
    let mut unjudged: BTreeMap<(String, String), Vec<Run>> = BTreeMap::new();
    for (id, run) in state.left_runs() {
        if run.len() >= 2 {
            let at = run[run.len() - 2].commit.clone();
            unjudged
                .entry((at, component(&id)))
                .or_default()
                .push((id, run));
        }
    }
    let found = unproven
        .into_iter()
        .map(|(key, entries)| ("unproven", key, Found::Unproven(entries)))
        .chain(
            unjudged
                .into_iter()
                .map(|(key, runs)| ("unjudged", key, Found::Unjudged(runs))),
        );
    let mut out = Vec::new();
    for (kind, (commit, component), what) in found {
        let path = format!(
            "{}backlog/{kind}-{component}-{}.md",
            ctx.prefix,
            ctx.short(&commit)?
        );
        // An item filed under any prefix of the commit at least 7 long counts, whatever
        // length `rev-parse --short` gave it then.
        let named = format!("{}backlog/{kind}-{component}-", ctx.prefix);
        let range = format!("{commit}..{}", ctx.base);
        let added = ctx.git(&[
            "log",
            "--first-parent",
            "--diff-filter=A",
            "--name-only",
            "--format=",
            &range,
            "--",
            &format!(":(top){}backlog/", ctx.prefix),
        ])?;
        let filed = added.lines().any(|line| {
            line.strip_prefix(&named)
                .and_then(|rest| rest.strip_suffix(".md"))
                .is_some_and(|p| p.len() >= MIN_NAME && commit.starts_with(p))
        });
        if !filed {
            out.push(Candidate {
                found: commit,
                component,
                what,
                path,
            });
        }
    }
    Ok(out)
}

/// Fails with `stopped` when the call's stop request is set (`files-unproven`).
fn go_on(stop: &AtomicBool) -> Result<(), Error> {
    if stop.load(Ordering::SeqCst) {
        return Err(Error::Refused("stopped".to_string()));
    }
    Ok(())
}

/// Writes the items in a worktree of `origin/main` on `branch`, pushes it and lands it, and
/// returns how many items and requirements it filed of each kind, or none when nothing is
/// left to file: no `unproven` entry at its current version and no `unjudged` requirement
/// that `origin/main` holds. `stop` is judged once one is found, before each of those steps.
fn attempt(
    ctx: &Ctx,
    candidates: &[Candidate],
    branch: &str,
    top: &Path,
    stop: &AtomicBool,
) -> Result<Option<Filed>, Error> {
    let at = top.to_string_lossy();
    ctx.git(&["worktree", "add", "--quiet", "-B", branch, &at, &ctx.base])?;
    let dir = top.join(&ctx.prefix);
    let (graph, _) = clm_model::load(&dir);
    // What each candidate still names, found before anything is written: the entries at
    // their current version, and the left requirements that still exist.
    let filed: Vec<(&Candidate, Found)> = candidates
        .iter()
        .filter_map(|c| {
            let what = match &c.what {
                Found::Unproven(entries) => Found::Unproven(
                    entries
                        .iter()
                        .filter(|e| {
                            e.split_once('@').is_some_and(|(id, version)| {
                                graph
                                    .requirement(id)
                                    .is_some_and(|r| r.version() == version)
                            })
                        })
                        .cloned()
                        .collect(),
                ),
                Found::Unjudged(runs) => Found::Unjudged(
                    runs.iter()
                        .filter(|(id, _)| graph.requirement(id).is_some())
                        .cloned()
                        .collect(),
                ),
            };
            let empty = match &what {
                Found::Unproven(e) => e.is_empty(),
                Found::Unjudged(r) => r.is_empty(),
            };
            (!empty).then_some((c, what))
        })
        .collect();
    if filed.is_empty() {
        return Ok(None);
    }
    go_on(stop)?;
    let cites = cites(&dir, &graph)?;
    let mut counts = Filed::default();
    for (c, what) in &filed {
        let item = top.join(&c.path);
        if let Some(parent) = item.parent() {
            fs::create_dir_all(parent)?;
        }
        let (text, count, requirements) = match what {
            Found::Unproven(current) => (
                unproven_text(ctx, c, current, &cites)?,
                &mut counts.unproven,
                current.len(),
            ),
            Found::Unjudged(runs) => (
                unjudged_text(ctx, &c.component, runs)?,
                &mut counts.unjudged,
                runs.len(),
            ),
        };
        fs::write(&item, text)?;
        count.0 += 1;
        count.1 += requirements;
    }
    clm_git::git(top, &["add", "--all"])?;
    clm_git::git(
        top,
        &[
            "commit",
            "--quiet",
            "--message",
            "File the requirements the mutation pass found unproven or left unjudged",
        ],
    )?;
    go_on(stop)?;
    ctx.git(&[
        "push",
        "--quiet",
        "--force",
        clm_git::REMOTE,
        &format!("{branch}:refs/heads/{branch}"),
    ])?;
    // The branch is committed and pushed: the worktree has done its work.
    clm_git::remove_worktree(&ctx.root, top)?;
    go_on(stop)?;
    clm_land::land(&ctx.root, Some(branch), ctx.env)?;
    Ok(Some(counts))
}

/// The `unproven` item for one component: the entries and the tests that cite each.
fn unproven_text(
    ctx: &Ctx,
    c: &Candidate,
    current: &[String],
    cites: &[Cite],
) -> Result<String, Error> {
    let short = ctx.short(&c.found)?;
    let component = &c.component;
    let mut out = format!(
        "---\ntitle: Tests of {component} notice no change to its code\nkind: bug\n\
         components: [{component}]\nunproven: [{}]\n---\n\n",
        current.join(", ")
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

/// The `unjudged` item for one component: each requirement and the records of its run,
/// oldest first, each with its commit and how that pass left it.
fn unjudged_text(ctx: &Ctx, component: &str, runs: &[Run]) -> Result<String, Error> {
    let mut out = format!(
        "---\ntitle: The mutation pass can't judge requirements of {component}\nkind: bug\n\
         components: [{component}]\n---\n\n\
         Each of the mutation passes after the lands named left these requirements of \
         `{component}` without a verdict. A pass that was stopped, or that left one only \
         because another requirement's block failed, isn't listed:\n\n"
    );
    for (id, run) in runs {
        out.push_str(&format!("- `{id}`:\n"));
        for left in run.iter().rev() {
            out.push_str(&format!(
                "  - `{}`: {}\n",
                ctx.short(&left.commit)?,
                how(left)
            ));
        }
    }
    out.push_str(
        "\nMend what keeps the pass from its verdict (the failing tool, or `mutate_minutes`), \
         then delete this item.\n",
    );
    Ok(out)
}

/// How one pass left a requirement (`files-unproven`).
fn how(left: &Left) -> String {
    if let Some(why) = &left.why {
        return match why.as_str() {
            "" => "its block's tool failed".to_string(),
            why => format!("its block's tool failed: {why}"),
        };
    }
    let ended = left.ended.as_deref().unwrap_or_default();
    if let Some(m) = ended.strip_prefix("limit ") {
        format!("the pass reached its limit of {m} minutes")
    } else if let Some(n) = ended.strip_prefix("status ") {
        format!("the command failed with status {n}")
    } else if let Some(n) = ended.strip_prefix("signal ") {
        format!("the command was ended by signal {n}")
    } else {
        "no reason recorded".to_string()
    }
}
