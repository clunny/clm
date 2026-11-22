//! Keeps branches converged with `main`: `clm rebase` and `clm autosync` (see
//! specs/sync/SPEC.md).

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use clm_git::{Options, REMOTE};
use clm_regenerate::{Regenerated, exists_at, extract_present, git, io, regenerated, run};
use root::{Root, object};
use scratch::Scratch;

pub use command::{AutosyncArgs, RebaseArgs, autosync_command, rebase_command};

mod command;
mod items;
mod renames;
mod root;
mod scratch;

/// `main` on the remote, as fetched: what every sync rebases onto.
fn main_ref() -> String {
    format!("{REMOTE}/main")
}
/// The paths autosync snapshots (`autosync-policy`), as `clm gate` counts spec paths.
const SPEC_PATHS: [&str; 8] = [
    "specs",
    "changes",
    "backlog",
    "docs",
    "ARCHITECTURE.md",
    "CAPABILITIES.md",
    "CLAUDE.md",
    "AGENTS.md",
];

/// A refusal (`Refused`, exit 1) or a git failure (`Git`, exit 2): `clm-git`'s shared
/// `Failure` (`shared-plumbing-in-git`).
pub use clm_git::Failure as Error;

/// What a `clm rebase` run found and did.
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
/// uncommitted edit; an error makes the run incomplete. `env` is the environment, where the
/// temporary worktree's `TMPDIR` is read.
pub fn sync(
    root: &Path,
    env: &dyn Fn(&str) -> Option<String>,
    settings: impl FnOnce(&Path) -> Result<clm_health::Settings, String>,
) -> Result<Report, Error> {
    let place = Root::of(root)?;
    let marker = git_path(root, "clm/sync-rebase")?;
    let merging = git_path(root, "rebase-merge")?;
    let applying = git_path(root, "rebase-apply")?;
    // `rebase`: a rebase it started is one whose `rebase-merge` directory holds the file it
    // writes there when its rebase stops; git removes the directory, and the file with it,
    // however that rebase is finished or given up, so a later rebase by hand never holds it.
    // A run killed before it could write the file, or a marker an older `clm` wrote, is
    // read from the marker (`claims`).
    let kept = fs::read_to_string(&marker).ok();
    let ours = merging.join(OURS).exists()
        || merging.is_dir() && kept.as_deref().is_some_and(|t| claims(t, &merging));
    if !ours {
        if applying.join("applying").exists() {
            return Err(Error::Refused(
                "a `git am` is in progress; finish it with `git am --continue` or give it up \
                 with `git am --abort`"
                    .into(),
            ));
        }
        if merging.exists() || applying.exists() {
            return Err(Error::Refused(
                "a rebase `clm rebase` didn't start is in progress; finish it with `git rebase \
                 --continue` or give it up with `git rebase --abort`"
                    .into(),
            ));
        }
    }
    // The commit it rebases onto, whether the branch is a build's and whose
    // (`build-branches`), and the changes this branch adds (live, or archived on the branch),
    // with which of them step 4 regenerates, decided before the rebase starts and kept in the
    // marker: mid-rebase `HEAD` is a partial branch, and a resume must use the same driver,
    // regenerate against the same `main` the rebase used, not a newer one, and read the branch
    // as it was. A change counts as added only if it is also absent at the fresh merge base,
    // so one that `main` archived or removed since the branch was cut isn't re-applied. The
    // marker holds `onto`, the `HEAD` it rebases (`=head <commit>`), the build line, then a
    // line per added change: its ID when it is regenerated, `added <id>` when it isn't.
    let (onto, build, added, fresh) = if ours {
        let text = kept.unwrap_or_default();
        let mut lines = text.lines();
        let onto = lines.next().unwrap_or_default().to_string();
        let mut build = None;
        let mut added = Vec::new();
        for line in lines {
            if line.starts_with('=') {
                build = build.or_else(|| Build::parse(line));
            } else {
                added.push(match line.strip_prefix("added ") {
                    Some(id) => (id.to_string(), false),
                    None => (line.to_string(), true),
                });
            }
        }
        let build = match build {
            Some(build) => build,
            // A marker an older `clm` wrote: asked again, of the branch the rebase replays.
            None => {
                let read = |f: &str| fs::read_to_string(merging.join(f)).unwrap_or_default();
                let name = read("head-name");
                let name = name.trim();
                let name = name.strip_prefix("refs/heads/").unwrap_or(name);
                let tip = read("orig-head");
                let tip = tip.trim();
                let merged = !tip.is_empty() && merge_base(root, tip, &onto)?.is_some();
                Build::of(root, name, &onto, tip, merged)?
            }
        };
        (onto, build, added, None)
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
        let build = Build::of(root, &branch, &onto, "HEAD", fresh.is_some())?;
        let work = uncommitted(root, &place, build.yes, &onto, fresh.as_deref())?;
        if !work.is_empty() {
            return Ok(Report::left(
                work.iter().map(|p| format!("uncommitted {p}")).collect(),
            ));
        }
        if marker.exists() {
            // Left by a rebase it started that has since finished or been given up.
            fs::remove_file(&marker).map_err(|e| io(&marker, e))?;
        }
        let added = added_changes(root, &onto, fresh.as_deref())?;
        (onto, build, added, fresh)
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
        match renames::regenerate(root, &place, &onto, merge_base, &added)? {
            renames::Outcome::Other => {}
            renames::Outcome::Left(lines) => return Ok(Report::left(lines)),
            renames::Outcome::Done => {
                return conclude(
                    root,
                    &place,
                    env,
                    settings,
                    &onto,
                    &build,
                    base.as_deref(),
                    &base_file,
                    &[],
                    true,
                );
            }
        }
    }

    // `sync-owns-merge-drivers`: on a branch that adds a change, whatever its status,
    // `specs/` keeps the side being rebased onto (a driver that leaves `%A`): a file nobody
    // wrote by hand, which step 4 rebuilds when the change is applied. git matches a pattern
    // holding a slash from the repository's top, so it names the root's `specs/`.
    let keep = !added.is_empty();
    let attributes = git_path(root, "clm/attributes")?;
    let rules = if !keep {
        String::new()
    } else {
        format!("{} merge=clm-keep\n", place.pattern("specs/**"))
    };
    write(&attributes, &rules)?;
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
        let head = git(root, &["rev-parse", "HEAD"])?;
        let mut text = format!("{onto}\n=head {head}\n{}", build.line());
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
        // Stopped: the rebase in progress is this one's until git ends it.
        if merging.is_dir() {
            write(&merging.join(OURS), "")?;
        }
        // Named from the root, so a conflict outside it reads `../<path>` and is never one of
        // the root's `specs/`; a `specs/` one is handed back to git root-relative.
        let conflicts: Vec<String> = names(
            root,
            &[
                "diff",
                "--no-relative",
                "--name-only",
                "-z",
                "--diff-filter=U",
            ],
        )?
        .iter()
        .map(|p| place.name(p))
        .collect();
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

    conclude(
        root,
        &place,
        env,
        settings,
        &onto,
        &build,
        base.as_deref(),
        &base_file,
        &applied,
        false,
    )
}

