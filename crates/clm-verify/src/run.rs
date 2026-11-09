//! `verify-run` and `verify-commit`: the check run on a commit in the verify tree, and the
//! `verified` record it earns.

use std::path::{Path, PathBuf};

use clm_config::VerifyMode;
use clm_ledger::Body;

use crate::chain::State;
use crate::{Ctx, Error, Outcome, bisect, import, tree};

/// What a check run on one commit came to.
pub(crate) struct Probe {
    pub(crate) passed: bool,
    /// The log, when the run was given one and wrote it.
    pub(crate) log: Option<PathBuf>,
}

/// Runs `[verify] check` on `commit` in the verify tree, with `CLM_SELECTION` naming what
/// `clm affected --base <green>` selects when there is a green commit, under the clone's
/// check lock. Its output goes to stderr, and to `log` when given.
pub(crate) fn check(
    ctx: &Ctx,
    commit: &str,
    green: Option<&str>,
    log: Option<&Path>,
) -> Result<Probe, Error> {
    let tree = tree::Tree::at(ctx, commit)?;
    let selection = green.and_then(|green| selection(ctx, &tree.dir, green));
    let run = clm_land::run_check(
        &ctx.verify.check,
        &tree.dir,
        selection.as_ref().map(|s| s.path.as_path()),
        &clm_land::CheckEnv::default(),
        log,
    )?;
    Ok(Probe {
        passed: run.passed,
        log: log.filter(|_| run.logged).map(Path::to_path_buf),
    })
}

/// The file `clm affected --base <green>`'s output goes in, removed with its directory when
/// dropped.
struct Selection {
    _dir: tempfile::TempDir,
    path: PathBuf,
}

/// What the commit in `dir` can break since `green`, written as `clm affected` prints it;
/// when that fails it says so on stderr and the check runs every test.
fn selection(ctx: &Ctx, dir: &Path, green: &str) -> Option<Selection> {
    let write = || -> Result<Selection, String> {
        let selected =
            clm_affected::affected(dir, green, &ctx.affected_all).map_err(|e| e.to_string())?;
        let text = match selected {
            clm_affected::Selection::All(path) => format!("all {path}\n"),
            clm_affected::Selection::Nodes { nodes, files } => nodes
                .iter()
                .map(|n| format!("node {n}\n"))
                .chain(files.iter().map(|f| format!("file {f}\n")))
                .collect(),
        };
        let parent = (ctx.env)("TMPDIR")
            .filter(|t| !t.is_empty())
            .map_or_else(std::env::temp_dir, PathBuf::from);
        let dir = tempfile::Builder::new()
            .prefix("verify-sel-")
            .tempdir_in(std::path::absolute(parent).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        let path = dir.path().join("selection");
        std::fs::write(&path, text).map_err(|e| e.to_string())?;
        Ok(Selection { _dir: dir, path })
    };
    write()
        .map_err(|why| eprintln!("clm affected failed: {why}; the check runs every test"))
        .ok()
}

/// Verifies the commit at position `end` of the chain in `local` and `pipeline` mode: runs
/// the check, finds the culprit of a red result, appends the record and reports.
pub(crate) fn verify_at(ctx: &Ctx, state: &State, end: usize) -> Result<Outcome, Error> {
    let commit = &state.chain[end];
    let short = ctx.short(commit)?;
    let green = state.last_green(end);
    let log = tree::places(ctx)?
        .join("verify-logs")
        .join(format!("{short}.log"));
    let run = check(
        ctx,
        commit,
        green.map(|g| state.chain[g].as_str()),
        Some(&log),
    )?;
    if run.passed {
        let _ = std::fs::remove_file(&log);
        return record(ctx, state, end, None, true, None);
    }
    let (_, lands) = state.unverified(end);
    let culprit = match lands {
        [only] => only.clone(),
        _ => state.chain[bisect::first_red(ctx, state, green, end)?].clone(),
    };
    let note = bisect::recheck_note(ctx, state, &culprit)?;
    let mut outcome = record(ctx, state, end, Some(culprit), false, None)?;
    outcome.notes.extend(note);
    if let Some(log) = run.log {
        outcome
            .notes
            .push(format!("output is at {}", log.display()));
    }
    Ok(outcome)
}

/// Appends the `verified` record for the commit at position `end`, covering the lands since
/// the last verified commit, and reports it. A red result's components are those the culprit's
/// diff changes, or those of every covered land when it has none.
pub(crate) fn record(
    ctx: &Ctx,
    state: &State,
    end: usize,
    culprit: Option<String>,
    passed: bool,
    run: Option<String>,
) -> Result<Outcome, Error> {
    let (since, lands) = state.unverified(end);
    let commit = state.chain[end].clone();
    let components = match passed {
        true => Vec::new(),
        false => bisect::components(ctx, &commit, culprit.as_deref(), lands)?,
    };
    let body = Body::Verified {
        commit: commit.clone(),
        passed,
        since: since.map(|s| state.chain[s].clone()),
        lands: lands.to_vec(),
        culprit: culprit.clone(),
        components: components.clone(),
        mode: ledger_mode(ctx.verify.mode),
        run,
    };
    let appended = clm_ledger::append(&ctx.root, body, ctx.env)?;
    let mut outcome = report(
        ctx,
        &commit,
        passed,
        lands.len(),
        culprit.as_deref(),
        &components,
    )?;
    if !appended.pushed {
        outcome
            .notes
            .push("the record is committed locally; `clm reconnect` pushes it".to_string());
    }
    Ok(outcome)
}

pub(crate) fn ledger_mode(mode: VerifyMode) -> clm_ledger::VerifyMode {
    match mode {
        VerifyMode::Local => clm_ledger::VerifyMode::Local,
        VerifyMode::Pipeline => clm_ledger::VerifyMode::Pipeline,
        VerifyMode::Import => clm_ledger::VerifyMode::Import,
    }
}

/// The line a result is reported as, and its exit code.
pub(crate) fn report(
    ctx: &Ctx,
    commit: &str,
    passed: bool,
    lands: usize,
    culprit: Option<&str>,
    components: &[String],
) -> Result<Outcome, Error> {
    let short = ctx.short(commit)?;
    if passed {
        return Ok(Outcome::new(
            format!("verify passed {short} {lands} lands"),
            0,
        ));
    }
    let culprit = match culprit {
        Some(c) => ctx.short(c)?,
        None => "unknown".to_string(),
    };
    Ok(Outcome::new(
        format!(
            "verify failed {short} culprit {culprit} components {}",
            components.join(" ")
        ),
        1,
    ))
}

/// `clm verify --commit <sha>`: the answer from the ledger when a record names the commit,
/// else the verification of it as `verify-run` or `verify-import` verifies the head.
pub(crate) fn commit(ctx: &Ctx, state: &State, sha: &str) -> Result<Outcome, Error> {
    let (full, end) = state.resolve(ctx, sha)?;
    if let Some(Body::Verified {
        passed,
        lands,
        culprit,
        components,
        ..
    }) = state.newest(&full).map(|r| &r.body)
    {
        return report(
            ctx,
            &full,
            *passed,
            lands.len(),
            culprit.as_deref(),
            components,
        );
    }
    match ctx.verify.mode {
        VerifyMode::Import => import::import_commit(ctx, state, end),
        _ => verify_at(ctx, state, end),
    }
}
