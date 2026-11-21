//! `effort/transcripts`: which transcript entries count, and how.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use clm_effort::Checkout::{self, Branch, Detached, Unlinked};

/// The branch pinned for each `<session>/<agent>`.
fn pins(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(s, b)| (s.to_string(), b.to_string()))
        .collect()
}

/// Claude Code's project directory name for `dir`: every character but an ASCII letter or
/// digit replaced by `-`.
fn encoded(dir: &Path) -> String {
    dir.to_string_lossy()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

/// The time `secs` seconds after 2026-10-30T00:00:00Z.
fn ts(secs: u64) -> String {
    format!(
        "2026-10-30T{:02}:{:02}:{:02}.000Z",
        secs / 3600,
        secs / 60 % 60,
        secs % 60
    )
}

/// One assistant entry of session `sid` on `branch`, run in `cwd`.
fn entry(
    sid: &str,
    branch: &str,
    cwd: &Path,
    secs: u64,
    id: &str,
    model: &str,
    usage: (u64, u64),
) -> String {
    serde_json::json!({
        "sessionId": sid,
        "gitBranch": branch,
        "cwd": cwd,
        "timestamp": ts(secs),
        "type": "assistant",
        "message": {
            "id": id,
            "model": model,
            "usage": {"input_tokens": usage.0, "output_tokens": usage.1},
        },
    })
    .to_string()
}

/// `lines` written to `<transcripts>/<encoded cwd>/<file>`.
fn write(transcripts: &Path, cwd: &Path, file: &str, lines: &[String]) {
    let path = transcripts.join(encoded(cwd)).join(file);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, lines.join("\n") + "\n").unwrap();
}

struct Fixture {
    _dir: tempfile::TempDir,
    transcripts: PathBuf,
    cwd: PathBuf,
}

fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let transcripts = dir.path().join("projects");
    let cwd = dir.path().join("x").join("clm");
    Fixture {
        transcripts,
        cwd,
        _dir: dir,
    }
}

const OPUS: &str = "claude-opus-5-5";

#[test]
fn repeated_message_counted_once() {
    // req:effort/transcripts@50c1e34f
    let f = fixture();
    let line = entry("s1", "impl/a", &f.cwd, 0, "m1", OPUS, (10, 5));
    write(&f.transcripts, &f.cwd, "s1.jsonl", &[line.clone(), line]);
    let (work, warnings) = clm_effort::sessions(
        &f.transcripts,
        &[(f.cwd.clone(), Unlinked)],
        &pins(&[]),
        None,
    );
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(work.len(), 1, "{work:?}");
    let usage = work[0].models[OPUS];
    assert_eq!((usage.input, usage.output), (10, 5));
    assert_eq!(
        (work[0].session.as_str(), work[0].branch.as_str()),
        ("s1", "impl/a")
    );
}

#[test]
fn sessions_split_by_branch() {
    // req:effort/transcripts@50c1e34f
    let f = fixture();
    write(
        &f.transcripts,
        &f.cwd,
        "s1.jsonl",
        &[
            entry("s1", "impl/a", &f.cwd, 0, "m1", OPUS, (1, 1)),
            entry("s1", "impl/b", &f.cwd, 10, "m2", OPUS, (2, 2)),
        ],
    );
    let (work, _) = clm_effort::sessions(
        &f.transcripts,
        &[(f.cwd.clone(), Unlinked)],
        &pins(&[]),
        None,
    );
    let branches: Vec<&str> = work.iter().map(|w| w.branch.as_str()).collect();
    assert_eq!(branches, ["impl/a", "impl/b"]);
    // Only one branch's when asked.
    let (work, _) = clm_effort::sessions(
        &f.transcripts,
        &[(f.cwd.clone(), Unlinked)],
        &pins(&[]),
        Some("impl/b"),
    );
    assert_eq!(work.len(), 1);
    assert_eq!(work[0].models[OPUS].input, 2);
}