/// The file `clm rebase` writes into git's `rebase-merge` directory when its rebase stops.
const OURS: &str = "clm-rebase";

/// Whether the marker `text` claims the rebase in `merging`, which lacks [`OURS`]: one it
/// began and was stopped before it could write that file, as by a kill, replays the `HEAD`
/// the marker records (`=head <commit>`, git's `orig-head`); a rebase begun by hand after
/// its own finished replays the rebased branch, another commit. A marker an older `clm`
/// wrote records no `=` line and claims it, as that `clm` read it.
fn claims(text: &str, merging: &Path) -> bool {
    let mut equals = text.lines().filter(|l| l.starts_with('='));
    let Some(head) = text.lines().find_map(|l| l.strip_prefix("=head ")) else {
        return equals.next().is_none();
    };
    fs::read_to_string(merging.join("orig-head")).is_ok_and(|o| o.trim() == head)
}

/// `build-branches`: whether the branch is a build's, and its component, asked of the gate
/// once per run.
struct Build {
    yes: bool,
    component: Option<String>,
}

impl Build {
    /// The gate's answer for `name` at `head` against `onto` (`gate/evaluate-at`): a build's
    /// when its kind is `code`, `light` or `mixed`, or it is named `impl/*` and needs no
    /// review; with no merge base (`merged` false) by its name alone. Its component is
    /// `<component>` of `impl/<component>`, else the one the gate reads from its files.
    fn of(root: &Path, name: &str, onto: &str, head: &str, merged: bool) -> Result<Build, Error> {
        let named = name.starts_with("impl/");
        let by_name = name
            .strip_prefix("impl/")
            .filter(|c| !c.is_empty() && !c.contains('/'))
            .map(str::to_string);
        if !merged {
            return Ok(Build {
                yes: named,
                component: by_name,
            });
        }
        let outcome = clm_gate::evaluate_at(root, onto, head, false).map_err(Error::Git)?;
        let yes = matches!(
            outcome.kind,
            clm_gate::Kind::Code | clm_gate::Kind::Light | clm_gate::Kind::Mixed
        ) || named && !outcome.review;
        Ok(Build {
            yes,
            component: if yes {
                by_name.or(outcome.component)
            } else {
                None
            },
        })
    }

