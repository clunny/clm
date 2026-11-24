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
    /// It names no branch, or names `HEAD` or `@`, with or without a leading `+`, among its
    /// refspecs, from a checkout on `main`.
    OnMain,
    /// It names no branch, and git says the branch pushes to `main` (`@{push}`).
    PushesTo(String),
    /// It names no branch, or a named one, and a configured push refspec sends to `main`.
    Configured(String),
    /// It names this branch, whose upstream is `main`, under this `push.default`, `upstream`
    /// or `tracking`.
    Upstream(String, String),
    /// It names no branch, and this setting, as `<key>=<value>`, pushes every branch.
    EveryBranch(String),
    /// It names no branch, or a named one, and a `--config-env` or include option, as
    /// written, takes a setting the guard can't read that could move where it goes.
    Unreadable(String),
    /// It names no branch, or a named one, and the segment assigns this variable, which
    /// sets configuration the guard doesn't read (`CONFIG_VARIABLES`).
    SetByVariable(&'static str),
    /// It names no branch, or a named one, and git didn't answer within the budget.
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
                format!(
                    "{lead}{checkout} is on `main`, so a push without a branch, or of `HEAD` \
                     or `@`, pushes `main`"
                )
            }
            Reach::PushesTo(to) => {
                format!("{lead}the branch of {checkout} pushes to `main` (`@{{push}}` is `{to}`)")
            }
            Reach::Configured(refspec) => {
                format!("{lead}a configured push refspec sends to `main` (`{refspec}`)")
            }
            Reach::Upstream(branch, mode) => {
                let alias = if mode == "tracking" {
                    ", the old name of `upstream`"
                } else {
                    ""
                };
                format!(
                    "{lead}the branch `{branch}`'s upstream is `main`, and git pushes it there \
                     under `push.default={mode}`{alias}"
                )
            }
            Reach::EveryBranch(setting) => {
                format!("{lead}`{setting}` makes git push every branch, `main` among them")
            }
            Reach::Unreadable(option) => format!(
                "{lead}`{option}` takes a setting the guard can't read, and that setting could \
                 send the push to `main`"
            ),
            Reach::SetByVariable(variable) => format!(
                "{lead}the command sets `{variable}`, which gives git configuration the guard \
                 doesn't read, and that configuration could send the push to `main`"
            ),
        };
        Some(format!(
            "clm: {test}. Push your branch explicitly: `git push origin HEAD:<branch>`."
        ))
    }
}

/// The variables that set git configuration the guard doesn't read, in the order a reason
/// names the first the segment assigns (`push-guard`). Each can also set `core.worktree` or
/// `core.bare`, and none is passed to the queries.
const CONFIG_VARIABLES: [&str; 4] = [
    "GIT_CONFIG_COUNT",
    "GIT_CONFIG_PARAMETERS",
    "GIT_CONFIG_GLOBAL",
    "GIT_CONFIG_SYSTEM",
];

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
    // The first `--config-env` or include option, as written, whose setting could move where
    // a push goes and which the guard can't read; none of them is passed to the queries.
    let mut unreadable: Option<String> = None;
    // The first variable the segment assigns that sets configuration the guard doesn't read.
    let set_by_variable = CONFIG_VARIABLES
        .into_iter()
        .find(|n| command.assigned(n).is_some());
    // Whether the segment names another git directory or work tree, or may through such a
    // variable, so the guard can't tell which repository it reaches (`push-guard`).
    let mut elsewhere = set_by_variable.is_some()
        || ["GIT_DIR", "GIT_WORK_TREE", "GIT_COMMON_DIR"]
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
                if let Some(v) = value.filter(|_| name.is_some_and(includes)) {
                    unreadable.get_or_insert_with(|| format!("{t} {v}"));
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
                    config_env(&mut elsewhere, &mut unreadable, v, written);
                }
                i += 1;
            }
            _ if t.starts_with("--config-env=") => {
                let v = &t["--config-env=".len()..];
                config_env(&mut elsewhere, &mut unreadable, v, t.to_string());
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
    // Only a push with no refspec or a named one goes where settings say.
    let bare = refspecs.is_empty();
    if !bare && !refspecs.iter().any(|r| named(r)) {
        return None;
    }
    // Decided without asking git, which couldn't see the setting either.
    if let Some(option) = unreadable {
        return settle(Reach::Unreadable(option), lost);
    }
    if let Some(variable) = set_by_variable {
        return settle(Reach::SetByVariable(variable), lost);
    }
    // A named refspec without its leading `+`.
    let names = refspecs
        .iter()
        .filter(|r| named(r))
        .map(|r| r.strip_prefix('+').unwrap_or(r));
    let push = Push {
        bare,
        head: bare || names.clone().any(is_head),
        branches: names
            .filter(|r| !is_head(r))
            .map(|r| {
                r.strip_prefix("refs/heads/")
                    .or_else(|| r.strip_prefix("heads/"))
                    .unwrap_or(r)
            })
            .collect(),
    };
    let reach = query.as_ref()?.reach(&push).unwrap_or_else(Some)?;
    settle(reach, lost)
}

/// Whether a refspec is named: `HEAD`, `@` or a branch's name, which git may map through
/// `remote.<name>.push` or the branch's upstream. One holding a `:`, a `*` or an expansion
/// is judged as written.
fn named(refspec: &str) -> bool {
    !refspec.contains([':', '*', '$', '`'])
}

