//! The Azure DevOps adapter: pull requests through the Azure CLI (`azure-through-az-cli`).
//! `az` holds the sign-in; every other operation is the trait's default, as on `git`.

use std::path::{Path, PathBuf};

use clm_git::{Options, current_branch, run};
use serde_json::Value;

use crate::{Error, Host, check_body, nowhere_to_file};

/// `host = "azure"`: pull requests through `az repos pr`.
pub struct Azure;

/// Runs `az repos pr <args>` with the clone's remote detected and JSON output, and parses
/// what it prints.
fn pr(root: &Path, args: &[&str]) -> Result<Value, Error> {
    let out = pr_raw(root, args)?;
    serde_json::from_slice(&out).map_err(|e| Error::Git(format!("az repos pr: bad JSON: {e}")))
}

/// What `az repos pr <args>` prints, unparsed.
fn pr_raw(root: &Path, args: &[&str]) -> Result<Vec<u8>, Error> {
    let mut all = vec!["repos", "pr"];
    all.extend(args);
    all.extend(["--detect", "true", "--output", "json"]);
    let out = run(
        root,
        "az",
        &all,
        Options {
            stdin: None,
            env: &[],
        },
    )
    // A failed `az` (not installed, not signed in) is a refusal with its message: exit 1.
    .map_err(|e| Error::Refused(e.to_string()))?;
    Ok(out)
}

/// The active pull request whose source is `branch`, as `az` reports it.
fn active(root: &Path, branch: &str) -> Result<Option<Value>, Error> {
    let listed = pr(
        root,
        &[
            "list",
            "--source-branch",
            branch,
            "--target-branch",
            "main",
            "--status",
            "active",
        ],
    )?;
    Ok(listed.as_array().and_then(|prs| prs.first()).cloned())
}

/// A pull request's web page, `<repository web URL>/pullrequest/<id>`: the `url` field of
/// `az`'s answer is the REST API's.
fn web_url(pr: &Value) -> Result<String, Error> {
    match (
        pr["repository"]["webUrl"].as_str(),
        pr["pullRequestId"].as_u64(),
    ) {
        (Some(web), Some(id)) => Ok(format!("{web}/pullrequest/{id}")),
        _ => Err(Error::Git(
            "az repos pr: no pullRequestId or repository.webUrl in the answer".into(),
        )),
    }
}

/// What `az repos pr create` answered, its whitespace runs joined into single spaces and cut
/// to its first 500 characters.
fn said(answer: &[u8]) -> String {
    let text = String::from_utf8_lossy(answer);
    let joined = text.split_whitespace().collect::<Vec<_>>().join(" ");
    joined.chars().take(500).collect()
}

/// `file` made absolute against the caller's directory: `az` runs in the repository's root
/// and reads `@<file>` relative to that.
fn absolute(file: &Path) -> Result<PathBuf, Error> {
    std::path::absolute(file).map_err(|e| Error::Refused(format!("{}: {e}", file.display())))
}

impl Host for Azure {
    fn page_open(
        &self,
        root: &Path,
        branch: &str,
        title: &str,
        body_file: &Path,
    ) -> Result<Option<String>, Error> {
        if let Some(open) = active(root, branch)? {
            return web_url(&open).map(Some);
        }
        let description = format!("@{}", absolute(body_file)?.display());
        let created = pr_raw(
            root,
            &[
                "create",
                "--source-branch",
                branch,
                "--target-branch",
                "main",
                "--title",
                title,
                "--description",
                &description,
            ],
        )?;
        let answer = serde_json::from_slice(&created)
            .map_err(|e| Error::Git(format!("az repos pr: bad JSON: {e}")))?;
        // `create` may have opened the pull request without saying so readably: look again.
        match web_url(&answer) {
            Ok(url) => Ok(Some(url)),
            Err(_) => match active(root, branch)?.as_ref().map(web_url) {
                Some(Ok(url)) => Ok(Some(url)),
                _ => Err(Error::Git(format!(
                    "az repos pr create: no pullRequestId or repository.webUrl in the answer: {}",
                    said(&created)
                ))),
            },
        }
    }

    fn page_body(&self, root: &Path, file: &Path) -> Result<Option<String>, Error> {
        let file = absolute(file)?;
        check_body(root, &file)?;
        let branch = current_branch(root)?.ok_or_else(|| {
            Error::Refused("`HEAD` is detached; check out the page's branch".into())
        })?;
        let id = active(root, &branch)?
            .and_then(|open| open["pullRequestId"].as_u64())
            .ok_or_else(|| Error::Refused(format!("`{branch}` has no active pull request")))?;
        let id = id.to_string();
        let description = format!("@{}", file.display());
        pr(
            root,
            &["update", "--id", &id, "--description", &description],
        )?;
        Ok(Some(format!("updated #{id}")))
    }

    fn issue(&self, root: &Path, _: &str, body_file: &Path, _: &str) -> Result<String, Error> {
        nowhere_to_file(root, body_file)
    }
}
