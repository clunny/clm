//! `clm land [<branch>]` (`land/land`, `land/landed-branch-cleanup`, `land/offline-land`,
//! `land/selected-tests`, `land/signed-land`).

use std::path::Path;

use clm_git::REMOTE;
use clm_ledger::{Body, Entry, LandKind};

use crate::exe::Exe;
use crate::lock;
use crate::lock::{Lane, Rank};
use crate::scratch::Scratch;
use crate::{CheckHow, Selected};
use crate::{Env, Error, Output, Repo, Verdict, is_claim};
use clm_approval::{Approvals, Policy};
use clm_model::Priority;

const ATTEMPTS: usize = 10;

/// How a land's failing check is reported.
const ON_MERGE: &str = "fails on the merge with main";

/// Lands `branch` (default: the current one) on `main`: see the node's Design, steps 1–9.
pub fn land(root: &Path, branch: Option<&str>, env: &Env) -> Result<Output, Error> {
    let repo = Repo::open(root, env)?;
    let branch = repo.approval.branch_name(branch, "landed")?;
    let check = repo.settings.land.value.as_ref().map(|l| l.check.clone());
    // What a `checked` record names: the project's command, or `clm check` alone.
    let command = check.clone().unwrap_or_else(|| "clm check".into());
    // Before any subcommand runs, so none runs from a binary the check can take away.
    let exe = Exe::new(&repo)?;
    let exe = exe.path();
    // Everything the land keeps, under `<home>/.clm`.
    let places = repo.places()?;
    // Where `[land] check` builds, and whether it is the clone's shared `.clm/land`.
    let target = check.as_ref().map(|_| repo.target(&places));
    let checked_with = |fold: &clm_ledger::Fold, key: &str| {
        fold.checks.get(key).is_some_and(|records| {
            records
                .iter()
                .any(|r| matches!(&r.body, Body::Checked { command: c, .. } if *c == command))
        })
    };
    // The effort the branch's sessions spent here, read once, so neither the reading nor a
    // ledger push of its own lengthens the time `main` has moved with no `landed` record
    // (`land/records-effort`). Unreadable, it only warns.
    let options = clm_effort::Options {
        branch: Some(branch.clone()),
        ..Default::default()
    };
    let (effort, effort_warnings) = match clm_effort::unrecorded(&repo.root, &options, env) {
        Ok(read) => {
            let warnings = read.warnings.iter().map(|w| format!("effort: {w}"));
            (read.bodies, warnings.collect())
        }
        Err(e) => (Vec::new(), vec![format!("effort not recorded: {e}")]),
    };
    // 1. Fetch, pick the head, fold, and merge the trees: before the worktree it checks in
    // exists, a land knows whether its check will run.
    let prepare = || -> Result<Prepared, Error> {
        repo.fetch()?;
        let (head, on_origin) = repo.approval.head(&branch, true)?;
        let base = repo
            .rev(repo.base())?
            .ok_or_else(|| Error::Refused(format!("no `{}` to land on", repo.base())))?;
        // A signed repository signs this land's record, so a lander without a key is
        // refused before any check runs (`land/signed-land`).
        let signed = repo.approval.signed(&base)?;
        if signed {
            repo.approval.require_key()?;
        }
        // The policy committed at the base, read afresh each attempt.
        let policy = repo.approval.policy(&base)?;
        let fold = clm_ledger::fold(&repo.root)?;
        // A spec branch holds spec paths only, whatever its gate says
        // (`land/spec-branch-holds-spec-paths`).
        if branch.starts_with("spec/") {
            let files = clm_gate::files_outside_spec_paths(&repo.root, &base, &head)
                .map_err(Error::Refused)?;
            if !files.is_empty() {
                return Err(Error::Refused(format!(
                    "a spec branch holds only spec paths; {branch} changes {}",
                    files.join(", ")
                )));
            }
        }
        // A component whose slow tests broke after a land waits for its fix; the components
        // of a red result nobody took in yet only warn (`land/verify-stop`).
        let warnings = crate::verify_stop::check(&repo, &base, &head, &fold)?;
        // 2. The fingerprint, and a request for changes that holds it, before the gate, the
        // lock, the merge and the checks (`land/changes-requested-holds`).
        let fingerprint = repo.approval.fingerprint(&base, &head)?;
        if let Some(why) = clm_approval::held(&fold, &branch, &fingerprint) {
            return Err(Error::Refused(why));
        }
        // A head behind in `specs/` lands on the merge git writes without regard to its
        // `specs/` conflicts, its `specs/` regenerated (`land/regenerates-behind-specs`).
        let direct = unconflicted_tree(&repo, &base, &head)?;
        let merge_base = repo.git(&["merge-base", &base, &head])?;
        let behind = specs_changed_on_both(&repo, &merge_base, &base, &head)?
            && crate::derived::adds_or_applies(&repo, &merge_base, &head)?;
        let tree = match direct {
            Some(tree) => Some(tree),
            None if behind => specs_agreed_tree(&repo, &base, &head)?,
            None => None,
        };
        // A conflict outside `specs/` is refused as any conflict is.
        let behind = behind && tree.is_some();
        let merged = match tree {
            Some(tree) => {
                let key = clm_ledger::code_key(&repo.root, &tree)?;
                Some((tree, key))
            }
            None => None,
        };
        // No record names the merged key: a `clm.toml` that differs only in what
        // `settings_unchanged` sets aside is looked up by the key without it, here, before
        // the lock is decided on (`land/settings-unchanged-is-checked`).
        let via_settings = match &merged {
            Some((tree, key)) if !checked_with(&fold, key) => {
                match settings_key(&repo, tree, &base)? {
                    Some(settings_key) => checked_with(&fold, &settings_key),
                    None => false,
                }
            }
            _ => false,
        };
        Ok(Prepared {
            head,
            on_origin,
            base,
            signed,
            policy,
            fingerprint,
            fold,
            merged,
            via_settings,
            behind,
            warnings,
        })
    };
    // The check runs when the merge is clean and its code key wasn't checked; a conflicting
    // merge runs none, and steps 3 and 4 report it.
    let check_runs = |p: &Prepared| {
        p.merged
            .as_ref()
            .is_some_and(|(_, key)| !p.via_settings && !checked_with(&p.fold, key))
    };
    let mut out = Output {
        warnings: effort_warnings,
        ..Default::default()
    };
    let mut temp: Option<Scratch> = None;
    for attempt in 1..=ATTEMPTS {
        let mut p = prepare()?;
        // The priority this land's tickets carry, worked out once, before its first wait
        // of the attempt (`land/lock-queue`).
        let mut priority: Option<Priority> = None;
        let mut lock = None;
        if let Some((target, true)) = &target
            && check_runs(&p)
        {
            // Unchecked code: the check runs every test the branch can break, so an ordinary
            // ticket (`land/lock-queue`).
            let rank = Rank {
                lane: Lane::Ordinary,
                priority: land_priority(&repo, &branch, &p, &mut priority, &mut out),
            };
            let held = lock::land(target, rank)?;
            // The land it waited for may have moved `main` or checked this code meanwhile.
            if held.waited {
                p = prepare()?;
            }
            if check_runs(&p) {
                lock = Some(held);
            }
        }
        let Prepared {
            head,
            on_origin,
            base,
            signed,
            policy,
            fingerprint,
            fold,
            merged,
            via_settings,
            behind,
            warnings,
        } = p;
        for warning in warnings {
            if !out.warnings.contains(&warning) {
                out.warnings.push(warning);
            }
        }
        // Under the lock, the clone's land tree, so cargo sees one workspace path land after
        // land; else a temporary worktree, since another land may hold the land tree.
        // The temporary one is kept across attempts and checked out afresh.
        let land_tree;
        let dir: &Scratch = match (&lock, &temp) {
            (Some(_), _) => {
                land_tree = Scratch::land_tree(&repo, &places, &head)?;
                &land_tree
            }
            (None, Some(t)) => {
                t.checkout(&head)?;
                t
            }
            (None, None) => temp.insert(Scratch::temp(&repo, &head)?),
        };

        // 2. Whether the fingerprint is approved.
        let approvals = repo.approval.approval(
            &fold,
            &fingerprint,
            &head,
            signed.then_some(&*base),
            policy.other,
        )?;
        let approved = approvals.accepted;
        let hint = approval_hint(&approvals, &branch, &policy);

        // 3. The gate on the head; a head behind in `specs/` is gated on its merge commit,
        // which contains the base, once that is made.
        let head_verdict = if behind {
            None
        } else {
            Some(repo.gate(exe, dir.path(), approved, &hint)?)
        };

        // 4. The merged tree.
        let (tree, key) = merged_tree(&repo, &base, &head, merged, behind)?;
        // Read whatever the head gates as, since the merge's gate can make it a spec land.
        let changes = repo.approval.changes(&head)?;
        // A head behind in `specs/` has its own `specs/` proved derived and the merged one
        // regenerated onto the base (`land/regenerates-behind-specs`).
        let tree = if behind {
            crate::derived::behind_tree(&repo, &base, &head, &tree, dir)?
        } else {
            tree
        };

        // 5. The merge commit, checked out. A head behind in `specs/` adds or applies a
        // change, so it lands as a spec land.
        let mut message = format!("Merge {branch} into main");
        let spec_land = head_verdict.is_none_or(|v| matches!(v, Verdict::Spec { .. }));
        if let (true, Some(id)) = (spec_land, changes.first()) {
            message.push_str(&format!("\n\nChange: {id}"));
        }
        let merge = repo.git(&[
            "commit-tree",
            &tree,
            "-p",
            &base,
            "-p",
            &head,
            "-m",
            &message,
        ])?;
        dir.checkout(&merge)?;
        // Before any check: on a head that adds a change, `specs/` must be derived. A head
        // behind in `specs/` was regenerated, so it holds by construction, and its gate runs
        // here, on the merge commit.
        if head_verdict.is_some() {
            crate::derived::check(&repo, &base, &head, dir.path())?;
        } else {
            repo.gate(exe, dir.path(), approved, &hint)?;
        }

        // 6. The checks, unless the code was checked, and the gate again. The merge commit's
        // code key is its tree's.
        let checked = via_settings || checked_with(&fold, &key);
        // Whether `[land] check` runs, and the selection it gets: on unchecked code the tests
        // the branch can break, on checked code only the readers of what differs
        // (`land/selected-tests`).
        let mut runs = !checked && check.is_some();
        let mut selection = None;
        if check.is_some() {
            match repo.selection(exe, dir.path(), &base, checked) {
                Selected::Nothing => {}
                Selected::Tests(s) => {
                    selection = Some(s);
                    runs = true;
                }
                Selected::Failed => runs = true,
            }
        }
        // Readers to run on checked code: the check compiles into the shared target, so
        // under its lock and in the land tree, not the temporary worktree.
        let check_lane = if checked && selection.is_some() {
            Lane::Priority
        } else {
            Lane::Ordinary
        };
        let check_rank = Rank {
            lane: check_lane,
            priority: match priority {
                Some(priority) => priority,
                None if runs => {
                    land_priority_at(&repo, &branch, &base, &head, &mut priority, &mut out)
                }
                None => Priority::P5,
            },
        };
        let land_tree_for_readers;
        let dir: &Scratch = match &target {
            Some((target, true)) if checked && runs => {
                // A readers' selection that ran takes the priority lane; one that failed, so
                // that every test runs, the ordinary (`land/lock-queue`).
                lock = Some(lock::land(target, check_rank)?);
                drop(temp.take());
                land_tree_for_readers = Scratch::land_tree(&repo, &places, &merge)?;
                &land_tree_for_readers
            }
            _ => dir,
        };
        if runs && let (Some(check), Some((target, shared))) = (&check, &target) {
            let mut vars: Vec<(&str, &Path)> = selection
                .iter()
                .map(|s| ("CLM_SELECTION", s.path.as_path()))
                .collect();
            // Incremental data in the target every land shares only grows, so a check
            // that builds there runs without it unless the caller asked for it
            // (`land/shared-land-target`).
            let incremental;
            if *shared {
                incremental = repo.incremental();
                vars.push(("CARGO_INCREMENTAL", Path::new(&incremental)));
            }
            // Kept for a failing check, by branch, beside `.clm/land`.
            let log = places.join("land-logs").join(format!("{branch}.log"));
            // The slot of the place that names the branch, so the tests key their ports and
            // paths apart from other builds' (`land/check-slot`); a land always takes the
            // check lock (`land/check-lock`).
            let slot = repo.slot(&branch)?;
            repo.project_check(
                dir.path(),
                check,
                Some(target),
                &vars,
                ON_MERGE,
                CheckHow {
                    log: Some(&log),
                    slot,
                    rank: Some(check_rank),
                },
            )?;
        }
        if checked {
            repo.clm_check(exe, dir.path(), false, ON_MERGE)?;
            // The tree's own key is checked now, so the `landed` record names a checked key
            // and a retry finds it (`land/settings-unchanged-is-checked`).
            if !checked_with(&fold, &key) {
                repo.append(Body::Checked {
                    code_key: key.clone(),
                    command: command.clone(),
                })?;
            }
        } else {
            repo.clm_check(exe, dir.path(), true, ON_MERGE)?;
            repo.append(Body::Checked {
                code_key: key.clone(),
                command: command.clone(),
            })?;
        }
        let verdict = repo.gate(exe, dir.path(), approved, &hint)?;
        let (kind, review) = match verdict {
            Verdict::Code => (LandKind::Code, false),
            Verdict::Spec { review } => (LandKind::Spec, review),
        };
        // Only a spec land names its changes.
        let changes = if kind == LandKind::Spec {
            changes
        } else {
            Vec::new()
        };
        let landed = Body::Landed {
            commit: merge.clone(),
            branch: branch.clone(),
            kind,
            review,
            fingerprint,
            code_key: Some(key),
            changes: changes.clone(),
        };
        // Signed before the push, so a land that can't sign pushes nothing; the pushes of
        // this merge commit share the record.
        let landed: Entry = match signed {
            true => repo.approval.sign(landed, clm_approval::LAND)?.into(),
            false => landed.into(),
        };

        // 7. The push.
        if push_main(&repo, &merge, &base, &branch)? == Pushed::Moved {
            out.warnings
                .push(format!("main moved during attempt {attempt}; trying again"));
            continue;
        }
        // The lock covered the gate on the merge commit, `clm check` and the push, so no other
        // land checked its commit out before this one was on `main`. The cap on the shared
        // target (`land/land-target-cap`) runs under it too; the rest runs outside it.
        if let (Some(_), Some((target, _)), Some(land)) =
            (&lock, &target, repo.settings.land.value.as_ref())
        {
            crate::cap::apply(target, land.cap_gb(), &mut out.warnings);
        }
        drop(lock);
        drop(temp.take());

        // 8. The effort read before the first attempt and the record, in one commit, and the
        // branch removed.
        let mut entries: Vec<clm_ledger::Entry> = effort.iter().cloned().map(Into::into).collect();
        entries.push(landed);
        let appended = repo
            .append_all(entries)?
            .pop()
            .expect("the landed record was appended last");

        // The push to `main` went through, so the land stands; the push named, or this
        // clone's next append, sends the record.
        if !appended.pushed {
            out.warnings.push(format!(
                "the landed record isn't on origin yet: git push origin clm/ledger/{}",
                appended.record.writer
            ));
        }
        cleanup(&repo, &branch, &head, on_origin, &mut out.warnings);

        // 9. What landed.
        out.lines.push(match verdict {
            Verdict::Code => format!("landed {merge} code"),
            Verdict::Spec { review: false } => format!("landed {merge} spec"),
            Verdict::Spec { review: true } => format!("landed {merge} spec review"),
        });
        if branch.starts_with("impl/") {
            free_build_target(&repo, &branch, &mut out);
        }
        return Ok(out);
    }
    Err(Error::Refused(format!(
        "`main` keeps moving: gave up after {ATTEMPTS} attempts"
    )))
}

