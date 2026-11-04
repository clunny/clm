//! Keeps branches converged with `main`: `clm sync` and `clm autosync` (see
//! specs/sync/SPEC.md).

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use clm_git::{Options, REMOTE};
use clm_regenerate::{Regenerated, exists_at, extract_present, git, io, regenerated, run};
use scratch::Scratch;

mod items;
mod renames;
mod scratch;

/// `main` on the remote, as fetched: what every sync rebases onto.
fn main_ref() -> String {
    format!("{REMOTE}/main")
}
/// The paths autosync snapshots (`autosync-policy`), as `clm gate` counts spec paths.
const SPEC_PATHS: [&str; 7] = [
    "specs",
    "changes",
    "backlog",
    "docs",
    "ARCHITECTURE.md",
    "CAPABILITIES.md",
    "CLAUDE.md",
];

/// A refusal (`Refused`, exit 1) or a git failure (`Git`, exit 2): `clm-git`'s shared
/// `Failure` (`shared-plumbing-in-git`).
pub use clm_git::Failure as Error;

/// What a `clm sync` run found and did.
#[derive(Debug, PartialEq, Eq)]
pub struct Report {
    /// One line each: what is left for the caller (`conflict <path>`, a stale delta, a
    /// `spec changed` report, a check violation) and what it did (`activated <node>`,
    /// `archived <id>`, `filed <item>`).
    pub lines: Vec<String>,
    /// Nothing is left: the branch is up to date and passes.
    pub ok: bool,
    /// What went wrong without changing the outcome, such as a temporary worktree it
    /// couldn't remove; the caller prints them on stderr.
    pub warnings: Vec<String>,
}

impl Report {
    /// A run that stopped with `left` for the caller.
    fn left(left: Vec<String>) -> Self {
        Report {
            lines: left,
            ok: false,
            warnings: Vec::new(),
        }
    }
}

