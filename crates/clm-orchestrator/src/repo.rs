//! The commands that work on a checkout and its `origin`: `next` and `claim`, the `claims`
//! rows of `clm status`, and the git helpers the other commands share.

use clm_git::{git, now};
use std::collections::BTreeMap;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use clm_check::Marker;
use clm_model::{BacklogKind, Graph};

use crate::queue::{ClaimRef, Queue, State, claim_state, claims_section, queue_from_refs};
use crate::{Error, REMOTE};

pub(crate) const BRANCH_PREFIX: &str = "impl/";
pub(crate) const PROPOSE_PREFIX: &str = "propose/";

/// The branch checked out at `root`; `None` when detached or not a repository.
pub(crate) fn current_branch(root: &Path) -> Option<String> {
    clm_git::current_branch(root).ok().flatten()
}

/// What `clm next` prints: the `ready` nodes, read from `origin/main` after fetching, the
/// most urgent first, at most as many as the clone has free build places of `max_builds`
/// (`build-places`). The places only cut the list: they never change which nodes are ready.
pub fn next(root: &Path, expiry_hours: u64, max_builds: u64) -> Result<Vec<String>, Error> {
    let mut ready = queue_on_main(root, expiry_hours)?.0.next();
    let held = crate::places(root, expiry_hours)?
        .iter()
        .filter(|p| p.counts)
        .count();
    let free = max_builds.saturating_sub(held as u64);
    ready.truncate(usize::try_from(free).unwrap_or(usize::MAX));
    Ok(ready)
}

/// `clm next --wait` (`next-wait`): looks as [`next`] does and, while a look prints nothing,
/// waits `interval_minutes` (a positive decimal number) and looks again, until a look
/// returns nodes. The first look's failure is returned; a later look's failure is passed to
/// `note` as `clm next failed: <reason>` and waited out. An interval that isn't a positive
/// number is refused before any look.
pub fn next_wait(
    root: &Path,
    expiry_hours: u64,
    max_builds: u64,
    interval_minutes: &str,
    note: &mut dyn FnMut(&str),
) -> Result<Vec<String>, Error> {
    let interval = interval_minutes
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|m| *m > 0.0)
        .and_then(|m| Duration::try_from_secs_f64(m * 60.0).ok())
        .filter(|d| !d.is_zero())
        .ok_or_else(|| {
            Error::Refused(format!(
                "the interval must be a positive number of minutes, not `{interval_minutes}`"
            ))
        })?;
    let mut ready = next(root, expiry_hours, max_builds)?;
    while ready.is_empty() {
        std::thread::sleep(interval);
        match next(root, expiry_hours, max_builds) {
            Ok(nodes) => ready = nodes,
            Err(Error::Spec(violations)) => note(&format!(
                "clm next failed: the specs don't load ({} violations)",
                violations.len()
            )),
            Err(Error::Refused(why) | Error::Git(why)) => note(&format!("clm next failed: {why}")),
        }
    }
    Ok(ready)
}

/// Fetches, then builds the queue from `origin/main`'s tree and the claims on `origin`,
/// which it also returns.
fn queue_on_main(
    root: &Path,
    expiry_hours: u64,
) -> Result<(Queue, BTreeMap<String, Claim>), Error> {
    fetch(root)?;
    let claims = claims(root, BRANCH_PREFIX)?;
    let refs: Vec<ClaimRef> = claims.iter().map(|(node, c)| c.to_ref(node)).collect();
    let queue = with_main(root, |graph, markers| {
        Ok(queue_from_refs(graph, markers, &refs, now(), expiry_hours))
    })?;
    Ok((queue, claims))
}

/// Claims `node` by pushing `impl/<node>`, or `backlog/<slug>` by pushing `propose/<slug>`
/// (`propose-claims`), and returns the branch name.
pub fn claim(root: &Path, node: &str, expiry_hours: u64) -> Result<String, Error> {
    match node.strip_prefix("backlog/") {
        Some(slug) => crate::propose::claim_backlog(root, slug, expiry_hours),
        None => claim_node(root, node, expiry_hours),
    }
}

