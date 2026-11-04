//! `clm effort record` through the binary (`effort/record-command`).

#[path = "../../clm-ledger/tests/common/mod.rs"]
mod common;

use std::process::Command;

#[test]
fn effort_record_cli() {
    // req:effort/record-command@2365b153
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("repo");
    std::fs::create_dir_all(root.join("specs")).unwrap();
    common::repo(&root, "Ann");
    std::fs::write(root.join("specs/SPEC.md"), "---\ntitle: T\n---\n").unwrap();
    std::fs::write(root.join("clm.toml"), "host = \"none\"\n").unwrap();
    let transcripts = dir.path().join("projects");
    let encoded: String = root
        .to_string_lossy()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    std::fs::create_dir_all(transcripts.join(&encoded)).unwrap();
    let line = |branch: &str, id: &str| {
        serde_json::json!({
            "sessionId": "s1",
            "gitBranch": branch,
            "cwd": root,
            "timestamp": "2026-10-30T00:00:00.000Z",
            "message": {"id": id, "model": "claude-opus-5-5", "usage": {"input_tokens": 3, "output_tokens": 4}},
        })
        .to_string()
    };
    std::fs::write(
        transcripts.join(&encoded).join("s1.jsonl"),
        format!("{}\n{}\n", line("impl/a", "m1"), line("impl/b", "m2")),
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_clm"))
        .args(["effort", "record", "--branch", "impl/a", "--transcripts"])
        .arg(&transcripts)
        .arg("--root")
        .arg(&root)
        .env("HOME", dir.path())
        .env_remove("CLM_WRITER")
        .env_remove("CLAUDE_CONFIG_DIR")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{stdout}{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        stdout,
        "effort impl/a s1 7 tokens 0s\nrecorded 1 sessions on 1 branches\n"
    );
    // A relative `--cwd` is taken from the directory `clm` runs in.
    let elsewhere = dir.path().join("elsewhere");
    let encoded_elsewhere: String = elsewhere
        .to_string_lossy()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    std::fs::create_dir_all(transcripts.join(&encoded_elsewhere)).unwrap();
    let there =
        line("impl/a", "m3").replace(&*root.to_string_lossy(), &elsewhere.to_string_lossy());
    std::fs::write(
        transcripts.join(&encoded_elsewhere).join("s1.jsonl"),
        there + "\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_clm"))
        .args([
            "effort",
            "record",
            "--branch",
            "impl/a",
            "--cwd",
            "elsewhere",
            "--transcripts",
        ])
        .arg(&transcripts)
        .arg("--root")
        .arg(&root)
        .current_dir(dir.path())
        .env("HOME", dir.path())
        .env_remove("CLM_WRITER")
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "effort impl/a s1 14 tokens 0s\nrecorded 1 sessions on 1 branches\n",
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    // Outside a CLM repository it refuses.
    std::fs::remove_file(root.join("specs/SPEC.md")).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_clm"))
        .args(["effort", "record", "--root"])
        .arg(&root)
        .env("HOME", dir.path())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
}

/// A CLM repository with capabilities `p` and `q`, served by node `a`'s two rules.
fn capabilities_repo(dir: &std::path::Path) -> std::path::PathBuf {
    let root = dir.join("repo");
    std::fs::create_dir_all(root.join("specs/a")).unwrap();
    common::repo(&root, "Ann");
    let files = [
        (
            "specs/SPEC.md",
            "---\ntitle: T\nstatus: active\nowners: []\n---\n\n## Contract\nc\n",
        ),
        (
            "CAPABILITIES.md",
            "## Capabilities\n\n### Capability: p\nA person can p.\n\n#### Why\nw\n\n#### Scenario: s\n- ok\n\n### Capability: q\nA person can q.\n\n#### Why\nw\n\n#### Scenario: s\n- ok\n",
        ),
        (
            "specs/a/SPEC.md",
            "---\ntitle: A\nstatus: active\nowners: []\nserves: [capability/p]\n---\n\n## Contract\nc\n\n## Requirements\n\n### Requirement: x\nIt SHALL.\n\n#### Scenario: s\n- ok\n\n### Requirement: y\nIt SHALL.\n\n#### Scenario: s\n- ok\n\n#### Serves\n- capability/q\n",
        ),
        ("clm.toml", "host = \"none\"\n"),
    ];
    common::commit(&root, &files, "specs");
    root
}