/// Rebases the current branch onto `origin/main`, regenerates the `specs/` of any applied
/// change it adds or `done` new archived change (`sync-new-archived-change`), reports a
/// spec that moved under an `impl/<node>` branch, archives the changes an `impl/*` branch
/// completes, files the open web proposals on an `impl/*` branch, files reconsider items
/// for an `impl/<node>` branch's node, and runs `clm check`. Running it again after
/// resolving what it reports carries on. Never pushes.
///
/// `settings` loads, from the CLM root it is given, `config`'s `health`, or says why the
/// settings can't be loaded, as a line to print. It is called once on every branch that
/// gets past the rebase and the regenerate step, with the CLM root of `HEAD` checked out in
/// a temporary worktree, so it reads the settings `main` brought in and never an
/// uncommitted edit; an error makes the run incomplete.
pub fn sync(
    root: &Path,
    settings: impl FnOnce(&Path) -> Result<clm_health::Settings, String>,
) -> Result<Report, Error> {
    let marker = git_path(root, "clm/sync-rebase")?;
    let rebasing =
        git_path(root, "rebase-merge")?.exists() || git_path(root, "rebase-apply")?.exists();
    let ours = rebasing && marker.exists();
    if !rebasing && marker.exists() {
        // A rebase it started has since finished or been aborted.
        fs::remove_file(&marker).map_err(|e| io(&marker, e))?;
    }
    // The commit it rebases onto and the changes this branch adds (live, or archived on the
    // branch), with which of them step 4 regenerates, decided before the rebase starts and
    // kept in the marker: mid-rebase `HEAD` is a partial branch, and a resume must use the
    // same driver and regenerate against the same `main` the rebase used, not a newer one. A change counts as added only if it is
    // also absent at the fresh merge base, so one that `main` archived or removed since the
    // branch was cut isn't re-applied. The marker holds `onto`, then a line per added change:
    // its ID when it is regenerated, `added <id>` when it isn't.
    let (onto, added, fresh) = if ours {
        let text = fs::read_to_string(&marker).map_err(|e| io(&marker, e))?;
        let mut lines = text.lines();
        let onto = lines.next().unwrap_or_default().to_string();
        let added = lines
            .map(|l| match l.strip_prefix("added ") {
                Some(id) => (id.to_string(), false),
                None => (l.to_string(), true),
            })
            .collect();
        (onto, added, None)
    } else {
        let Some(branch) = clm_git::current_branch(root)? else {
            return Err(Error::Refused(
                "`HEAD` is detached; check out a branch".into(),
            ));
        };
        if branch == "main" {
            return Err(Error::Refused("on `main`; sync a branch instead".into()));
        }
        clm_git::fetch(root)?;
        let onto = git(root, &["rev-parse", &main_ref()])?;
        let fresh = merge_base(root, "HEAD", &onto)?;
        let work = uncommitted(root, &branch, &onto, fresh.as_deref())?;
        if !work.is_empty() {
            return Ok(Report::left(
                work.iter().map(|p| format!("uncommitted {p}")).collect(),
            ));
        }
        let added = added_changes(root, &onto, fresh.as_deref())?;
        (onto, added, fresh)
    };
    let applied: Vec<String> = added
        .iter()
        .filter(|(_, applied)| *applied)
        .map(|(id, _)| id.clone())
        .collect();
    // The branch's merge base with `main` before the rebase, for the spec-changed step. Kept
    // in its own file until a run completes, so a resume, or a rebase finished with plain
    // git, still knows what the rebase brought in.
    let base_file = git_path(root, "clm/sync-base")?;
    let kept = fs::read_to_string(&base_file)
        .ok()
        .map(|b| b.trim().to_string());
    let base = if ours {
        kept
    } else {
        let base = match kept {
            // Not `clm_git::is_ancestor`: a kept commit since collected is just stale.
            Some(k) if succeeded(root, &["merge-base", "--is-ancestor", &k, "HEAD"])?.is_some() => {
                Some(k)
            }
            _ => fresh.clone(),
        };
        match &base {
            Some(b) => write(&base_file, b)?,
            None if base_file.exists() => {
                fs::remove_file(&base_file).map_err(|e| io(&base_file, e))?
            }
            None => {}
        }
        base
    };

    // `regenerate-renames`: a branch that adds a rename is rebuilt on `origin/main`, not
    // rebased.
    if !ours && let Some(merge_base) = fresh.as_deref() {
        match renames::regenerate(root, &onto, merge_base, &added)? {
            renames::Outcome::Other => {}
            renames::Outcome::Left(lines) => return Ok(Report::left(lines)),
            renames::Outcome::Done => {
                return conclude(root, settings, &onto, base.as_deref(), &base_file, &[]);
            }
        }
    }

    // `sync-owns-merge-drivers`: on a branch that adds a change, whatever its status,
    // `specs/` keeps the side being rebased onto (a driver that leaves `%A`): a file nobody
    // wrote by hand, which step 4 rebuilds when the change is applied.
    let keep = !added.is_empty();
    let attributes = git_path(root, "clm/attributes")?;
    let rules = if !keep {
        ""
    } else {
        "specs/** merge=clm-keep\n"
    };
    write(&attributes, rules)?;
    let config = [
        "-c".to_string(),
        format!("core.attributesFile={}", attributes.display()),
        "-c".to_string(),
        "merge.clm-keep.name=keep the side being rebased onto".to_string(),
        "-c".to_string(),
        "merge.clm-keep.driver=true".to_string(),
        "-c".to_string(),
        "rebase.backend=merge".to_string(),
    ];
    let mut continuing = ours;
    if !ours {
        let mut text = onto.clone();
        for (id, applied) in &added {
            text.push('\n');
            if !applied {
                text.push_str("added ");
            }
            text.push_str(id);
        }
        write(&marker, &text)?;
    }
    // `sync-carries-unrelated-edits`: should putting the set-aside edits back conflict, git
    // still exits 0 and leaves them on the stash list as a new `autostash` entry.
    let stash = stash_tip(root)?;
    loop {
        let mut args: Vec<&str> = config.iter().map(String::as_str).collect();
        if continuing {
            args.extend(["rebase", "--continue"]);
        } else {
            args.extend(["rebase", "--autostash", &onto]);
        }
        let editor = [("GIT_EDITOR", OsStr::new("true"))];
        let options = Options {
            env: &editor,
            ..Options::default()
        };
        let stderr = match run(root, &args, options) {
            Ok(_) => break,
            Err(clm_git::Error::Failed { stderr, .. }) => stderr,
            Err(e) => return Err(e.into()),
        };
        let conflicts = names(root, &["diff", "--name-only", "-z", "--diff-filter=U"])?;
        if conflicts.is_empty() {
            // The `-c` overrides would bury what git said, such as the path it won't
            // overwrite.
            return Err(Error::Git(format!("git rebase: {stderr}")));
        }
        let (specs, others): (Vec<&str>, Vec<&str>) = conflicts
            .iter()
            .map(String::as_str)
            .partition(|p| keep && p.starts_with("specs/"));
        // Conflicts the driver can't take, such as modify/delete, go the same way: keep
        // the side being rebased onto, and the regenerate step rebuilds `specs/`. Settled
        // before the rest are reported, so the caller is never left one under `specs/`.
        for path in specs {
            if git(root, &["ls-files", "--stage", "--", path])?
                .lines()
                .any(|l| l.split_whitespace().nth(2) == Some("2"))
            {
                git(root, &["checkout", "--ours", "--", path])?;
                git(root, &["add", "--", path])?;
            } else {
                git(root, &["rm", "-q", "--", path])?;
            }
        }
        if !others.is_empty() {
            return Ok(Report::left(
                others.iter().map(|p| format!("conflict {p}")).collect(),
            ));
        }
        continuing = true;
    }
    fs::remove_file(&marker).map_err(|e| io(&marker, e))?;
    if let Some(tip) = stash_tip(root)?.filter(|t| Some(t) != stash.as_ref())
        && git(root, &["log", "-g", "-1", "--format=%gs", "refs/stash"])? == "autostash"
    {
        return Err(Error::Git(format!(
            "git rebase: Applying autostash resulted in conflicts. Your changes are safe in \
             the stash as {tip}; recover them with `git stash apply {tip}`"
        )));
    }

    conclude(root, settings, &onto, base.as_deref(), &base_file, &applied)
}