#[test]
fn subagent_file_joins_its_session() {
    // req:effort/transcripts@50c1e34f
    let f = fixture();
    write(
        &f.transcripts,
        &f.cwd,
        "s1.jsonl",
        &[entry("s1", "impl/a", &f.cwd, 0, "m1", OPUS, (10, 1))],
    );
    let sonnet = "claude-sonnet-5-5";
    write(
        &f.transcripts,
        &f.cwd,
        "s1/subagents/agent-x.jsonl",
        &[entry("s1", "impl/a", &f.cwd, 30, "m2", sonnet, (7, 2))],
    );
    let (work, _) = clm_effort::sessions(
        &f.transcripts,
        &[(f.cwd.clone(), Unlinked)],
        &pins(&[]),
        None,
    );
    assert_eq!(work.len(), 1, "{work:?}");
    assert_eq!(work[0].models[OPUS].input, 10);
    assert_eq!(work[0].models[sonnet].input, 7);
    assert_eq!(work[0].active_seconds, 30);
}

#[test]
fn synthetic_model_skipped() {
    // req:effort/transcripts@50c1e34f
    let f = fixture();
    write(
        &f.transcripts,
        &f.cwd,
        "s1.jsonl",
        &[
            entry("s1", "impl/a", &f.cwd, 0, "m1", OPUS, (10, 5)),
            entry("s1", "impl/a", &f.cwd, 5, "m2", "<synthetic>", (99, 99)),
        ],
    );
    let (work, _) = clm_effort::sessions(
        &f.transcripts,
        &[(f.cwd.clone(), Unlinked)],
        &pins(&[]),
        None,
    );
    assert_eq!(work.len(), 1);
    assert_eq!(work[0].models.keys().collect::<Vec<_>>(), [OPUS]);
    assert_eq!(work[0].models[OPUS].input, 10);
}

#[test]
fn active_time_leaves_out_long_gaps() {
    // req:effort/transcripts@50c1e34f
    let f = fixture();
    let lines: Vec<String> = [0, 60, 120, 1000]
        .iter()
        .enumerate()
        .map(|(n, s)| entry("s1", "impl/a", &f.cwd, *s, &format!("m{n}"), OPUS, (1, 1)))
        .collect();
    write(&f.transcripts, &f.cwd, "s1.jsonl", &lines);
    let (work, _) = clm_effort::sessions(
        &f.transcripts,
        &[(f.cwd.clone(), Unlinked)],
        &pins(&[]),
        None,
    );
    assert_eq!(work[0].active_seconds, 120);
    assert_eq!(work[0].models[OPUS].input, 4);
}

#[test]
fn other_cwd_not_counted() {
    // req:effort/transcripts@50c1e34f
    // `/x/clm-wt`'s directory name starts with `/x/clm`'s, so it is read, but its entries
    // aren't under the counted directory.
    let f = fixture();
    let other = f.cwd.with_file_name("clm-wt");
    write(
        &f.transcripts,
        &other,
        "s1.jsonl",
        &[entry("s1", "impl/a", &other, 0, "m1", OPUS, (10, 5))],
    );
    let (work, _) = clm_effort::sessions(
        &f.transcripts,
        &[(f.cwd.clone(), Unlinked)],
        &pins(&[]),
        None,
    );
    assert!(work.is_empty(), "{work:?}");
}

#[test]
fn bad_lines_warned_once() {
    // req:effort/transcripts@50c1e34f
    let f = fixture();
    let good = entry("s1", "impl/a", &f.cwd, 0, "m1", OPUS, (10, 5));
    write(
        &f.transcripts,
        &f.cwd,
        "s1.jsonl",
        &["{not json".into(), good, "nor this".into()],
    );
    let (work, warnings) = clm_effort::sessions(
        &f.transcripts,
        &[(f.cwd.clone(), Unlinked)],
        &pins(&[]),
        None,
    );
    assert_eq!(work.len(), 1);
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert_eq!(warnings[0], "skipped 2 transcript lines that aren't JSON");
}

