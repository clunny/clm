//! Cutting a release (`release-guard`, `humans-cut-releases`): dispatching the release
//! workflow, writing a release, and re-running a release run.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::gh;
use crate::shell::command::Command;

/// Whether a segment with `tokens` dispatches the release workflow or writes a release
/// with `gh`.
pub(crate) fn cuts_release(tokens: &[String]) -> bool {
    let Some(args) = gh::args(tokens) else {
        return false;
    };
    let words = gh::words(args);
    match (words.first().copied(), words.get(1).copied()) {
        // The workflow argument is the first word after `workflow run`. The first word
        // after the values of the flags `gh workflow run` takes is judged too, so a value
        // before the workflow (`--ref main release.yml`) doesn't hide it.
        (Some("workflow"), Some("run")) => {
            let run = args.iter().position(|t| t == "run").unwrap_or(0);
            let takes_value = |t: &str| {
                matches!(
                    t,
                    "-R" | "--repo" | "-r" | "--ref" | "-f" | "-F" | "--field" | "--raw-field"
                )
            };
            let past_values = (run + 1..args.len())
                .find(|&i| !args[i].starts_with('-') && !takes_value(&args[i - 1]))
                .map(|i| args[i].as_str());
            [words.get(2).copied(), past_values]
                .into_iter()
                .flatten()
                .any(names_release)
        }
        (Some("release"), Some(sub)) => {
            ["create", "edit", "upload", "delete", "delete-asset"].contains(&sub)
        }
        (Some("api"), _) => {
            args.iter()
                .any(|t| t.contains("/dispatches") || t.contains("/releases"))
                && gh::api_writes(&args[1..])
        }
        _ => false,
    }
}

/// Whether a workflow argument names the release or the site workflow, as gh matches
/// workflows: in any case, by name (`release`, `site`) or by file (`release.yml`,
/// `release.yaml`, `site.yml`, `site.yaml`, or a path ending in one).
fn names_release(workflow: &str) -> bool {
    release_name(workflow) || release_file(workflow)
}

/// Whether a workflow name is `release` or `site`, in any case.
fn release_name(name: &str) -> bool {
    name.eq_ignore_ascii_case("release") || name.eq_ignore_ascii_case("site")
}

/// Whether a path's file name is `release.yml`, `release.yaml`, `site.yml` or
/// `site.yaml`, in any case.
fn release_file(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);
    ["release", "site"].iter().any(|stem| {
        [".yml", ".yaml"]
            .iter()
            .any(|ext| name.eq_ignore_ascii_case(&format!("{stem}{ext}")))
    })
}

