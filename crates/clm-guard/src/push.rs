//! Moving `main` (`push-guard`, `guard-blocks-push-to-main`): a `git push` that reaches
//! it, and the host's other routes to it through `gh`.

use std::cell::OnceCell;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::gh;
use crate::shell::command::Command;

/// How a `git push` reaches `main`.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Reach {
    /// It names `main`, or pushes every branch.
    Explicit,
    /// It names no branch, or only `HEAD`, from a checkout on `main`.
    OnMain,
    /// It names no branch, and git says the branch pushes to `main` (`@{push}`).
    PushesTo(String),
    /// It names no branch, or only `HEAD`, and a configured push refspec sends to `main`.
    Configured(String),
    /// It names no branch, or only `HEAD`, and git didn't answer within the budget.
    Unanswered,
}

impl Reach {
    /// For a push that names no branch, which test decided, and what to do instead. `lost`
    /// is the last `cd` that set the push's checkout when the guard couldn't follow it, as
    /// written: the checkout is then only the one the guard fell back to.
    pub fn why(&self, lost: Option<&str>) -> Option<String> {
        let (lead, checkout) = match lost {
            Some(cd) => (
                format!(
                    "the guard couldn't follow `{cd}`, so it judged the push from the checkout \
                     it fell back to, and "
                ),
                "that checkout",
            ),
            None => (String::new(), "the checkout it pushes from"),
        };
        let test = match self {
            Reach::Explicit => return None,
            Reach::Unanswered => "git didn't answer in time, so the guard couldn't tell \
                                  whether the push reaches `main`"
                .to_string(),
            Reach::OnMain => {
                format!("{lead}{checkout} is on `main`, so a push without a branch pushes `main`")
            }
            Reach::PushesTo(to) => {
                format!("{lead}the branch of {checkout} pushes to `main` (`@{{push}}` is `{to}`)")
            }
            Reach::Configured(refspec) => {
                format!("{lead}a configured push refspec sends to `main` (`{refspec}`)")
            }
        };
        Some(format!(
            "clm: {test}. Push your branch explicitly: `git push origin HEAD:<branch>`."
        ))
    }
}

/// Whether a refspec's destination reaches `main`: `main`, `heads/main`, `refs/heads/main`,
/// empty (`:` pushes every matching branch), or a wildcard that can match `main`. The
/// destination is the part after the last `:`, or the whole refspec, without a leading `+`.
fn reaches_main(refspec: &str) -> bool {
    let dst = refspec.rsplit(':').next().unwrap_or(refspec);
    let dst = dst.strip_prefix('+').unwrap_or(dst);
    matches!(dst, "main" | "heads/main" | "refs/heads/main" | "")
        || dst.split_once('*').is_some_and(|(pre, _)| {
            "refs/heads/main".starts_with(pre)
                || "heads/main".starts_with(pre)
                || "main".starts_with(pre)
        })
}

/// The time one command's push queries share, counted from its first: even after the
/// release lookups' 30 seconds, the guard answers before Claude Code's 60-second hook
/// timeout, which would let the command through (`bounded-runs`).
const PUSH_BUDGET: Duration = Duration::from_secs(20);

/// The push queries' shared budget, which starts with the first query.
#[derive(Default)]
pub(crate) struct Budget(OnceCell<Instant>);

impl Budget {
    /// What is left of the budget, starting it on the first call.
    fn remaining(&self) -> Duration {
        let deadline = *self.0.get_or_init(|| Instant::now() + PUSH_BUDGET);
        deadline.saturating_duration_since(Instant::now())
    }
}

/// How a segment whose command is `command`, run from `checkout`, reaches `main` with
/// `git push`, if it does, its queries spending `budget`; with the `cd` the guard couldn't
/// follow that set `checkout` (`lost`), unless the push's own absolute `-C` sets it outright.
pub fn reach<'a>(
    command: &Command,
    checkout: Option<&Path>,
    lost: Option<&'a str>,
    budget: &Budget,
) -> Option<(Reach, Option<&'a str>)> {
    if !command.is("git") {
        return None;
    }
    let args = command.args;
    // Global options up to the subcommand: `-C` moves the checkout, and the options that
    // decide where a push goes are passed to the queries.
    let mut dirs: Vec<&str> = Vec::new();
    let mut options: Vec<String> = Vec::new();
    let mut i = 0;
    while let Some(t) = args.get(i).map(String::as_str) {
        let value = args.get(i + 1).map(String::as_str);
        i += 1;
        match t {
            "-C" => {
                dirs.extend(value);
                i += 1;
            }
            "-c" => {
                if let Some(v) = value.filter(|v| decides_push(v)) {
                    options.extend(["-c".to_string(), v.to_string()]);
                }
                i += 1;
            }
            "--git-dir" | "--work-tree" => {
                options.extend(value.map(|v| format!("{t}={v}")));
                i += 1;
            }
            "--namespace" => i += 1,
            _ if t.starts_with("--git-dir=") || t.starts_with("--work-tree=") => {
                options.push(t.to_string())
            }
            _ if t.starts_with("-C") => dirs.push(&t[2..]),
            _ if t.starts_with('-') => {}
            _ => {
                i -= 1;
                break;
            }
        }
    }
    if args.get(i).map(String::as_str) != Some("push") {
        return None;
    }
    let push = &args[i + 1..];
    if push.iter().any(|t| t == "--all" || t == "--mirror") {
        return Some((Reach::Explicit, None));
    }
    let mut positionals = Vec::new();
    let mut j = 0;
    while let Some(t) = push.get(j) {
        match t.as_str() {
            "-o" | "--push-option" | "--repo" | "--receive-pack" | "--exec" => j += 1,
            _ if t.starts_with('-') => {}
            _ => positionals.push(t.as_str()),
        }
        j += 1;
    }
    let refspecs = positionals.get(1..).unwrap_or_default();
    if refspecs.iter().any(|r| reaches_main(r)) {
        return Some((Reach::Explicit, None));
    }
    let bare = refspecs.is_empty();
    if !(bare || matches!(refspecs, ["HEAD" | "@"])) {
        return None;
    }
    let mut checkout = checkout?.to_path_buf();
    for dir in &dirs {
        checkout.push(dir);
    }
    let lost = lost.filter(|_| !dirs.iter().any(|d| Path::new(d).is_absolute()));
    let env: Vec<(&str, &OsStr)> = ["GIT_DIR", "GIT_WORK_TREE"]
        .into_iter()
        .filter_map(|n| command.assigned(n).map(|v| (n, OsStr::new(v))))
        .collect();
    let query = Query {
        checkout,
        options,
        env: &env,
        budget,
    };
    query
        .reach(bare)
        .unwrap_or_else(Some)
        .map(|reach| (reach, lost))
}