fn capabilities(root: &std::path::Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_clm"))
        .arg("capabilities")
        .args(args)
        .arg("--root")
        .arg(root)
        .env("HOME", root)
        .env_remove("CLM_WRITER")
        .output()
        .unwrap()
}

#[test]
fn capabilities_cli_lists() {
    // req:effort/capabilities-list@daa12c28
    let dir = tempfile::tempdir().unwrap();
    let out = capabilities(&capabilities_repo(dir.path()), &[]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{stdout}{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(
        lines[0],
        "capabilities 2 · 0 ⚑ · costs are estimates: a component's cost is split by its rules' share"
    );
    assert!(
        lines[1].starts_with("  p  ≈$0.00  0 tok  1 commits  0 reviews"),
        "{stdout}"
    );
    assert!(lines[2].starts_with("  q  "), "{stdout}");
    assert_eq!(lines.len(), 5, "{stdout}");
}

#[test]
fn capabilities_cli_says_components() {
    // req:effort/says-component@06d9e61a
    let dir = tempfile::tempdir().unwrap();
    let root = capabilities_repo(dir.path());
    let caps = std::fs::read_to_string(root.join("CAPABILITIES.md")).unwrap();
    let caps = format!(
        "{caps}\n### Capability: r\nA person can r.\n\n#### Why\nw\n\n#### Scenario: s\n- ok\n"
    );
    common::commit(&root, &[("CAPABILITIES.md", caps.as_str())], "r");
    let out = capabilities(&root, &["r"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(0), "{stdout}");
    assert_eq!(
        stdout.lines().nth(2),
        Some("0 rules · no components"),
        "{stdout}"
    );
}

#[test]
fn capabilities_cli_page() {
    // req:effort/capability-page@dd387c2f
    let dir = tempfile::tempdir().unwrap();
    let out = capabilities(&capabilities_repo(dir.path()), &["q"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{stdout}{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        stdout.starts_with("q\nA person can q.\n1 rules · components a\n"),
        "{stdout}"
    );
    assert_eq!(
        stdout.lines().last(),
        Some(
            "decide: clm capabilities decide q --value high|medium|low --decision keep|cut|redesign|accept"
        )
    );
}

#[test]
fn capabilities_json_parses() {
    // req:effort/capabilities-json@f95edf16
    let dir = tempfile::tempdir().unwrap();
    let root = capabilities_repo(dir.path());
    let out = capabilities(&root, &["--format", "json"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let report: clm_effort::Report = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report.capabilities.len(), 2);
    let out = capabilities(&root, &["p", "--format", "json"]);
    let cost: clm_effort::Cost = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!((cost.slug.as_str(), cost.commits), ("p", Some(1)));
}

#[test]
fn capabilities_unknown_slug_exits_1() {
    // req:effort/capability-page@dd387c2f
    let dir = tempfile::tempdir().unwrap();
    let out = capabilities(&capabilities_repo(dir.path()), &["nope"]);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(String::from_utf8_lossy(&out.stdout), "no capability nope\n");
}

#[test]
fn capabilities_refused_outside_a_clm_repository() {
    // req:effort/capability-page@dd387c2f
    let dir = tempfile::tempdir().unwrap();
    let root = capabilities_repo(dir.path());
    std::fs::remove_file(root.join("specs/SPEC.md")).unwrap();
    let out = capabilities(&root, &[]);
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("not a CLM repository"));
}

/// `capabilities_repo`'s files in a clone with a bare `origin`, so the ledger can push.
fn decide_repo(dir: &std::path::Path) -> std::path::PathBuf {
    let plain = capabilities_repo(&dir.join("plain"));
    let root = common::clone_with_origin(dir, "repo", "Ann");
    for path in [
        "specs/SPEC.md",
        "CAPABILITIES.md",
        "specs/a/SPEC.md",
        "clm.toml",
    ] {
        let to = root.join(path);
        std::fs::create_dir_all(to.parent().unwrap()).unwrap();
        std::fs::copy(plain.join(path), to).unwrap();
    }
    common::commit(&root, &[], "specs");
    root
}

/// A friction item `slug` on `a/y`, which serves `q` alone (its own `#### Serves`).
fn flag(root: &std::path::Path, slug: &str) {
    let out = Command::new(env!("CARGO_BIN_EXE_clm"))
        .args(["hashes", "a", "--root"])
        .arg(root)
        .output()
        .unwrap();
    let hashes = String::from_utf8_lossy(&out.stdout);
    let version = hashes
        .lines()
        .find_map(|l| l.strip_prefix("requirement:y "))
        .map(|h| &h[..8])
        .unwrap_or_else(|| panic!("{hashes}"))
        .to_string();
    std::fs::create_dir_all(root.join("backlog")).unwrap();
    std::fs::write(
        root.join(format!("backlog/{slug}.md")),
        format!("---\ntitle: Y is hard\nkind: friction\ncomponents: [a]\nrequirement: a/y@{version}\n---\n\nIt needed a fake.\n"),
    )
    .unwrap();
}

fn decided(root: &std::path::Path) -> Vec<clm_ledger::Record> {
    let fold = clm_ledger::fold(root).unwrap();
    fold.decisions.values().flatten().cloned().collect()
}

#[test]
fn decide_appends_decided() {
    // req:effort/decide-command@e1ca5b43
    let dir = tempfile::tempdir().unwrap();
    let root = decide_repo(dir.path());
    let out = capabilities(
        &root,
        &["decide", "p", "--value", "high", "--decision", "keep"],
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{stdout}{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        stdout,
        "decided capability/p keep (value high), settles 0 flags\n"
    );
    let records = decided(&root);
    assert_eq!(records.len(), 1);
    assert_eq!(
        records[0].body,
        clm_ledger::Body::Decided {
            capability: "capability/p".into(),
            value: clm_ledger::Rating::High,
            decision: clm_ledger::Decision::Keep,
            settles: vec![],
        }
    );
}

#[test]
fn decide_settles_open_flags() {
    // req:effort/decide-command@e1ca5b43
    let dir = tempfile::tempdir().unwrap();
    let root = decide_repo(dir.path());
    flag(&root, "f1");
    flag(&root, "f2");
    let out = capabilities(
        &root,
        &["decide", "q", "--value", "low", "--decision", "accept"],
    );
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "decided capability/q accept (value low), settles 2 flags\n",
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let records = decided(&root);
    let clm_ledger::Body::Decided { settles, .. } = &records[0].body else {
        panic!("{records:?}");
    };
    assert_eq!(settles, &["f1", "f2"]);
    // A flag filed afterwards is unsettled on `q`.
    flag(&root, "f3");
    let out = capabilities(&root, &["q", "--format", "json"]);
    let cost: clm_effort::Cost = serde_json::from_slice(&out.stdout).unwrap();
    let open: Vec<&str> = cost
        .flags
        .iter()
        .filter(|f| !f.settled)
        .map(|f| f.slug.as_str())
        .collect();
    assert_eq!(open, ["f3"]);
}

#[test]
fn decide_unknown_slug() {
    // req:effort/decide-command@e1ca5b43
    let dir = tempfile::tempdir().unwrap();
    let root = decide_repo(dir.path());
    let out = capabilities(
        &root,
        &["decide", "nope", "--value", "high", "--decision", "keep"],
    );
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(String::from_utf8_lossy(&out.stdout), "no capability nope\n");
    assert!(decided(&root).is_empty());
}

#[test]
fn decide_after_parent_options() {
    // req:effort/decide-command@e1ca5b43
    let dir = tempfile::tempdir().unwrap();
    let root = decide_repo(dir.path());
    let out = Command::new(env!("CARGO_BIN_EXE_clm"))
        .args(["capabilities", "--root"])
        .arg(&root)
        .args(["decide", "p", "--value", "high", "--decision", "keep"])
        .env("HOME", &root)
        .env_remove("CLM_WRITER")
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "decided capability/p keep (value high), settles 0 flags\n",
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    // `decide` is never a slug, wherever it stands.
    for args in [&["--format", "json", "decide"][..], &["decide"]] {
        let out = capabilities(&root, args);
        assert_ne!(out.status.code(), Some(0), "{args:?}");
        assert!(out.stdout.is_empty(), "{args:?}");
    }
}

#[test]
fn decide_refused_outside_a_clm_repository() {
    // req:effort/decide-command@e1ca5b43
    let dir = tempfile::tempdir().unwrap();
    let root = decide_repo(dir.path());
    std::fs::remove_file(root.join("specs/SPEC.md")).unwrap();
    let out = capabilities(
        &root,
        &["decide", "p", "--value", "high", "--decision", "keep"],
    );
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("not a CLM repository"));
}

#[test]
fn decide_ledger_failure_exits_2() {
    // req:effort/decide-command@e1ca5b43
    let dir = tempfile::tempdir().unwrap();
    let root = decide_repo(dir.path());
    // A file where the ledger branches' ref directory goes, so no ledger ref can be written.
    std::fs::write(root.join(".git/refs/heads/clm"), "x\n").unwrap();
    let out = capabilities(
        &root,
        &["decide", "p", "--value", "high", "--decision", "keep"],
    );
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    assert!(out.stdout.is_empty());
}

/// `decide_repo` with an architectural constraint `e` enforced by `a/y`.
fn architecture_repo(dir: &std::path::Path) -> std::path::PathBuf {
    let root = decide_repo(dir);
    std::fs::write(
        root.join("ARCHITECTURE.md"),
        "## Constraints\n\n### Constraint: e\nGit SHALL hold e.\n\n#### Scenario: s\n- ok\n\n#### Enforced by\n- a/y\n\n### Constraint: u\nIt SHALL be a judgement.\n\n#### Scenario: s\n- ok\n\n#### Untested\nA judgement.\n",
    )
    .unwrap();
    common::commit(&root, &[], "architecture");
    root
}

fn architecture(root: &std::path::Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_clm"))
        .arg("architecture")
        .args(args)
        .arg("--root")
        .arg(root)
        .env("HOME", root)
        .env_remove("CLM_WRITER")
        .output()
        .unwrap()
}

#[test]
fn architecture_decide_appends_decided() {
    // req:effort/architecture-decide-command@669a0425
    let dir = tempfile::tempdir().unwrap();
    let root = architecture_repo(dir.path());
    flag(&root, "f1");
    let out = architecture(
        &root,
        &["decide", "e", "--value", "medium", "--decision", "redesign"],
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{stdout}{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        stdout,
        "decided architecture/e redesign (value medium), settles 1 flags\n"
    );
    let records = decided(&root);
    assert_eq!(records.len(), 1);
    assert_eq!(
        records[0].body,
        clm_ledger::Body::Decided {
            capability: "architecture/e".into(),
            value: clm_ledger::Rating::Medium,
            decision: clm_ledger::Decision::Redesign,
            settles: vec!["f1".into()],
        }
    );
}

#[test]
fn architecture_decide_unknown_slug_and_refusals() {
    // req:effort/architecture-decide-command@669a0425
    let dir = tempfile::tempdir().unwrap();
    let root = architecture_repo(dir.path());
    let args = |slug: &'static str| ["decide", slug, "--value", "high", "--decision", "keep"];
    let out = architecture(&root, &args("nope"));
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "no architectural constraint nope\n"
    );
    assert!(decided(&root).is_empty());
    // After the parent's options, as `capabilities decide` allows; a slug before `decide`
    // is a usage error, and `decide` is never a slug.
    let out = Command::new(env!("CARGO_BIN_EXE_clm"))
        .args(["architecture", "--root"])
        .arg(&root)
        .args(args("e"))
        .env("HOME", &root)
        .env_remove("CLM_WRITER")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let mut before = vec!["e"];
    before.extend(args("x"));
    let out = architecture(&root, &before);
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("`decide` takes the slug after it"));
    let out = architecture(&root, &["decide"]);
    assert_ne!(out.status.code(), Some(0));
    assert!(out.stdout.is_empty());
    // Outside a CLM repository it refuses.
    std::fs::remove_file(root.join("specs/SPEC.md")).unwrap();
    let out = architecture(&root, &args("e"));
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("not a CLM repository"));
}

#[test]
fn architecture_decide_ledger_failure_exits_2() {
    // req:effort/architecture-decide-command@669a0425
    let dir = tempfile::tempdir().unwrap();
    let root = architecture_repo(dir.path());
    // A file where the ledger branches' ref directory goes, so no ledger ref can be written.
    std::fs::write(root.join(".git/refs/heads/clm"), "x\n").unwrap();
    let out = architecture(
        &root,
        &["decide", "u", "--value", "high", "--decision", "keep"],
    );
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    assert!(out.stdout.is_empty());
}

#[test]
fn architecture_cli_lists_and_pages() {
    // req:effort/architecture-list@a45a68a0
    let dir = tempfile::tempdir().unwrap();
    let root = architecture_repo(dir.path());
    let out = architecture(&root, &[]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(0), "{stdout}");
    let first = "architecture 2 · 0 held · 0 ⚑ · costs are estimates: an entry's cost is its enforcing rules' share, counted apart from the capabilities'";
    assert_eq!(stdout.lines().next(), Some(first), "{stdout}");
    // Nothing cites `a/y`, so `e` isn't held yet.
    assert!(stdout.contains("held 0/1"), "{stdout}");
    let out = architecture(&root, &["e"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("pending a/y"), "{stdout}");
    assert_eq!(
        stdout.lines().last(),
        Some(
            "decide: clm architecture decide e --value high|medium|low --decision keep|cut|redesign|accept"
        )
    );
}

#[test]
fn architecture_json_parses() {
    // req:effort/architecture-json@a28978ed
    let dir = tempfile::tempdir().unwrap();
    let root = architecture_repo(dir.path());
    let out = architecture(&root, &["--format", "json"]);
    assert_eq!(out.status.code(), Some(0));
    let list: Vec<clm_effort::Constraint> = serde_json::from_slice(&out.stdout).unwrap();
    let slugs: Vec<&str> = list.iter().map(|c| c.slug.as_str()).collect();
    assert_eq!(slugs, ["e", "u"]);
    assert_eq!(list[0].enforced_by[0].current, Some(false));
    let out = architecture(&root, &["u", "--format", "json"]);
    let one: clm_effort::Constraint = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(one.untested.as_deref(), Some("A judgement."));
    let out = architecture(&root, &["nope", "--format", "json"]);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "no architectural constraint nope\n"
    );
}

#[test]
fn architecture_help_says_architectural_constraint() {
    // req:effort/architecture-list@a45a68a0
    for args in [&["--help"][..], &["decide", "--help"][..]] {
        let out = Command::new(env!("CARGO_BIN_EXE_clm"))
            .arg("architecture")
            .args(args)
            .output()
            .unwrap();
        let help = String::from_utf8_lossy(&out.stdout).to_lowercase();
        // Words only, so a line wrap or a backtick changes nothing.
        let words: Vec<&str> = help
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| !w.is_empty())
            .collect();
        let pair = |a: &str, b: &str| words.windows(2).any(|w| w[0] == a && w[1].starts_with(b));
        assert!(pair("architectural", "constraint"), "{help}");
        assert!(!pair("architecture", "requirement"), "{help}");
    }
}

#[test]
fn architecture_page_unknown_slug_as_text() {
    // req:effort/architecture-page@8bc35522
    let dir = tempfile::tempdir().unwrap();
    let root = architecture_repo(dir.path());
    let out = architecture(&root, &["nope"]);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "no architectural constraint nope\n"
    );
}
