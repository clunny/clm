//! `effort/transcripts`: which transcript entries count, and how.

use std::path::{Path, PathBuf};

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
    // req:effort/transcripts@8de9737c
    let f = fixture();
    let line = entry("s1", "impl/a", &f.cwd, 0, "m1", OPUS, (10, 5));
    write(&f.transcripts, &f.cwd, "s1.jsonl", &[line.clone(), line]);
    let (work, warnings) = clm_effort::sessions(&f.transcripts, std::slice::from_ref(&f.cwd), None);
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
    // req:effort/transcripts@8de9737c
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
    let (work, _) = clm_effort::sessions(&f.transcripts, std::slice::from_ref(&f.cwd), None);
    let branches: Vec<&str> = work.iter().map(|w| w.branch.as_str()).collect();
    assert_eq!(branches, ["impl/a", "impl/b"]);
    // Only one branch's when asked.
    let (work, _) =
        clm_effort::sessions(&f.transcripts, std::slice::from_ref(&f.cwd), Some("impl/b"));
    assert_eq!(work.len(), 1);
    assert_eq!(work[0].models[OPUS].input, 2);
}

#[test]
fn subagent_file_joins_its_session() {
    // req:effort/transcripts@8de9737c
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
    let (work, _) = clm_effort::sessions(&f.transcripts, std::slice::from_ref(&f.cwd), None);
    assert_eq!(work.len(), 1, "{work:?}");
    assert_eq!(work[0].models[OPUS].input, 10);
    assert_eq!(work[0].models[sonnet].input, 7);
    assert_eq!(work[0].active_seconds, 30);
}

#[test]
fn synthetic_model_skipped() {
    // req:effort/transcripts@8de9737c
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
    let (work, _) = clm_effort::sessions(&f.transcripts, std::slice::from_ref(&f.cwd), None);
    assert_eq!(work.len(), 1);
    assert_eq!(work[0].models.keys().collect::<Vec<_>>(), [OPUS]);
    assert_eq!(work[0].models[OPUS].input, 10);
}

#[test]
fn active_time_leaves_out_long_gaps() {
    // req:effort/transcripts@8de9737c
    let f = fixture();
    let lines: Vec<String> = [0, 60, 120, 1000]
        .iter()
        .enumerate()
        .map(|(n, s)| entry("s1", "impl/a", &f.cwd, *s, &format!("m{n}"), OPUS, (1, 1)))
        .collect();
    write(&f.transcripts, &f.cwd, "s1.jsonl", &lines);
    let (work, _) = clm_effort::sessions(&f.transcripts, std::slice::from_ref(&f.cwd), None);
    assert_eq!(work[0].active_seconds, 120);
    assert_eq!(work[0].models[OPUS].input, 4);
}

#[test]
fn other_cwd_not_counted() {
    // req:effort/transcripts@8de9737c
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
    let (work, _) = clm_effort::sessions(&f.transcripts, std::slice::from_ref(&f.cwd), None);
    assert!(work.is_empty(), "{work:?}");
}

#[test]
fn bad_lines_warned_once() {
    // req:effort/transcripts@8de9737c
    let f = fixture();
    let good = entry("s1", "impl/a", &f.cwd, 0, "m1", OPUS, (10, 5));
    write(
        &f.transcripts,
        &f.cwd,
        "s1.jsonl",
        &["{not json".into(), good, "nor this".into()],
    );
    let (work, warnings) = clm_effort::sessions(&f.transcripts, std::slice::from_ref(&f.cwd), None);
    assert_eq!(work.len(), 1);
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert_eq!(warnings[0], "skipped 2 transcript lines that aren't JSON");
}

#[test]
fn missing_dir_warns() {
    // req:effort/transcripts@8de9737c
    let f = fixture();
    let (work, warnings) = clm_effort::sessions(&f.transcripts, std::slice::from_ref(&f.cwd), None);
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
    // req:effort/transcripts@8de9737c
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
    let (work, _) = clm_effort::sessions(&f.transcripts, std::slice::from_ref(&f.cwd), None);
    let u = work[0].models[OPUS];
    assert_eq!(
        (u.input, u.output, u.cache_read, u.cache_write),
        (1, 2, 3, 4)
    );
}

#[test]
fn repeated_message_takes_the_largest_counts() {
    // req:effort/transcripts@8de9737c
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
    let (work, _) = clm_effort::sessions(&f.transcripts, std::slice::from_ref(&f.cwd), None);
    let u = work[0].models[OPUS];
    assert_eq!((u.input, u.output), (10, 850));
}

#[test]
fn a_gap_of_600_seconds_is_active() {
    // req:effort/transcripts@8de9737c
    let f = fixture();
    let lines = [
        entry("s1", "impl/a", &f.cwd, 0, "m1", OPUS, (1, 1)),
        entry("s1", "impl/a", &f.cwd, 600, "m2", OPUS, (1, 1)),
        entry("s1", "impl/a", &f.cwd, 1201, "m3", OPUS, (1, 1)),
    ];
    write(&f.transcripts, &f.cwd, "s1.jsonl", &lines);
    let (work, _) = clm_effort::sessions(&f.transcripts, std::slice::from_ref(&f.cwd), None);
    assert_eq!(work[0].active_seconds, 600);
}

#[test]
fn a_line_that_isnt_utf8_is_skipped_alone() {
    // req:effort/transcripts@8de9737c
    let f = fixture();
    let good = entry("s1", "impl/a", &f.cwd, 0, "m1", OPUS, (10, 5));
    let path = f.transcripts.join(encoded(&f.cwd)).join("s1.jsonl");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut bytes = good.into_bytes();
    bytes.extend_from_slice(b"\n{\"half\": \"\xc3");
    std::fs::write(&path, bytes).unwrap();
    let (work, warnings) = clm_effort::sessions(&f.transcripts, std::slice::from_ref(&f.cwd), None);
    assert_eq!(work.len(), 1, "{warnings:?}");
    assert_eq!(work[0].models[OPUS].input, 10);
    assert_eq!(warnings, ["skipped 1 transcript lines that aren't JSON"]);
}
