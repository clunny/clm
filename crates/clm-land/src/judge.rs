//! Judging a head before the lock, one call for a land alone and for each land taken with
//! others, so the two paths can't drift (`land/spec-branch-holds-spec-paths`,
//! `land/build-holds-own-files`, `land/deleted-tests-approved`, `land/verify-stop`).

use clm_ledger::Fold;

use crate::{Error, Repo};

/// Judges `branch`'s `head` against the fetched `base`: the claim its name makes, the citing
/// tests it loses and the components whose slow tests broke. Returns the warnings the
/// verify-stop check gives. The gate runs in-process once, before the lock, for the kind the
/// `spec/<id>` claim is checked against; its violations are the gate process's to report.
pub(crate) fn judge_head(
    repo: &Repo,
    branch: &str,
    base: &str,
    head: &str,
    fold: &Fold,
) -> Result<Vec<String>, Error> {
    let outcome = clm_gate::evaluate_at(&repo.root, base, head, false).map_err(Error::Git)?;
    // A build holds its own files (`land/build-holds-own-files`).
    crate::own_files::check(repo, branch, base, head)?;
    // A spec branch holds spec paths only, whatever its gate's process says
    // (`land/spec-branch-holds-spec-paths`).
    if branch.starts_with("spec/") && outcome.kind != clm_gate::Kind::Spec {
        let files =
            clm_gate::files_outside_spec_paths(&repo.root, base, head).map_err(Error::Git)?;
        return Err(Error::Refused(format!(
            "a spec branch holds only spec paths; {branch} changes {}",
            files.join(", ")
        )));
    }
    // No citing test lost that no approved list names, whatever the name
    // (`land/deleted-tests-approved`).
    crate::deleted_tests::check(repo, base, head)?;
    // A component whose slow tests broke after a land waits for its fix; the components of a
    // red result nobody took in yet only warn (`land/verify-stop`).
    crate::verify_stop::check(repo, base, head, fold)
}
