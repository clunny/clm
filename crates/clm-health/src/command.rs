//! `clm health` and `clm workaround` as calls: the arguments as the command line gives them,
//! the root, what `crates/clm` loads for them, the environment and the two streams in, the
//! exit status out (`commands-in-crate`, `commands-live-in-their-crates`).

use std::ffi::OsString;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use clap::{Args, FromArgMatches};

use crate::Settings;

/// The arguments of `clm health`.
#[derive(Args, Debug, Clone)]
pub struct HealthArgs {
    /// Hours after which an unmerged claim has lapsed.
    #[arg(long, default_value_t = clm_orchestrator::DEFAULT_EXPIRY_HOURS)]
    pub expiry: u64,
    /// Repository root (default: the CLM repository holding the current directory).
    #[arg(long)]
    pub root: Option<PathBuf>,
}

/// The arguments of `clm workaround`.
#[derive(Args, Debug, Clone)]
pub struct WorkaroundArgs {
    /// The requirement, `<component>/<slug>`.
    pub requirement: String,
    /// The flag's title, naming what made it hard.
    #[arg(long)]
    pub title: String,
    /// Repository root (default: the CLM repository holding the current directory).
    #[arg(long)]
    pub root: Option<PathBuf>,
}

/// `A` parsed from `args`, the words after the command's name; on a usage error (or a request
/// for help) the text clap renders, written where clap puts it, and the status to return.
fn parse<'a, A: Args + FromArgMatches, I, T>(
    name: &'static str,
    args: I,
    out: &'a mut dyn Write,
    err: &'a mut dyn Write,
) -> Result<A, u8>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let command = A::augment_args(clap::Command::new(name).bin_name(name).no_binary_name(true));
    let parsed = command
        .try_get_matches_from(args)
        .and_then(|m| A::from_arg_matches(&m));
    parsed.map_err(|e| {
        let stream = if e.use_stderr() { err } else { out };
        let _ = write!(stream, "{}", e.render());
        e.exit_code() as u8
    })
}

/// `clm health`: the report on `out`, exit 0 whatever it finds; 1, on `err`, when `settings`
/// (as `crates/clm` loaded them from `config`) couldn't be loaded or `specs/SPEC.md` is
/// missing; 2 when git fails or the arguments don't parse. `root` is the repository
/// `crates/clm` found, which `--root` overrides. The command reads no environment variable.
pub fn health_command<I, T>(
    args: I,
    root: &Path,
    settings: Result<Settings, clm_check::Violation>,
    _env: &dyn Fn(&str) -> Option<String>,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let args: HealthArgs = match parse("clm health", args, out, err) {
        Ok(args) => args,
        Err(status) => return status,
    };
    let root = args.root.as_deref().unwrap_or(root);
    let settings = match settings {
        Ok(s) => s,
        Err(v) => {
            let _ = writeln!(err, "clm health: {}", clm_check::format(&v));
            return 1;
        }
    };
    match crate::report(root, &settings, args.expiry) {
        Ok(report) => match write!(out, "{report}") {
            Ok(()) => 0,
            Err(e) => {
                let _ = writeln!(err, "clm health: {e}");
                1
            }
        },
        Err(e) => {
            let _ = writeln!(err, "clm health: {e}");
            e.exit_code()
        }
    }
}

/// `clm workaround`: the body read from `stdin`, the flag written and its path printed
/// (`workaround-command`); then exit 3, warning on `err`, for each capability the requirement
/// serves with `workaround_flag` (`config`'s `[effort] workaround_flag`, as `crates/clm`
/// loaded it) unsettled flags or more, and 0, warning, when they can't be counted. 1 when the
/// requirement isn't in the graph or the body can't be read, 2 when the arguments don't parse.
/// `root` is the repository `crates/clm` found, which `--root` overrides. The command reads no
/// environment variable.
pub fn workaround_command<I, T>(
    args: I,
    root: &Path,
    workaround_flag: Result<u64, String>,
    stdin: &mut dyn Read,
    _env: &dyn Fn(&str) -> Option<String>,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let args: WorkaroundArgs = match parse("clm workaround", args, out, err) {
        Ok(args) => args,
        Err(status) => return status,
    };
    let root = args.root.as_deref().unwrap_or(root);
    let mut body = String::new();
    if let Err(e) = stdin.read_to_string(&mut body) {
        let _ = writeln!(err, "clm workaround: reading the body: {e}");
        return 1;
    }
    let path = match crate::workaround(root, &args.requirement, &args.title, &body) {
        Ok(path) => path,
        Err(e) => {
            let _ = writeln!(err, "clm workaround: {e}");
            return e.exit_code();
        }
    };
    if let Err(e) = writeln!(out, "{}", path.display()) {
        let _ = writeln!(err, "clm workaround: {e}");
        return 1;
    }
    let counted = workaround_flag.and_then(|threshold| {
        crate::crowded(root, &args.requirement, threshold).map_err(|e| e.to_string())
    });
    match counted {
        Ok(crowded) if crowded.is_empty() => 0,
        Ok(crowded) => {
            for (capability, n) in crowded {
                let _ = writeln!(
                    err,
                    "clm workaround: warning: {n} unsettled workarounds on {capability}: stop and \
                     ask whether to redesign it (redesign-before-workaround)"
                );
            }
            3
        }
        Err(e) => {
            let _ = writeln!(
                err,
                "clm workaround: warning: the workarounds weren't counted: {e}"
            );
            0
        }
    }
}