/// What follows the rebase, or the regenerated rename commit, on `HEAD`: the regenerate
/// step for the changes in `applied`, the spec-changed report against `base`, archiving,
/// proposal items, and the judgement of `HEAD` (`judged-by-check`).
fn conclude(
    root: &Path,
    settings: impl FnOnce(&Path) -> Result<clm_health::Settings, String>,
    onto: &str,
    base: Option<&str>,
    base_file: &Path,
    applied: &[String],
) -> Result<Report, Error> {
    // The `merged` lines come first, whatever the outcome.
    let (mut lines, stale) = regenerate(root, onto, applied)?;
    if !stale.is_empty() {
        // The base file stays: the next run, once the delta is fixed, still owes the
        // spec-changed report for what this rebase brought in.
        lines.extend(stale);
        return Ok(Report::left(lines));
    }
    let branch = branch(root);
    let changed = spec_changed(root, branch.as_deref(), base, onto)?;
    let moved = !changed.is_empty();
    lines.extend(changed);
    let (done, left) = archive_complete(root, branch.as_deref())?;
    lines.extend(done);
    let mut warnings = Vec::new();
    // `file-proposal-items`: after archiving, judged by the graph the archive left, which
    // reads the working tree; a graph with violations files nothing, as it archives nothing.
    if branch.as_deref().is_some_and(|b| b.starts_with("impl/")) {
        let (graph, violations) = clm_model::load(root);
        if violations.is_empty() {
            let (filed, skipped) = items::file_proposals(root, &graph)?;
            lines.extend(filed);
            warnings.extend(skipped);
        }
    }
    // `judged-by-check` (`sync-judges-head`): `HEAD`'s commit, never the working tree, in a
    // temporary worktree removed on every exit path.
    let scratch = Scratch::new(root)?;
    let left: Vec<String> = left.iter().map(clm_check::format).collect();
    let judged = judge(root, &scratch, branch.as_deref(), settings, &left);
    let warning = scratch.remove();
    let (judged, passed) = match (judged, warning.as_deref()) {
        (Ok(judged), _) => judged,
        (Err(Error::Git(e)), Some(w)) => return Err(Error::Git(format!("{e}; {w}"))),
        (Err(Error::Refused(e)), Some(w)) => return Err(Error::Refused(format!("{e}; {w}"))),
        (Err(e), None) => return Err(e),
    };
    // Only now is the report delivered for good; a run that failed above still owes it.
    if base_file.exists() {
        fs::remove_file(base_file).map_err(|e| io(base_file, e))?;
    }
    lines.extend(judged);
    warnings.extend(warning);
    Ok(Report {
        lines,
        ok: !moved && passed && left.is_empty(),
        warnings,
    })
}

/// `judged-by-check`, on `HEAD` checked out in `scratch`: the settings `main` brought in,
/// loaded on every branch, so a `clm` that `clm check` and `clm land` would fail on them
/// never says `sync ok`; the reconsider items, scored there and filed in `root`; and every
/// `clm check` rule, its violations after `left`, what stopped an archive. Returns the
/// lines, each naming files by their path under `root`, and whether the settings loaded
/// and the check passed.
fn judge(
    root: &Path,
    scratch: &Scratch,
    branch: Option<&str>,
    load: impl FnOnce(&Path) -> Result<clm_health::Settings, String>,
    left: &[String],
) -> Result<(Vec<String>, bool), Error> {
    let mut lines = Vec::new();
    let settings = load(scratch.clm_root()).map_err(|e| scratch.unscratched(&e, root));
    if let (Ok(health), Some(node)) = (&settings, node_of(branch)) {
        let filed = items::file_reconsider(root, scratch.clm_root(), node, health)?;
        if filed.iter().any(|l| l.starts_with("filed ")) {
            scratch.checkout(&git(root, &["rev-parse", "HEAD"])?)?;
        }
        lines.extend(filed.iter().map(|l| scratch.unscratched(l, root)));
    }
    lines.extend(settings.as_ref().err().cloned());
    lines.extend(left.iter().cloned());
    let check = clm_check::run(scratch.clm_root());
    lines.extend(check.iter().map(|v| {
        clm_check::format(&clm_model::Violation {
            path: scratch.unscratched_path(&v.path, root),
            ..v.clone()
        })
    }));
    Ok((lines, settings.is_ok() && check.is_empty()))
}

