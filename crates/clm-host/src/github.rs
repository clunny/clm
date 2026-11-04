//! `host = "github"`: pull requests and issues through `gh`.

use std::path::Path;

use clm_git::{REMOTE, current_branch, gh, git};
use serde_json::Value;

use crate::{Error, Host, Shown};

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

fn json(what: &str, text: &str) -> Result<Value, Error> {
    serde_json::from_str(text).map_err(|e| Error::Git(format!("{what}: bad JSON: {e}")))
}

fn text(v: &Value) -> String {
    v.as_str().unwrap_or_default().to_string()
}

/// `gh repo view`'s answer for [`REMOTE`]'s repository.
fn repo_view(root: &Path) -> Result<Value, Error> {
    let out = gh(
        root,
        &["repo", "view", "--json", "isFork,parent,owner,name"],
    )?;
    json("gh repo view", &out)
}

/// `<owner>/<name>` of a repository in a `gh repo view` answer.
fn slug(repo: &Value) -> Option<String> {
    let owner = repo["owner"]["login"].as_str()?;
    Some(format!("{owner}/{}", repo["name"].as_str()?))
}

/// The parent of the repository `view` describes; `None` when it isn't a fork.
fn parent_of(view: &Value) -> Option<String> {
    if view["isFork"].as_bool() != Some(true) {
        return None;
    }
    slug(&view["parent"])
}

fn not_a_fork() -> Error {
    Error::Refused("the remote is not a fork".into())
}

/// The owner of [`REMOTE`]: from `gh repo view`, else from the remote's URL.
fn owner(root: &Path, view: &Value) -> Result<String, Error> {
    if let Some(login) = view["owner"]["login"].as_str() {
        return Ok(login.to_string());
    }
    let url = git(root, &["remote", "get-url", REMOTE])?;
    let path = url.trim_end_matches(".git");
    let path = path.rsplit_once(':').map_or(path, |(_, p)| p);
    path.rsplit('/')
        .nth(1)
        .map(String::from)
        .ok_or_else(|| Error::Git(format!("no owner in the remote `{url}`")))
}

const PR_FIELDS: &str = "author,state,title,body,headRepository,headRepositoryOwner,headRefName,maintainerCanModify,files";

/// A pull request or issue from `gh pr view` or `gh issue view`.
fn shown(v: &Value, pull_request: bool) -> Shown {
    let head_repo_url = match (
        v["headRepositoryOwner"]["login"].as_str(),
        v["headRepository"]["name"].as_str(),
    ) {
        (Some(o), Some(r)) => Some(format!("https://github.com/{o}/{r}")),
        _ => None,
    };
    Shown {
        pull_request,
        author: text(&v["author"]["login"]),
        state: text(&v["state"]),
        title: text(&v["title"]),
        body: text(&v["body"]),
        head_repo_url,
        head_branch: v["headRefName"].as_str().map(String::from),
        maintainers_can_edit: v["maintainerCanModify"].as_bool().unwrap_or(false),
        files: v["files"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|f| f["path"].as_str().map(String::from))
            .collect(),
    }
}

impl Host for GitHub {
    fn parent(&self, root: &Path) -> Result<Option<String>, Error> {
        Ok(parent_of(&repo_view(root)?))
    }

    fn sync(&self, root: &Path) -> Result<(), Error> {
        let view = repo_view(root)?;
        parent_of(&view).ok_or_else(not_a_fork)?;
        let fork = slug(&view).ok_or_else(|| Error::Git("gh repo view: no owner".into()))?;
        gh(root, &["repo", "sync", &fork, "--branch", "main"])?;
        Ok(())
    }

    fn open_on_parent(
        &self,
        root: &Path,
        branch: &str,
        title: &str,
        body_file: &Path,
    ) -> Result<String, Error> {
        let view = repo_view(root)?;
        let parent = parent_of(&view).ok_or_else(not_a_fork)?;
        let head = format!("{}:{branch}", owner(root, &view)?);
        let listed = [
            "pr", "list", "--repo", &parent, "--head", &head, "--state", "open", "--json", "url",
        ];
        if let Some(url) = json("gh pr list", &gh(root, &listed)?)?[0]["url"].as_str() {
            return Ok(url.to_string());
        }
        let body = body_file.to_string_lossy();
        let create = [
            "pr",
            "create",
            "--repo",
            &parent,
            "--base",
            "main",
            "--head",
            &head,
            "--title",
            title,
            "--body-file",
            &body,
        ];
        Ok(last_line(&gh(root, &create)?))
    }

    fn show(&self, root: &Path, number: u64) -> Result<Shown, Error> {
        let n = number.to_string();
        match gh(root, &["pr", "view", &n, "--json", PR_FIELDS]) {
            Ok(out) => Ok(shown(&json("gh pr view", &out)?, true)),
            // No pull request has that number: it may be an issue.
            Err(clm_git::Error::Failed { .. }) => {
                let fields = "author,state,title,body";
                let out = gh(root, &["issue", "view", &n, "--json", fields])?;
                Ok(shown(&json("gh issue view", &out)?, false))
            }
            Err(e) => Err(e.into()),
        }
    }

    fn fetch(&self, root: &Path, number: u64, branch: &str) -> Result<(), Error> {
        let refname = format!("refs/heads/{branch}");
        if git(root, &["show-ref", "--verify", "--quiet", &refname]).is_ok() {
            return Err(Error::Refused(format!("branch `{branch}` already exists")));
        }
        let spec = format!("pull/{number}/head:{refname}");
        git(root, &["fetch", REMOTE, &spec])?;
        Ok(())
    }

    fn comment(
        &self,
        root: &Path,
        number: u64,
        body_file: &Path,
        close: bool,
    ) -> Result<(), Error> {
        let n = number.to_string();
        let body = body_file.to_string_lossy();
        // `gh issue` takes a pull request's number too.
        gh(root, &["issue", "comment", &n, "--body-file", &body])?;
        if close {
            gh(root, &["issue", "close", &n])?;
        }
        Ok(())
    }

    fn label(&self, root: &Path, number: u64, label: &str) -> Result<(), Error> {
        let n = number.to_string();
        gh(root, &["issue", "edit", &n, "--add-label", label])?;
        Ok(())
    }

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