/// What a land refused by `gate/approval` tells the caller: the policy when it is `other`
/// and where it came from, why each approval of the diff doesn't count, and who to ask.
fn approval_hint(approvals: &Approvals, branch: &str, policy: &Policy) -> String {
    let mut hint = String::new();
    if policy.other {
        hint.push_str(&format!(
            "approval = \"other\" from {}: only an approval by someone who isn't an author \
             of the branch counts\n",
            policy.source
        ));
    }
    if !approvals.refused.is_empty() {
        hint.push_str("no approval of this diff counts:\n");
        for why in &approvals.refused {
            hint.push_str(&format!("  {why}\n"));
        }
    }
    // Under `approval = "other"`, the authors' and the lander's own approvals never count.
    let who = match approvals
        .refused
        .iter()
        .any(|w| w.contains("approval = \"other\""))
    {
        true => "another person",
        false => "the human",
    };
    hint.push_str(&format!("ask {who} to run `clm approve {branch}`"));
    hint
}

/// Frees a landed build's disk (`land/land-frees-build-target`): deletes `target/` at the
/// top of the worktree `clm land` runs in when that is a linked worktree with the landed
/// branch checked out, and says so; a failure only warns, since the land is done.
fn free_build_target(repo: &Repo, branch: &str, out: &mut Output) {
    let linked_on_branch = || -> Result<Option<std::path::PathBuf>, Error> {
        let top = std::path::PathBuf::from(repo.git(&["rev-parse", "--show-toplevel"])?);
        let main = repo.main_worktree()?;
        let canon = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
        if canon(&top) == canon(&main) || clm_git::current_branch(&top)?.as_deref() != Some(branch)
        {
            return Ok(None);
        }
        Ok(Some(top.join("target")))
    };
    let target = match linked_on_branch() {
        Ok(Some(target)) if target.exists() => target,
        Ok(_) => return,
        Err(e) => {
            out.warnings
                .push(format!("kept the build's `target/`: {e}"));
            return;
        }
    };
    match std::fs::remove_dir_all(&target) {
        Ok(()) => out.lines.push(format!("removed {}", target.display())),
        Err(e) => out.warnings.push(format!("kept {}: {e}", target.display())),
    }
}

