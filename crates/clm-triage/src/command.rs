//! `clm triage`, `clm draft` and `clm intake` as calls (`commands-in-crate`): their arguments,
//! what they print and their exit status. `crates/clm` routes each command here with the
//! settings it loads, since this crate loads none of its own.

use std::ffi::OsString;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use clap::Parser;
use clm_host::Host;
use clm_orchestrator::DEFAULT_EXPIRY_HOURS;

use crate::{Clock, DraftError, Error};

/// The environment as a lookup, as every command's call takes it.
pub type Env<'a> = &'a dyn Fn(&str) -> Option<String>;

/// The arguments of `clm triage`, as the command-line parser reads them.
#[derive(clap::Args, Debug)]
pub struct TriageArgs {
    /// Hours without a commit after which a backlog claim lapses.
    #[arg(long, default_value_t = DEFAULT_EXPIRY_HOURS)]
    pub expiry: u64,
    /// Repository root (default: the CLM repository holding the current directory).
    #[arg(long)]
    pub root: Option<PathBuf>,
}

/// The arguments of `clm intake`, as the command-line parser reads them.
#[derive(clap::Args, Debug)]
pub struct IntakeArgs {
    /// Repository root (default: the CLM repository holding the current directory).
    #[arg(long)]
    pub root: Option<PathBuf>,
}

/// The arguments of `clm draft`, as the command-line parser reads them.
#[derive(clap::Args, Debug)]
pub struct DraftArgs {
    /// The items' slugs, as a `clm triage` group line lists them; with `--fold`, the
    /// change IDs to fold.
    #[arg(
        required_unless_present_any = ["finish", "resume"],
        conflicts_with_all = ["finish", "resume"]
    )]
    pub slugs: Vec<String>,
    /// Write the prompt for a session that continues the draft of this change, whose
    /// session ended before it returned, from the spec branch it pushed.
    #[arg(long, value_name = "CHANGE", conflicts_with_all = ["finish", "fold", "answers"])]
    pub resume: Option<String>,
    /// Finish the draft of this change, whose subagent has returned, instead of starting
    /// one.
    #[arg(long, value_name = "CHANGE")]
    pub finish: Option<String>,
    /// Write the prompt for one session that folds the changes needing review named as arguments
    /// into one spec branch with one pull request, instead of drafting.
    #[arg(long, conflicts_with = "finish")]
    pub fold: bool,
    /// A file of what the human said in the triage session, put into the draft's
    /// prompt; a relative path is read against the current directory.
    #[arg(long, value_name = "FILE", conflicts_with_all = ["finish", "fold"])]
    pub answers: Option<PathBuf>,
    /// Hours without a commit after which a backlog claim lapses.
    #[arg(long, default_value_t = DEFAULT_EXPIRY_HOURS)]
    pub expiry: u64,
    /// Repository root (default: the CLM repository holding the current directory).
    #[arg(long)]
    pub root: Option<PathBuf>,
}

/// The settings `crates/clm` loads for `clm draft`: the `[build] setup` command, if any, and
/// `max_builds`.
#[derive(Debug, Clone)]
pub struct DraftSettings {
    pub setup: Option<String>,
    pub max_builds: u64,
}

/// `commands-in-crate`'s descriptions, which each call's help starts with.
const TRIAGE_ABOUT: &str = "Group the untaken backlog items into units of design work, beside \
    the spec work on origin and the items waiting on the human, most urgent first";
const INTAKE_ABOUT: &str = "Match the host's open issues and dependency alerts and the red \
    verify results on main to the backlog, and list the open pull requests from forks";
const DRAFT_ABOUT: &str = "Claim backlog items and write the prompt for a session that designs \
    one change from them unattended, printing a launcher that names it, holding one of the \
    clone's build places";

#[derive(Parser)]
#[command(
    name = "clm triage",
    bin_name = "clm triage",
    no_binary_name = true,
    about = TRIAGE_ABOUT,
    long_about = None
)]
struct TriageLine {
    #[command(flatten)]
    args: TriageArgs,
}