#[test]
fn missing_dir_warns() {
    // req:effort/transcripts@50c1e34f
    let f = fixture();
    let (work, warnings) = clm_effort::sessions(
        &f.transcripts,
        &[(f.cwd.clone(), Unlinked)],
        &pins(&[]),
        None,
    );
    assert!(work.is_empty());
    assert_eq!(
        warnings,
        [format!("no transcripts at {}", f.transcripts.display())]
    );
}

/// An entry of session `s1` on `impl/a` with the given `usage` object.
fn with_usage(cwd: &Path, secs: u64, id: &str, usage: serde_json::Value) -> String {
    serde_json::json!({
        "sessionId": "s1",
        "gitBranch": "impl/a",
        "cwd": cwd,
        "timestamp": ts(secs),
        "message": {"id": id, "model": OPUS, "usage": usage},
    })
    .to_string()
}

#[test]
fn usage_fields_mapped() {
    // req:effort/transcripts@50c1e34f
    let f = fixture();
    let usage = serde_json::json!({
        "input_tokens": 1,
        "output_tokens": 2,
        "cache_read_input_tokens": 3,
        "cache_creation_input_tokens": 4,
    });
    write(
        &f.transcripts,
        &f.cwd,
        "s1.jsonl",
        &[with_usage(&f.cwd, 0, "m1", usage)],
    );
    let (work, _) = clm_effort::sessions(
        &f.transcripts,
        &[(f.cwd.clone(), Unlinked)],
        &pins(&[]),
        None,
    );
    let u = work[0].models[OPUS];
    assert_eq!(
        (u.input, u.output, u.cache_read, u.cache_write),
        (1, 2, 3, 4)
    );
}

#[test]
fn repeated_message_takes_the_largest_counts() {
    // req:effort/transcripts@50c1e34f
    // A response's first content-block line may carry a count still streaming.
    let f = fixture();
    let early = serde_json::json!({"input_tokens": 10, "output_tokens": 12});
    let late = serde_json::json!({"input_tokens": 10, "output_tokens": 850});
    write(
        &f.transcripts,
        &f.cwd,
        "s1.jsonl",
        &[
            with_usage(&f.cwd, 0, "m1", early),
            with_usage(&f.cwd, 1, "m1", late),
        ],
    );
    let (work, _) = clm_effort::sessions(
        &f.transcripts,
        &[(f.cwd.clone(), Unlinked)],
        &pins(&[]),
        None,
    );
    let u = work[0].models[OPUS];
    assert_eq!((u.input, u.output), (10, 850));
}

#[test]
fn a_gap_of_600_seconds_is_active() {
    // req:effort/transcripts@50c1e34f
    let f = fixture();
    let lines = [
        entry("s1", "impl/a", &f.cwd, 0, "m1", OPUS, (1, 1)),
        entry("s1", "impl/a", &f.cwd, 600, "m2", OPUS, (1, 1)),
        entry("s1", "impl/a", &f.cwd, 1201, "m3", OPUS, (1, 1)),
    ];
    write(&f.transcripts, &f.cwd, "s1.jsonl", &lines);
    let (work, _) = clm_effort::sessions(
        &f.transcripts,
        &[(f.cwd.clone(), Unlinked)],
        &pins(&[]),
        None,
    );
    assert_eq!(work[0].active_seconds, 600);
}

#[test]
fn a_line_that_isnt_utf8_is_skipped_alone() {
    // req:effort/transcripts@50c1e34f
    let f = fixture();
    let good = entry("s1", "impl/a", &f.cwd, 0, "m1", OPUS, (10, 5));
    let path = f.transcripts.join(encoded(&f.cwd)).join("s1.jsonl");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut bytes = good.into_bytes();
    bytes.extend_from_slice(b"\n{\"half\": \"\xc3");
    std::fs::write(&path, bytes).unwrap();
    let (work, warnings) = clm_effort::sessions(
        &f.transcripts,
        &[(f.cwd.clone(), Unlinked)],
        &pins(&[]),
        None,
    );
    assert_eq!(work.len(), 1, "{warnings:?}");
    assert_eq!(work[0].models[OPUS].input, 10);
    assert_eq!(warnings, ["skipped 1 transcript lines that aren't JSON"]);
}