/// The tree `git merge-tree` writes for `head` merged into `base`, or none when it conflicts.
fn unconflicted_tree(repo: &Repo, base: &str, head: &str) -> Result<Option<String>, Error> {
    match clm_git::git(&repo.root, &["merge-tree", "--write-tree", base, head]) {
        Ok(out) => Ok(out.lines().next().map(str::to_string)),
        // Exit 1: conflicts.
        Err(clm_git::Error::Failed { status, .. }) if status.code() == Some(1) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// For a `head` whose merge conflicts: the tree `git merge-tree` writes for the merge of
/// `base` with a commit like `head` but for `specs/`, which it takes from `base`, so that
/// only a conflict outside `specs/` remains; none when there is one
/// (`land/regenerates-behind-specs`). Its `specs/` is `base`'s, to be regenerated.
fn specs_agreed_tree(repo: &Repo, base: &str, head: &str) -> Result<Option<String>, Error> {
    let dir = tempfile::Builder::new()
        .prefix("land-index-")
        .tempdir_in(crate::temp_parent(repo.env)?)?;
    let index = dir.path().join("index");
    let env = [("GIT_INDEX_FILE", index.as_os_str())];
    let git = |args: &[&str]| -> Result<String, Error> {
        let out = clm_git::run(
            &repo.root,
            "git",
            args,
            clm_git::Options {
                stdin: None,
                env: &env,
            },
        )?;
        Ok(String::from_utf8_lossy(&out).trim().to_string())
    };
    // Paths of the CLM root's `specs/` from the top of the repository.
    let prefix = repo.git(&["rev-parse", "--show-prefix"])?;
    let specs = format!("{prefix}specs");
    git(&["read-tree", head])?;
    git(&[
        "rm",
        "-r",
        "-f",
        "--cached",
        "--quiet",
        "--ignore-unmatch",
        "--",
        &format!(":(top){specs}"),
    ])?;
    let at_base = format!("{base}:{specs}");
    if repo.git(&["cat-file", "-e", &at_base]).is_ok() {
        git(&["read-tree", &format!("--prefix={specs}/"), &at_base])?;
    }
    let tree = git(&["write-tree"])?;
    let agreed = repo.git(&[
        "commit-tree",
        &tree,
        "-p",
        head,
        "-m",
        "head with the base's specs",
    ])?;
    unconflicted_tree(repo, base, &agreed)
}

/// Whether a file of `specs/` changed both between the merge base and `base` and between it
/// and `head`.
fn specs_changed_on_both(
    repo: &Repo,
    merge_base: &str,
    base: &str,
    head: &str,
) -> Result<bool, Error> {
    let specs = |tip: &str| -> Result<Vec<String>, Error> {
        let names = repo.git(&["diff", "--name-only", merge_base, tip, "--", "specs/"])?;
        Ok(names.lines().map(str::to_string).collect())
    };
    let ours = specs(base)?;
    Ok(specs(head)?.iter().any(|p| ours.contains(p)))
}

/// The code key of `tree` with the CLM root's `clm.toml` as it is at `base`, when the two
/// differ in their bytes and `clm-gate`'s `settings_unchanged` holds of them; none otherwise
/// (`land/settings-unchanged-is-checked`).
fn settings_key(repo: &Repo, tree: &str, base: &str) -> Result<Option<String>, Error> {
    let prefix = repo.git(&["rev-parse", "--show-prefix"])?;
    let path = format!("{prefix}clm.toml");
    // A missing file reads as none.
    let read = |rev: &str| -> Result<Option<Vec<u8>>, Error> {
        let spec = format!("{rev}:{path}");
        if repo.git(&["cat-file", "-e", &spec]).is_err() {
            return Ok(None);
        }
        let blob = clm_git::run(
            &repo.root,
            "git",
            &["cat-file", "blob", &spec],
            clm_git::Options::default(),
        )?;
        Ok(Some(blob))
    };
    let (before, after) = (read(base)?, read(tree)?);
    if before == after || !clm_gate::settings_unchanged(before.as_deref(), after.as_deref()) {
        return Ok(None);
    }
    Ok(Some(clm_ledger::code_key_with(
        &repo.root, tree, &path, base,
    )?))
}

/// What step 1 gathers for an attempt.
struct Prepared {
    head: String,
    /// Whether `head` is `origin`'s tip of the branch.
    on_origin: bool,
    base: String,
    /// Whether the repository is signed at `base`.
    signed: bool,
    policy: Policy,
    /// The fingerprint of `head`'s diff against `base`.
    fingerprint: String,
    fold: clm_ledger::Fold,
    /// The tree of `head` merged into `base` and its code key, or none when they conflict.
    merged: Option<(String, String)>,
    /// Whether the merged tree counts as checked through its settings-free key, though no
    /// `checked` record names its own (`land/settings-unchanged-is-checked`).
    via_settings: bool,
    /// Whether `head` is behind in `specs/`: it adds or applies a change and `specs/` changed
    /// on both sides, with no conflict outside it (`land/regenerates-behind-specs`).
    behind: bool,
    /// What `land/verify-stop` warns of.
    warnings: Vec<String>,
}

/// The merged tree and its code key, refused when the merge conflicts or both sides changed
/// `specs/` (unless the head is `behind` in it, whose `specs/` is regenerated).
fn merged_tree(
    repo: &Repo,
    base: &str,
    head: &str,
    merged: Option<(String, String)>,
    behind: bool,
) -> Result<(String, String), Error> {
    let rebase = |what: &str| {
        Error::Refused(format!(
            "{what}: run `clm rebase` on the branch, push it, and land it again"
        ))
    };
    let merged = merged.ok_or_else(|| rebase("the branch conflicts with main"))?;
    let merge_base = repo.git(&["merge-base", base, head])?;
    if !behind && specs_changed_on_both(repo, &merge_base, base, head)? {
        return Err(rebase("`specs/` changed on both the branch and main"));
    }
    Ok(merged)
}

/// What a push of the merge commit to `main` came to.
#[derive(Debug, PartialEq, Eq)]
enum Pushed {
    /// `main` on `origin` is the merge commit.
    Done,
    /// `main` moved on from the base: the next attempt starts again.
    Moved,
}

/// The waits, in seconds, before each push of the same merge commit after a refusal that
/// left `main` at the base (`land/land-push-retry`): fixed, as `clm claim`'s are.
const PUSH_RETRY_WAITS: [u64; 2] = [1, 2];

/// Pushes `merge` to `origin`'s `main`, leased on `base`, and judges a refusal by fetching
/// `main` again: the merge commit is a push that went through, another commit is `main`
/// moving, and the base is a refusal for another reason, pushed again after each wait. A
/// `main` still at the base after the last push is a git failure a rerun can safely retry;
/// a fetch that fails is one it can't, since the push may have gone through.
fn push_main(repo: &Repo, merge: &str, base: &str, branch: &str) -> Result<Pushed, Error> {
    let lease = format!("--force-with-lease=refs/heads/main:{base}");
    let spec = format!("{merge}:refs/heads/main");
    let mut waits = PUSH_RETRY_WAITS.into_iter();
    loop {
        // `--quiet` keeps what the remote printed, such as a hook's `remote:` lines.
        let refused = match repo.git(&["push", "--quiet", &lease, REMOTE, &spec]) {
            Ok(_) => return Ok(Pushed::Done),
            Err(e) => e,
        };
        // Directly, not through `Repo::fetch`, which calls a failure being offline. Any
        // failure to judge `main` reports the refused push, since it may have gone through.
        let tracking = format!("refs/remotes/{REMOTE}/main");
        let main = format!("+refs/heads/main:{tracking}");
        let judged = || -> Result<Option<String>, Error> {
            repo.git(&["fetch", "--quiet", REMOTE, &main])?;
            repo.rev(&tracking)
        };
        let Ok(now) = judged() else {
            return Err(refused);
        };
        match now {
            Some(now) if now == base => {}
            // The merge commit, or a commit another land built on it: the push went through.
            Some(now)
                if now == merge
                    || clm_git::is_ancestor(&repo.root, merge, &now)
                        .map_err(|_| Error::Git(refused.to_string()))? =>
            {
                return Ok(Pushed::Done);
            }
            _ => return Ok(Pushed::Moved),
        }
        match waits.next() {
            Some(secs) => std::thread::sleep(std::time::Duration::from_secs(secs)),
            None => {
                return Err(Error::Git(format!(
                    "{refused}\nthe failure may be transient: nothing reached `main` and no \
                     land was recorded, so running `clm land {branch}` again is safe"
                )));
            }
        }
    }
}

/// Deletes a landed claim branch from `origin`, leased on `head`, and the local branch when
/// it is checked out nowhere and holds nothing the land didn't; a failure warns.
fn cleanup(repo: &Repo, branch: &str, head: &str, on_origin: bool, warnings: &mut Vec<String>) {
    if !is_claim(branch) {
        return;
    }
    if on_origin {
        let host = clm_host::adapter(repo.settings.host.value);
        if let Err(e) = host.branch_delete(&repo.root, branch, head) {
            let why = match e {
                clm_host::Error::Refused(why)
                | clm_host::Error::Git(why)
                | clm_host::Error::Nowhere(why) => why,
            };
            warnings.push(format!("kept `{branch}` on origin: {why}"));
        }
    }
    let local = match repo.rev(&format!("refs/heads/{branch}")) {
        Ok(Some(tip)) => tip,
        _ => return,
    };
    let checked_out = clm_git::worktrees(&repo.root)
        .map(|ws| ws.iter().any(|w| w.branch.as_deref() == Some(branch)))
        .unwrap_or(true);
    let landed = clm_git::is_ancestor(&repo.root, &local, head).unwrap_or(false);
    if !checked_out
        && landed
        && let Err(e) = repo.git(&["branch", "--quiet", "-D", branch])
    {
        warnings.push(format!("kept the local `{branch}`: {e}"));
    }
}

/// The priority a land of `branch` carries, `orchestrator`'s `land_priority` for its head on
/// the fetched base, kept in `cache` for the rest of the attempt; `P5` with a warning when
/// it can't be worked out, since the order is never worth failing a land for.
fn land_priority(
    repo: &Repo,
    branch: &str,
    p: &Prepared,
    cache: &mut Option<Priority>,
    out: &mut Output,
) -> Priority {
    land_priority_at(repo, branch, &p.base, &p.head, cache, out)
}

fn land_priority_at(
    repo: &Repo,
    branch: &str,
    base: &str,
    head: &str,
    cache: &mut Option<Priority>,
    out: &mut Output,
) -> Priority {
    *cache.get_or_insert_with(|| {
        clm_orchestrator::land_priority(&repo.root, branch, base, head).unwrap_or_else(|e| {
            let why = match e {
                clm_orchestrator::Error::Refused(why) | clm_orchestrator::Error::Git(why) => why,
                clm_orchestrator::Error::Spec(_) => "the specs don't load".into(),
            };
            let warning = format!("land priority not worked out, queueing at P5: {why}");
            if !out.warnings.contains(&warning) {
                out.warnings.push(warning);
            }
            Priority::P5
        })
    })
}
