//! `clm checkpoint`: remove landed work, report work that isn't safe on `origin`, and say
//! whether the session can end (`checkpoint-before-done`; see
//! specs/orchestrator/checkpoint/SPEC.md). It reads git, the ledger and the clone's build
//! places only, and every `git` run goes through `clm-git`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use clm_git::git;
pub use clm_orchestrator::Error;
use clm_orchestrator::{DEFAULT_EXPIRY_HOURS, REMOTE, SweepOutcome, Swept, queue_from_refs};

/// What `clm checkpoint` prints, and whether the session can end.
#[derive(Debug, PartialEq, Eq)]
pub struct Report {
    pub lines: Vec<String>,
    pub ok: bool,
}

/// The branch families CLM's flow makes.
const FLOW: &[&str] = &["spec/", BUILD];
const BUILD: &str = "impl/";
const AUTOSYNC: &str = "autosync/spec/";
const PROPOSE: &str = "propose/";
/// The ledger branches, `clm/ledger/<person>/<clone>`.
const LEDGER: &str = "clm/ledger/";
/// The prefix of the branches Claude Code makes for a subagent's worktree.
const AGENT_BRANCH: &str = "worktree-agent-";
/// The clone's review mark (`checkpoint-review-mark`): local, shared by its worktrees, never
/// pushed. Named for the tiers it was made for, so existing clones keep their mark.
const REVIEW_MARK: &str = "refs/clm/tiers-verified";
/// The findings recomputing can add to an audit without it.
const RECOMPUTED: &[&str] = &[
    "ledger/understated-land",
    "ledger/unrated-land",
    "ledger/unapproved-land",
];
/// The findings of the recomputation that hold the review mark at their land.
const HOLDS_MARK: &[&str] = &["ledger/understated-land", "ledger/unrated-land"];
/// The finding of the whole chain's audit that holds the mark: a merge whose `landed` record
/// hasn't reached the clone yet, recomputed once it has.
const UNRECORDED: &str = "ledger/unrecorded-land";

