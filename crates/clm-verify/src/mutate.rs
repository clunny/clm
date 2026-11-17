//! `mutation-pass`: after a green verification, the project's `[verify] mutate` command is
//! asked whether the tests that cite each requirement built since the last pass notice a change
//! to the requirement's component, and a `mutated` record says what it found, and what it
//! didn't reach.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::os::fd::AsFd;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use clm_coverage::{TestSpan, marker_spans, markers_in, owner, tests_in};
use clm_ledger::Body;

use crate::chain::State;
use crate::{Call, Ctx, Error, run, tree};

/// How many mutants beyond the code the lands changed the command may try per requirement.
const LIMIT: usize = 20;

/// The available memory, in kB, each requirement judged at once is given: 2 GiB.
const KB_PER_JOB: u64 = 2_097_152;

/// How often the pass looks at its command while it runs.
const POLL: Duration = Duration::from_millis(50);

/// A test citing a requirement at its current version.
pub(crate) struct Cite {
    pub(crate) id: String,
    pub(crate) path: String,
    pub(crate) name: String,
    first: usize,
    last: usize,
}

/// What the command said of one requirement.
enum Verdict {
    Killed,
    Survived(u64),
    NotJudged,
}

/// How the command ended.
enum Ended {
    /// By itself, with its exit status; none when a signal ended it.
    Exited(Option<i32>),
    /// At `[verify] mutate_minutes`, that many.
    Limit(u64),
    /// At the call's stop request.
    Stopped,
}

/// Runs the pass for the commit at position `end` of the chain, which passed, and returns the
/// notes it leaves for stderr. A failure of the pass is a note, never an error: it changes
/// nothing about the verification.
pub(crate) fn pass(ctx: &Ctx, state: &State, end: usize, call: &Call) -> Vec<String> {
    let Some(command) = ctx.verify.mutate.as_deref() else {
        return Vec::new();
    };
    match run_pass(ctx, state, end, command, call) {
        Ok(notes) => notes,
        Err(why) => vec![format!("mutation: {why}; nothing recorded")],
    }
}

fn run_pass(
    ctx: &Ctx,
    state: &State,
    end: usize,
    command: &str,
    call: &Call,
) -> Result<Vec<String>, Error> {
    let commit = state.chain[end].clone();
    let tree = tree::Tree::at(ctx, &commit)?;
    let since = state.mutation_base(ctx, end)?;
    let (graph, _) = clm_model::load(&tree.dir);
    let cites = cites(&tree.dir, &graph)?;
    let fresh = match &since {
        Some(since) => due(&tree.dir, since, &cites)?,
        None => cites.iter().map(|c| c.id.clone()).collect(),
    };
    // What the newest record left goes first, each with the commit its diff runs from, and
    // only while a test still cites it at its current version; then the rest in ID order.
    let mut blocks: Vec<(String, Option<String>)> = Vec::new();
    for (id, base) in state.carried(end) {
        if cites.iter().any(|c| c.id == id) && !blocks.iter().any(|(b, _)| *b == id) {
            blocks.push((id, base));
        }
    }
    for id in fresh {
        if !blocks.iter().any(|(b, _)| *b == id) {
            blocks.push((id, since.clone()));
        }
    }
    let mut notes = Vec::new();
    let mut verdicts = BTreeMap::new();
    if !blocks.is_empty() {
        let scratch = scratch(ctx)?;
        let plan = scratch.path().join("plan");
        let result = scratch.path().join("result");
        fs::write(
            &plan,
            write_plan(&tree.dir, &graph, &commit, &blocks, &cites, scratch.path())?,
        )?;
        fs::write(&result, "")?;
        match launch(ctx, call, &tree.dir, &commit, command, &plan, &result)? {
            Ended::Exited(Some(0)) => {}
            Ended::Exited(status) => {
                let status = status.map_or("a signal".to_string(), |s| s.to_string());
                notes.push(format!("mutation: {command} failed with status {status}"));
            }
            Ended::Limit(minutes) => notes.push(format!(
                "mutation: stopped at its limit of {minutes} minutes"
            )),
            Ended::Stopped => notes.push("mutation: stopped".to_string()),
        }
        verdicts = read_verdicts(&String::from_utf8_lossy(&fs::read(&result)?));
    }
    let (mut killed, mut unproven, mut not_judged, mut left) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for (id, _) in &blocks {
        match verdicts.get(id) {
            Some(Verdict::Killed) => killed.push(id.clone()),
            Some(Verdict::Survived(n)) if *n > 0 => {
                let version = graph.requirement(id).map(|r| r.version().to_string());
                unproven.push(format!("{id}@{}", version.unwrap_or_default()));
            }
            Some(_) => not_judged.push(id.clone()),
            None => left.push(id.clone()),
        }
    }
    let mut counts = format!(
        "mutation: {} killed, {} unproven, {} not judged",
        killed.len(),
        unproven.len(),
        not_judged.len()
    );
    if !left.is_empty() {
        counts.push_str(&format!(", {} left", left.len()));
    }
    notes.push(counts);
    clm_ledger::append(
        &ctx.root,
        Body::mutated(commit, since, killed, unproven, not_judged).with_left(left),
        ctx.env,
    )?;
    Ok(notes)
}