    /// The marker's line for it: `=build <component>`, `=build` or `=other`; `=` starts no
    /// change ID.
    fn line(&self) -> String {
        match (self.yes, &self.component) {
            (true, Some(c)) => format!("=build {c}"),
            (true, None) => "=build".into(),
            (false, _) => "=other".into(),
        }
    }

    fn parse(line: &str) -> Option<Build> {
        let yes = |component: Option<&str>| Build {
            yes: true,
            component: component.map(str::to_string),
        };
        match line {
            "=other" => Some(Build {
                yes: false,
                component: None,
            }),
            "=build" => Some(yes(None)),
            _ => line.strip_prefix("=build ").map(|c| yes(Some(c))),
        }
    }

    /// The component of a build's branch, if it has one.
    fn component(&self) -> Option<&str> {
        self.component.as_deref().filter(|_| self.yes)
    }
}

/// What follows the rebase, or the regenerated rename commit, on `HEAD`: the regenerate
/// step for the changes in `applied`, the spec-changed report against `base`, archiving,
/// proposal items, and the judgement of `HEAD` (`judged-by-check`).
#[allow(clippy::too_many_arguments)]
fn conclude(
    root: &Path,
    place: &Root,
    env: &dyn Fn(&str) -> Option<String>,
    settings: impl FnOnce(&Path) -> Result<clm_health::Settings, String>,
    onto: &str,
    build: &Build,
    base: Option<&str>,
    base_file: &Path,
    applied: &[String],
    renamed: bool,
) -> Result<Report, Error> {
    // The `merged` lines come first, whatever the outcome.
    let (mut lines, stale) = regenerate(root, onto, applied, renamed)?;
    if !stale.is_empty() {
        // The base file stays: the next run, once the delta is fixed, still owes the
        // spec-changed report for what this rebase brought in.
        lines.extend(stale);
        return Ok(Report::left(lines));
    }
    let changed = spec_changed(root, place, build.component(), base, onto)?;
    let moved = !changed.is_empty();
    lines.extend(changed);
    let (done, left) = archive_complete(root, build.yes)?;
    lines.extend(done);
    let mut warnings = Vec::new();
    // `file-proposal-items`: after archiving, judged by the graph the archive left, which
    // reads the working tree; a graph with violations files nothing, as it archives nothing.
    if build.yes {
        let (graph, violations) = clm_model::load(root);
        if violations.is_empty() {
            let (filed, skipped) = items::file_proposals(root, &graph)?;
            lines.extend(filed);
            warnings.extend(skipped);
        }
    }
    // `judged-by-check` (`sync-judges-head`): `HEAD`'s commit, never the working tree, in a
    // temporary worktree removed on every exit path.
    let scratch = Scratch::new(root, place.prefix(), env)?;
    let left: Vec<String> = left.iter().map(clm_check::format).collect();
    let judged = judge(root, &scratch, build.component(), settings, &left);
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
/// never says `rebase ok`; the reconsider items, scored there and filed in `root`; and every
/// `clm check` rule, its violations after `left`, what stopped an archive. Returns the
/// lines, each naming files by their path under `root`, and whether the settings loaded
/// and the check passed.
fn judge(
    root: &Path,
    scratch: &Scratch,
    component: Option<&str>,
    load: impl FnOnce(&Path) -> Result<clm_health::Settings, String>,
    left: &[String],
) -> Result<(Vec<String>, bool), Error> {
    let mut lines = Vec::new();
    let settings = load(scratch.clm_root()).map_err(|e| scratch.unscratched(&e, root));
    if let (Ok(health), Some(node)) = (&settings, component) {
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
/// a sync of `branch` onto `onto`, sorted, each named from the CLM root (`place`), so a
/// file outside it reads `../<path>` and is never under the root's trees. It is
/// each staged change, which the commits sync makes would sweep in; each tracked file
/// modified or deleted under `specs/`, `changes/` or `backlog/`, which sync writes; each one
/// `onto` changed since `base` (every one without a base), which the rebase writes; on a
/// build's branch (`strict`) every one, since the later steps read the working tree as
/// `HEAD`; and each
/// untracked file not ignored under `specs/` or `changes/`, which a `Re-apply` or `Archive`
/// commit would sweep in. A rename counts under both its paths. Anything else the rebase
/// sets aside and puts back.
fn uncommitted(
    root: &Path,
    place: &Root,
    strict: bool,
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
                &[
                    "diff",
                    "--no-relative",
                    "--name-only",
                    "-z",
                    "--no-renames",
                    base,
                    onto,
                ],
            )?
            .iter()
            .map(|p| place.name(p))
            .collect(),
        ),
        None => None,
    };
    // Each entry is `XY <path>`: `X` the index against `HEAD`, `Y` the working tree against
    // the index.
    let mut paths = BTreeSet::new();
    for entry in &status {
        let (Some(xy), Some(path)) = (entry.get(..2), entry.get(3..).map(|p| place.name(p))) else {
            continue;
        };
        let (staged, modified) = (!xy.starts_with(' '), !xy.ends_with(' '));
        let stops = staged
            || modified
                && (strict
                    || ["specs/", "changes/", "backlog/"]
                        .iter()
                        .any(|d| path.starts_with(d))
                    || main_changed.as_ref().is_none_or(|m| m.contains(&path)));
        if stops {
            paths.insert(path);
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
            .map(|p| place.name(p)),
    );
    Ok(paths.into_iter().collect())
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

fn spec_at(root: &Path, place: &Root, rev: &str, node: &str) -> Result<SpecAt, Error> {
    // `extract-through-layer`: the root's paths, named from the repository's top, and the
    // graph loaded from the root's place in the extraction.
    let paths = ["specs", "ARCHITECTURE.md", "CAPABILITIES.md"].map(|p| place.under(p));
    let tmp = extract_present(root, rev, &paths.each_ref().map(String::as_str))?;
    let at = tmp.path().join(place.prefix());
    let (graph, _) = clm_model::load(&at);
    let bundle = clm_context::bundle(&graph, &at, node).map(|b| clm_context::hash(&b));
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

/// `spec-changed`: for a build's branch's component, when its bundle differs between `base`
/// (the merge base before the rebase) and `onto`, says so: `spec changed <node>`, each
/// `changed <path>` of the bundle that differs, each `requirement <id> <old> -> <new>` of the
/// node's own requirements whose version changed, appeared or went, and what to read.
/// Nothing without a component, or when nothing in the bundle moved.
fn spec_changed(
    root: &Path,
    place: &Root,
    component: Option<&str>,
    base: Option<&str>,
    onto: &str,
) -> Result<Vec<String>, Error> {
    let (Some(node), Some(base)) = (component, base) else {
        return Ok(Vec::new());
    };
    // Nothing a bundle is made of moved: no trees to extract.
    let touched = git(
        root,
        &[
            "diff",
            "--no-relative",
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
    let (before, after) = (
        spec_at(root, place, base, node)?,
        spec_at(root, place, onto, node)?,
    );
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
    let mut args = vec![
        "diff",
        "--no-relative",
        "--name-only",
        "-z",
        base,
        onto,
        "--",
    ];
    args.extend(&paths);
    out.extend(
        names(root, &args)?
            .iter()
            .map(|p| format!("changed {}", place.name(p))),
    );
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

/// `archive-complete` (`archive-on-sync`): on a build's branch (`build`), sets each `unbuilt`
/// node with nothing pending at `HEAD` to `active`, and archives each complete change whose
/// place in the archive is free, deleting the tracked backlog items it resolves, one
/// `Archive <id>` commit each (the activations ride the first, or an `Activate <node>` commit
/// of their own). Returns what it did (`activated <node>`, `archived <id>`) and each change
/// whose place was taken.
fn archive_complete(
    root: &Path,
    build: bool,
) -> Result<(Vec<String>, Vec<clm_model::Violation>), Error> {
    let mut done = Vec::new();
    if !build {
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
    let mut markers = clm_coverage::scan(root).req;
    markers.retain(|m| tracked.contains(&m.path));
    let pending = clm_coverage::pending(&graph, &markers);
    let activated: Vec<&clm_model::Node> = graph
        .nodes
        .values()
        .filter(|n| n.frontmatter.status == clm_model::Status::Specd)
        .filter(|n| !pending.iter().any(|p| p.node == n.id))
        .collect();
    // The one refusal `archive` can give once a status is `done`, tested before anything is
    // written: a change whose place is taken is left where it is, and the rest go on.
    let (complete, taken): (Vec<String>, Vec<String>) =
        clm_coverage::complete_changes(&graph, &markers)
            .into_iter()
            .partition(|id| !root.join(archived(id)).exists());
    let left = taken
        .iter()
        .map(|id| {
            let taken = archived(id);
            clm_model::Violation {
                message: format!("`{}` already exists", taken.display()),
                path: taken,
                rule: "sync/archive-complete",
            }
        })
        .collect();
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
    Ok((done, left))
}

/// Where change `id` goes in the archive.
fn archived(id: &str) -> PathBuf {
    Path::new("changes").join(clm_model::ARCHIVE_DIR).join(id)
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
/// `done`, is regenerated too (`reapply`).
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
        let text = git(root, &["show", &object("HEAD", file)])?;
        let regenerate =
            clm_model::parse_change_frontmatter(&text).is_some_and(|fm| fm.status == regenerated);
        ids.insert(id.to_string(), regenerate);
    }
    // A change `onto` holds unapplied, which `HEAD` applies (`reapply`):
    // `onto`'s `specs/` lacks its deltas, so they are regenerated as an added change's are.
    let status_at = |rev: &str, file: &str| -> Result<Option<clm_model::ChangeStatus>, Error> {
        let text = git(root, &["show", &object(rev, file)])?;
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

/// Every `change.md` under the root's `changes/` in `rev`'s tree, live and archived, as paths
/// relative to the root, as `git ls-tree` lists them from the directory it runs in.
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
/// other live `proposed` or `approved` changes the regenerated tree made stale are merged,
/// and those a merge or a hand resolution set to a base only the branch's tree had taken
/// back (`sync.regenerate/stale-delta-merge`). With no change in `ids`, that is done against
/// `onto`'s own `specs/`, writing nothing to `specs/`, only when the branch since `onto`
/// changed a live change's delta file, unless the branch is a rename's (`renamed`), whose
/// commit rebased those deltas itself. Returns the `merged` lines, and what is left: the
/// violations and the stale-delta report.
fn regenerate(
    root: &Path,
    onto: &str,
    ids: &[String],
    renamed: bool,
) -> Result<(Vec<String>, Vec<String>), Error> {
    if ids.is_empty() {
        if renamed || !deltas_moved(root, onto)? {
            return Ok((Vec::new(), Vec::new()));
        }
        return match regenerated(root, onto, &[], true)? {
            Regenerated::Tree(mut tree) => {
                let mut merged = std::mem::take(&mut tree.merged);
                let (lines, left) = tree.other_deltas(root, &[], onto)?;
                merged.extend(lines);
                Ok((merged, left))
            }
            Regenerated::Stopped { merged, left } => Ok((merged, left)),
        };
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
    // `sync.regenerate/stale-delta-merge`: the regenerated tree is the last stage.
    let (lines, left) = tree.other_deltas(root, ids, onto)?;
    merged_lines.extend(lines);
    Ok((merged_lines, left))
}

/// Whether a commit in `onto..HEAD` changes a delta file of a live `proposed` or `approved`
/// change at `HEAD`, read by path, never by a commit's message, so a `Merge` commit and a
/// hand resolution count alike.
fn deltas_moved(root: &Path, onto: &str) -> Result<bool, Error> {
    let mut dirs = Vec::new();
    for file in change_files(root, "HEAD")? {
        let Some(dir) = file.strip_suffix("change.md") else {
            continue;
        };
        if dir.starts_with(&format!("changes/{}/", clm_model::ARCHIVE_DIR)) {
            continue;
        }
        let text = git(root, &["show", &object("HEAD", &file)])?;
        if clm_model::parse_change_frontmatter(&text).is_some_and(|fm| fm.status.is_pending()) {
            dirs.push(format!("{dir}deltas"));
        }
    }
    if dirs.is_empty() {
        return Ok(false);
    }
    let range = format!("{onto}..HEAD");
    let mut args = vec!["log", "--format=%H", &range, "--"];
    args.extend(dirs.iter().map(String::as_str));
    Ok(!git(root, &args)?.is_empty())
}

/// `autosync-policy`: on a `spec/*` branch, pushes `HEAD` with the working tree's spec paths
/// (untracked files included) to `origin` as `autosync/<branch>`, leaving the working tree,
/// the index and the branch untouched. Does nothing on other branches or when `origin`
/// already has the same tree. A `GIT_SSH_COMMAND` in this process's environment is git's.
pub fn autosync(root: &Path) -> Result<(), Error> {
    snapshot(root, std::env::var_os("GIT_SSH_COMMAND").as_deref())
}

/// `autosync` with `ssh`, the `GIT_SSH_COMMAND` of the environment the caller was given,
/// handed to git as it is; with none, ssh may neither ask nor wait long.
fn snapshot(root: &Path, ssh: Option<&OsStr>) -> Result<(), Error> {
    let Some(branch) = clm_git::current_branch(root).ok().flatten() else {
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
        if root.join(p).exists() || exists_at(root, "HEAD", &format!("./{p}"))? {
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
    let env = [
        ("GIT_TERMINAL_PROMPT", OsStr::new("0")),
        (
            "GIT_SSH_COMMAND",
            ssh.unwrap_or(OsStr::new("ssh -o BatchMode=yes -o ConnectTimeout=10")),
        ),
    ];
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