/// The main checkout `<dir>/x/clm`, counted as no linked worktree, and a linked worktree
/// under it, counted with `branch` checked out, or detached.
fn with_worktree(f: &Fixture, branch: Option<&str>) -> (PathBuf, Vec<(PathBuf, Checkout)>) {
    let worktree = f.cwd.join(".claude").join("worktrees").join("a");
    let held = branch.map_or(Detached, |b| Branch(b.to_string()));
    let counted = vec![(f.cwd.clone(), Unlinked), (worktree.clone(), held)];
    (worktree, counted)
}

/// The main checkout alone: the worktree it held is gone.
fn worktree_gone(f: &Fixture) -> Vec<(PathBuf, Checkout)> {
    vec![(f.cwd.clone(), Unlinked)]
}

/// The branch and session `work` counts for, with each one's input tokens.
fn by_branch(work: &[clm_effort::SessionWork]) -> Vec<(&str, &str, u64)> {
    work.iter()
        .map(|w| {
            (
                w.branch.as_str(),
                w.session.as_str(),
                w.models.values().map(|u| u.input).sum(),
            )
        })
        .collect()
}

/// A subagent `agent-x` of session `s1`, launched on `main`, whose entries ran in `worktree`.
fn subagent(f: &Fixture, worktree: &Path) {
    write(
        &f.transcripts,
        &f.cwd,
        "s1/subagents/agent-x.jsonl",
        &[
            entry("s1", "main", worktree, 0, "m1", OPUS, (7, 1)),
            entry(
                "s1",
                "main",
                &worktree.join("crates"),
                30,
                "m2",
                OPUS,
                (3, 1),
            ),
        ],
    );
}

#[test]
fn subagent_in_a_worktree_counts_for_its_branch() {
    // req:effort/transcripts@50c1e34f
    let f = fixture();
    let (worktree, counted) = with_worktree(&f, Some("impl/a"));
    // The session runs in the main checkout, on `main`; its subagent in the worktree.
    subagent(&f, &worktree);
    let (work, _) = clm_effort::sessions(&f.transcripts, &counted, &pins(&[]), None);
    assert_eq!(by_branch(&work), [("impl/a", "s1/agent-x", 10)], "{work:?}");
    assert_eq!(work[0].active_seconds, 30);
    // A filter on the branch the work counts for finds it, though no line names it.
    let (work, _) = clm_effort::sessions(&f.transcripts, &counted, &pins(&[]), Some("impl/a"));
    assert_eq!(by_branch(&work), [("impl/a", "s1/agent-x", 10)], "{work:?}");
    let (work, _) = clm_effort::sessions(&f.transcripts, &counted, &pins(&[]), Some("main"));
    assert!(work.is_empty(), "{work:?}");
}

#[test]
fn subagent_in_a_detached_worktree_counts_nothing() {
    // req:effort/transcripts@50c1e34f
    let f = fixture();
    let (worktree, counted) = with_worktree(&f, None);
    subagent(&f, &worktree);
    let (work, _) = clm_effort::sessions(&f.transcripts, &counted, &pins(&[]), None);
    assert!(work.is_empty(), "{work:?}");
}

#[test]
fn pinned_subagent_whose_worktree_is_gone() {
    // req:effort/transcripts@50c1e34f
    let f = fixture();
    let (worktree, _) = with_worktree(&f, Some("impl/a"));
    subagent(&f, &worktree);
    let pinned = pins(&[("s1/agent-x", "impl/a")]);
    let (work, _) = clm_effort::sessions(&f.transcripts, &worktree_gone(&f), &pinned, None);
    assert_eq!(by_branch(&work), [("impl/a", "s1/agent-x", 10)], "{work:?}");
}

#[test]
fn pinned_subagent_in_a_reused_worktree() {
    // req:effort/transcripts@50c1e34f
    let f = fixture();
    let (worktree, counted) = with_worktree(&f, Some("impl/b"));
    subagent(&f, &worktree);
    let pinned = pins(&[("s1/agent-x", "impl/a")]);
    let (work, _) = clm_effort::sessions(&f.transcripts, &counted, &pinned, None);
    assert_eq!(by_branch(&work), [("impl/a", "s1/agent-x", 10)], "{work:?}");
    let (work, _) = clm_effort::sessions(&f.transcripts, &counted, &pinned, Some("impl/b"));
    assert!(work.is_empty(), "{work:?}");
}