/// Every test of the tree citing a requirement of a component at its current version, groups
/// left out (`check.coverage/test-spans`).
pub(crate) fn cites(dir: &Path, graph: &clm_model::Graph) -> Result<Vec<Cite>, Error> {
    let mut found: Vec<Cite> = Vec::new();
    for path in clm_git::tracked_files(dir)? {
        let Ok(text) = fs::read_to_string(dir.join(&path)) else {
            continue;
        };
        if !text.contains("req:") {
            continue;
        }
        let spans = tests_in(&path, &text);
        for marker in markers_in(&path, &text) {
            let Some(version) = &marker.version else {
                continue;
            };
            let current = graph
                .requirement(&marker.id)
                .is_some_and(|r| r.version() == version);
            if !current || marker.id.split('/').next().is_some_and(is_layer) {
                continue;
            }
            let Some(test) = innermost(&spans, marker.line) else {
                continue;
            };
            let cite = Cite {
                id: marker.id,
                path: path.to_string_lossy().into_owned(),
                name: test.name.clone(),
                first: test.first,
                last: test.last,
            };
            let same = |c: &Cite| c.id == cite.id && c.path == cite.path && c.name == cite.name;
            if !found.iter().any(same) {
                found.push(cite);
            }
        }
    }
    Ok(found)
}

fn is_layer(component: &str) -> bool {
    component == "capability" || component == "architecture"
}

/// The smallest test (not group) holding `line`.
fn innermost(spans: &[TestSpan], line: usize) -> Option<&TestSpan> {
    spans
        .iter()
        .filter(|t| !t.group && t.first <= line && line <= t.last)
        .min_by_key(|t| t.last - t.first)
}

/// The requirements a test citing them was added or changed in more than its markers since
/// `since`.
fn due(dir: &Path, since: &str, cites: &[Cite]) -> Result<BTreeSet<String>, Error> {
    let changed: BTreeSet<String> =
        clm_git::git(dir, &["diff", "--name-only", "--relative", since, "HEAD"])?
            .lines()
            .map(str::to_string)
            .collect();
    let mut due = BTreeSet::new();
    for cite in cites {
        if due.contains(&cite.id) || !changed.contains(&cite.path) {
            continue;
        }
        let path = Path::new(&cite.path);
        let now = fs::read_to_string(dir.join(path))?;
        let before = clm_git::git(dir, &["show", &format!("{since}:./{}", cite.path)])
            .ok()
            .and_then(|text| {
                tests_in(path, &text)
                    .into_iter()
                    .find(|t| !t.group && t.name == cite.name)
                    .map(|t| body(&text, t.first, t.last))
            });
        if before.is_none_or(|before| before != body(&now, cite.first, cite.last)) {
            due.insert(cite.id.clone());
        }
    }
    Ok(due)
}

