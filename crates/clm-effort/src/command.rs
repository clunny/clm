//! `clm effort`, `clm capabilities` and `clm architecture` as calls: their arguments, what they
//! print and their exit status (`commands-in-crate`).

use std::ffi::OsString;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand, ValueEnum};
use clm_ledger::{Decision, Rating};

use crate::decide::decide_architecture;
use crate::history::architecture_review;
use crate::{
    Error, Options, Recorded, architecture_text, capabilities, constraint_text, decide, list_text,
    page_text, record,
};

/// The work Claude Code sessions spent, recorded in the ledger.
#[derive(clap::Args, Debug)]
pub struct EffortArgs {
    #[command(subcommand)]
    command: EffortCommand,
}

impl EffortArgs {
    /// The `--root` the command line gives.
    pub fn root(&self) -> Option<&Path> {
        let EffortCommand::Record { root, .. } = &self.command;
        root.as_deref()
    }
}

#[derive(Subcommand, Debug)]
enum EffortCommand {
    /// Read this machine's transcripts and append an `effort` record for each session and
    /// branch whose counts changed.
    Record {
        /// Only this branch's sessions.
        #[arg(long)]
        branch: Option<String>,
        /// The transcripts directory (default: `$CLAUDE_CONFIG_DIR/projects`, else
        /// `~/.claude/projects`).
        #[arg(long)]
        transcripts: Option<PathBuf>,
        /// A directory whose sessions count, besides the repository's worktrees.
        #[arg(long = "cwd")]
        cwds: Vec<PathBuf>,
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
    },
}

/// What each capability costs, from the ledger's effort records and git: the list, or one
/// capability's page.
#[derive(clap::Args, Debug)]
#[command(subcommand_negates_reqs = true)]
pub struct CapabilitiesArgs {
    #[command(subcommand)]
    command: Option<CapabilitiesCommand>,
    /// The capability's slug (default: list them all; `decide` is the subcommand).
    slug: Option<String>,
    /// Text, or the report (with a slug, the capability's cost) as JSON.
    #[arg(long, value_enum, default_value = "text")]
    format: Format,
    /// Repository root (default: the CLM repository holding the current directory).
    #[arg(long)]
    root: Option<PathBuf>,
}

impl CapabilitiesArgs {
    /// The `--root` the command line gives, `decide`'s first.
    pub fn root(&self) -> Option<&Path> {
        match &self.command {
            Some(CapabilitiesCommand::Decide {
                root: Some(root), ..
            }) => Some(root),
            _ => self.root.as_deref(),
        }
    }
}

#[derive(Subcommand, Debug)]
enum CapabilitiesCommand {
    /// Record a review decision on a capability, settling the workaround flags open on it.
    Decide {
        /// The capability's slug.
        slug: String,
        /// What the capability is worth.
        #[arg(long, value_enum)]
        value: RatingArg,
        /// What to do with it.
        #[arg(long, value_enum)]
        decision: DecisionArg,
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
    },
}

/// What each architectural constraint costs and whether its enforcing rules are built: the
/// list, or one constraint's page.
#[derive(clap::Args, Debug)]
#[command(subcommand_negates_reqs = true)]
pub struct ArchitectureArgs {
    #[command(subcommand)]
    command: Option<ArchitectureCommand>,
    /// The constraint's slug (default: list them all; `decide` is the subcommand).
    slug: Option<String>,
    /// Text, or the list (with a slug, the constraint's entry) as JSON.
    #[arg(long, value_enum, default_value = "text")]
    format: Format,
    /// Repository root (default: the CLM repository holding the current directory).
    #[arg(long)]
    root: Option<PathBuf>,
}

impl ArchitectureArgs {
    /// The `--root` the command line gives, `decide`'s first.
    pub fn root(&self) -> Option<&Path> {
        match &self.command {
            Some(ArchitectureCommand::Decide {
                root: Some(root), ..
            }) => Some(root),
            _ => self.root.as_deref(),
        }
    }
}

