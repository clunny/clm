//! A reviewer's request for changes holds a branch (`changes-requested-holds`,
//! `web-review-feedback-as-ledger-records`): while a `changes_requested` record for the
//! branch names its fingerprint and its writer's person hasn't approved that fingerprint
//! since, `clm land` refuses it, printing what the reviewers asked.

use clm_ledger::{Body, Fold};

/// The refusal for `branch` at `fingerprint` when a request for changes holds it: one
/// `land/changes-requested` line per holding request, one `land/comment` line per comment
/// at the fingerprint, and what clears it. None when nothing holds it.
pub fn held(fold: &Fold, branch: &str, fingerprint: &str) -> Option<String> {
    // Under a heading of their own, as the gate's violations print in a refusal, so each
    // violation line starts its line in `check`'s format.
    let mut lines = vec!["a request for changes holds the land:".to_string()];
    for request in fold.change_requests.get(branch).into_iter().flatten() {
        let Body::ChangesRequested {
            fingerprint: at_fp,
            text,
            ..
        } = &request.body
        else {
            continue;
        };
        // Whether the requester cleared it is ledger's rule, the one the web app shows too.
        if at_fp == fingerprint && !fold.cleared(request) {
            lines.push(format!(
                "{branch}: land/changes-requested: {} at {}: {}",
                request.writer,
                request.at,
                flat(text)
            ));
        }
    }
    if lines.len() == 1 {
        return None;
    }
    for comment in fold.comments.get(branch).into_iter().flatten() {
        if let Body::Commented {
            fingerprint: at_fp,
            target,
            text,
            ..
        } = &comment.body
            && at_fp == fingerprint
        {
            lines.push(format!(
                "{branch}: land/comment: {} on {} at {}: {}",
                comment.writer,
                target.as_deref().unwrap_or("the change"),
                comment.at,
                flat(text)
            ));
        }
    }
    lines.push(format!(
        "a reviewer asked for changes to this diff ({fingerprint}): push a change to \
         `{branch}`, or have the requester run `clm approve {branch}`"
    ));
    Some(lines.join("\n"))
}

/// `text` trimmed, each run of whitespace that holds a line break made one space.
fn flat(text: &str) -> String {
    let mut out = String::new();
    let mut run = String::new();
    for c in text.trim().chars() {
        if c.is_whitespace() {
            run.push(c);
            continue;
        }
        if run.contains(['\n', '\r']) {
            out.push(' ');
        } else {
            out.push_str(&run);
        }
        run.clear();
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_flattened() {
        // req:land.approval/changes-requested-holds@27e6c16b
        assert_eq!(flat("  a\n  b\r\n\tc  d\t e \n"), "a b c  d\t e");
        assert_eq!(flat("one"), "one");
    }
}