/// Runs the checkpoint for the repository holding `root`, from its main checkout. The report
/// ends with `wrap-up ok` or `wrap-up incomplete` (`checkpoint`).
pub fn checkpoint(root: &Path) -> Result<Report, Error> {
    let common = git(
        root,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )?;
    // The caller's own checkout. Its worktree stays, even merged: removing it would pull the
    // directory out from under the session that ran this.
    let top = git(root, &["rev-parse", "--show-toplevel"])?;
    let top = Path::new(&top);
    let caller = top.canonicalize().unwrap_or_else(|_| top.to_path_buf());
    let root = &Path::new(&common)
        .parent()
        .ok_or_else(|| Error::Git(format!("no main checkout for {}", common)))?
        .canonicalize()
        .map_err(|e| Error::Git(format!("{}: {e}", root.display())))?;
    fetch(root)?;
    // The places that no longer count, judged before step 1 deletes a landed build's
    // branches: with them gone, its place would count by its `taken` time instead.
    let mut lines = free_places(root)?;
    let main = format!("{REMOTE}/main");
    let landed = Landed::read(root, &main)?;
    let worktrees = worktrees(root)?;
    let mut local = refs(root, "refs/heads/")?;
    let remote = refs(root, &format!("refs/remotes/{REMOTE}/"))?;
    let landed_local = landed.branches(root, "refs/heads/")?;
    let landed_remote = landed.branches(root, &format!("refs/remotes/{REMOTE}/"))?;

    // 1. Landed local work: builds' worktrees, the main checkout and plain branches. A
    // landed branch checked out in any other worktree, such as a human's, stays with it.
    // The main checkout also leaves a build's `worktree-agent-*` branch once `origin/main`
    // holds its tip.
    let mut checked_out = BTreeSet::new();
    let mut left = BTreeSet::new();
    for (i, wt) in worktrees.iter().enumerate() {
        let Some(b) = wt.branch.as_deref() else {
            continue;
        };
        checked_out.insert(b.to_string());
        let done = if is_flow(b) {
            landed_local.contains(b)
        } else {
            i == 0
                && b.starts_with(AGENT_BRANCH)
                && clm_git::is_ancestor(root, &format!("refs/heads/{b}"), &main)?
        };
        if !done || !wt.path.exists() || !clean(&wt.path)? {
            continue;
        }
        if i == 0 {
            // `main` may be checked out elsewhere; then the checkout stays as it is.
            if git(root, &["switch", "-q", "main"]).is_err() {
                continue;
            }
            // A local `main` that diverged stays behind; nothing on it is lost.
            let _ = git(
                root,
                &["merge", "-q", "--ff-only", &format!("{REMOTE}/main")],
            );
        } else if b.starts_with(BUILD)
            && is_build_worktree(&wt.path)
            && !caller.starts_with(&wt.path)
        {
            match clm_git::remove_worktree(root, &wt.path) {
                Ok(()) => lines.push(format!("removed {}", wt.path.display())),
                Err(e) => {
                    // The landed branch stays for the next wrap-up, and nothing of this
                    // directory is judged: git may no longer know it as a worktree.
                    lines.push(format!(
                        "left {}: {}; delete the directory by hand, then run git worktree prune",
                        wt.path.display(),
                        e.to_string().replace('\n', " ").trim()
                    ));
                    left.insert(wt.path.clone());
                    continue;
                }
            }
        } else {
            continue;
        }
        git(root, &["branch", "-q", "-D", b])?;
        lines.push(format!("removed {b}"));
        local.remove(b);
    }
    let plain: Vec<String> = local
        .keys()
        .filter(|b| is_flow(b) && !checked_out.contains(*b) && landed_local.contains(*b))
        .cloned()
        .collect();
    for b in plain {
        git(root, &["branch", "-q", "-D", &b])?;
        lines.push(format!("removed {b}"));
        local.remove(&b);
    }
    // The branches Claude Code made for builds, left behind once each switched to its
    // `impl/<node>`.
    let agents = git(
        root,
        &[
            "for-each-ref",
            "--format=%(refname:short)",
            &format!("refs/heads/{AGENT_BRANCH}*"),
        ],
    )?;
    for b in agents.lines() {
        if !checked_out.contains(b)
            && clm_git::is_ancestor(root, &format!("refs/heads/{b}"), &main)?
        {
            git(root, &["branch", "-q", "-D", b])?;
            lines.push(format!("removed {b}"));
        }
    }

    // 5 (first half). `origin/main`'s tree: its violations, its complete changes, and what
    // holds backlog items there, which step 2 needs.
    let tmp = clm_git::extract_tree(root, &main)?;
    let tree = tmp.path();
    // The files of `origin/main`'s commit, as `clm check` sees the tracked files of a
    // checkout of it, those under directories that start with `.` among them.
    let listing = listed_files(root, &main)?;
    let mut violations: Vec<String> = clm_check::run_listed(tree, &listing)
        .iter()
        .map(|v| format!("{main}: {}", clm_check::format(v)))
        .collect();
    // `clm ledger verify` on `origin/main`: without `--review` over the whole chain, then with
    // it for the lands after the review mark only (`checkpoint-review-mark`).
    // The commit audited, resolved once so a fetch by another session in between can't
    // move the mark past lands this run never recomputed.
    let audited = git(root, &["rev-parse", &format!("{main}^{{commit}}")])?;
    let mark = review_mark(root, &audited)?;
    let (findings, held_at) = verify(root, &audited, mark.as_deref())?;
    violations.extend(
        findings
            .iter()
            .map(|f| format!("{main}: {}: {}", f.rule, f.message)),
    );
    let (graph, load_violations) = clm_model::load(tree);
    // The violations above say why nothing is judged complete or done on a graph that
    // doesn't load.
    let (complete, held) = if load_violations.is_empty() {
        let markers = clm_check::scan(tree).req;
        // Claims play no part in which changes are complete.
        let queue = queue_from_refs(&graph, &markers, &[], 0, DEFAULT_EXPIRY_HOURS);
        let held: BTreeSet<String> = graph
            .changes
            .values()
            .filter(|c| !c.archived)
            .flat_map(|c| c.frontmatter.resolves.iter().cloned())
            .collect();
        (queue.complete, Some(held))
    } else {
        (Vec::new(), None)
    };

    // 2. Landed branches, released snapshots and done backlog claims on `origin`.
    let claim_done = |slug: &str| {
        held.as_ref().is_some_and(|held| {
            held.contains(slug) || !tree.join("backlog").join(format!("{slug}.md")).is_file()
        })
    };
    let gone: Vec<(&String, &String)> = remote
        .iter()
        .filter(|(b, _)| {
            landed_remote.contains(*b)
                || b.strip_prefix(AUTOSYNC)
                    .is_some_and(|id| change_in_main(root, id))
                || b.strip_prefix(PROPOSE).is_some_and(claim_done)
        })
        .collect();
    if !gone.is_empty() {
        // Each delete is leased on the tip judged above, so a push made since the fetch
        // survives.
        let mut args = vec!["push".to_string(), "-q".into(), REMOTE.into()];
        for (b, tip) in &gone {
            args.push(format!("--force-with-lease=refs/heads/{b}:{tip}"));
            args.push(format!(":refs/heads/{b}"));
        }
        git(root, &args.iter().map(String::as_str).collect::<Vec<_>>())?;
        lines.extend(gone.iter().map(|(b, _)| format!("removed {REMOTE}/{b}")));
    }

    // 3. Work that isn't safe on `origin`. Only the session's own makes the checkpoint
    // incomplete: the checkout it runs in, the branch checked out there and this clone's
    // ledger. A build's worktree is never the session's own: `implement` already has each
    // build's outcome.
    let mut unsafe_work = Vec::new();
    let mut elsewhere = Vec::new();
    let mut own_branches = BTreeSet::new();
    for (i, wt) in worktrees.iter().enumerate() {
        let path = &wt.path;
        if !path.exists() || left.contains(path) {
            continue;
        }
        let own = *path == caller;
        // The main checkout may have moved to `main` above.
        let branch = if i == 0 {
            clm_git::current_branch(root)?
        } else {
            wt.branch.clone()
        };
        if own && let Some(b) = &branch {
            own_branches.insert(b.clone());
        }
        if branch.as_deref().is_some_and(is_flow) && !clean(path)? {
            let line = format!("dirty {}", path.display());
            if own {
                unsafe_work.push(line);
            } else {
                elsewhere.push(format!("elsewhere {line}"));
            }
        }
    }
    let remote_now = refs(root, &format!("refs/remotes/{REMOTE}/"))?;
    let pushed = |b: &str| -> Result<bool, Error> {
        Ok(remote_now.contains_key(b)
            && git(
                root,
                &[
                    "rev-list",
                    "--count",
                    &format!("{REMOTE}/{b}..refs/heads/{b}"),
                ],
            )? == "0")
    };
    // This clone's ledger: the local ledger branches whose clone part is the one `append`
    // wrote to `.git/clm/writer` in the common git directory, under whatever person part
    // it has had. Before the first append there is none.
    let clone = std::fs::read_to_string(Path::new(&common).join("clm/writer"))
        .ok()
        .map(|c| format!("/{}", c.trim()))
        .filter(|c| c.len() > 1);
    let own_ledger = |b: &&String| {
        b.starts_with(LEDGER) && clone.as_ref().is_some_and(|c| b.ends_with(c.as_str()))
    };
    for b in local.keys().filter(own_ledger) {
        if !pushed(b)? {
            unsafe_work.push(format!("unpushed {b}"));
        }
    }
    let mut open = Vec::new();
    for b in local.keys().filter(|b| is_flow(b)) {
        // A branch landed at its tip is on `main`, whatever `origin` holds now.
        if landed_local.contains(b) {
            continue;
        }
        if pushed(b)? {
            open.push(format!("open {b}"));
        } else if own_branches.contains(b) {
            unsafe_work.push(format!("unpushed {b}"));
        } else {
            elsewhere.push(format!("elsewhere unpushed {b}"));
        }
    }

    // 5 (second half). The complete changes on `origin/main`, for `implement` to archive
    // (`archive-on-sync`), and the violations.
    let ok = unsafe_work.is_empty() && violations.is_empty();
    lines.extend(unsafe_work);
    lines.extend(elsewhere);
    lines.extend(open);
    lines.extend(complete.iter().map(|id| format!("complete {id}")));
    lines.extend(violations);
    // A land found clean is never recomputed by this clone again; one with a finding is,
    // until a human acts on it.
    let next = match &held_at {
        Some(land) => format!("{land}^1"),
        None => audited,
    };
    git(root, &["update-ref", REVIEW_MARK, &next])?;
    lines.push(format!("wrap-up {}", if ok { "ok" } else { "incomplete" }));
    Ok(Report { lines, ok })
}