#[derive(Subcommand, Debug)]
enum ArchitectureCommand {
    /// Record a review decision on an architectural constraint, settling the workaround flags
    /// open on it.
    Decide {
        /// The constraint's slug.
        slug: String,
        /// What the constraint is worth.
        #[arg(long, value_enum)]
        value: RatingArg,
        /// What to do with it.
        #[arg(long, value_enum)]
        decision: DecisionArg,
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum Format {
    Text,
    Json,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum RatingArg {
    High,
    Medium,
    Low,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum DecisionArg {
    Keep,
    Cut,
    Redesign,
    Accept,
}

// The parsers of each command alone: its own arguments, without its name. Each carries the
// command's description as its `about` and its full name as `bin_name`, so its help and its
// subcommands' `Usage:` lines read as `clm` prints them (`commands-in-crate`).
#[derive(Parser)]
#[command(
    name = "clm effort",
    bin_name = "clm effort",
    no_binary_name = true,
    about = "The work Claude Code sessions spent, recorded in the ledger"
)]
struct EffortLine {
    #[command(flatten)]
    args: EffortArgs,
}

#[derive(Parser)]
#[command(
    name = "clm capabilities",
    bin_name = "clm capabilities",
    no_binary_name = true,
    about = "What each capability costs, from the ledger's effort records and git: the list, or \
             one capability's page"
)]
struct CapabilitiesLine {
    #[command(flatten)]
    args: CapabilitiesArgs,
}

#[derive(Parser)]
#[command(
    name = "clm architecture",
    bin_name = "clm architecture",
    no_binary_name = true,
    about = "What each architectural constraint costs and whether its enforcing rules are \
             built: the list, or one constraint's page"
)]
struct ArchitectureLine {
    #[command(flatten)]
    args: ArchitectureArgs,
}

/// `path` as given on the command line run in `cwd`: absolute, its `.` components gone, as
/// `std::path::absolute` leaves it.
fn from(cwd: &Path, path: &Path) -> PathBuf {
    let joined = cwd.join(path);
    std::path::absolute(&joined).unwrap_or(joined)
}

/// The environment lookup every call takes.
type Env<'a> = &'a dyn Fn(&str) -> Option<String>;

/// Parses `args` as `P`, or writes clap's message (usage errors to `err`, help to `out`) and
/// gives the status: 2 for a usage error, 0 for help.
fn parse<P: Parser, I, T>(args: I, out: &mut dyn Write, err: &mut dyn Write) -> Result<P, u8>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    P::try_parse_from(args).map_err(|e| {
        let written = if e.use_stderr() {
            write!(err, "{}", e.render()).map(|()| 2)
        } else {
            write!(out, "{}", e.render()).map(|()| 0)
        };
        written.unwrap_or(2)
    })
}

/// `clm effort` with `args` as the command line gives them after the command's name, in
/// `root` unless `--root` overrides it, relative paths taken from `cwd`. Writes what the
/// command prints to `out` and `err` and returns its exit status.
pub fn effort_command<I, T>(
    args: I,
    root: &Path,
    cwd: &Path,
    env: Env,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let EffortLine { args } = match parse(args, out, err) {
        Ok(line) => line,
        Err(code) => return code,
    };
    let EffortCommand::Record {
        branch,
        transcripts,
        cwds,
        root: given,
    } = args.command;
    let options = Options {
        branch,
        transcripts: transcripts.map(|p| from(cwd, &p)),
        cwds: cwds.into_iter().map(|p| from(cwd, &p)).collect(),
    };
    let root = given.map_or_else(|| root.to_path_buf(), |r| from(cwd, &r));
    let written = match record(&root, &options, env) {
        Ok(done) => print_recorded(&done, out, err).map(|()| 0),
        Err(e) => failed("clm effort", &e, err),
    };
    written.unwrap_or(2)
}

/// `clm capabilities`, as `effort_command` takes its arguments.
pub fn capabilities_command<I, T>(
    args: I,
    root: &Path,
    cwd: &Path,
    env: Env,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let CapabilitiesLine { args } = match parse(args, out, err) {
        Ok(line) => line,
        Err(code) => return code,
    };
    let root = args
        .root()
        .map_or_else(|| root.to_path_buf(), |r| from(cwd, r));
    let written = match args.command {
        Some(CapabilitiesCommand::Decide {
            slug,
            value,
            decision,
            ..
        }) => match args.slug {
            Some(parent) => misplaced("capabilities", &parent, err),
            None => decided(
                "capabilities",
                decide,
                &root,
                &slug,
                value,
                decision,
                env,
                out,
                err,
            ),
        },
        None => match capabilities(&root) {
            // Its own line, so it starts `not a CLM repository` (`capabilities-list`).
            Err(e @ Error::NotClm(_)) => writeln!(err, "{e}").map(|()| e.exit_code()),
            loaded => page(
                "capabilities",
                loaded,
                args.slug.as_deref(),
                args.format,
                |report, slug| match slug {
                    None => Some((list_text(report), serde_json::to_string_pretty(report))),
                    Some(slug) => report
                        .capabilities
                        .iter()
                        .find(|c| c.slug == slug)
                        .map(|cost| (page_text(cost), serde_json::to_string_pretty(cost))),
                },
                |slug| format!("no capability {slug}"),
                out,
                err,
            ),
        },
    };
    written.unwrap_or(2)
}

