//! `clm draft --finish <change>`, run once a draft's subagent has returned: frees the
//! draft's place (`build-places`) and, when the change has landed, removes the clean
//! worktrees Claude Code made for it and its branches; run on a fold's batch once it has
//! landed, it tidies the fold's worktree and branch the same way. A draft that hasn't landed, such as a
//! change that needs review, waiting for its fold or for approval, keeps its claims, branches and
//! worktree: only the place goes.

use std::env;
use std::path::Path;

use clm_git::git;
use clm_model::parse_change_frontmatter;
use clm_orchestrator::{PlaceFile, REMOTE, free_place, places};

use crate::{Error, fetch_origin};

/// Frees place `change` and, when `origin/main` holds `changes/<change>/` or
/// `changes/archive/<change>/`, or the local `spec/<change>` has landed (a fold's
/// `spec/<batch>`, by [`branch_landed`]), removes each clean worktree under the main worktree's
/// `.claude/worktrees/` that has `spec/<change>` checked out, then deletes `spec/<change>`
/// locally and on `origin` with `autosync/spec/<change>`. Returns what it did, one line
/// each.
pub fn finish(root: &Path, change: &str, expiry_hours: u64) -> Result<String, Error> {
    let root = &root
        .canonicalize()
        .map_err(|e| Error::Refused(format!("{}: {e}", root.display())))?;
    if change.is_empty() || change.contains('/') {
        return Err(Error::Refused(format!("`{change}` isn't a change ID")));
    }
    fetch_origin(root)?;
    let mut done = Vec::new();
    let branch = format!("spec/{change}");
    // A fold's branch `spec/<batch>` names no change, so only the ancestry test finds it.
    let by_tree = landed(root, change);
    let by_branch = branch_landed(root, &branch)?;
    let place = places(root, expiry_hours)?
        .into_iter()
        .find(|p| p.name == change);
    if let Some(place) = place {
        if let Ok(file) = &place.file {
            let waiting = !(by_tree || by_branch);
            release_claims(root, change, file, expiry_hours, waiting, &mut done)?;
        }
        free_place(root, change)?;
        done.push(format!("freed {change}"));
    }
    if !by_tree && !by_branch {
        return Ok(done.join("\n"));
    }
    let list = worktrees(root)?;
    // Claude Code's worktrees, under the main worktree whichever worktree this runs in.
    let Some(main) = list.first() else {
        return Ok(done.join("\n"));
    };
    let ours = main.path.join(".claude/worktrees");
    // The worktree this runs in is the orchestrating session's own, never a draft's.
    let here = env::current_dir().ok().and_then(|d| d.canonicalize().ok());
    let own = |p: &Path| root.starts_with(p) || here.as_ref().is_some_and(|h| h.starts_with(p));
    let mut kept = false;
    for wt in &list[1..] {
        // One whose directory is gone has nothing to remove.
        if wt.branch.as_deref() != Some(branch.as_str()) || !wt.path.is_dir() {
            continue;
        }
        if !wt.path.starts_with(&ours) || own(&wt.path) {
            // Not a draft's to remove, but it still holds the branch's work.
            kept = true;
            continue;
        }
        if holds_only_history(&wt.path) {
            let path = wt.path.to_string_lossy();
            // Claude Code locks the worktrees it makes; unlocking one that isn't fails.
            let _ = git(root, &["worktree", "unlock", &path]);
            // One that can't be removed is reported, and the others are still tidied.
            match git(root, &["worktree", "remove", "--force", &path]) {
                Ok(_) => done.push(format!("removed {}", wt.path.display())),
                Err(e) => {
                    kept = true;
                    done.push(format!("kept {}: {e}", wt.path.display()));
                }
            }
        } else {
            kept = true;
            done.push(format!("kept {}: uncommitted changes", wt.path.display()));
        }
    }
    // Landed by the ancestry test alone, `origin`'s branches are judged by their own tips:
    // `spec/<change>` goes only when its tip is in `main`, and `autosync/spec/<change>`
    // only when no worktree was kept, since a kept worktree's work is what it holds.
    let main_ref = format!("{REMOTE}/main");
    let doomed = |name: &str, tip: &str| -> Result<bool, Error> {
        Ok(if by_tree {
            true
        } else if name == branch {
            clm_git::is_ancestor(root, tip, &main_ref)?
        } else {
            !kept
        })
    };
    delete_branches(root, &branch, doomed, &mut done)?;
    Ok(done.join("\n"))
}