/// Whether a named refspec, without its leading `+`, is `HEAD` or `@` (git's name for
/// `HEAD`), for which git pushes the current branch.
fn is_head(refspec: &str) -> bool {
    matches!(refspec, "HEAD" | "@")
}

/// A push with no refspec or a named one, as its queries judge it.
struct Push<'a> {
    /// It has no refspec.
    bare: bool,
    /// It has no refspec, or a refspec that is `HEAD` or `@`, with or without a leading
    /// `+`, whatever other refspecs it has: git then pushes the current branch.
    head: bool,
    /// The branches its named refspecs other than `HEAD` and `@` name.
    branches: Vec<&'a str>,
}

/// Reads a `--config-env` option's `value`, `<name>=<variable>`, its name being the part
/// before its last `=`: one that may move the repository names another work tree
/// (`elsewhere`), and the first whose name may move where a push goes is kept, as
/// `written`, in `unreadable`. Neither is passed to the queries, since the guard can't read
/// the variable.
fn config_env(elsewhere: &mut bool, unreadable: &mut Option<String>, value: &str, written: String) {
    let name = value.rsplit_once('=').map_or(value, |(name, _)| name);
    *elsewhere |= moves_repository(name);
    if unreadable.is_none() && (decides_push(name) || includes(name)) {
        *unreadable = Some(written);
    }
}

/// Whether a setting's `name` may move the repository: `core.worktree`, `core.bare`, or an
/// include, whose file can set either.
fn moves_repository(name: &str) -> bool {
    includes(name)
        || ["core.worktree", "core.bare"]
            .iter()
            .any(|n| name.eq_ignore_ascii_case(n))
}

/// Whether a setting's `name` includes a config file, which can set any setting:
/// `include.path` or `includeIf.<condition>.path`, without regard to ASCII case.
fn includes(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    name.starts_with("include.") || name.starts_with("includeif.")
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

    /// How a push with no refspec or a named one reaches `main`, by asking git;
    /// `Err(Reach::Unanswered)` when a query runs out of the budget.
    fn reach(&self, push: &Push) -> Result<Option<Reach>, Reach> {
        if push.head
            && self
                .ask(&["symbolic-ref", "-q", "HEAD"])?
                .as_deref()
                .map(str::trim)
                == Some("refs/heads/main")
        {
            return Ok(Some(Reach::OnMain));
        }
        let settings = self.settings()?;
        let values = |key: String| {
            settings
                .iter()
                .filter(move |(k, _)| *k == key)
                .map(|(_, v)| v.as_str())
        };
        let of = |section: &'static str, var: &'static str| {
            settings
                .iter()
                .filter(move |(k, _)| k.starts_with(section) && k.ends_with(var))
        };
        if let Some((_, refspec)) = of("remote.", ".push").find(|(_, v)| reaches_main(v)) {
            return Ok(Some(Reach::Configured(refspec.clone())));
        }
        let default = || values("push.default".to_string());
        if let Some(mode) = default().find(|v| matches!(*v, "upstream" | "tracking"))
            && let Some(branch) = push.branches.iter().find(|b| {
                values(format!("branch.{b}.merge"))
                    .any(|v| matches!(v, "main" | "heads/main" | "refs/heads/main"))
            })
        {
            return Ok(Some(Reach::Upstream(branch.to_string(), mode.to_string())));
        }
        // Only `push.default` and a configured refspec move where a bare `HEAD` or a named
        // branch goes.
        if !push.bare {
            return Ok(None);
        }
        if default().any(|v| v == "matching") {
            return Ok(Some(Reach::EveryBranch(
                "push.default=matching".to_string(),
            )));
        }
        let mirrors = |v: &str| {
            !["false", "no", "off", "0"]
                .iter()
                .any(|f| v.eq_ignore_ascii_case(f))
        };
        if let Some((key, value)) = of("remote.", ".mirror").find(|(_, v)| mirrors(v)) {
            let setting = match value.as_str() {
                "" => key.clone(),
                value => format!("{key}={value}"),
            };
            return Ok(Some(Reach::EveryBranch(setting)));
        }
        let to = self.ask(&["rev-parse", "--symbolic-full-name", "@{push}"])?;
        let to = to.map(|to| to.trim().to_string());
        let to_main = |to: &String| {
            to.strip_prefix("refs/remotes/")
                .and_then(|r| r.strip_suffix("/main"))
                .is_some_and(|remote| !remote.is_empty() && !remote.contains('/'))
        };
        Ok(to.filter(to_main).map(Reach::PushesTo))
    }

    /// The settings query's entries, as key and value, every one of a key in the order git
    /// prints them; a setting with no value has an empty one. A failed query, as when no
    /// setting matches, has none.
    fn settings(&self) -> Result<Vec<(String, String)>, Reach> {
        let pattern = r"^(push\.default|remote\..*\.(push|mirror)|branch\..*\.merge)$";
        let out = self.ask(&["config", "-z", "--get-regexp", pattern])?;
        Ok(out
            .iter()
            .flat_map(|out| out.split('\0'))
            .filter(|entry| !entry.is_empty())
            .map(|entry| {
                let (key, value) = entry.split_once('\n').unwrap_or((entry, ""));
                (key.to_string(), value.to_string())
            })
            .collect())
    }

    /// The query's output as it printed it; `Ok(None)` when it fails, which decides nothing, and
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
            Ok(out) => Ok(Some(String::from_utf8_lossy(&out).into_owned())),
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
