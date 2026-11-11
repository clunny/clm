//! `pr_open`: the optional branch, title and body of `clm pr open`, filled in from the
//! branch (`pr-open-defaults`).

use std::path::Path;

use clm_git::{REMOTE, current_branch, git};

use crate::keep::{TempFile, keep_body};
use crate::{Error, adapter};

/// Opens the pull request for `branch` (default: the checked-out one), titled `title` with
/// `body_file`'s contents, drawing whatever is missing from the branch, then asks the
/// host's adapter. `git` and `none` answer `None` first, whatever the arguments.
pub fn pr_open(
    host: clm_config::Host,
    root: &Path,
    branch: Option<&str>,
    title: Option<&str>,
    body_file: Option<&Path>,
) -> Result<Option<String>, Error> {
    if matches!(host, clm_config::Host::Git | clm_config::Host::None) {
        return Ok(None);
    }
    let branch = match branch {
        Some(branch) => branch.to_string(),
        None => current_branch(root)?
            .ok_or_else(|| Error::Refused("HEAD is detached; name the branch".into()))?,
    };
    let drawn = if title.is_none() || body_file.is_none() {
        Some(describe(root, &branch)?)
    } else {
        None
    };
    let title = title
        .map(String::from)
        .or_else(|| drawn.as_ref().map(|(title, _)| title.clone()))
        .unwrap_or_default();
    let (text, temp) = match body_file {
        Some(file) => (std::fs::read_to_string(root.join(file)).ok(), None),
        None => {
            let body = drawn.map(|(_, body)| body).unwrap_or_default();
            let file = TempFile::write(&body)?;
            (Some(body), Some(file))
        }
    };
    let file = match (&temp, body_file) {
        (Some(temp), _) => temp.0.as_path(),
        (None, Some(file)) => file,
        (None, None) => unreachable!("a body is either given or written"),
    };
    adapter(host)
        .page_open(root, &branch, &title, file)
        .map_err(|e| match text.filter(|t| !t.trim().is_empty()) {
            Some(text) => keeping(root, &branch, &text, e),
            None => e,
        })
}

/// `error`, ending `; the body is kept at <path>` once `text` is written there; unchanged
/// when it can't be written.
fn keeping(root: &Path, branch: &str, text: &str, error: Error) -> Error {
    let Ok(path) = keep_body(root, branch, text) else {
        return error;
    };
    let note = |why: String| format!("{why}; the body is kept at {}", path.display());
    match error {
        Error::Refused(why) => Error::Refused(note(why)),
        Error::Git(why) => Error::Git(note(why)),
        Error::Nowhere(report) => Error::Nowhere(report),
    }
}

/// The title and body drawn from `branch`: its change's `## Intent` on `spec/<id>`,
/// otherwise its commits' subjects.
fn describe(root: &Path, branch: &str) -> Result<(String, String), Error> {
    if let Some(id) = branch.strip_prefix("spec/")
        && let Ok(change) = git(root, &["show", &format!("{branch}:changes/{id}/change.md")])
        && let Some(intent) = intent(&change)
    {
        return Ok((first_sentence(&intent), intent));
    }
    let range = format!("{REMOTE}/main..{branch}");
    let log = git(root, &["log", "--reverse", "--format=%s", &range])
        .map_err(|e| Error::Refused(e.to_string()))?;
    let Some(oldest) = log.lines().next() else {
        return Err(Error::Refused(format!(
            "`{branch}` has no commit beyond origin/main and no change to describe"
        )));
    };
    let body: String = log.lines().map(|s| format!("- {s}\n")).collect();
    Ok((oldest.to_string(), body))
}

/// The text of the `## Intent` section, up to the next `## ` heading, trimmed; `None` when
/// it is missing or blank.
fn intent(change: &str) -> Option<String> {
    let mut lines = change.lines().skip_while(|l| l.trim_end() != "## Intent");
    lines.next()?;
    let text: Vec<&str> = lines.take_while(|l| !l.starts_with("## ")).collect();
    let text = text.join("\n").trim().to_string();
    (!text.is_empty()).then_some(text)
}

/// The first paragraph with its line breaks joined by spaces, up to and including the
/// first `.`, `?` or `!` followed by a space or the paragraph's end.
fn first_sentence(text: &str) -> String {
    let paragraph = text
        .split("\n\n")
        .next()
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .collect::<Vec<_>>()
        .join(" ");
    let mut chars = paragraph.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        if matches!(c, '.' | '?' | '!') && chars.peek().is_none_or(|&(_, next)| next == ' ') {
            return paragraph[..i + c.len_utf8()].to_string();
        }
    }
    paragraph
}