/// `clm architecture`, as `effort_command` takes its arguments.
pub fn architecture_command<I, T>(
    args: I,
    root: &Path,
    cwd: &Path,
    env: Env,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let ArchitectureLine { args } = match parse(args, out, err) {
        Ok(line) => line,
        Err(code) => return code,
    };
    let root = args
        .root()
        .map_or_else(|| root.to_path_buf(), |r| from(cwd, r));
    let written = match args.command {
        Some(ArchitectureCommand::Decide {
            slug,
            value,
            decision,
            ..
        }) => match args.slug {
            Some(parent) => misplaced("architecture", &parent, err),
            None => decided(
                "architecture",
                decide_architecture,
                &root,
                &slug,
                value,
                decision,
                env,
                out,
                err,
            ),
        },
        None => page(
            "architecture",
            architecture_review(&root),
            args.slug.as_deref(),
            args.format,
            |list, slug| match slug {
                None => Some((architecture_text(list), serde_json::to_string_pretty(list))),
                Some(slug) => list
                    .iter()
                    .find(|c| c.slug == slug)
                    .map(|entry| (constraint_text(entry), serde_json::to_string_pretty(entry))),
            },
            |slug| format!("no architectural constraint {slug}"),
            out,
            err,
        ),
    };
    written.unwrap_or(2)
}

fn print_recorded(done: &Recorded, out: &mut dyn Write, err: &mut dyn Write) -> io::Result<()> {
    for warning in &done.warnings {
        writeln!(err, "warning: {warning}")?;
    }
    for line in &done.lines {
        writeln!(out, "{line}")?;
    }
    Ok(())
}

/// Writes `e` to `err` after `prefix` and gives its exit status.
fn failed(prefix: &str, e: &Error, err: &mut dyn Write) -> io::Result<u8> {
    writeln!(err, "{prefix}: {e}").map(|()| e.exit_code())
}

/// `clm <command> <slug> decide ...` isn't a decision on `<slug>`.
fn misplaced(command: &str, parent: &str, err: &mut dyn Write) -> io::Result<u8> {
    writeln!(
        err,
        "clm {command}: `decide` takes the slug after it: clm {command} decide <slug> (not \
         {parent})"
    )
    .map(|()| 2)
}

/// A list or one entry's page, as text or JSON, from `loaded`; `select` gives the lines and
/// the JSON for a slug, or `None` when there is no entry `missing` names.
#[allow(clippy::too_many_arguments)]
fn page<T>(
    command: &str,
    loaded: Result<T, Error>,
    slug: Option<&str>,
    format: Format,
    select: impl Fn(&T, Option<&str>) -> Option<(Vec<String>, serde_json::Result<String>)>,
    missing: impl Fn(&str) -> String,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> io::Result<u8> {
    let loaded = match loaded {
        Ok(loaded) => loaded,
        Err(e) => return failed(&format!("clm {command}"), &e, err),
    };
    let Some((lines, json)) = select(&loaded, slug) else {
        return writeln!(out, "{}", missing(slug.unwrap_or_default())).map(|()| 1);
    };
    let text = match format {
        Format::Text => lines.join("\n"),
        Format::Json => json.unwrap_or_default(),
    };
    writeln!(out, "{text}").map(|()| 0)
}

type Decide = fn(&Path, &str, Rating, Decision, Env) -> Result<Recorded, Error>;

/// `clm capabilities decide` or `clm architecture decide`: appends a `decided` record
/// (`decide-command`, `architecture-decide-command`).
#[allow(clippy::too_many_arguments)]
fn decided(
    command: &str,
    decide: Decide,
    root: &Path,
    slug: &str,
    value: RatingArg,
    decision: DecisionArg,
    env: Env,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> io::Result<u8> {
    let value = match value {
        RatingArg::High => Rating::High,
        RatingArg::Medium => Rating::Medium,
        RatingArg::Low => Rating::Low,
    };
    let decision = match decision {
        DecisionArg::Keep => Decision::Keep,
        DecisionArg::Cut => Decision::Cut,
        DecisionArg::Redesign => Decision::Redesign,
        DecisionArg::Accept => Decision::Accept,
    };
    match decide(root, slug, value, decision, env) {
        Ok(done) => done
            .lines
            .iter()
            .try_for_each(|line| writeln!(out, "{line}"))
            .map(|()| 0),
        // As the page for an unknown slug says it.
        Err(e @ (Error::NoCapability(_) | Error::NoArchitecture(_))) => {
            writeln!(out, "{e}").map(|()| 1)
        }
        Err(e) => failed(&format!("clm {command} decide"), &e, err),
    }
}
