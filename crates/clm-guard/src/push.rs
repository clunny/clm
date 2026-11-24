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
    /// It names no branch, or only `HEAD`, and a `--config-env` option, as written, takes a
    /// setting that could move where it goes from the environment, which the guard can't read.
    FromEnvironment(String),
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
            Reach::FromEnvironment(option) => format!(
                "{lead}`{option}` takes a setting from the environment, which the guard can't \
                 read, and that setting could send the push to `main`"
            ),
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

/// The push queries' shared budget (`hook-budgets`), which starts with the first query.
pub(crate) struct Budget {
    total: Duration,
    start: OnceCell<Instant>,
}

impl Budget {
    pub(crate) fn new(total: Duration) -> Budget {
        Budget {
            total,
            start: OnceCell::new(),
        }
    }

    /// What is left of the budget, starting it on the first call.
    fn remaining(&self) -> Duration {
        let deadline = *self.start.get_or_init(|| Instant::now() + self.total);
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
    // The first `--config-env` option, as written, whose setting could move where a push of
    // the current branch goes; no `--config-env` option is passed to the queries.
    let mut from_environment: Option<String> = None;
    // Whether the segment names another git directory or work tree, so the guard can't tell
    // which repository it reaches (`push-guard`).
    let mut elsewhere = ["GIT_DIR", "GIT_WORK_TREE", "GIT_COMMON_DIR"]
        .iter()
        .any(|n| command.assigned(n).is_some());
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
                // A `-c` setting's name is the part before its first `=`.
                let name = value.map(|v| v.split('=').next().unwrap_or_default());
                elsewhere |= name.is_some_and(moves_repository);
                if let Some(v) = value.filter(|_| name.is_some_and(decides_push)) {
                    options.extend(["-c".to_string(), v.to_string()]);
                }
                i += 1;
            }
            "--git-dir" | "--work-tree" => {
                elsewhere = true;
                options.extend(value.map(|v| format!("{t}={v}")));
                i += 1;
            }
            // Its value is no subcommand (`push-guard`).
            "--namespace" => i += 1,
            "--config-env" => {
                if let Some(v) = value {
                    let written = format!("{t} {v}");
                    config_env(&mut elsewhere, &mut from_environment, v, written);
                }
                i += 1;
            }
            _ if t.starts_with("--config-env=") => {
                let v = &t["--config-env=".len()..];
                config_env(&mut elsewhere, &mut from_environment, v, t.to_string());
            }
            _ if t.starts_with("--git-dir=") || t.starts_with("--work-tree=") => {
                elsewhere = true;
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
    let lost = lost.filter(|_| !dirs.iter().any(|d| Path::new(d).is_absolute()));
    let env: Vec<(&str, &OsStr)> = ["GIT_DIR", "GIT_WORK_TREE"]
        .into_iter()
        .filter_map(|n| command.assigned(n).map(|v| (n, OsStr::new(v))))
        .collect();
    // With no checkout from the walk (the guard's own directory is gone), the push's own
    // last absolute `-C` gives one, and the directories after it join it (`approve-guard`).
    let start = match checkout {
        Some(checkout) => Some((checkout.to_path_buf(), &dirs[..])),
        None => dirs
            .iter()
            .rposition(|d| Path::new(d).is_absolute())
            .map(|k| (PathBuf::from(dirs[k]), &dirs[k + 1..])),
    };
    let query = start.map(|(mut checkout, dirs)| {
        for dir in dirs {
            checkout.push(dir);
        }
        Query {
            checkout,
            options,
            env: &env,
            budget,
        }
    });
    // A push the cases would block goes through when it isn't under CLM; that is asked only
    // for such a push.
    let settle = |reach: Reach, reported: Option<&'a str>| {
        let outside =
            !elsewhere && lost.is_none() && query.as_ref().is_some_and(Query::outside_clm);
        (!outside).then_some((reach, reported))
    };
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
    if push.iter().any(|t| t == "--all" || t == "--mirror")
        || refspecs.iter().any(|r| reaches_main(r))
    {
        return settle(Reach::Explicit, None);
    }
    let bare = refspecs.is_empty();
    if !(bare || matches!(refspecs, ["HEAD" | "@"])) {
        return None;
    }
    // Decided without asking git, which couldn't see the setting either.
    if let Some(option) = from_environment {
        return settle(Reach::FromEnvironment(option), lost);
    }
    let reach = query.as_ref()?.reach(bare).unwrap_or_else(Some)?;
    settle(reach, lost)
}

/// Reads a `--config-env` option's `value`, `<name>=<variable>`, its name being the part
/// before its last `=`: one naming `core.worktree` or `core.bare` names another work tree
/// (`elsewhere`), and the first whose name may move where a push goes is kept, as
/// `written`, in `from_environment`. Neither is passed to the queries, since the guard can't
/// read the variable.
fn config_env(
    elsewhere: &mut bool,
    from_environment: &mut Option<String>,
    value: &str,
    written: String,
) {
    let name = value.rsplit_once('=').map_or(value, |(name, _)| name);
    *elsewhere |= moves_repository(name);
    if from_environment.is_none() && decides_push(name) {
        *from_environment = Some(written);
    }
}

/// Whether a setting's `name` moves the repository: `core.worktree` or `core.bare`.
fn moves_repository(name: &str) -> bool {
    ["core.worktree", "core.bare"]
        .iter()
        .any(|n| name.eq_ignore_ascii_case(n))
}

/// Whether a setting's `name` may move where a push goes: those under `push.`, `remote.`
/// and `branch.`. Only such `-c` settings are passed to the queries: the command hasn't been
/// allowed yet, so no other setting of it reaches git.
fn decides_push(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
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
    /// Whether the push isn't under CLM (`push-guard`): its checkout is an existing
    /// directory in a git repository with no `specs/SPEC.md` at or above it up to the
    /// repository's top, and none committed anywhere in the repository. Whatever can't be
    /// told counts as under CLM.
    fn outside_clm(&self) -> bool {
        let Ok(dir) = self.checkout.canonicalize() else {
            return false;
        };
        if !dir.is_dir()
            || clm_hooks::clm_root(&dir).is_some()
            || !dir.ancestors().any(|a| a.join(".git").exists())
        {
            return false;
        }
        let found = self.ask(&["ls-files", "-z", "--", ":(top,glob)**/specs/SPEC.md"]);
        matches!(found, Ok(Some(out)) if out.is_empty())
    }

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
    gh::calls(tokens).any(call_moves_main)
}

/// Whether one `gh` call with `args` moves or merges into `main` through the host.
fn call_moves_main(args: &[String]) -> bool {
    let args = match gh::words(args)[..] {
        [(_, "pr"), (_, "merge")] => return true,
        [(api, "api"), ..] => &args[api + 1..],
        _ => return false,
    };
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
    if !gh::api_writes(args) {
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
