//! `clm effort record` (`effort/record-command`).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Path, PathBuf};

use clm_ledger::{Body, Entry, Usage};

use crate::{SessionWork, sessions};

/// What `clm effort record` reads.
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// Only this branch's sessions.
    pub branch: Option<String>,
    /// The transcripts directory, `transcripts_dir` when `None`.
    pub transcripts: Option<PathBuf>,
    /// Directories counted besides the repository's worktrees.
    pub cwds: Vec<PathBuf>,
}

/// What `record` printed and warned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recorded {
    pub lines: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug)]
pub enum Error {
    /// The command doesn't run here: no CLM repository.
    Refused(String),
    /// `CAPABILITIES.md` holds no capability with this slug.
    NoCapability(String),
    /// `ARCHITECTURE.md` holds no entry with this slug.
    NoArchitecture(String),
    Git(clm_git::Error),
    Ledger(clm_ledger::Error),
    /// Measuring the nodes' sizes failed.
    Io(std::io::Error),
}

impl Error {
    /// 1 when refused or the capability is unknown, 2 when git, the ledger or reading the files fails.
    pub fn exit_code(&self) -> u8 {
        match self {
            Error::Refused(_) | Error::NoCapability(_) | Error::NoArchitecture(_) => 1,
            Error::Git(_) | Error::Ledger(_) | Error::Io(_) => 2,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Refused(why) => f.write_str(why),
            Error::NoCapability(slug) => write!(f, "no capability {slug}"),
            Error::NoArchitecture(slug) => write!(f, "no architectural constraint {slug}"),
            Error::Git(e) => e.fmt(f),
            Error::Ledger(e) => e.fmt(f),
            Error::Io(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for Error {}

impl From<clm_git::Error> for Error {
    fn from(e: clm_git::Error) -> Self {
        Error::Git(e)
    }
}

impl From<clm_ledger::Error> for Error {
    fn from(e: clm_ledger::Error) -> Self {
        Error::Ledger(e)
    }
}

/// Claude Code's transcripts directory: `$CLAUDE_CONFIG_DIR/projects`, else
/// `$HOME/.claude/projects`.
fn transcripts_dir(env: &dyn Fn(&str) -> Option<String>) -> Option<PathBuf> {
    if let Some(dir) = env("CLAUDE_CONFIG_DIR").filter(|d| !d.is_empty()) {
        return Some(PathBuf::from(dir).join("projects"));
    }
    env("HOME")
        .filter(|h| !h.is_empty())
        .map(|home| PathBuf::from(home).join(".claude").join("projects"))
}

/// What `record` would append, unappended: for `clm land`, which appends it with its
/// `landed` record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unrecorded {
    /// The `effort` bodies.
    pub bodies: Vec<Body>,
    pub warnings: Vec<String>,
}

/// The `effort` bodies `record` would append, and the reading's warnings.
pub fn unrecorded(
    root: &Path,
    options: &Options,
    env: &dyn Fn(&str) -> Option<String>,
) -> Result<Unrecorded, Error> {
    let (new, warnings) = grown(root, options, env)?;
    Ok(Unrecorded {
        bodies: new.iter().map(body).collect(),
        warnings,
    })
}

/// Appends, in one commit, an `effort` record for each session and branch of the
/// repository's worktrees (and `options.cwds`) whose counts grew past the fold's.
pub fn record(
    root: &Path,
    options: &Options,
    env: &dyn Fn(&str) -> Option<String>,
) -> Result<Recorded, Error> {
    let (new, warnings) = grown(root, options, env)?;
    if new.is_empty() {
        return Ok(Recorded {
            lines: vec!["nothing new to record".into()],
            warnings,
        });
    }
    let entries: Vec<Entry> = new.iter().map(|w| body(w).into()).collect();
    clm_ledger::append_all(root, entries, env)?;
    let mut lines: Vec<String> = new
        .iter()
        .map(|w| {
            let work = w
                .models
                .values()
                .fold(0u64, |n, u| n.saturating_add(u.work()));
            format!(
                "effort {} {} {work} tokens {}s",
                w.branch, w.session, w.active_seconds
            )
        })
        .collect();
    let branches: BTreeSet<&str> = new.iter().map(|w| w.branch.as_str()).collect();
    lines.push(format!(
        "recorded {} sessions on {} branches",
        new.len(),
        branches.len()
    ));
    Ok(Recorded { lines, warnings })
}

/// The sessions whose counts grew past the fold's, each count the larger of the two, and
/// the reading's warnings.
fn grown(
    root: &Path,
    options: &Options,
    env: &dyn Fn(&str) -> Option<String>,
) -> Result<(Vec<SessionWork>, Vec<String>), Error> {
    if !root.join("specs/SPEC.md").is_file() {
        return Err(Error::Refused(format!(
            "not a CLM repository: {} has no specs/SPEC.md",
            root.display()
        )));
    }
    let mut counted: Vec<PathBuf> = clm_git::worktrees(root)?
        .into_iter()
        .map(|w| w.path)
        .collect();
    counted.extend(options.cwds.iter().cloned());
    let mut warnings = Vec::new();
    let work = match options.transcripts.clone().or_else(|| transcripts_dir(env)) {
        Some(dir) => {
            let (work, mut read) = sessions(&dir, &counted, options.branch.as_deref());
            warnings.append(&mut read);
            work
        }
        None => {
            warnings.push("no transcripts: neither CLAUDE_CONFIG_DIR nor HOME is set".into());
            Vec::new()
        }
    };
    let fold = clm_ledger::fold(root)?;
    // Each count is the larger of what was read and what the fold holds, so a run that sees
    // less of a session never shrinks its record.
    let new: Vec<SessionWork> = work
        .into_iter()
        .filter_map(|w| {
            let held = fold
                .efforts
                .get(&clm_ledger::effort_id(&w.session, &w.branch));
            match held.map(|r| &r.body) {
                Some(
                    held @ Body::Effort {
                        models,
                        active_seconds,
                        ..
                    },
                ) => {
                    let merged = larger(w, models, *active_seconds);
                    (body(&merged) != *held).then_some(merged)
                }
                _ => Some(w),
            }
        })
        .collect();
    Ok((new, warnings))
}

/// `work` with each count, and the active seconds, the larger of its own and the held ones.
fn larger(mut work: SessionWork, held: &BTreeMap<String, Usage>, active: u64) -> SessionWork {
    for (model, h) in held {
        let u = work.models.entry(model.clone()).or_default();
        u.input = u.input.max(h.input);
        u.output = u.output.max(h.output);
        u.cache_read = u.cache_read.max(h.cache_read);
        u.cache_write = u.cache_write.max(h.cache_write);
    }
    work.active_seconds = work.active_seconds.max(active);
    work
}

fn body(work: &SessionWork) -> Body {
    Body::Effort {
        branch: work.branch.clone(),
        session: work.session.clone(),
        models: work.models.clone(),
        active_seconds: work.active_seconds,
    }
}
