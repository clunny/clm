//! The `## Cuts` section that starts the body of a spec branch that cuts anything
//! (`host/pr-open`, `host/pr-body`, `gate/cuts`).

use std::cell::RefCell;
use std::path::{Path, PathBuf};

use clm_git::REMOTE;

use crate::Error;
use crate::keep::TempFile;

/// What the last [`with_cuts`] left for `pr_open`, which keeps a body that didn't reach the
/// pull request whole (`keep-unopened-body`).
pub(crate) enum Sent {
    /// The body went out with its section: this is the text to keep.
    Body(String),
    /// The gate couldn't be evaluated, so nothing was sent and nothing is kept.
    Unevaluated,
}

thread_local! {
    static SENT: RefCell<Option<Sent>> = const { RefCell::new(None) };
}

fn leave(sent: Sent) {
    SENT.with(|cell| *cell.borrow_mut() = Some(sent));
}

/// The file to send as `branch`'s body: `file` itself, unless `gate`'s `evaluate_at` for
/// `branch` against `origin/main` gives kind `spec` and a cut, when it is a temporary file,
/// living as long as the returned guard, holding `## Cuts`, a `- <cut>` line per cut and a
/// blank line before `file`'s contents. Refuses when the gate can't be evaluated.
pub(crate) fn with_cuts(
    root: &Path,
    branch: &str,
    file: &Path,
) -> Result<(PathBuf, Option<TempFile>), Error> {
    let base = format!("{REMOTE}/main");
    let outcome = clm_gate::evaluate_at(root, &base, branch, false).map_err(|why| {
        leave(Sent::Unevaluated);
        Error::Refused(format!(
            "can't evaluate the gate for `{branch}` against {base}: {why}"
        ))
    })?;
    if outcome.kind != clm_gate::Kind::Spec || outcome.cuts.is_empty() {
        return Ok((file.to_path_buf(), None));
    }
    let body = std::fs::read_to_string(root.join(file))
        .or_else(|_| std::fs::read_to_string(file))
        .map_err(|e| Error::Refused(format!("{}: {e}", file.display())))?;
    let mut text = String::from("## Cuts\n");
    for cut in &outcome.cuts {
        text.push_str(&format!("- {cut}\n"));
    }
    text.push('\n');
    text.push_str(&body);
    let temp = TempFile::write(&text)?;
    leave(Sent::Body(text));
    Ok((temp.0.clone(), Some(temp)))
}

/// What the last [`with_cuts`] since the previous call left, taken.
pub(crate) fn take_sent() -> Option<Sent> {
    SENT.with(|sent| sent.borrow_mut().take())
}
