//! `host = "github"`: pull requests and issues through `gh`.

use std::path::Path;

use clm_git::{current_branch, gh};
use serde_json::Value;

use crate::{Error, Host};

pub(crate) struct GitHub;

/// The open PR whose head is `branch`, from `gh pr view`: no PR, or one that is merged,
/// closed or another branch's, is `None`.
fn open_pr(root: &Path, branch: &str) -> Result<Option<Value>, Error> {
    let args = [
        "pr",
        "view",
        branch,
        "--json",
        "number,url,state,headRefName",
    ];
    let view = match gh(root, &args) {
        Ok(view) => view,
        // `gh pr view` fails when the branch has no PR at all: that's no page, not a failure.
        Err(clm_git::Error::Failed { stderr, .. }) if stderr.contains("no pull requests found") => {
            return Ok(None);
        }
        Err(e) => return Err(e.into()),
    };
    let pr: Value = serde_json::from_str(&view)
        .map_err(|e| Error::Git(format!("gh pr view: bad JSON: {e}")))?;
    let open = pr["state"].as_str() == Some("OPEN") && pr["headRefName"].as_str() == Some(branch);
    Ok(open.then_some(pr))
}

/// The last line of a command's output: where `gh` prints the URL it made.
fn last_line(out: &str) -> String {
    out.lines().last().unwrap_or_default().trim().to_string()
}

impl Host for GitHub {
    fn page_open(
        &self,
        root: &Path,
        branch: &str,
        title: &str,
        body_file: &Path,
    ) -> Result<Option<String>, Error> {
        if let Some(url) =
            open_pr(root, branch)?.and_then(|pr| pr["url"].as_str().map(String::from))
        {
            return Ok(Some(url));
        }
        let body = body_file.to_string_lossy();
        let out = gh(
            root,
            &[
                "pr",
                "create",
                "--base",
                "main",
                "--head",
                branch,
                "--title",
                title,
                "--body-file",
                &body,
            ],
        )?;
        Ok(Some(last_line(&out)))
    }

    fn page_body(&self, root: &Path, file: &Path) -> Result<Option<String>, Error> {
        crate::check_body(root, file)?;
        let branch = current_branch(root)?.ok_or_else(|| {
            Error::Refused("`HEAD` is detached; check out the page's branch".into())
        })?;
        let number = open_pr(root, &branch)?.and_then(|pr| pr["number"].as_u64());
        let Some(number) = number else {
            return Err(Error::Refused(format!(
                "`{branch}` has no open PR whose head it is"
            )));
        };
        gh(
            root,
            &[
                "api",
                "-X",
                "PATCH",
                &format!("repos/{{owner}}/{{repo}}/pulls/{number}"),
                "-F",
                &format!("body=@{}", file.display()),
            ],
        )?;
        Ok(Some(format!("updated #{number}")))
    }

    fn issue(
        &self,
        root: &Path,
        title: &str,
        body_file: &Path,
        repo: &str,
    ) -> Result<String, Error> {
        let body = body_file.to_string_lossy();
        let args = [
            "issue",
            "create",
            "--repo",
            repo,
            "--title",
            title,
            "--body-file",
            &body,
        ];
        Ok(last_line(&gh(root, &args)?))
    }
}