/// The commit `refs/stash` points at, if any.
fn stash_tip(root: &Path) -> Result<Option<String>, Error> {
    succeeded(root, &["rev-parse", "-q", "--verify", "refs/stash"])
}

/// `sync-ignores-untracked`, `sync-carries-unrelated-edits`: the uncommitted work that stops
/// a sync of `branch` onto `onto`, sorted, as paths from the top of the repository. It is
/// each staged change, which the commits sync makes would sweep in; each tracked file
/// modified or deleted under `specs/`, `changes/` or `backlog/`, which sync writes; each one
/// `onto` changed since `base` (every one without a base), which the rebase writes; on
/// `impl/*` every one, since the later steps read the working tree as `HEAD`; and each
/// untracked file not ignored under `specs/` or `changes/`, which a `Re-apply` or `Archive`
/// commit would sweep in. A rename counts under both its paths. Anything else the rebase
/// sets aside and puts back.
fn uncommitted(
    root: &Path,
    branch: &str,
    onto: &str,
    base: Option<&str>,
) -> Result<Vec<String>, Error> {
    let status = names(
        root,
        &[
            "status",
            "--porcelain",
            "-z",
            "--no-renames",
            "--untracked-files=no",
        ],
    )?;
    let main_changed: Option<BTreeSet<String>> = match base {
        Some(base) => Some(
            names(
                root,
                &["diff", "--name-only", "-z", "--no-renames", base, onto],
            )?
            .into_iter()
            .collect(),
        ),
        None => None,
    };
    let strict = branch.starts_with("impl/");
    // Each entry is `XY <path>`: `X` the index against `HEAD`, `Y` the working tree against
    // the index.
    let mut paths = BTreeSet::new();
    for entry in &status {
        let (Some(xy), Some(path)) = (entry.get(..2), entry.get(3..)) else {
            continue;
        };
        let (staged, modified) = (!xy.starts_with(' '), !xy.ends_with(' '));
        let stops = staged
            || modified
                && (strict
                    || ["specs/", "changes/", "backlog/"]
                        .iter()
                        .any(|d| path.starts_with(d))
                    || main_changed.as_ref().is_none_or(|m| m.contains(path)));
        if stops {
            paths.insert(path.to_string());
        }
    }
    let untracked = git(
        root,
        &[
            "ls-files",
            "--others",
            "--exclude-standard",
            "--full-name",
            "-z",
            "--",
            "specs",
            "changes",
        ],
    )?;
    paths.extend(
        untracked
            .split('\0')
            .filter(|p| !p.is_empty())
            .map(str::to_string),
    );
    Ok(paths.into_iter().collect())
}

/// The branch checked out, if any.
fn branch(root: &Path) -> Option<String> {
    clm_git::current_branch(root).ok().flatten()
}

/// The node an `impl/<node>` branch builds; a node ID never contains `/`.
fn node_of(branch: Option<&str>) -> Option<&str> {
    branch?
        .strip_prefix("impl/")
        .filter(|node| !node.is_empty() && !node.contains('/'))
}

/// A node's spec at a commit: its bundle hash, its requirements' versions by ID, the paths
/// its bundle is read from whole (`ARCHITECTURE.md`, the bundle nodes' `SPEC.md`s, its own
/// assets), and the capability blocks the bundle holds, the part of `CAPABILITIES.md` it
/// reads: those it serves, or every one for the node that builds the capability tests.
/// `None` for the bundle when the node isn't there.
struct SpecAt {
    bundle: Option<String>,
    versions: BTreeMap<String, String>,
    paths: BTreeSet<String>,
    capability_blocks: Vec<String>,
}

fn spec_at(root: &Path, rev: &str, node: &str) -> Result<SpecAt, Error> {
    let tmp = extract_present(root, rev, &["specs", "ARCHITECTURE.md", "CAPABILITIES.md"])?;
    let (graph, _) = clm_model::load(tmp.path());
    let bundle = clm_context::bundle(&graph, tmp.path(), node).map(|b| clm_context::hash(&b));
    let mut paths: BTreeSet<String> = ["ARCHITECTURE.md".to_string()].into();
    let capability_blocks = graph.nodes.get(node).map_or_else(Vec::new, |n| {
        clm_context::served_capabilities(&graph, n)
            .into_iter()
            .map(|c| c.text.clone())
            .collect()
    });
    for id in clm_context::bundle_nodes(&graph, node) {
        let spec = &graph.nodes[&id].path;
        paths.insert(spec.to_string_lossy().into_owned());
        if id == node
            && let Some(dir) = spec.parent()
        {
            paths.insert(dir.join("assets").to_string_lossy().into_owned());
        }
    }
    let versions = graph.nodes.get(node).map_or_else(BTreeMap::new, |n| {
        n.requirements
            .iter()
            .map(|r| (r.id.clone(), r.version().to_string()))
            .collect()
    });
    Ok(SpecAt {
        bundle,
        versions,
        paths,
        capability_blocks,
    })
}