/// What one command segment's `gh run rerun` re-runs, and where `gh` would look it up.
pub(crate) struct Rerun {
    /// The run ID (`id_word`).
    run: Option<String>,
    /// The values of `-j`/`--job` that are IDs (`id_word`).
    jobs: Vec<String>,
    /// The repository the lookups ask.
    repo: Repo,
    /// The segment's own `GH_REPO=` and `GH_HOST=` assignments.
    env: Vec<(&'static str, String)>,
    /// The checkout the segment runs in.
    dir: PathBuf,
}

/// The repository a re-run's lookups ask.
enum Repo {
    /// None named: `gh repo view` resolves it in the re-run's checkout.
    Checkout,
    /// `-R`/`--repo` `[<host>/]<owner>/<repo>`.
    Named { host: Option<String>, path: String },
    /// A `-R`/`--repo` value that isn't of that form: its lookups fail.
    Unreadable,
}

/// A word that names a run or job: all digits, or a shell expansion (`$RUN`, a backtick)
/// whose value the guard can't know, so its lookup fails. Any other word is prose.
fn id_word(word: &str) -> Option<String> {
    let digits = !word.is_empty() && word.bytes().all(|b| b.is_ascii_digit());
    (digits || word.contains(['$', '`'])).then(|| word.to_string())
}

/// The re-run of a segment with `tokens` and command `command`, run from `checkout`, when
/// it runs `gh run rerun` with a run or job ID.
pub fn rerun(tokens: &[String], command: &Command, checkout: Option<&Path>) -> Option<Rerun> {
    let args = gh::args(tokens)?;
    let words = gh::words(args);
    if words.get(..2) != Some(&["run", "rerun"][..]) {
        return None;
    }
    let run = words.get(2).and_then(|w| id_word(w));
    // A flag's values, alone or glued on (`--job=1`, `-j1`).
    let flag = |long: &str, short: &str| -> Vec<String> {
        args.iter()
            .enumerate()
            .filter_map(|(i, t)| {
                if t == long || t == short {
                    return args.get(i + 1).cloned();
                }
                t.strip_prefix(long)
                    .and_then(|v| v.strip_prefix('='))
                    .or_else(|| t.strip_prefix(short).filter(|v| !v.is_empty()))
                    .map(String::from)
            })
            .collect()
    };
    let jobs: Vec<String> = flag("--job", "-j")
        .iter()
        .filter_map(|j| id_word(j))
        .collect();
    if run.is_none() && jobs.is_empty() {
        return None;
    }
    // `-R` wins over `GH_REPO`, and the last `-R` over the others, as for `gh`.
    let repo = match flag("--repo", "-R").last() {
        Some(value) => repo_value(value),
        None => Repo::Checkout,
    };
    let env = ["GH_REPO", "GH_HOST"]
        .into_iter()
        .filter_map(|name| command.assigned(name).map(|v| (name, v.to_string())))
        .collect();
    let dir = checkout.unwrap_or(Path::new(".")).to_path_buf();
    Some(Rerun {
        run,
        jobs,
        repo,
        env,
        dir,
    })
}

/// A `-R`/`--repo` value `[<host>/]<owner>/<repo>`.
fn repo_value(value: &str) -> Repo {
    let parts: Vec<&str> = value.split('/').collect();
    let named = |host: Option<&str>, owner, repo| Repo::Named {
        host: host.map(String::from),
        path: format!("{owner}/{repo}"),
    };
    match parts[..] {
        _ if parts.iter().any(|p| p.is_empty()) => Repo::Unreadable,
        [owner, repo] => named(None, owner, repo),
        [host, owner, repo] => named(Some(host), owner, repo),
        _ => Repo::Unreadable,
    }
}

/// The time the guard's lookups share, under Claude Code's default 60-second hook
/// timeout: a hook that times out is treated as allowing.
const LOOKUP_BUDGET: Duration = Duration::from_secs(30);

/// Whether any of `reruns` re-runs a release run or one that can't be looked up, within
/// `LOOKUP_BUDGET`, stopping at the first.
pub(crate) fn reruns_release(reruns: &[Rerun]) -> bool {
    let deadline = Instant::now() + LOOKUP_BUDGET;
    reruns.iter().any(|rerun| rerun.releases(deadline))
}

impl Rerun {
    /// Whether a run this re-runs is a release run or can't be looked up by `deadline`.
    fn releases(&self, deadline: Instant) -> bool {
        // Resolved once, and only when a lookup needs it.
        let repo = std::cell::OnceCell::new();
        let lookup = |kind: &str, id: &str| {
            // A shell expansion's value is unknown, so it can't be looked up.
            if !id.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            let (host, path) = repo.get_or_init(|| self.repository(deadline)).as_ref()?;
            let mut args = vec!["api".to_string()];
            args.extend(
                host.iter()
                    .flat_map(|h| ["--hostname".to_string(), h.clone()]),
            );
            args.push(format!("repos/{path}/actions/{kind}/{id}"));
            serde_json::from_slice::<Value>(&self.gh(args, deadline)?).ok()
        };
        let run_releases = |id: &str| lookup("runs", id).is_none_or(|run| release_run(&run));
        self.run.as_deref().is_some_and(run_releases)
            || self.jobs.iter().any(|id| {
                let run_id = lookup("jobs", id).and_then(|job| job["run_id"].as_u64());
                run_id
                    .filter(|&n| n > 0)
                    .is_none_or(|n| run_releases(&n.to_string()))
            })
    }

    /// The repository the lookups ask, as `([<host>], <owner>/<repo>)`: `-R`'s, or the one
    /// `gh repo view --json url` reports, which resolves it as `gh run rerun` does; `None`
    /// when it can't be read.
    fn repository(&self, deadline: Instant) -> Option<(Option<String>, String)> {
        match &self.repo {
            Repo::Named { host, path } => Some((host.clone(), path.clone())),
            Repo::Unreadable => None,
            Repo::Checkout => {
                let args = ["repo", "view", "--json", "url"].map(String::from).to_vec();
                let view: Value = serde_json::from_slice(&self.gh(args, deadline)?).ok()?;
                let (host, path) = repo_from_url(view["url"].as_str()?)?;
                Some((Some(host), path))
            }
        }
    }

    /// One `gh` call's stdout, run in the re-run's checkout with its environment; `None`
    /// when it fails or outlasts `deadline` (`bounded-runs`). A call still running then is
    /// killed, and once `deadline` has passed no call is started.
    fn gh(&self, args: Vec<String>, deadline: Instant) -> Option<Vec<u8>> {
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        let env: Vec<(&str, &OsStr)> = self.env.iter().map(|(n, v)| (*n, OsStr::new(v))).collect();
        let options = clm_git::Options {
            stdin: None,
            env: &env,
        };
        let limit = deadline.saturating_duration_since(Instant::now());
        clm_git::run_within(&self.dir, "gh", &args, options, limit).ok()
    }
}

/// The host and `<owner>/<repo>` of a repository URL: the part after `://`, in exactly three
/// non-empty parts.
fn repo_from_url(url: &str) -> Option<(String, String)> {
    let (_, rest) = url.split_once("://")?;
    match rest.split('/').collect::<Vec<_>>()[..] {
        [host, owner, repo] if [host, owner, repo].iter().all(|p| !p.is_empty()) => {
            Some((host.to_string(), format!("{owner}/{repo}")))
        }
        _ => None,
    }
}

/// Whether a looked-up run is a release run (`release-run-lookup`): its workflow file, cut
/// at any `@`, is `release.yml`, `release.yaml`, `site.yml` or `site.yaml`, or its workflow
/// is named `release` or `site`, in any case. A run without a string `path` and `name` may
/// be one.
fn release_run(run: &Value) -> bool {
    let (Some(path), Some(name)) = (run["path"].as_str(), run["name"].as_str()) else {
        return true;
    };
    release_file(path.split('@').next().unwrap_or(path)) || release_name(name)
}