/// Why a node in `state` can't be claimed, or `None` when it can.
pub fn refusal(node: &str, state: Option<&State>) -> Option<String> {
    let quoted = |items: &[String]| {
        items
            .iter()
            .map(|i| format!("`{i}`"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    Some(match state {
        None => format!("no component `{node}`"),
        Some(State::Ready { .. }) => return None,
        Some(State::Converged) => format!(
            "`{node}` is converged: nothing is pending. A small change that needs no spec change \
             takes no claim: land it from a branch of its own with `clm land`"
        ),
        Some(State::Blocked { by }) => format!(
            "`{node}` is blocked by {}",
            quoted(
                &by.iter()
                    .map(|b| b.blocker().to_string())
                    .collect::<Vec<_>>()
            )
        ),
        Some(State::Claimed { .. }) => {
            format!("`{node}` is claimed: `{REMOTE}/{BRANCH_PREFIX}{node}` has a live commit")
        }
    })
}

pub fn claim_node(root: &Path, node: &str, expiry_hours: u64) -> Result<String, Error> {
    node_id(node)?;
    let (queue, mut claims) = queue_on_main(root, expiry_hours)?;
    if let Some(why) = refusal(node, queue.node(node).map(|n| &n.state)) {
        return Err(Error::Refused(why));
    }
    // The queue says the node isn't claimed, so any branch there is released: lapsed or
    // merged.
    push_node_claim(root, node, claims.remove(node))
}

/// Claims `node` for a code-only fix of bug or deferred item `item` (`backlog/<slug>`), whatever its
/// state in the queue, unless another session holds it (`code-fix-resolves-bug`).
pub fn claim_fix(root: &Path, node: &str, item: &str, expiry_hours: u64) -> Result<String, Error> {
    node_id(node)?;
    let slug = item
        .strip_prefix("backlog/")
        .filter(|s| !s.is_empty() && !s.contains('/'))
        .ok_or_else(|| Error::Refused(format!("`{item}` isn't a backlog item")))?;
    let graph = main_with_node(root, node)?;
    let fm = &graph
        .backlog
        .get(slug)
        .ok_or_else(|| {
            Error::Refused(format!(
                "no such item: `{item}.md` isn't in `{REMOTE}/main`"
            ))
        })?
        .frontmatter;
    if !matches!(fm.kind, BacklogKind::Bug | BacklogKind::Deferred) {
        return Err(Error::Refused(format!(
            "`{item}` is neither a bug nor deferred work: its kind is `{}`",
            fm.kind
        )));
    }
    if !fm.nodes.iter().any(|n| n == node) {
        return Err(Error::Refused(format!("`{item}` doesn't name `{node}`")));
    }
    let claim = released_claim(root, node, expiry_hours)?;
    let branch = format!("{BRANCH_PREFIX}{node}");
    can_switch(root, &branch)?;
    push_node_claim(root, node, claim)?;
    switch_to_claim(root, &branch).map_err(|why| {
        Error::Git(format!(
            "`{branch}` is claimed on `{REMOTE}` but the checkout couldn't switch to it: \
             {why}\nfinish by hand with `git switch -C {branch} {REMOTE}/{branch}`"
        ))
    })?;
    Ok(branch)
}

/// Fetches and loads `origin/main`'s graph, refusing when it has no node `node`.
fn main_with_node(root: &Path, node: &str) -> Result<Graph, Error> {
    fetch(root)?;
    let graph = load(extract_main(root)?.path())?;
    if !graph.nodes.contains_key(node) {
        return Err(Error::Refused(format!("no component `{node}`")));
    }
    Ok(graph)
}

/// The claim on `origin/impl/<node>` as of the fetch, released (lapsed or merged) or none,
/// or the refusal of a live one: the queue's own rule, so every kind of claim agrees.
fn released_claim(root: &Path, node: &str, expiry_hours: u64) -> Result<Option<Claim>, Error> {
    let claim = claims(root, BRANCH_PREFIX)?.remove(node);
    if let Some(c) = &claim
        && claim_state(&c.to_ref(node), now(), expiry_hours) == "live"
    {
        let lapses_at = c.time.saturating_add(expiry_hours.saturating_mul(3600));
        return Err(Error::Refused(
            refusal(node, Some(&State::Claimed { lapses_at })).unwrap_or_default(),
        ));
    }
    Ok(claim)
}

/// Refuses, before anything is pushed, a fix claim whose switch to `branch` would fail
/// (`fix-claims-switch`): uncommitted changes to tracked files, `branch` checked out in
/// another worktree, or a local `branch` holding commits `origin/main` doesn't.
fn can_switch(root: &Path, branch: &str) -> Result<(), Error> {
    if !git(root, &["status", "--porcelain", "--untracked-files=no"])?.is_empty() {
        return Err(Error::Refused(
            "the checkout has uncommitted changes to tracked files: commit them or set them \
             aside, then claim again"
                .to_string(),
        ));
    }
    let top = canonical(Path::new(&git(root, &["rev-parse", "--show-toplevel"])?));
    if let Some(w) = clm_git::worktrees(root)?
        .into_iter()
        .find(|w| w.branch.as_deref() == Some(branch) && canonical(&w.path) != top)
    {
        let path = canonical(&w.path);
        let remedy = if path.exists() {
            format!(
                "switch it off the branch or remove it (`git worktree remove {}`)",
                path.display()
            )
        } else {
            "it no longer exists, so run `git worktree prune`".to_string()
        };
        return Err(Error::Refused(format!(
            "`{branch}` is checked out in the worktree at {}: {remedy}, then claim again",
            path.display()
        )));
    }
    let local = git(
        root,
        &[
            "for-each-ref",
            "--format=%(objectname)",
            &format!("refs/heads/{branch}"),
        ],
    )?;
    if !local.is_empty() && !clm_git::is_ancestor(root, &local, &format!("{REMOTE}/main"))? {
        return Err(Error::Refused(format!(
            "the local `{branch}` holds commits `{REMOTE}/main` doesn't: land it, or delete \
             it (`git branch -D {branch}`), then claim again"
        )));
    }
    Ok(())
}

/// `path` with symbolic links resolved, or as given when it doesn't exist.
fn canonical(path: &Path) -> std::path::PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// Moves the local `branch` to `origin/<branch>`, tracking it, then switches the checkout
/// to it: the ref before the files, so a rejected ref update leaves the working tree as it
/// was. A switch that fails after the ref moved leaves the local branch at the claim, where
/// the hand remedy puts it anyway. A checkout already on `branch` is reset with `git switch -C`, since `git branch -f`
/// refuses the current branch. The error is git's reason.
fn switch_to_claim(root: &Path, branch: &str) -> Result<(), String> {
    let remote = format!("{REMOTE}/{branch}");
    let run = |args: &[&str]| git(root, args).map(drop).map_err(|e| e.to_string());
    if current_branch(root).as_deref() == Some(branch) {
        run(&["switch", "-q", "-C", branch, "--track", &remote])
    } else {
        run(&["branch", "-q", "-f", "--track", branch, &remote])?;
        run(&["switch", "-q", branch])
    }
}

/// Refuses `node` unless it can be a node ID: a node ID never contains `/`.
fn node_id(node: &str) -> Result<(), Error> {
    if node.is_empty() || node.contains('/') {
        return Err(Error::Refused(format!("`{node}` isn't a component ID")));
    }
    Ok(())
}

/// Pushes a new claim to `impl/<node>`, whose released branch, if any, is `released`: the
/// lease expects its tip, or no branch at all.
fn push_node_claim(root: &Path, node: &str, released: Option<Claim>) -> Result<String, Error> {
    let expect = released.map_or(String::new(), |c| c.sha);
    push_new_claim(root, node, &format!("{BRANCH_PREFIX}{node}"), &expect)
}

/// Pushes a new claim commit on `origin/main` to `branch`, expecting `expect` as its tip
/// (empty: no branch), with the retry and rejection handling the Design describes.
pub(crate) fn push_new_claim(
    root: &Path,
    what: &str,
    branch: &str,
    expect: &str,
) -> Result<String, Error> {
    let main = format!("{REMOTE}/main");
    let tree = git(root, &["rev-parse", &format!("{main}^{{tree}}")])?;
    let parent = git(root, &["rev-parse", &main])?;
    let commit = git(
        root,
        &[
            "commit-tree",
            &tree,
            "-p",
            &parent,
            "-m",
            &format!("Claim {what}\n\nclaim-id: {}", claim_id()),
        ],
    )?;
    let commit = commit.as_str();
    // A rejected push isn't always a lost race, so the tip on `origin` decides. Two retries
    // cover a transient failure, such as a lock held by a parallel push from this repo or a
    // forge's `fatal error in commit_refs`; the lease still guards each swap.
    let mut waits = RETRY_WAITS_SECS.iter();
    loop {
        let Err(why) = push_claim(root, branch, commit, expect)? else {
            return Ok(branch.to_string());
        };
        let tip = remote_tip(root, branch).map_err(|e| match e {
            Error::Git(m) => Error::Git(format!("git push: {why}\n{m}")),
            e => e,
        })?;
        if tip == commit {
            // The push landed but failed locally, so the tracking ref `clm run` makes its
            // local branch from may not have moved.
            git(
                root,
                &[
                    "update-ref",
                    &format!("refs/remotes/{REMOTE}/{branch}"),
                    commit,
                ],
            )?;
            return Ok(branch.to_string());
        }
        if tip != expect {
            return Err(Error::Refused(format!(
                "`{what}` was claimed by someone else first"
            )));
        }
        let Some(&wait) = waits.next() else {
            // The tip shows nothing was claimed, so running the command again is safe.
            return Err(Error::Git(format!(
                "git push: {why}\nthe failure may be transient: nothing was claimed, so it \
                 is safe to run the same command again"
            )));
        };
        std::thread::sleep(std::time::Duration::from_secs(wait));
    }
}

/// The fixed waits before each retry of a rejected claim push whose tip didn't move.
const RETRY_WAITS_SECS: [u64; 2] = [1, 2];

/// Pushes `commit` to `branch` on `origin` if its tip is still `expect` (empty: no branch).
/// The inner `Err` is a rejected push, with git's reason.
fn push_claim(
    root: &Path,
    branch: &str,
    commit: &str,
    expect: &str,
) -> Result<Result<(), String>, Error> {
    let pushed = git(
        root,
        &[
            "push",
            REMOTE,
            &format!("--force-with-lease=refs/heads/{branch}:{expect}"),
            &format!("{commit}:refs/heads/{branch}"),
        ],
    );
    match pushed {
        Ok(_) => Ok(Ok(())),
        Err(clm_git::Error::Failed { stderr, .. }) => Ok(Err(stderr)),
        Err(e) => Err(e.into()),
    }
}

/// The tip of `branch` on `origin` as `ls-remote` sees it now, or "" when there is none.
fn remote_tip(root: &Path, branch: &str) -> Result<String, Error> {
    let out = git(
        root,
        &["ls-remote", REMOTE, &format!("refs/heads/{branch}")],
    )?;
    Ok(out
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .to_string())
}

/// The `impl/*` claim refs as of the last fetch, as `queue_from_refs` takes them: empty
/// when `root` isn't a git repository. Never fetches.
pub fn claim_refs(root: &Path) -> Vec<ClaimRef> {
    claims(root, BRANCH_PREFIX)
        .unwrap_or_default()
        .iter()
        .map(|(node, c)| c.to_ref(node))
        .collect()
}

/// The `propose/*` backlog claims as of the last fetch, `node` holding the slug; a backlog
/// claim is never merged (`propose-claims`). Never fetches.
pub fn propose_refs(root: &Path) -> Vec<ClaimRef> {
    claims(root, PROPOSE_PREFIX)
        .unwrap_or_default()
        .into_iter()
        .map(|(slug, c)| ClaimRef {
            node: slug,
            tip_time: c.time,
            merged: false,
        })
        .collect()
}

/// [`claims_section`] for the refs at `root` as of the last fetch. Never fetches.
pub fn status_claims(root: &Path) -> Vec<Vec<String>> {
    claims_section(&claim_refs(root), &propose_refs(root), now())
}

pub(crate) fn load(root: &Path) -> Result<Graph, Error> {
    let (graph, violations) = clm_model::load(root);
    if violations.is_empty() {
        Ok(graph)
    } else {
        Err(Error::Spec(violations))
    }
}

/// Runs `f` on the graph at `origin/main` and the markers of its files, extracted with a
/// temporary index so neither the checkout nor the real index is touched.
pub(crate) fn with_main<T>(
    root: &Path,
    f: impl FnOnce(&Graph, &[Marker]) -> Result<T, Error>,
) -> Result<T, Error> {
    let tmp = extract_main(root)?;
    let tree = tmp.path();
    let graph = load(tree)?;
    let markers = clm_check::scan(tree).req;
    f(&graph, &markers)
}

/// `origin/main`'s tree in a temporary directory, kept while the `TempDir` lives.
pub(crate) fn extract_main(root: &Path) -> Result<tempfile::TempDir, Error> {
    Ok(clm_git::extract_tree(root, &format!("{REMOTE}/main"))?)
}

/// A claim branch on `origin` as of the last fetch.
pub struct Claim {
    pub sha: String,
    /// Committer time of the branch tip, in Unix seconds.
    pub time: u64,
    /// The tip is an ancestor of `origin/main`: its PR has landed.
    pub merged: bool,
}

impl Claim {
    /// This claim as `queue_from_refs` takes it, for the branch named `name` after `impl/`.
    fn to_ref(&self, name: &str) -> ClaimRef {
        ClaimRef {
            node: name.to_string(),
            tip_time: self.time,
            merged: self.merged,
        }
    }
}

/// A random ID that makes every claim commit unique. Two sessions with the same identity
/// claiming in the same second would otherwise build the same commit, and pushing a commit
/// the branch already holds succeeds without checking the lease.
fn claim_id() -> String {
    use std::hash::{BuildHasher, Hasher};
    let mut h = std::collections::hash_map::RandomState::new().build_hasher();
    h.write_u128(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos()),
    );
    h.write_u32(std::process::id());
    format!("{:016x}", h.finish())
}

pub(crate) fn fetch(root: &Path) -> Result<(), Error> {
    // Not `clm_git::fetch`: claims and spec work whose branches were deleted must go.
    git(root, &["fetch", "--prune", "--quiet", REMOTE])?;
    Ok(())
}

/// Claims keyed by name, from `refs/remotes/origin/<prefix>`. A name containing `/`, such
/// as the archive branch `impl/archive/<id>`, is no claim and is skipped
/// (`nested-branches-not-claims`).
pub(crate) fn claims(root: &Path, prefix: &str) -> Result<BTreeMap<String, Claim>, Error> {
    let full = format!("refs/remotes/{REMOTE}/{prefix}");
    let refs = git(
        root,
        &[
            "for-each-ref",
            "--format=%(refname) %(objectname) %(committerdate:unix)",
            &full,
        ],
    )?;
    let main = format!("refs/remotes/{REMOTE}/main");
    Ok(refs
        .lines()
        .filter_map(|l| {
            let mut parts = l.split(' ');
            let name = parts.next()?.strip_prefix(&full)?.to_string();
            if name.contains('/') {
                return None;
            }
            let sha = parts.next()?.to_string();
            let time = parts.next()?.parse().ok()?;
            // `clm status` reads these offline, where there may be no `origin/main` yet:
            // then nothing has merged.
            let merged = clm_git::is_ancestor(root, &sha, &main).unwrap_or(false);
            Some((name, Claim { sha, time, merged }))
        })
        .collect())
}