#[derive(Parser)]
#[command(
    name = "clm intake",
    bin_name = "clm intake",
    no_binary_name = true,
    about = INTAKE_ABOUT,
    long_about = None
)]
struct IntakeLine {
    #[command(flatten)]
    args: IntakeArgs,
}

#[derive(Parser)]
#[command(
    name = "clm draft",
    bin_name = "clm draft",
    no_binary_name = true,
    about = DRAFT_ABOUT,
    long_about = None
)]
struct DraftLine {
    #[command(flatten)]
    args: DraftArgs,
}

/// `clm triage` with `args`, the strings the command line gives after the command's name, in
/// `root` unless `--root` overrides it, a relative `--root` read against `cwd`, the directory
/// it runs in. Writes what the command prints to `out` and `err` and returns its exit status:
/// 0 with the lines, 1 when it refused or the specs don't load, 2 when git failed or for
/// arguments it can't parse. `_env` is taken as every command's call takes it; this command
/// reads none of it.
pub fn triage_command<I, T>(
    args: I,
    root: &Path,
    cwd: &Path,
    _env: Env,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let args = match parse::<TriageLine, _, _>(args, out, err) {
        Ok(line) => line.args,
        Err(code) => return code,
    };
    let root = &args
        .root
        .as_ref()
        .map_or_else(|| root.to_path_buf(), |r| cwd.join(r));
    let written = match crate::triage(root, args.expiry) {
        Ok(t) => t
            .warnings
            .iter()
            .try_for_each(|w| writeln!(err, "clm triage: warning: {w}"))
            .and_then(|()| report("triage", Ok(t.lines.join("\n")), out, err)),
        Err(e) => report("triage", Err(e), out, err),
    };
    written.unwrap_or(2)
}

/// `clm intake`, as [`triage_command`] takes its arguments, with the host adapter
/// `crates/clm` made from the `host` setting, or why it couldn't be made (exit 1).
pub fn intake_command<I, T>(
    args: I,
    root: &Path,
    cwd: &Path,
    _env: Env,
    host: Result<Box<dyn Host>, String>,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let args = match parse::<IntakeLine, _, _>(args, out, err) {
        Ok(line) => line.args,
        Err(code) => return code,
    };
    let root = &args
        .root
        .as_ref()
        .map_or_else(|| root.to_path_buf(), |r| cwd.join(r));
    let written = match host {
        Ok(host) => report(
            "intake",
            crate::intake(root, &*host).map(|lines| lines.join("\n")),
            out,
            err,
        ),
        Err(e) => writeln!(err, "clm intake: {e}").map(|()| 1),
    };
    written.unwrap_or(2)
}