/// Whether a `-c` setting may move where a push goes, and so is passed to the queries:
/// those under `push.`, `remote.` and `branch.`. The command hasn't been allowed yet, so
/// no other setting of it reaches git.
fn decides_push(setting: &str) -> bool {
    let name = setting
        .split('=')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    ["push.", "remote.", "branch."]
        .iter()
        .any(|p| name.starts_with(p))
}

/// The git queries for one push, run through `clm-git` in its checkout with the push's
/// own options and environment, within the command's budget.
struct Query<'a> {
    checkout: PathBuf,
    options: Vec<String>,
    env: &'a [(&'a str, &'a OsStr)],
    budget: &'a Budget,
}

impl Query<'_> {
    /// How a push with no refspec (`bare`) or only `HEAD` reaches `main`, by asking git;
    /// `Err(Reach::Unanswered)` when a query runs out of the budget.
    fn reach(&self, bare: bool) -> Result<Option<Reach>, Reach> {
        if self.ask(&["symbolic-ref", "-q", "HEAD"])?.as_deref() == Some("refs/heads/main") {
            return Ok(Some(Reach::OnMain));
        }
        let configured = self.ask(&["config", "--get-regexp", r"^remote\..*\.push$"])?;
        if let Some(refspec) = configured
            .iter()
            .flat_map(|out| out.lines())
            .filter_map(|line| line.split_once(' ').map(|(_, v)| v.trim()))
            .find(|v| reaches_main(v))
        {
            return Ok(Some(Reach::Configured(refspec.to_string())));
        }
        // Only `push.default` and a configured refspec move where a bare `HEAD` goes.
        if !bare {
            return Ok(None);
        }
        let to = self.ask(&["rev-parse", "--symbolic-full-name", "@{push}"])?;
        let to_main = |to: &String| {
            to.strip_prefix("refs/remotes/")
                .and_then(|r| r.strip_suffix("/main"))
                .is_some_and(|remote| !remote.is_empty() && !remote.contains('/'))
        };
        Ok(to.filter(to_main).map(Reach::PushesTo))
    }

    /// The query's trimmed output; `Ok(None)` when it fails, which decides nothing, and
    /// `Err(Reach::Unanswered)` when it outlasts the budget or the budget is spent before
    /// it starts, which `run_within` reports alike.
    fn ask(&self, args: &[&str]) -> Result<Option<String>, Reach> {
        let mut all: Vec<&str> = self.options.iter().map(String::as_str).collect();
        all.extend(args);
        let options = clm_git::Options {
            stdin: None,
            env: self.env,
        };
        let limit = self.budget.remaining();
        match clm_git::run_within(&self.checkout, "git", &all, options, limit) {
            Ok(out) => Ok(Some(String::from_utf8_lossy(&out).trim().to_string())),
            Err(clm_git::Error::TimedOut { .. }) => Err(Reach::Unanswered),
            Err(_) => Ok(None),
        }
    }
}

/// Whether a segment with `tokens` moves or merges into `main` through the host with `gh`:
/// a write to `git/refs/heads/main`, a merge with base `main`, merging a pull request, or
/// a GraphQL mutation that merges or moves a ref.
pub(crate) fn host_moves_main(tokens: &[String]) -> bool {
    let Some(args) = gh::args(tokens) else {
        return false;
    };
    let words = gh::words(args);
    if words.get(..2) == Some(&["pr", "merge"][..]) {
        return true;
    }
    if words.first() != Some(&"api") {
        return false;
    }
    if args.iter().any(|t| t == "graphql") {
        return args.iter().any(|t| {
            [
                "mergePullRequest",
                "mergeBranch",
                "enablePullRequestAutoMerge",
                "updateRef",
            ]
            .iter()
            .any(|m| t.contains(m))
        });
    }
    if !gh::api_writes(&args[1..]) {
        return false;
    }
    let paths = || {
        args.iter()
            .map(|t| t.split('?').next().unwrap_or(t).trim_end_matches('/'))
    };
    let merges_main = || {
        gh::api_fields(args)
            .iter()
            .any(|f| matches!(*f, "base=main" | "base=refs/heads/main"))
            || args.iter().any(|t| t.starts_with("--input"))
    };
    paths().any(|p| p.ends_with("git/refs/heads/main"))
        || (paths().any(|p| p.ends_with("/merges")) && merges_main())
        || paths().any(|p| {
            let parts: Vec<&str> = p.rsplit('/').take(3).collect();
            matches!(parts[..], ["merge", n, "pulls"] if !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
        })
}
