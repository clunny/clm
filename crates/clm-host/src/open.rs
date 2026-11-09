//! `pr_open`: the optional branch, title and body of `clm pr open`, filled in from the
//! branch (`pr-open-defaults`).

use std::path::{Path, PathBuf};

use clm_git::{current_branch, git};

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
    match body_file {
        Some(file) => adapter(host).page_open(root, &branch, &title, file),
        None => {
            let body = drawn.map(|(_, body)| body).unwrap_or_default();
            let file = TempBody::write(&body)?;
            adapter(host).page_open(root, &branch, &title, &file.0)
        }
    }
}

/// A body written to a temporary file outside the worktree, removed on drop.
struct TempBody(PathBuf);

impl TempBody {
    fn write(body: &str) -> Result<Self, Error> {
        let path = std::env::temp_dir().join(format!("clm-pr-body-{}.md", std::process::id()));
        let refuse = |e: std::io::Error| Error::Refused(format!("{}: {e}", path.display()));
        // `create_new` refuses a file or symlink already at the path.
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(refuse)?;
        let body = std::io::Write::write_all(&mut file, body.as_bytes());
        let guard = TempBody(path.clone());
        body.map_err(|e| Error::Refused(format!("{}: {e}", path.display())))?;
        Ok(guard)
    }
}

impl Drop for TempBody {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
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
    let range = format!("origin/main..{branch}");
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
