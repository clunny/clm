//! `mutation-pass`: after a green verification, the project's `[verify] mutate` command is
//! asked whether the tests that cite each requirement built since the last pass notice a change
//! to the requirement's component, and a `mutated` record says what it found.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::os::fd::AsFd;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use clm_coverage::{TestSpan, marker_spans, markers_in, owner, tests_in};
use clm_ledger::Body;

use crate::chain::State;
use crate::{Ctx, Error, run, tree};

/// How many mutants beyond the code the lands changed the command may try per requirement.
const LIMIT: usize = 20;

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
}

/// Runs the pass for the commit at position `end` of the chain, which passed, and returns the
/// notes it leaves for stderr. A failure of the pass is a note, never an error: it changes
/// nothing about the verification.
pub(crate) fn pass(ctx: &Ctx, state: &State, end: usize) -> Vec<String> {
    let Some(command) = ctx.verify.mutate.as_deref() else {
        return Vec::new();
    };
    match run_pass(ctx, state, end, command) {
        Ok(notes) => notes,
        Err(why) => vec![format!("mutation: {why}; nothing recorded")],
    }
}

fn run_pass(ctx: &Ctx, state: &State, end: usize, command: &str) -> Result<Vec<String>, Error> {
    let commit = state.chain[end].clone();
    let tree = tree::Tree::at(ctx, &commit)?;
    let since = state.mutation_base(end);
    let (graph, _) = clm_model::load(&tree.dir);
    let cites = cites(&tree.dir, &graph)?;
    let due = match &since {
        Some(since) => due(&tree.dir, since, &cites)?,
        None => BTreeSet::new(),
    };
    let mut notes = Vec::new();
    let mut verdicts = BTreeMap::new();
    if let (Some(since), false) = (&since, due.is_empty()) {
        let scratch = scratch(ctx)?;
        let plan = scratch.path().join("plan");
        let result = scratch.path().join("result");
        fs::write(
            &plan,
            write_plan(
                &tree.dir,
                &graph,
                since,
                &commit,
                &due,
                &cites,
                scratch.path(),
            )?,
        )?;
        fs::write(&result, "")?;
        let status = launch(ctx, &tree.dir, &commit, command, &plan, &result)?;
        if status != Some(0) {
            let status = status.map_or("a signal".to_string(), |s| s.to_string());
            return Ok(vec![format!(
                "mutation: {command} failed with status {status}; nothing recorded"
            )]);
        }
        verdicts = read_verdicts(&fs::read_to_string(&result)?);
    }
    let (mut killed, mut unproven, mut not_judged) = (Vec::new(), Vec::new(), Vec::new());
    for id in &due {
        match verdicts.get(id) {
            Some(Verdict::Killed) => killed.push(id.clone()),
            Some(Verdict::Survived(n)) if *n > 0 => {
                let version = graph.requirement(id).map(|r| r.version().to_string());
                unproven.push(format!("{id}@{}", version.unwrap_or_default()));
            }
            _ => not_judged.push(id.clone()),
        }
    }
    notes.push(format!(
        "mutation: {} killed, {} unproven, {} not judged",
        killed.len(),
        unproven.len(),
        not_judged.len()
    ));
    clm_ledger::append(
        &ctx.root,
        Body::Mutated {
            commit,
            since,
            killed,
            unproven,
            not_judged,
        },
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

/// The plan text for the due requirements: per requirement in ID order, its component's code
/// files, the diff of them since `since` when there is one, its citing tests and the limit.
#[allow(clippy::too_many_arguments)]
fn write_plan(
    dir: &Path,
    graph: &clm_model::Graph,
    since: &str,
    commit: &str,
    due: &BTreeSet<String>,
    cites: &[Cite],
    scratch: &Path,
) -> Result<String, Error> {
    let tracked = clm_git::tracked_files(dir)?;
    let mut plan = String::new();
    for (n, id) in due.iter().enumerate() {
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
        if !code.is_empty() {
            let mut args = vec!["diff", since, commit, "--"];
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

/// Runs the command through `sh -c` in the verify tree, its output copied to stderr, with a
/// slot of its own and no check lock, and returns its exit status.
fn launch(
    ctx: &Ctx,
    dir: &Path,
    commit: &str,
    command: &str,
    plan: &Path,
    result: &Path,
) -> Result<Option<i32>, Error> {
    let build = tree::places(ctx)?.join("verify-build");
    let output = std::io::stderr().as_fd().try_clone_to_owned()?;
    let mut shell = Command::new("sh");
    shell
        .arg("-c")
        .arg(command)
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::from(output))
        .env_remove("CLM_SELECTION")
        .envs(run::env_of(ctx, commit)?)
        .env("CLM_LAND_DIR", build)
        .env("CLM_MUTATE", plan)
        .env("CLM_MUTATE_RESULT", result)
        .env("CLM_SLOT", (ctx.max_builds + 1).to_string());
    Ok(shell.status()?.code())
}

/// The verdict lines of the result file: `<id> killed` and `<id> survived <n>`; a line saying
/// `<id> not-judged`, or anything else, leaves the requirement not judged.
fn read_verdicts(text: &str) -> BTreeMap<String, Verdict> {
    let mut out = BTreeMap::new();
    for line in text.lines() {
        let words: Vec<&str> = line.split_whitespace().collect();
        let verdict = match words.as_slice() {
            [_, "killed"] => Verdict::Killed,
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