/// `spec-changed`: on a branch `impl/<node>`, when the node's bundle differs between `base`
/// (the merge base before the rebase) and `onto`, says so: `spec changed <node>`, each
/// `changed <path>` of the bundle that differs, each `requirement <id> <old> -> <new>` of the
/// node's own requirements whose version changed, appeared or went, and what to read.
/// Nothing on other branches, or when nothing in the bundle moved.
fn spec_changed(
    root: &Path,
    branch: Option<&str>,
    base: Option<&str>,
    onto: &str,
) -> Result<Vec<String>, Error> {
    let (Some(node), Some(base)) = (node_of(branch), base) else {
        return Ok(Vec::new());
    };
    // Nothing a bundle is made of moved: no trees to extract.
    let touched = git(
        root,
        &[
            "diff",
            "--name-only",
            base,
            onto,
            "--",
            "specs",
            "ARCHITECTURE.md",
            "CAPABILITIES.md",
        ],
    )?;
    if touched.is_empty() {
        return Ok(Vec::new());
    }
    let (before, after) = (spec_at(root, base, node)?, spec_at(root, onto, node)?);
    if before.bundle == after.bundle {
        return Ok(Vec::new());
    }
    let mut out = vec![format!("spec changed {node}")];
    let mut paths: Vec<&str> = before
        .paths
        .union(&after.paths)
        .map(String::as_str)
        .collect();
    // Only an edit to a capability block the bundle holds moves it: name the file for that
    // alone.
    if before.capability_blocks != after.capability_blocks {
        paths.push("CAPABILITIES.md");
    }
    let mut args = vec!["diff", "--name-only", base, onto, "--"];
    args.extend(&paths);
    out.extend(git(root, &args)?.lines().map(|p| format!("changed {p}")));
    let ids: BTreeSet<&String> = before
        .versions
        .keys()
        .chain(after.versions.keys())
        .collect();
    for id in ids {
        let (old, new) = (before.versions.get(id), after.versions.get(id));
        if old != new {
            out.push(format!(
                "requirement {id} {} -> {}",
                old.map_or("added", String::as_str),
                new.map_or("removed", String::as_str)
            ));
        }
    }
    out.push(format!(
        "read: run `clm pending {node}` and `git diff {base} {} -- {}`, then adapt your \
         work to the new text",
        main_ref(),
        paths.join(" ")
    ));
    Ok(out)
}

/// `archive-complete` (`archive-on-sync`): on a branch `impl/*`, sets each `specd` node with
/// nothing pending at `HEAD` to `active`, and archives each complete change, deleting the
/// backlog items it resolves, one `Archive <id>` commit each (the activations ride the first,
/// or an `Activate <node>` commit of their own). Returns what it did (`activated <node>`,
/// `archived <id>`) and what stopped an archive.
fn archive_complete(
    root: &Path,
    branch: Option<&str>,
) -> Result<(Vec<String>, Vec<clm_model::Violation>), Error> {
    let mut done = Vec::new();
    if !branch.is_some_and(|b| b.starts_with("impl/")) {
        return Ok((done, Vec::new()));
    }
    let (graph, violations) = clm_model::load(root);
    if !violations.is_empty() {
        // `clm check` reports them; a graph that doesn't load says nothing reliable.
        return Ok((done, Vec::new()));
    }
    // What is at `HEAD`: no tracked file is modified, but untracked files may sit anywhere
    // outside `specs/` and `changes/`, and they aren't on the branch.
    let tracked: BTreeSet<PathBuf> = git(root, &["ls-files", "-z"])?
        .split('\0')
        .filter(|p| !p.is_empty())
        .map(PathBuf::from)
        .collect();
    let mut markers = clm_check::scan(root).req;
    markers.retain(|m| tracked.contains(&m.path));
    let pending = clm_check::pending(&graph, &markers);
    let activated: Vec<&clm_model::Node> = graph
        .nodes
        .values()
        .filter(|n| n.frontmatter.status == clm_model::Status::Specd)
        .filter(|n| !pending.iter().any(|p| p.node == n.id))
        .collect();
    let complete = clm_check::complete_changes(&graph, &markers);
    // The one refusal `archive` can give once a status is `done`, tested before anything is
    // written, so a refused archive leaves the tree as it was.
    for id in &complete {
        let taken = Path::new("changes").join(clm_model::ARCHIVE_DIR).join(id);
        if root.join(&taken).exists() {
            return Ok((
                done,
                vec![clm_model::Violation {
                    path: taken.clone(),
                    rule: "sync/archive-complete",
                    message: format!("`{}` already exists", taken.display()),
                }],
            ));
        }
    }
    for node in &activated {
        let path = root.join(&node.path);
        write(&path, &with_status(&node.source, "active"))?;
        git(root, &["add", "--", &node.path.to_string_lossy()])?;
        done.push(format!("activated {}", node.id));
        if complete.is_empty() {
            git(
                root,
                &["commit", "-q", "-m", &format!("Activate {}", node.id)],
            )?;
        }
    }
    for id in &complete {
        let change = &graph.changes[id];
        let md = root.join(&change.path);
        let text = fs::read_to_string(&md).map_err(|e| io(&md, e))?;
        write(&md, &with_status(&text, "done"))?;
        if let Err(v) = clm_apply::archive(root, id) {
            let why: Vec<String> = v.iter().map(clm_check::format).collect();
            return Err(Error::Git(format!("archiving {id}: {}", why.join("; "))));
        }
        // Each pathspec names something git tracks: `backlog` itself may not exist.
        let mut staged = vec!["add", "-A", "--", "changes"];
        let items: Vec<String> = change
            .frontmatter
            .resolves
            .iter()
            .filter_map(|slug| graph.backlog.get(slug))
            .filter(|item| tracked.contains(&item.path))
            .map(|item| item.path.to_string_lossy().into_owned())
            .collect();
        for item in &items {
            let path = root.join(item);
            fs::remove_file(&path).map_err(|e| io(&path, e))?;
            staged.push(item);
        }
        git(root, &staged)?;
        git(root, &["commit", "-q", "-m", &format!("Archive {id}")])?;
        done.push(format!("archived {id}"));
    }
    Ok((done, Vec::new()))
}

