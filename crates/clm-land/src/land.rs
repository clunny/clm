//! `clm land [<branch>]` (`land/land`, `land/landed-branch-cleanup`, `land/offline-land`,
//! `land/selected-tests`, `land/signed-land`).

use std::path::Path;

use clm_git::REMOTE;
use clm_ledger::{Body, Entry, LandKind};

use crate::exe::Exe;
use crate::lock;
use crate::scratch::Scratch;
use crate::{Env, Error, Output, Repo, Verdict, is_claim};
use clm_approval::{Approvals, Policy};

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
        // 2. The fingerprint, and a request for changes that holds it, before the gate, the
        // lock, the merge and the checks (`land/changes-requested-holds`).
        let fingerprint = repo.approval.fingerprint(&base, &head)?;
        if let Some(why) = clm_approval::held(&fold, &branch, &fingerprint) {
            return Err(Error::Refused(why));
        }
        let merged = match unconflicted_tree(&repo, &base, &head)? {
            Some(tree) => {
                let key = clm_ledger::code_key(&repo.root, &tree)?;
                Some((tree, key))
            }
            None => None,
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
        })
    };
    // The check runs when the merge is clean and its code key wasn't checked; a conflicting
    // merge runs none, and steps 3 and 4 report it.
    let check_runs = |p: &Prepared| {
        p.merged
            .as_ref()
            .is_some_and(|(_, key)| !checked_with(&p.fold, key))
    };
    let mut out = Output {
        warnings: effort_warnings,
        ..Default::default()
    };
    let mut temp: Option<Scratch> = None;
    for attempt in 1..=ATTEMPTS {
        let mut p = prepare()?;
        let mut lock = None;
        if let Some((target, true)) = &target
            && check_runs(&p)
        {
            let (file, waited) = lock::lock(target)?;
            // The land it waited for may have moved `main` or checked this code meanwhile.
            if waited {
                p = prepare()?;
            }
            if check_runs(&p) {
                lock = Some(file);
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
        } = p;
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

        // 3. The gate on the head.
        let verdict = repo.gate(exe, dir.path(), approved, &hint)?;

        // 4. The merged tree.
        let (tree, key) = merged_tree(&repo, &base, &head, merged)?;
        // Read whatever the head gates as, since the merge's gate can make it a spec land.
        let changes = repo.approval.changes(&head)?;

        // 5. The merge commit, checked out.
        let mut message = format!("Merge {branch} into main");
        if let (Verdict::Spec { .. }, Some(id)) = (verdict, changes.first()) {
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
        // Before any check: on a head that adds a change, `specs/` must be derived.
        crate::derived::check(&repo, &base, &head, dir.path())?;

        // 6. The checks, unless the code was checked, and the gate again. The merge commit's
        // code key is its tree's.
        if checked_with(&fold, &key) {
            repo.clm_check(exe, dir.path(), false, ON_MERGE)?;
        } else {
            if let Some(check) = &check {
                let (target, shared) = target.as_ref().expect("a check has a target");
                // The tests the branch can break, against the `main` it fetched.
                let selection = repo.selection(exe, dir.path(), &base);
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
                repo.project_check(dir.path(), check, Some(target), &vars, ON_MERGE, Some(&log))?;
            }
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
        if repo.remote {
            if push_main(&repo, &merge, &base, &branch)? == Pushed::Moved {
                out.warnings
                    .push(format!("main moved during attempt {attempt}; trying again"));
                continue;
            }
        } else {
            advance_local_main(&repo, &merge, &base)?;
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
        if repo.remote && !appended.pushed {
            out.warnings.push(format!(
                "the landed record isn't on origin yet: git push origin clm/ledger/{}",
                appended.record.writer
            ));
        }
        cleanup(&repo, &branch, &head, on_origin, &mut out.warnings);

        // 9. What landed.
        let local = if repo.remote { "" } else { " (local)" };
        out.lines.push(match verdict {
            Verdict::Code => format!("landed {merge} code{local}"),
            Verdict::Spec { review: false } => format!("landed {merge} spec{local}"),
            Verdict::Spec { review: true } => format!("landed {merge} spec review{local}"),
        });
        if repo.remote && branch.starts_with("impl/") {
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
}

/// The merged tree and its code key, refused when the merge conflicts or both sides changed
/// `specs/`.
fn merged_tree(
    repo: &Repo,
    base: &str,
    head: &str,
    merged: Option<(String, String)>,
) -> Result<(String, String), Error> {
    let rebase = |what: &str| {
        Error::Refused(format!(
            "{what}: run `clm rebase` on the branch, push it, and land it again"
        ))
    };
    let merged = merged.ok_or_else(|| rebase("the branch conflicts with main"))?;
    let merge_base = repo.git(&["merge-base", base, head])?;
    let specs = |tip: &str| -> Result<Vec<String>, Error> {
        let names = repo.git(&["diff", "--name-only", &merge_base, tip, "--", "specs/"])?;
        Ok(names.lines().map(str::to_string).collect())
    };
    let ours = specs(base)?;
    if specs(head)?.iter().any(|p| ours.contains(p)) {
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

/// Moves the local `main` from `base` to `merge`: a fast-forward in the worktree that has it
/// checked out, else the ref alone.
fn advance_local_main(repo: &Repo, merge: &str, base: &str) -> Result<(), Error> {
    if repo.rev("refs/heads/main")?.as_deref() != Some(base) {
        return Err(Error::Refused(
            "the local `main` moved during the land: land again".into(),
        ));
    }
    let worktrees = clm_git::worktrees(&repo.root)?;
    match worktrees
        .iter()
        .find(|w| w.branch.as_deref() == Some("main"))
    {
        Some(w) => clm_git::git(&w.path, &["merge", "--quiet", "--ff-only", merge]).map(drop)?,
        None => repo
            .git(&["update-ref", "refs/heads/main", merge, base])
            .map(drop)?,
    }
    Ok(())
}

/// Deletes a landed claim branch from `origin`, leased on `head`, and the local branch when
/// it is checked out nowhere and holds nothing the land didn't; a failure warns.
fn cleanup(repo: &Repo, branch: &str, head: &str, on_origin: bool, warnings: &mut Vec<String>) {
    // Without a remote the local branch is the only copy, which `clm reconnect` may yet ask
    // to land again, so it stays.
    if !is_claim(branch) || !repo.remote {
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