/// Whether the worktree at `path` holds nothing outside history: no unstaged change, no
/// untracked file that isn't ignored, and an index whose tree is the tree of a commit
/// reachable from its `HEAD`, which may not be `HEAD` itself when the branch moved on in
/// another worktree. Anything git can't answer, such as an unmerged index, keeps it.
fn holds_only_history(path: &Path) -> bool {
    git(path, &["diff", "--quiet"]).is_ok()
        && git(path, &["ls-files", "--others", "--exclude-standard"]).is_ok_and(|o| o.is_empty())
        && git(path, &["write-tree"]).is_ok_and(|tree| {
            git(path, &["log", "--format=%T", "HEAD"]).is_ok_and(|l| l.lines().any(|t| t == tree))
        })
}

/// Deletes on `origin` each `propose/<slug>` the draft's `place` names, leased on the tip
/// read, printing `released propose/<slug>`. A claim committed at or after the place's
/// `taken` plus the expiry is another session's and stays; so does, while the change is
/// `waiting` for review or its fold, the claim of an item the change doesn't resolve but
/// whose `backlog/` file its branch changes, since that edit reaches `main` only with the
/// change. A claim already gone, or one whose lease is refused, is skipped.
fn release_claims(
    root: &Path,
    change: &str,
    place: &PlaceFile,
    expiry_hours: u64,
    waiting: bool,
    done: &mut Vec<String>,
) -> Result<(), Error> {
    let moved = if waiting {
        moved_items(root, change)?
    } else {
        Vec::new()
    };
    for slug in place
        .branches
        .iter()
        .filter_map(|b| b.strip_prefix("propose/"))
    {
        let name = format!("propose/{slug}");
        let rf = format!("refs/remotes/{REMOTE}/{name}");
        let Ok(read) = git(root, &["log", "-1", "--format=%H %ct", &rf]) else {
            continue;
        };
        // A time that can't be read leaves the claim, which may be another session's.
        let Some((sha, committed)) = read
            .split_once(' ')
            .and_then(|(sha, time)| Some((sha, time.parse::<u64>().ok()?)))
        else {
            continue;
        };
        if committed >= place.taken + expiry_hours * 3600 || moved.iter().any(|m| m == slug) {
            continue;
        }
        let lease = format!("--force-with-lease=refs/heads/{name}:{sha}");
        let target = format!(":refs/heads/{name}");
        if let Err(e) = git(root, &["push", "--quiet", &lease, REMOTE, &target]) {
            // The ref gone or moved is the lease doing its work; anything else is git's.
            let now = git(root, &["ls-remote", "--heads", REMOTE, &name])?;
            if now.split_whitespace().next() == Some(sha) {
                return Err(e.into());
            }
            continue;
        }
        done.push(format!("released {name}"));
    }
    Ok(())
}

/// The items whose `backlog/<slug>.md` `origin/spec/<change>` changes since its merge base
/// with `origin/main` and whose `change.md` doesn't resolve.
fn moved_items(root: &Path, change: &str) -> Result<Vec<String>, Error> {
    let spec = format!("refs/remotes/{REMOTE}/spec/{change}");
    if git(root, &["rev-parse", "--verify", "-q", &spec]).is_err() {
        return Ok(Vec::new());
    }
    let path = format!("{spec}:changes/{change}/change.md");
    let resolves = git(root, &["show", &path])
        .ok()
        .and_then(|text| parse_change_frontmatter(&text))
        .map(|fm| fm.resolves)
        .unwrap_or_default();
    let range = format!("{REMOTE}/main...{spec}");
    let changed = git(root, &["diff", "--name-only", &range, "--", "backlog/"])?;
    Ok(changed
        .lines()
        .filter_map(|f| f.strip_prefix("backlog/")?.strip_suffix(".md"))
        .filter(|slug| !resolves.iter().any(|r| r == slug))
        .map(str::to_string)
        .collect())
}