#[test]
fn pinned_subagent_under_a_branch_filter() {
    // req:effort/transcripts@50c1e34f
    // No line names `impl/a`, and the worktree that had it checked out is gone.
    let f = fixture();
    let (worktree, _) = with_worktree(&f, Some("impl/a"));
    subagent(&f, &worktree);
    let pinned = pins(&[("s1/agent-x", "impl/a")]);
    let gone = worktree_gone(&f);
    let (work, _) = clm_effort::sessions(&f.transcripts, &gone, &pinned, Some("impl/a"));
    assert_eq!(by_branch(&work), [("impl/a", "s1/agent-x", 10)], "{work:?}");
    // Another agent's pin isn't this file's.
    let other = pins(&[("s1/agent-y", "impl/a")]);
    let (work, _) = clm_effort::sessions(&f.transcripts, &gone, &other, Some("impl/a"));
    assert!(work.is_empty(), "{work:?}");
}

#[test]
fn pinned_subagent_outside_every_counted_directory_counts_nothing() {
    // req:effort/transcripts@50c1e34f
    // A pinned entry still counts only when its `cwd` is under a counted directory.
    let f = fixture();
    let away = f.cwd.with_file_name("away");
    subagent(&f, &away);
    let pinned = pins(&[("s1/agent-x", "impl/a")]);
    let (work, _) = clm_effort::sessions(&f.transcripts, &worktree_gone(&f), &pinned, None);
    assert!(work.is_empty(), "{work:?}");
}

#[test]
fn unpinned_subagent_whose_worktree_is_gone() {
    // req:effort/transcripts@50c1e34f
    let f = fixture();
    let (worktree, _) = with_worktree(&f, Some("impl/a"));
    subagent(&f, &worktree);
    let (work, _) = clm_effort::sessions(&f.transcripts, &worktree_gone(&f), &pins(&[]), None);
    assert_eq!(by_branch(&work), [("main", "s1", 10)], "{work:?}");
}

#[test]
fn subagent_in_the_main_checkout_keeps_its_branch() {
    // req:effort/transcripts@50c1e34f
    // The main checkout holds the worktree, but isn't a linked worktree itself.
    let f = fixture();
    let (_, counted) = with_worktree(&f, Some("impl/a"));
    write(
        &f.transcripts,
        &f.cwd,
        "s1/subagents/agent-x.jsonl",
        &[entry("s1", "main", &f.cwd, 0, "m1", OPUS, (7, 1))],
    );
    let (work, _) = clm_effort::sessions(&f.transcripts, &counted, &pins(&[]), None);
    assert_eq!(by_branch(&work), [("main", "s1", 7)], "{work:?}");
}

#[test]
fn own_entries_in_a_worktree_keep_their_branch() {
    // req:effort/transcripts@50c1e34f
    let f = fixture();
    let (worktree, counted) = with_worktree(&f, Some("impl/a"));
    write(
        &f.transcripts,
        &worktree,
        "s1.jsonl",
        &[entry("s1", "spec/x", &worktree, 0, "m1", OPUS, (7, 1))],
    );
    let (work, _) = clm_effort::sessions(&f.transcripts, &counted, &pins(&[]), None);
    assert_eq!(by_branch(&work), [("spec/x", "s1", 7)], "{work:?}");
}

/// `lines`, raw bytes each, written to `<transcripts>/<encoded cwd>/<file>`.
fn write_bytes(transcripts: &Path, cwd: &Path, file: &str, lines: &[&[u8]]) {
    let path = transcripts.join(encoded(cwd)).join(file);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, lines.join(&b'\n')).unwrap();
}