/// The review mark's commit, or `None` (recompute the whole chain) when the ref is unset,
/// doesn't resolve to a commit, or isn't an ancestor of `main`, as after a rewrite.
fn review_mark(root: &Path, main: &str) -> Result<Option<String>, Error> {
    let commit = format!("{REVIEW_MARK}^{{commit}}");
    match git(root, &["rev-parse", "--verify", "-q", &commit]) {
        Ok(mark) => Ok(clm_git::is_ancestor(root, &mark, main)?.then_some(mark)),
        // `--verify -q` exits 1, silently, for a ref that is unset or isn't a commit.
        Err(clm_git::Error::Failed { status, .. }) if status.code() == Some(1) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// `clm ledger verify` on `main` without `--review` over the whole chain, then with it after
/// `mark`, keeping from the second only the findings recomputing can add and that the first
/// didn't give. Also returns the oldest commit after `mark`, by the chain's order, with a
/// finding holding the mark: the recomputation's `HOLDS_MARK`, or the first audit's
/// `UNRECORDED`.
fn verify(
    root: &Path,
    main: &str,
    mark: Option<&str>,
) -> Result<(Vec<clm_ledger::Finding>, Option<String>), Error> {
    let mut findings = clm_ledger::verify(root, main, None, false).map_err(ledger_error)?;
    let recomputed = clm_ledger::verify(root, main, mark, true).map_err(ledger_error)?;
    let held: BTreeSet<String> = recomputed
        .iter()
        .filter(|f| HOLDS_MARK.contains(&f.rule))
        .chain(findings.iter().filter(|f| f.rule == UNRECORDED))
        .map(|f| f.commit.clone())
        .collect();
    for f in recomputed {
        let new = !findings
            .iter()
            .any(|g| (g.rule, &g.commit, &g.message) == (f.rule, &f.commit, &f.message));
        if RECOMPUTED.contains(&f.rule) && new {
            findings.push(f);
        }
    }
    let oldest = if held.is_empty() {
        None
    } else {
        let range = match mark {
            Some(mark) => format!("{mark}..{main}"),
            None => main.to_string(),
        };
        // Newest first, so the last one held is the oldest.
        git(root, &["rev-list", "--first-parent", &range])?
            .lines()
            .rfind(|c| held.contains(*c))
            .map(String::from)
    };
    Ok((findings, oldest))
}

/// Deletes the clone's places that don't count (`build-places`), with the default expiry, in
/// one sweep under the places lock, as `freed <name>`. A file that can't be read or parsed
/// counts, so it stays, and a place the sweep couldn't delete stays too, each as
/// `place <path>: <reason>` for the human. Neither line makes the checkpoint incomplete; a
/// sweep that fails is a git failure.
fn free_places(root: &Path) -> Result<Vec<String>, Error> {
    let swept =
        clm_orchestrator::free_stale_places(root, DEFAULT_EXPIRY_HOURS).map_err(|e| match e {
            Error::Refused(why) => Error::Git(why),
            e => e,
        })?;
    Ok(swept
        .into_iter()
        .filter_map(|Swept { place, outcome }| {
            let path = place.path.display().to_string();
            match (outcome, place.file) {
                (SweepOutcome::Freed, _) => Some(format!("freed {}", place.name)),
                // The sweep's reason may already name the file.
                (SweepOutcome::Failed(why), _) => {
                    let why = why.strip_prefix(&format!("{path}: ")).unwrap_or(&why);
                    Some(format!("place {path}: {why}"))
                }
                (SweepOutcome::Kept, Err(reason)) => Some(format!("place {path}: {reason}")),
                (SweepOutcome::Kept, Ok(_)) => None,
            }
        })
        .collect())
}

fn is_flow(branch: &str) -> bool {
    FLOW.iter().any(|p| branch.starts_with(p))
}

/// Whether `path` is where Claude Code puts a subagent's worktree: a directory `agent-*`
/// directly in a `.claude/worktrees/` directory.
fn is_build_worktree(path: &Path) -> bool {
    fn name(p: Option<&Path>) -> Option<&str> {
        p.and_then(Path::file_name).and_then(|n| n.to_str())
    }
    let parent = path.parent();
    name(Some(path)).is_some_and(|n| n.starts_with("agent-"))
        && name(parent) == Some("worktrees")
        && name(parent.and_then(Path::parent)) == Some(".claude")
}

/// What landed on `origin/main`. A branch is landed when its tip is an ancestor of
/// `origin/main` and isn't on its first-parent chain: its commits reached `main` through a
/// merge, `clm land`'s or, before the cutover, a pull request's, whether or not a `landed`
/// record names it. A tip that is merely in `main`, as a new branch's is, is a commit
/// `main` itself was at, on the chain, and doesn't count.
struct Landed {
    main: String,
    /// `origin/main`'s first-parent chain.
    chain: BTreeSet<String>,
}

impl Landed {
    fn read(root: &Path, main: &str) -> Result<Landed, Error> {
        let chain: BTreeSet<String> = git(root, &["rev-list", "--first-parent", main])?
            .lines()
            .map(str::to_string)
            .collect();
        Ok(Landed {
            main: main.to_string(),
            chain,
        })
    }

    /// The `spec/*` and `impl/*` branches under `prefix` (`refs/heads/` or
    /// `refs/remotes/origin/`) landed at their current tips, in one ancestry query.
    fn branches(&self, root: &Path, prefix: &str) -> Result<BTreeSet<String>, Error> {
        let merged = format!("--merged={}", self.main);
        let patterns: Vec<String> = FLOW.iter().map(|f| format!("{prefix}{f}")).collect();
        let mut args = vec!["for-each-ref", &merged, "--format=%(refname) %(objectname)"];
        args.extend(patterns.iter().map(String::as_str));
        Ok(git(root, &args)?
            .lines()
            .filter_map(|l| {
                let (name, tip) = l.split_once(' ')?;
                (!self.chain.contains(tip))
                    .then(|| name.strip_prefix(prefix).map(str::to_string))?
            })
            .collect())
    }
}

fn ledger_error(e: clm_ledger::Error) -> Error {
    Error::Git(format!("ledger: {e}"))
}

/// Branches under `prefix` (`refs/heads/` or `refs/remotes/origin/`) and their tips.
fn refs(root: &Path, prefix: &str) -> Result<BTreeMap<String, String>, Error> {
    let mut patterns: Vec<String> = FLOW.iter().map(|f| format!("{prefix}{f}")).collect();
    patterns.push(format!("{prefix}{AUTOSYNC}"));
    patterns.push(format!("{prefix}{PROPOSE}"));
    patterns.push(format!("{prefix}{LEDGER}"));
    let mut args = vec!["for-each-ref", "--format=%(refname) %(objectname)"];
    args.extend(patterns.iter().map(String::as_str));
    Ok(git(root, &args)?
        .lines()
        .filter_map(|l| {
            let (name, sha) = l.split_once(' ')?;
            Some((name.strip_prefix(prefix)?.to_string(), sha.to_string()))
        })
        .collect())
}

/// The files `rev` holds, as `git ls-tree -r --name-only` lists them.
fn listed_files(root: &Path, rev: &str) -> Result<Vec<std::path::PathBuf>, Error> {
    let out = clm_git::run(
        root,
        "git",
        &["ls-tree", "-r", "--name-only", "-z", rev],
        clm_git::Options::default(),
    )?;
    Ok(out
        .split(|b| *b == 0)
        .filter(|name| !name.is_empty())
        .map(|name| std::path::PathBuf::from(String::from_utf8_lossy(name).into_owned()))
        .collect())
}

fn fetch(root: &Path) -> Result<(), Error> {
    // `--prune`: branches deleted on `origin` must not be judged or reported. A fetch that
    // lost a ref race with a build in another worktree of the clone is tried again.
    clm_git::fetch_retrying(root, &["fetch", "--prune", "--quiet", REMOTE])
        .map_err(|e| Error::Git(e.to_string()))
}

/// Every worktree, the main checkout first. Paths are canonical where they exist, so they
/// compare with the caller's.
fn worktrees(root: &Path) -> Result<Vec<clm_git::Worktree>, Error> {
    let mut list = clm_git::worktrees(root)?;
    for wt in &mut list {
        if let Ok(path) = wt.path.canonicalize() {
            wt.path = path;
        }
    }
    Ok(list)
}

/// No uncommitted changes and no untracked files that aren't ignored.
fn clean(dir: &Path) -> Result<bool, Error> {
    Ok(git(dir, &["status", "--porcelain"])?.is_empty())
}

/// Whether change `id` is in `origin/main`, live or archived.
fn change_in_main(root: &Path, id: &str) -> bool {
    ["changes", "changes/archive"].iter().any(|dir| {
        git(
            root,
            &[
                "cat-file",
                "-e",
                &format!("{REMOTE}/main:{dir}/{id}/change.md"),
            ],
        )
        .is_ok()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_dirs_directly_in_claude_worktrees_are_builds() {
        // req:orchestrator.checkpoint/checkpoint@9bbd3578
        assert!(is_build_worktree(Path::new("/r/.claude/worktrees/agent-1")));
        assert!(!is_build_worktree(Path::new("/r/.claude/worktrees/p3")));
        assert!(!is_build_worktree(Path::new("/r/worktrees/agent-1")));
        assert!(!is_build_worktree(Path::new("/r/.claude/x/agent-1")));
        assert!(!is_build_worktree(Path::new(
            "/r/.claude/worktrees/a/agent-1"
        )));
    }
}