/// Lines `first` to `last` of `text` without their markers, and without the lines a marker
/// leaves nothing else on.
fn body(text: &str, first: usize, last: usize) -> Vec<String> {
    text.lines()
        .skip(first - 1)
        .take(last + 1 - first)
        .filter_map(|line| {
            let mut kept = String::new();
            let mut at = 0;
            for m in marker_spans(line) {
                kept.push_str(&line[at..m.range.start]);
                at = m.range.end;
            }
            kept.push_str(&line[at..]);
            let kept = kept.trim();
            let bare =
                kept.trim_matches(|c: char| matches!(c, '/' | '#' | '*') || c.is_whitespace());
            (!bare.is_empty()).then(|| kept.to_string())
        })
        .collect()
}

/// A directory for the plan, the result and the diff, in `TMPDIR`.
pub(crate) fn scratch(ctx: &Ctx) -> Result<tempfile::TempDir, Error> {
    let parent = (ctx.env)("TMPDIR")
        .filter(|t| !t.is_empty())
        .map_or_else(std::env::temp_dir, PathBuf::from);
    Ok(tempfile::Builder::new()
        .prefix("verify-mutate-")
        .tempdir_in(std::path::absolute(parent)?)?)
}

/// The plan text for the due requirements, in the order given: per requirement, its
/// component's code files, the diff of them from its base when there is one, its citing tests
/// and the limit.
fn write_plan(
    dir: &Path,
    graph: &clm_model::Graph,
    commit: &str,
    blocks: &[(String, Option<String>)],
    cites: &[Cite],
    scratch: &Path,
) -> Result<String, Error> {
    let tracked = clm_git::tracked_files(dir)?;
    let mut plan = String::new();
    for (n, (id, base)) in blocks.iter().enumerate() {
        let component = id.split('/').next().unwrap_or_default();
        let code: Vec<String> = tracked
            .iter()
            .filter(|p| owner(graph, p) == Some(component))
            .map(|p| p.to_string_lossy().into_owned())
            .collect();
        plan.push_str(&format!("requirement {id}\n"));
        for path in &code {
            plan.push_str(&format!("code {path}\n"));
        }
        if let (Some(base), false) = (base, code.is_empty()) {
            let mut args = vec!["diff", base.as_str(), commit, "--"];
            args.extend(code.iter().map(String::as_str));
            let diff = clm_git::git(dir, &args)?;
            if !diff.is_empty() {
                let file = scratch.join(format!("diff-{n}"));
                fs::write(&file, format!("{diff}\n"))?;
                plan.push_str(&format!("diff {}\n", file.display()));
            }
        }
        for cite in cites.iter().filter(|c| &c.id == id) {
            plan.push_str(&format!("test {}\t{}\n", cite.path, cite.name));
        }
        plan.push_str(&format!("limit {LIMIT}\n"));
    }
    Ok(plan)
}

/// How many requirements the command may judge at once: one per 2 GiB of the `MemAvailable`
/// line of `memory`, at least 1 and at most `ceiling`; 1 with no ceiling, or when the file or
/// the line can't be read.
fn jobs(memory: &Path, ceiling: Option<u64>) -> u64 {
    let Some(ceiling) = ceiling else {
        return 1;
    };
    let available = fs::read_to_string(memory).ok().and_then(|text| {
        text.lines()
            .find_map(|l| l.strip_prefix("MemAvailable:"))
            .and_then(|rest| rest.split_whitespace().next()?.parse::<u64>().ok())
    });
    available.map_or(1, |kb| (kb / KB_PER_JOB).min(ceiling).max(1))
}