#[test]
fn how_counts_are_read() {
    // req:effort/transcripts@50c1e34f
    let f = fixture();
    let early = serde_json::json!({"input_tokens": 10, "output_tokens": 12});
    let late = serde_json::json!({"input_tokens": 10, "output_tokens": 850});
    // No `message.id`, and an `output_tokens` that isn't written as an integer.
    let no_id = serde_json::json!({
        "sessionId": "s1", "gitBranch": "impl/a", "cwd": f.cwd, "timestamp": ts(0),
        "message": {"model": OPUS, "usage": {"input_tokens": 5, "output_tokens": "7"}},
    })
    .to_string();
    let no_message = serde_json::json!({
        "sessionId": "s1", "gitBranch": "impl/a", "cwd": f.cwd, "timestamp": ts(60),
    })
    .to_string();
    write(
        &f.transcripts,
        &f.cwd,
        "s1.jsonl",
        &[
            with_usage(&f.cwd, 0, "m1", early),
            with_usage(&f.cwd, 0, "m1", late),
            no_id.clone(),
            no_id,
            no_message,
        ],
    );
    let (work, warnings) = clm_effort::sessions(
        &f.transcripts,
        &[(f.cwd.clone(), Unlinked)],
        &pins(&[]),
        None,
    );
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(work.len(), 1, "{work:?}");
    let u = work[0].models[OPUS];
    assert_eq!((u.input, u.output), (20, 850));
    assert_eq!(work[0].active_seconds, 60);
}

/// An entry of session `sid` on `impl/a` with `timestamp` as written, a response of its own
/// with `input_tokens` 1, or with no `message` when `id` is `None`.
fn at(cwd: &Path, sid: &str, timestamp: &str, id: Option<&str>) -> String {
    let mut entry = serde_json::json!({
        "sessionId": sid, "gitBranch": "impl/a", "cwd": cwd, "timestamp": timestamp,
    });
    if let Some(id) = id {
        entry["message"] =
            serde_json::json!({"id": id, "model": OPUS, "usage": {"input_tokens": 1}});
    }
    entry.to_string()
}

#[test]
fn timestamps() {
    // req:effort/transcripts@50c1e34f
    let f = fixture();
    let s1 = [
        "2026-10-30T00:02:00Z",
        "2026-10-30T00:00:00.000Z",
        "2026-10-30T02:01:00.5+02:00",
        "yesterday",
        "2026-10-30T0:03:00Z",
        "+2026-10-30T00:04:00Z",
        "2026-10-30T00:05:00.Z",
        "2026-10-30T00:05:75Z",
    ];
    let s3 = [
        "2027-01-01T00:01:00Z",
        "2026-13-01T00:00:00Z",
        "2026-12-31T24:02:00Z",
    ];
    let mut lines: Vec<String> = s1
        .iter()
        .enumerate()
        .map(|(n, t)| at(&f.cwd, "s1", t, Some(&format!("a{n}"))))
        .collect();
    lines.push(at(&f.cwd, "s2", "yesterday", None));
    lines.extend(
        s3.iter()
            .enumerate()
            .map(|(n, t)| at(&f.cwd, "s3", t, Some(&format!("c{n}")))),
    );
    write(&f.transcripts, &f.cwd, "s.jsonl", &lines);
    let (work, warnings) = clm_effort::sessions(
        &f.transcripts,
        &[(f.cwd.clone(), Unlinked)],
        &pins(&[]),
        None,
    );
    assert!(warnings.is_empty(), "{warnings:?}");
    let got: Vec<(&str, u64, u64)> = work
        .iter()
        .map(|w| (w.session.as_str(), w.models[OPUS].input, w.active_seconds))
        .collect();
    assert_eq!(got, [("s1", 8, 120), ("s3", 3, 0)], "{work:?}");
}