/// `source` with the `status:` line of its frontmatter set to `status`; every other byte,
/// line endings included, stays as it is.
fn with_status(source: &str, status: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut in_frontmatter = false;
    let mut done = false;
    for (i, line) in source.split_inclusive('\n').enumerate() {
        let text = line.trim_end_matches(['\n', '\r']);
        let ending = &line[text.len()..];
        if text == "---" {
            in_frontmatter = i == 0;
        } else if in_frontmatter && !done && text.starts_with("status:") {
            done = true;
            out.push_str(&format!("status: {status}{ending}"));
            continue;
        }
        out.push_str(line);
    }
    out
}

/// The merge base of `a` and `b`; `None` when they share no history (`git merge-base` exits
/// 1), an error when git fails otherwise.
fn merge_base(root: &Path, a: &str, b: &str) -> Result<Option<String>, Error> {
    match git(root, &["merge-base", a, b]) {
        Ok(base) => Ok(Some(base)),
        Err(clm_git::Error::Failed { status, stderr, .. })
            if status.code() == Some(1) && stderr.is_empty() =>
        {
            Ok(None)
        }
        Err(e) => Err(e.into()),
    }
}

/// `git <args>`'s trimmed stdout, or `None` when git ran and failed, as a query such as
/// `cat-file -e` answers no.
fn succeeded(root: &Path, args: &[&str]) -> Result<Option<String>, Error> {
    match git(root, args) {
        Ok(out) => Ok(Some(out)),
        Err(clm_git::Error::Failed { .. }) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// The paths a `git <args>` that holds `-z` prints, untrimmed.
fn names(root: &Path, args: &[&str]) -> Result<Vec<String>, Error> {
    let out = run(root, args, Options::default())?;
    Ok(String::from_utf8_lossy(&out)
        .split('\0')
        .filter(|p| !p.is_empty())
        .map(str::to_string)
        .collect())
}

/// The changes the branch adds, live and archived together, sorted by ID, each with whether
/// step 4 regenerates it. A live change is added when its `changes/<id>/change.md` is at
/// `HEAD` and at neither `onto` nor `base` (the merge base of `HEAD` and `onto`, when there
/// is one), and regenerated when its status is `applied`. A *new archived change*
/// (`sync-new-archived-change`) is added when its `changes/archive/<id>/change.md` is at
/// `HEAD` and at neither of them, and `changes/<id>/change.md` is at neither either, so a
/// change `main` applied and the branch archives is never re-applied; it is regenerated when
/// its status is `done`. A `change.md` whose frontmatter doesn't parse is added, not
/// regenerated. An archived copy of a change `HEAD` also holds live is left to the live one.
/// A change `onto` holds `proposed` or `approved` that `HEAD` has `applied`, or archived
/// `done`, is regenerated too (`regenerate-applied-live-changes`).
fn added_changes(
    root: &Path,
    onto: &str,
    base: Option<&str>,
) -> Result<Vec<(String, bool)>, Error> {
    // One listing per revision, so the archive's size costs no process per change.
    let head = change_files(root, "HEAD")?;
    let onto_files = change_files(root, onto)?;
    let mut known = onto_files.clone();
    if let Some(base) = base {
        known.extend(change_files(root, base)?);
    }
    let archive = format!("{}/", clm_model::ARCHIVE_DIR);
    let mut ids = BTreeMap::new();
    for file in &head {
        let Some(rel) = file
            .strip_prefix("changes/")
            .and_then(|f| f.strip_suffix("/change.md"))
        else {
            continue;
        };
        let (id, archived) = match rel.strip_prefix(&archive) {
            Some(id) => (id, true),
            None => (rel, false),
        };
        if id.contains('/') || known.contains(file) {
            continue;
        }
        let live = format!("changes/{id}/change.md");
        if archived && (known.contains(&live) || head.contains(&live)) {
            continue;
        }
        let regenerated = if archived {
            clm_model::ChangeStatus::Done
        } else {
            clm_model::ChangeStatus::Applied
        };
        let text = git(root, &["show", &format!("HEAD:{file}")])?;
        let regenerate =
            clm_model::parse_change_frontmatter(&text).is_some_and(|fm| fm.status == regenerated);
        ids.insert(id.to_string(), regenerate);
    }
    // A change `onto` holds unapplied, which `HEAD` applies (`regenerate-applied-live-changes`):
    // `onto`'s `specs/` lacks its deltas, so they are regenerated as an added change's are.
    let status_at = |rev: &str, file: &str| -> Result<Option<clm_model::ChangeStatus>, Error> {
        let text = git(root, &["show", &format!("{rev}:{file}")])?;
        Ok(clm_model::parse_change_frontmatter(&text).map(|fm| fm.status))
    };
    for file in &onto_files {
        let Some(id) = file
            .strip_prefix("changes/")
            .and_then(|f| f.strip_suffix("/change.md"))
            .filter(|id| !id.contains('/'))
        else {
            continue;
        };
        let archived = format!("changes/{archive}{id}/change.md");
        // Only a change `HEAD` holds can be applied there; skip the rest without a `git show`.
        if !head.contains(file) && !head.contains(&archived) {
            continue;
        }
        if !status_at(onto, file)?.is_some_and(clm_model::ChangeStatus::is_pending) {
            continue;
        }
        let applies = if head.contains(file) {
            status_at("HEAD", file)? == Some(clm_model::ChangeStatus::Applied)
        } else {
            head.contains(&archived)
                && status_at("HEAD", &archived)? == Some(clm_model::ChangeStatus::Done)
        };
        if applies {
            ids.insert(id.to_string(), true);
        }
    }
    Ok(ids.into_iter().collect())
}

/// Every `change.md` under `changes/` in `rev`'s tree, live and archived, as paths from the
/// top of the repository.
fn change_files(root: &Path, rev: &str) -> Result<BTreeSet<String>, Error> {
    let files = git(
        root,
        &["ls-tree", "-r", "--name-only", rev, "--", "changes"],
    )?;
    Ok(files
        .lines()
        .filter(|f| f.ends_with("/change.md"))
        .map(str::to_string)
        .collect())
}

/// `regenerate-applied-specs`: sets `specs/` to `onto`'s with each change in `ids`
/// re-applied (`regenerated`), committing the result when it differs from `HEAD`. A change
/// that doesn't apply stops it before it writes anything to `specs/`. Then the deltas of the
/// other live `proposed` or `approved` changes the regenerated tree made stale are merged
/// too (`other-delta-merge`). Returns the `merged` lines, and what is left: the violations
/// and the stale-delta report.
fn regenerate(
    root: &Path,
    onto: &str,
    ids: &[String],
) -> Result<(Vec<String>, Vec<String>), Error> {
    if ids.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }
    let mut tree = match regenerated(root, onto, ids, true)? {
        Regenerated::Tree(tree) => tree,
        Regenerated::Stopped { merged, left } => return Ok((merged, left)),
    };
    let mut merged_lines = std::mem::take(&mut tree.merged);

    // Mirror the regenerated `specs/` into the working tree.
    let fresh = files_under(tree.path(), Path::new("specs"))?;
    for tracked in names(root, &["ls-files", "-z", "--", "specs"])? {
        if !fresh.contains(Path::new(&tracked)) {
            let path = root.join(&tracked);
            fs::remove_file(&path).map_err(|e| io(&path, e))?;
            // Drop directories it leaves empty, so a removed node leaves no directory.
            for dir in path.ancestors().skip(1) {
                if dir == root.join("specs") || fs::remove_dir(dir).is_err() {
                    break;
                }
            }
        }
    }
    for rel in &fresh {
        let from = tree.path().join(rel);
        let text = fs::read(&from).map_err(|e| io(&from, e))?;
        let to = root.join(rel);
        if let Some(dir) = to.parent() {
            fs::create_dir_all(dir).map_err(|e| io(dir, e))?;
        }
        fs::write(&to, text).map_err(|e| io(&to, e))?;
    }
    if !git(root, &["status", "--porcelain", "--", "specs"])?.is_empty() {
        git(root, &["add", "-A", "--", "specs"])?;
        git(
            root,
            &[
                "commit",
                "-q",
                "-m",
                &format!("Re-apply {}", ids.join(", ")),
            ],
        )?;
    }
    // `other-delta-merge`: the regenerated tree is the last stage.
    let (lines, left) = tree.other_deltas(root, ids, onto)?;
    merged_lines.extend(lines);
    Ok((merged_lines, left))
}

/// `autosync-policy`: on a `spec/*` branch, pushes `HEAD` with the working tree's spec paths
/// (untracked files included) to `origin` as `autosync/<branch>`, leaving the working tree,
/// the index and the branch untouched. Does nothing on other branches or when `origin`
/// already has the same tree.
pub fn autosync(root: &Path) -> Result<(), Error> {
    let Some(branch) = branch(root) else {
        return Ok(());
    };
    if !branch.starts_with("spec/") {
        return Ok(());
    }
    let tmp = tempfile::TempDir::new().map_err(|e| Error::Git(format!("temp dir: {e}")))?;
    let index = tmp.path().join("index");
    let env = [("GIT_INDEX_FILE", index.as_os_str())];
    let indexed = |args: &[&str]| {
        let options = Options {
            env: &env,
            ..Options::default()
        };
        run(root, args, options)
    };
    indexed(&["read-tree", "HEAD"])?;
    // A path is named only if it exists, or is tracked so its deletion is recorded.
    let mut paths = Vec::new();
    for p in SPEC_PATHS {
        if root.join(p).exists() || exists_at(root, "HEAD", p)? {
            paths.push(p);
        }
    }
    if !paths.is_empty() {
        let mut args = vec!["add", "-A", "--"];
        args.extend(&paths);
        indexed(&args)?;
    }
    let tree = String::from_utf8_lossy(&indexed(&["write-tree"])?)
        .trim_end()
        .to_string();
    let head = git(root, &["rev-parse", "HEAD"])?;
    let snapshot = if tree == git(root, &["rev-parse", "HEAD^{tree}"])? {
        head
    } else {
        git(
            root,
            &[
                // Plumbing: unlike `git commit`, it ignores `commit.gpgsign`, so a
                // snapshot never asks for a signing passphrase.
                "commit-tree",
                &tree,
                "-p",
                &head,
                "-m",
                &format!("autosync {branch}"),
            ],
        )?
    };
    for known in [
        format!("{REMOTE}/{branch}"),
        format!("{REMOTE}/autosync/{branch}"),
    ] {
        let verify = ["rev-parse", "--verify", "-q", &format!("{known}^{{tree}}")];
        if succeeded(root, &verify)?.is_some_and(|t| t == tree) {
            return Ok(());
        }
    }
    // Nothing may prompt or wait long: no credential prompts, and ssh fails fast rather
    // than asking for a passphrase or host key, unless the user set their own ssh command.
    let mut env = vec![("GIT_TERMINAL_PROMPT", OsStr::new("0"))];
    if std::env::var_os("GIT_SSH_COMMAND").is_none() {
        env.push((
            "GIT_SSH_COMMAND",
            OsStr::new("ssh -o BatchMode=yes -o ConnectTimeout=10"),
        ));
    }
    let push = [
        "push",
        "--quiet",
        REMOTE,
        &format!("+{snapshot}:refs/heads/autosync/{branch}"),
    ];
    let options = Options {
        env: &env,
        ..Options::default()
    };
    run(root, &push, options)?;
    git(
        root,
        &[
            "update-ref",
            &format!("refs/remotes/{REMOTE}/autosync/{branch}"),
            &snapshot,
        ],
    )?;
    Ok(())
}

/// `git rev-parse --git-path <path>`, which is right in worktrees too.
fn git_path(root: &Path, path: &str) -> Result<PathBuf, Error> {
    Ok(root.join(git(root, &["rev-parse", "--git-path", path])?))
}

/// The files under `base/dir`, as paths relative to `base`.
fn files_under(base: &Path, dir: &Path) -> Result<BTreeSet<PathBuf>, Error> {
    let mut files = BTreeSet::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(rel) = stack.pop() {
        let full = base.join(&rel);
        if !full.exists() {
            continue;
        }
        for entry in fs::read_dir(&full).map_err(|e| io(&full, e))? {
            let entry = entry.map_err(|e| io(&full, e))?;
            let child = rel.join(entry.file_name());
            if entry.file_type().map_err(|e| io(&full, e))?.is_dir() {
                stack.push(child);
            } else {
                files.insert(child);
            }
        }
    }
    Ok(files)
}

fn write(path: &Path, text: &str) -> Result<(), Error> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| io(dir, e))?;
    }
    fs::write(path, text).map_err(|e| io(path, e))
}