/// Runs the command through `sh -c` in the verify tree, in a process group of its own, its
/// output copied to stderr, with slots of its own and no check lock, and ends the group at
/// `[verify] mutate_minutes` or at the call's stop request.
fn launch(
    ctx: &Ctx,
    call: &Call,
    dir: &Path,
    commit: &str,
    command: &str,
    plan: &Path,
    result: &Path,
) -> Result<Ended, Error> {
    let build = tree::places(ctx)?.join("verify-build");
    let output = std::io::stderr().as_fd().try_clone_to_owned()?;
    let jobs = jobs(call.memory, ctx.verify.mutate_jobs);
    let mut shell = Command::new("sh");
    shell
        .arg("-c")
        .arg(command)
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::from(output))
        .process_group(0)
        .env_remove("CLM_SELECTION")
        .envs(run::env_of(ctx, commit)?)
        .env("CLM_LAND_DIR", build)
        .env("CLM_MUTATE", plan)
        .env("CLM_MUTATE_RESULT", result)
        .env("CLM_MUTATE_JOBS", jobs.to_string())
        .env("CLM_SLOT", (ctx.max_builds + 1).to_string());
    let mut child = shell.spawn()?;
    let group = group_of(&child);
    let limit = ctx.verify.mutate_minutes.map(|m| {
        (
            m,
            call.minute
                .saturating_mul(u32::try_from(m).unwrap_or(u32::MAX)),
        )
    });
    let started = Instant::now();
    let watched = (|| loop {
        if let Some(status) = child.try_wait()? {
            return Ok(Ended::Exited(status.code()));
        }
        let why = match limit {
            _ if call.stop.load(Ordering::SeqCst) => Some(Ended::Stopped),
            Some((minutes, length)) if started.elapsed() >= length => Some(Ended::Limit(minutes)),
            _ => None,
        };
        if let Some(why) = why {
            end_group(&mut child, call.grace)?;
            return Ok(why);
        }
        std::thread::sleep(POLL);
    })();
    if watched.is_err() {
        // Nothing watches the command once the pass gives up on it, so it ends here.
        signal(group, libc::SIGKILL);
    }
    watched
}

/// Sends `signal` to the process group `group`, and says whether the group holds a process.
fn signal(group: libc::pid_t, signal: libc::c_int) -> bool {
    // SAFETY: `killpg` takes plain integers and touches no memory of ours.
    unsafe { libc::killpg(group, signal) == 0 }
}

/// Sends `SIGTERM` to the group `child` leads, and `SIGKILL` when it hasn't ended after
/// `grace`. The command has ended when no process of its group is left: `sh -c` may end at
/// once while a script it started ignores the signal.
fn end_group(child: &mut Child, grace: Duration) -> Result<(), Error> {
    let group = group_of(child);
    signal(group, libc::SIGTERM);
    let asked = Instant::now();
    let ended = |child: &mut Child| -> Result<bool, Error> {
        // Signal 0 only asks whether the group holds a process; once the leader is reaped
        // and the group is empty its ID may be reused, so nothing is sent to it then.
        Ok(child.try_wait()?.is_some() && !signal(group, 0))
    };
    while asked.elapsed() < grace {
        if ended(child)? {
            return Ok(());
        }
        std::thread::sleep(POLL);
    }
    if !ended(child)? {
        signal(group, libc::SIGKILL);
        child.wait()?;
    }
    Ok(())
}

/// The process group `child` leads: it was started with `process_group(0)`, so its ID is
/// the group's.
fn group_of(child: &Child) -> libc::pid_t {
    child.id() as libc::pid_t
}

/// The verdict lines of the result file that end in a newline, the last for a requirement
/// holding: `<id> killed`, `<id> survived <n>` and `<id> not-judged`; a line that doesn't
/// parse is ignored.
fn read_verdicts(text: &str) -> BTreeMap<String, Verdict> {
    let mut out = BTreeMap::new();
    for line in text.split_inclusive('\n').filter(|l| l.ends_with('\n')) {
        let words: Vec<&str> = line.split_whitespace().collect();
        let verdict = match words.as_slice() {
            [_, "killed"] => Verdict::Killed,
            [_, "not-judged"] => Verdict::NotJudged,
            [_, "survived", n] => match n.parse() {
                Ok(n) => Verdict::Survived(n),
                Err(_) => continue,
            },
            _ => continue,
        };
        out.insert(words[0].to_string(), verdict);
    }
    out
}