/// Whether the local `branch` has landed as `clm wrap-up` judges a branch: its tip is an
/// ancestor of `origin/main` and isn't on `origin/main`'s first-parent chain, so a branch
/// cut from `main` with no commit of its own hasn't. The rule is wrap-up's, built here
/// rather than shared, since this crate doesn't depend on `clm-checkpoint`.
fn branch_landed(root: &Path, branch: &str) -> Result<bool, Error> {
    let Ok(tip) = git(
        root,
        &[
            "rev-parse",
            "--verify",
            "-q",
            &format!("refs/heads/{branch}"),
        ],
    ) else {
        return Ok(false);
    };
    let main = format!("refs/remotes/{REMOTE}/main");
    if !clm_git::is_ancestor(root, &tip, &main)? {
        return Ok(false);
    }
    let chain = git(root, &["rev-list", "--first-parent", &main])?;
    Ok(!chain.lines().any(|c| c == tip))
}

/// Deletes the landed draft's `branch`: the local one when no worktree has it checked out,
/// and, of it and its `autosync/` twin on `origin`, each that `doomed(name, tip)` allows,
/// leased on the tip the fetch read. A folded draft's change reached `main` on the fold's
/// branch, so nothing else deletes them.
fn delete_branches(
    root: &Path,
    branch: &str,
    doomed: impl Fn(&str, &str) -> Result<bool, Error>,
    done: &mut Vec<String>,
) -> Result<(), Error> {
    let exists = |rf: &str| git(root, &["rev-parse", "--verify", "-q", rf]).ok();
    let checked_out = clm_git::worktrees(root)?
        .iter()
        .any(|wt| wt.branch.as_deref() == Some(branch));
    if let Some(tip) = exists(&format!("refs/heads/{branch}")).filter(|_| !checked_out) {
        // Commits that reached neither `main` nor `origin`'s branch would be lost.
        let pushed = [format!("{REMOTE}/main"), format!("{REMOTE}/{branch}")]
            .iter()
            .any(|base| clm_git::is_ancestor(root, &tip, base).unwrap_or(false));
        if pushed {
            git(root, &["branch", "-D", "--", branch])?;
            done.push(format!("removed {branch}"));
        } else {
            done.push(format!("kept {branch}: commits not on {REMOTE}"));
        }
    }
    for name in [branch.to_string(), format!("autosync/{branch}")] {
        let Some(tip) = exists(&format!("refs/remotes/{REMOTE}/{name}")) else {
            continue;
        };
        if !doomed(&name, &tip)? {
            continue;
        }
        let lease = format!("--force-with-lease=refs/heads/{name}:{tip}");
        git(
            root,
            &[
                "push",
                "--quiet",
                &lease,
                REMOTE,
                &format!(":refs/heads/{name}"),
            ],
        )?;
        done.push(format!("removed {REMOTE}/{name}"));
    }
    Ok(())
}

/// Whether the draft `change` has landed, by its place's test: `origin/main`'s tree holds
/// `changes/<change>/` or `changes/archive/<change>/`.
pub(crate) fn landed(root: &Path, change: &str) -> bool {
    ["changes", "changes/archive"].iter().any(|dir| {
        let rev = format!("refs/remotes/{REMOTE}/main:{dir}/{change}");
        git(root, &["rev-parse", "--verify", "-q", &rev]).is_ok()
    })
}

/// Every worktree, the main checkout first, with canonical paths where they exist so they
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