/// `clm draft`, as [`triage_command`] takes its arguments, with the settings `crates/clm`
/// loaded, or why they didn't load, which refuses a draft or a resume (exit 1) and matters
/// to neither `--finish` nor `--fold`. A relative `--answers` path is read against `cwd`, and
/// the change ID is dated by the system clock in the time zone `env` gives as `TZ`. It exits
/// 0 with the launcher, 1 when nothing was drafted or it refused, 2 when git failed or for
/// arguments it can't parse, and 3 when the clone has no free place.
pub fn draft_command<I, T>(
    args: I,
    root: &Path,
    cwd: &Path,
    env: Env,
    settings: Result<DraftSettings, String>,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let args = match parse::<DraftLine, _, _>(args, out, err) {
        Ok(line) => line.args,
        Err(code) => return code,
    };
    let root = &args
        .root
        .as_ref()
        .map_or_else(|| root.to_path_buf(), |r| cwd.join(r));
    let settings = settings.map_err(|e| Error::Refused(format!("settings: {e}")));
    let written = if let Some(change) = &args.finish {
        report(
            "draft",
            crate::finish::finish_in(root, change, args.expiry, cwd),
            out,
            err,
        )
    } else if args.fold {
        match crate::fold(root, &args.slugs) {
            Ok(fold) => fold
                .left
                .iter()
                .try_for_each(|(id, why)| writeln!(err, "left {id}: {why}"))
                .and_then(|()| match fold.launcher {
                    Some(launcher) => report("draft", Ok(launcher), out, err),
                    None => Ok(1),
                }),
            Err(e) => report("draft", Err(e), out, err),
        }
    } else if let Some(change) = &args.resume {
        let drafted = settings.map_err(DraftError::from).and_then(|s| {
            crate::resume(root, change, s.setup.as_deref(), s.max_builds, args.expiry)
        });
        drafted_report(drafted.map(Some), out, err)
    } else {
        let answers = args.answers.map(|a| cwd.join(a));
        let clock = Clock {
            now: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |d| d.as_secs()),
            tz: env("TZ").map(OsString::from),
        };
        let draft = settings.map_err(DraftError::from).and_then(|s| {
            crate::draft(
                root,
                &args.slugs,
                answers.as_deref(),
                s.setup.as_deref(),
                s.max_builds,
                args.expiry,
                &clock,
            )
        });
        match draft {
            Ok(draft) => draft
                .left
                .iter()
                .try_for_each(|(slug, why)| writeln!(err, "left {slug}: {why}"))
                .and_then(|()| drafted_report(Ok(draft.drafted), out, err)),
            Err(e) => drafted_report(Err(e), out, err),
        }
    };
    written.unwrap_or(2)
}

/// Writes a draft's outcome: the launcher on `out` and `draft <id>` last on `err` with 0;
/// nothing drafted with 1; no free place as `clm draft: <why>` with 3; anything else as
/// [`report`] writes it.
fn drafted_report(
    drafted: Result<Option<crate::Drafted>, DraftError>,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> io::Result<u8> {
    match drafted {
        Ok(Some(d)) => {
            let code = report("draft", Ok(d.launcher), out, err)?;
            writeln!(err, "draft {}", d.id)?;
            Ok(code)
        }
        Ok(None) => Ok(1),
        Err(DraftError::NoPlace(why)) => writeln!(err, "clm draft: {why}").map(|()| 3),
        Err(DraftError::Failed(e)) => report("draft", Err(e), out, err),
    }
}

/// Parses a command's arguments, or writes clap's message (help on `out` with 0, a usage
/// error on `err` with 2) and gives the status.
fn parse<L, I, T>(args: I, out: &mut dyn Write, err: &mut dyn Write) -> Result<L, u8>
where
    L: Parser,
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    L::try_parse_from(args).map_err(|e| {
        let written = if e.use_stderr() {
            write!(err, "{}", e.render()).map(|()| 2)
        } else {
            write!(out, "{}", e.render()).map(|()| 0)
        };
        written.unwrap_or(2)
    })
}

/// Writes a result and gives its exit status: the text on `out` and 0; a refusal as
/// `clm <command>: <why>` on `err` and 1; the specs' violations, one line each
/// (`model/violation-line`), on `err` for `clm draft` and on `out` otherwise, and 1
/// (`draft-prompt-file`, `commands-in-crate`); git's failure as `clm <command>: <error>` on `err` and 2.
fn report(
    command: &str,
    result: Result<String, Error>,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> io::Result<u8> {
    match result {
        Ok(text) if text.is_empty() => Ok(0),
        Ok(text) => writeln!(out, "{text}").map(|()| 0),
        Err(Error::Refused(why)) => writeln!(err, "clm {command}: {why}").map(|()| 1),
        Err(Error::Spec(violations)) => {
            let to: &mut dyn Write = if command == "draft" { err } else { out };
            for v in &violations {
                writeln!(to, "{v}")?;
            }
            Ok(1)
        }
        Err(Error::Git(e)) => writeln!(err, "clm {command}: {e}").map(|()| 2),
    }
}