#[test]
fn lines_that_count_nothing() {
    // req:effort/transcripts@50c1e34f
    let f = fixture();
    let one = entry("s1", "impl/a", &f.cwd, 0, "m1", OPUS, (10, 5));
    let no_branch = serde_json::json!({
        "sessionId": "s1", "cwd": f.cwd, "timestamp": ts(5),
        "message": {"id": "m2", "model": OPUS, "usage": {"input_tokens": 99}},
    })
    .to_string();
    write_bytes(
        &f.transcripts,
        &f.cwd,
        "s1.jsonl",
        &[
            one.as_bytes(),
            b"",
            b"{not json",
            b"{\"half\": \"\xc3",
            b"[1]",
            no_branch.as_bytes(),
            b"{\"gitBranch\": \"impl/b\"",
            b"{\"gitBranch\": \"impl/b\", \"half\": \"\xc3",
        ],
    );
    // A file that can't be read: a link to nothing.
    let dir = f.transcripts.join(encoded(&f.cwd));
    std::os::unix::fs::symlink(dir.join("nowhere"), dir.join("y.jsonl")).unwrap();
    let counted = [(f.cwd.clone(), Unlinked)];
    let (work, warnings) = clm_effort::sessions(&f.transcripts, &counted, &pins(&[]), None);
    assert_eq!(by_branch(&work), [("impl/a", "s1", 10)], "{work:?}");
    assert_eq!(warnings, ["skipped 4 transcript lines that aren't JSON"]);
    let (work, warnings) =
        clm_effort::sessions(&f.transcripts, &counted, &pins(&[]), Some("impl/b"));
    assert!(work.is_empty(), "{work:?}");
    assert_eq!(warnings, ["skipped 2 transcript lines that aren't JSON"]);
}

#[test]
fn names_json_escapes_are_read_whole() {
    // req:effort/transcripts@50c1e34f
    // A branch whose name holds a control character is written escaped, so no line holds
    // its raw text: every line is read. A worktree path holding one has every unpinned
    // subagent line read.
    let f = fixture();
    let tab = "impl/a\tb";
    write(
        &f.transcripts,
        &f.cwd,
        "s1.jsonl",
        &[entry("s1", tab, &f.cwd, 0, "m1", OPUS, (4, 1))],
    );
    let counted = [(f.cwd.clone(), Unlinked)];
    let (work, _) = clm_effort::sessions(&f.transcripts, &counted, &pins(&[]), Some(tab));
    assert_eq!(by_branch(&work), [(tab, "s1", 4)], "{work:?}");

    let worktree = f.cwd.join(".claude").join("worktrees").join("a\u{1}b");
    let counted = vec![
        (f.cwd.clone(), Unlinked),
        (worktree.clone(), Branch("impl/c".into())),
    ];
    subagent(&f, &worktree);
    let (work, _) = clm_effort::sessions(&f.transcripts, &counted, &pins(&[]), Some("impl/c"));
    assert_eq!(by_branch(&work), [("impl/c", "s1/agent-x", 10)], "{work:?}");

    // `"` and `\` are escaped as well; an entry with no `timestamp` keeps its tokens.
    for name in ["impl/a\"b", "impl/a\\b"] {
        let f = fixture();
        let mut line = serde_json::from_str::<serde_json::Value>(&entry(
            "s1",
            name,
            &f.cwd,
            0,
            "m1",
            OPUS,
            (4, 1),
        ))
        .unwrap();
        line.as_object_mut().unwrap().remove("timestamp");
        write(&f.transcripts, &f.cwd, "s1.jsonl", &[line.to_string()]);
        let counted = [(f.cwd.clone(), Unlinked)];
        let (work, _) = clm_effort::sessions(&f.transcripts, &counted, &pins(&[]), Some(name));
        assert_eq!(by_branch(&work), [(name, "s1", 4)], "{work:?}");
        assert_eq!(work[0].active_seconds, 0);
    }
    for dir in ["a\"b", "a\\b"] {
        let f = fixture();
        let worktree = f.cwd.join(".claude").join("worktrees").join(dir);
        let counted = vec![
            (f.cwd.clone(), Unlinked),
            (worktree.clone(), Branch("impl/c".into())),
        ];
        subagent(&f, &worktree);
        let (work, _) = clm_effort::sessions(&f.transcripts, &counted, &pins(&[]), Some("impl/c"));
        assert_eq!(
            by_branch(&work),
            [("impl/c", "s1/agent-x", 10)],
            "{dir} {work:?}"
        );
    }
}
