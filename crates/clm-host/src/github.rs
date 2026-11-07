//! `host = "github"`: pull requests and issues through `gh`.

use std::path::Path;

use clm_git::{REMOTE, current_branch, gh, git};
use serde_json::Value;

use crate::{Alert, Error, Host, Listed, Run, Shown};

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

/// The owner of the repository `view` describes.
fn owner(view: &Value) -> Result<String, Error> {
    view["owner"]["login"]
        .as_str()
        .map(String::from)
        .ok_or_else(|| Error::Git("gh repo view: no owner".into()))
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
        author_id: 0,
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

/// `shown` with its author's numeric account ID, from `gh api users/<login>`.
fn with_author_id(root: &Path, mut shown: Shown) -> Result<Shown, Error> {
    let user = gh(root, &["api", &format!("users/{}", shown.author)])?;
    shown.author_id = json("gh api users", &user)?["id"]
        .as_u64()
        .ok_or_else(|| Error::Git("gh api users: no id".into()))?;
    Ok(shown)
}

/// The open items `kind` (`issue` or `pr`) lists, each with its number and author's login;
/// a pull request counts only when `gh` reports its head in another repository.
fn listed(root: &Path, kind: &str) -> Result<Vec<Listed>, Error> {
    let args = [
        kind,
        "list",
        "--state",
        "open",
        "--json",
        if kind == "pr" {
            "number,author,isCrossRepository"
        } else {
            "number,author"
        },
        "--limit",
        "1000",
    ];
    let items = json(&format!("gh {kind} list"), &gh(root, &args)?)?;
    Ok(items
        .as_array()
        .into_iter()
        .flatten()
        .filter(|item| kind != "pr" || item["isCrossRepository"].as_bool() == Some(true))
        .filter_map(|item| {
            Some(Listed {
                number: item["number"].as_u64()?,
                pull_request: kind == "pr",
                author: text(&item["author"]["login"]),
            })
        })
        .collect())
}

/// A field of an alert as one word: whitespace and control characters become `?`, and a
/// missing or empty field is `unknown`.
fn word(v: &Value) -> String {
    match v.as_str() {
        Some(s) if !s.is_empty() => s
            .chars()
            .map(|c| {
                if c.is_whitespace() || c.is_control() {
                    '?'
                } else {
                    c
                }
            })
            .collect(),
        _ => "unknown".into(),
    }
}

fn alert(v: &Value) -> Option<Alert> {
    let severity = match v["security_advisory"]["severity"].as_str() {
        Some(s @ ("low" | "medium" | "high" | "critical")) => s.to_string(),
        _ => "unknown".to_string(),
    };
    Some(Alert {
        number: v["number"].as_u64()?,
        severity,
        ecosystem: word(&v["dependency"]["package"]["ecosystem"]),
        package: word(&v["dependency"]["package"]["name"]),
        manifest: word(&v["dependency"]["manifest_path"]),
    })
}

/// A run's conclusion as one word: lower-case letters and `_`, else `unknown`.
fn conclusion(v: &Value) -> String {
    match v.as_str() {
        Some(s) if !s.is_empty() && s.chars().all(|c| c.is_ascii_lowercase() || c == '_') => {
            s.to_string()
        }
        _ => "unknown".into(),
    }
}

/// A run from `gh run list`; `None` when its head isn't a full commit hash.
fn run(v: &Value) -> Option<Run> {
    let head = v["headSha"].as_str()?;
    if head.len() != 40 || !head.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    Some(Run {
        id: v["databaseId"].as_u64()?,
        head: head.to_string(),
        completed: v["status"].as_str() == Some("completed"),
        conclusion: conclusion(&v["conclusion"]),
        url: text(&v["url"]),
    })
}

impl Host for GitHub {
    fn runs(&self, root: &Path, workflow: &str) -> Result<Vec<Run>, Error> {
        let args = [
            "run",
            "list",
            "--workflow",
            workflow,
            "--branch",
            "main",
            "--event",
            "push",
            "--limit",
            "100",
            "--json",
            "databaseId,headSha,status,conclusion,url",
        ];
        let items = json("gh run list", &gh(root, &args)?)?;
        Ok(items
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(run)
            .collect())
    }

    fn alerts(&self, root: &Path) -> Result<Vec<Alert>, Error> {
        let args = [
            "api",
            "--paginate",
            "-X",
            "GET",
            "repos/{owner}/{repo}/dependabot/alerts",
            "-f",
            "state=open",
        ];
        let out = gh(root, &args)?;
        // `--paginate` prints each page's array in turn, with nothing between them.
        let mut alerts = Vec::new();
        for page in serde_json::Deserializer::from_str(&out).into_iter::<Value>() {
            let page = page.map_err(|e| Error::Git(format!("gh api alerts: bad JSON: {e}")))?;
            let items = page
                .as_array()
                .ok_or_else(|| Error::Git("gh api alerts: a page is not a list".into()))?;
            for item in items {
                alerts.push(
                    alert(item).ok_or_else(|| {
                        Error::Git("gh api alerts: an alert has no number".into())
                    })?,
                );
            }
        }
        Ok(alerts)
    }

    fn list(&self, root: &Path) -> Result<Vec<Listed>, Error> {
        let mut all = listed(root, "issue")?;
        all.extend(listed(root, "pr")?);
        Ok(all)
    }

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
        let head = format!("{}:{branch}", owner(&view)?);
        // `--head` takes the bare branch name, so the fork's owner is matched on the result.
        let listed = [
            "pr",
            "list",
            "--repo",
            &parent,
            "--head",
            branch,
            "--state",
            "open",
            "--json",
            "url,headRepositoryOwner",
        ];
        let open = json("gh pr list", &gh(root, &listed)?)?;
        let fork_owner = owner(&view)?;
        let mine =
            open.as_array().into_iter().flatten().find(|pr| {
                pr["headRepositoryOwner"]["login"].as_str() == Some(fork_owner.as_str())
            });
        if let Some(url) = mine.and_then(|pr| pr["url"].as_str()) {
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
            Ok(out) => with_author_id(root, shown(&json("gh pr view", &out)?, true)),
            // No pull request has that number: it may be an issue.
            Err(clm_git::Error::Failed { stderr, .. }) if stderr.contains("Could not resolve") => {
                let fields = "author,state,title,body";
                let out = gh(root, &["issue", "view", &n, "--json", fields])?;
                with_author_id(root, shown(&json("gh issue view", &out)?, false))
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
