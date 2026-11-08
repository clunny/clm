//! `verify-import`: the result of a host pipeline's run, recorded without running a check.

use clm_config::VerifyMode;
use clm_host::Run;

use crate::chain::State;
use crate::{Ctx, Error, Outcome, run};

/// `clm verify --import`: whatever result there is. Outside `import` mode there is none to
/// read, and no host is asked.
pub(crate) fn import_all(ctx: &Ctx, state: &State) -> Result<Outcome, Error> {
    if ctx.verify.mode != VerifyMode::Import {
        return Ok(Outcome::new("verify nothing to import", 0));
    }
    import_head(ctx, state)
}

/// The newest completed run on an unverified land, recorded; `verify pending` when there is
/// none.
pub(crate) fn import_head(ctx: &Ctx, state: &State) -> Result<Outcome, Error> {
    let (_, lands) = state.unverified_lands();
    let landed = |run: &Run| lands.contains(&run.head);
    import(ctx, state, state.chain.len() - 1, landed)
}

/// `clm verify --commit <sha>` in `import` mode: the run on that commit.
pub(crate) fn import_commit(ctx: &Ctx, state: &State, end: usize) -> Result<Outcome, Error> {
    let commit = &state.chain[end];
    import(ctx, state, end, |run: &Run| &run.head == commit)
}

/// Records the newest completed run, that `wanted` accepts, whose conclusion is `success` or
/// `failure`, as the verification of its commit; `pending` of the commit at `end` when there
/// is none.
fn import(
    ctx: &Ctx,
    state: &State,
    end: usize,
    wanted: impl Fn(&Run) -> bool,
) -> Result<Outcome, Error> {
    let Some(workflow) = ctx.verify.workflow.as_deref() else {
        return Err(Error::Refused(
            "`[verify]` in import mode names no workflow".to_string(),
        ));
    };
    let runs = clm_host::adapter(ctx.host)
        .runs(&ctx.root, workflow)
        .map_err(|e| match e {
            clm_host::Error::Refused(why)
            | clm_host::Error::Nowhere(why)
            | clm_host::Error::Git(why) => Error::Refused(why),
        })?;
    let result = runs.into_iter().find(|run| {
        run.completed && matches!(run.conclusion.as_str(), "success" | "failure") && wanted(run)
    });
    let Some(result) = result.filter(|r| state.index(&r.head).is_some()) else {
        let pending = ctx.short(&state.chain[end])?;
        return Ok(Outcome::new(format!("verify pending {pending}"), 4));
    };
    let at = state.index(&result.head).expect("a run on a land of main");
    let (_, covered) = state.unverified(at);
    // One covered land is the culprit of a red result; several leave the search to
    // `clm verify --bisect`.
    let passed = result.conclusion == "success";
    let culprit = match covered {
        [only] if !passed => Some(only.clone()),
        _ => None,
    };
    run::record(ctx, state, at, culprit, passed, Some(result.url))
}
