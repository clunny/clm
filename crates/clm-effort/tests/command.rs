//! `clm effort`, `clm capabilities` and `clm architecture` as `clm-effort`'s calls
//! (`effort/commands-in-crate`): what each prints and how it exits.

#[path = "../../clm-ledger/tests/common/mod.rs"]
mod common;

use std::path::{Path, PathBuf};

/// What a call wrote and returned.
#[derive(Debug)]
struct Out {
    code: u8,
    stdout: String,
    stderr: String,
}

type Call = fn(
    std::vec::IntoIter<String>,
    &Path,
    &Path,
    &dyn Fn(&str) -> Option<String>,
    &mut dyn std::io::Write,
    &mut dyn std::io::Write,
) -> u8;

/// Calls `call` with `args` in `root`, run from `cwd`, with only `HOME` (at `home`) set.
fn call(call: Call, root: &Path, cwd: &Path, home: &Path, args: &[&str]) -> Out {
    let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
    let args: Vec<String> = args.iter().map(|a| a.to_string()).collect();
    let code = call(
        args.into_iter(),
        root,
        cwd,
        &common::env(home, &[]),
        &mut stdout,
        &mut stderr,
    );
    Out {
        code,
        stdout: String::from_utf8(stdout).unwrap(),
        stderr: String::from_utf8(stderr).unwrap(),
    }
}

/// `clm effort <args>` in `root`, as run from `cwd`.
fn effort(root: &Path, cwd: &Path, home: &Path, args: &[&str]) -> Out {
    call(clm_effort::effort_command, root, cwd, home, args)
}

/// Claude Code's name for the project directory of the sessions run in `path`.
fn encoded(path: &Path) -> String {
    path.to_string_lossy()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

/// A CLM repository at `<dir>/repo` and, under `<dir>/projects`, a session `s1` run there on
/// `impl/a` and `impl/b`, and on `impl/a` in `<dir>/elsewhere`.
fn record_repo(dir: &Path) -> PathBuf {
    let root = dir.join("repo");
    std::fs::create_dir_all(root.join("specs")).unwrap();
    common::repo(&root, "Ann");
    std::fs::write(root.join("specs/SPEC.md"), "---\ntitle: T\n---\n").unwrap();
    std::fs::write(root.join("clm.toml"), "host = \"none\"\n").unwrap();
    let transcripts = dir.join("projects");
    let line = |cwd: &Path, branch: &str, id: &str| {
        serde_json::json!({
            "sessionId": "s1",
            "gitBranch": branch,
            "cwd": cwd,
            "timestamp": "2026-10-30T00:00:00.000Z",
            "message": {"id": id, "model": "claude-opus-5-5", "usage": {"input_tokens": 3, "output_tokens": 4}},
        })
        .to_string()
    };
    let elsewhere = dir.join("elsewhere");
    let sessions = [
        (
            &root,
            format!(
                "{}\n{}\n",
                line(&root, "impl/a", "m1"),
                line(&root, "impl/b", "m2")
            ),
        ),
        (
            &elsewhere,
            format!("{}\n", line(&elsewhere, "impl/a", "m3")),
        ),
    ];
    for (cwd, lines) in sessions {
        std::fs::create_dir_all(transcripts.join(encoded(cwd))).unwrap();
        std::fs::write(transcripts.join(encoded(cwd)).join("s1.jsonl"), lines).unwrap();
    }
    root
}

#[test]
fn effort_record_cli() {
    // req:effort/record-command@2365b153
    // req:effort/commands-in-crate@c08e640f
    let dir = tempfile::tempdir().unwrap();
    let root = record_repo(dir.path());
    let root_arg = root.to_string_lossy().into_owned();
    let transcripts_arg = dir.path().join("projects").to_string_lossy().into_owned();
    let nowhere = dir.path().join("nowhere");
    let out = effort(
        &nowhere,
        &root,
        dir.path(),
        &[
            "record",
            "--branch",
            "impl/a",
            "--transcripts",
            &transcripts_arg,
            "--root",
            &root_arg,
        ],
    );
    assert_eq!(out.code, 0, "{out:?}");
    assert_eq!(
        out.stdout,
        "effort impl/a s1 7 tokens 0s\nrecorded 1 sessions on 1 branches\n"
    );
    // A relative `--cwd` is taken from the directory `clm` runs in.
    let out = effort(
        &nowhere,
        dir.path(),
        dir.path(),
        &[
            "record",
            "--branch",
            "impl/a",
            "--cwd",
            "elsewhere",
            "--transcripts",
            &transcripts_arg,
            "--root",
            &root_arg,
        ],
    );
    assert_eq!(
        out.stdout, "effort impl/a s1 14 tokens 0s\nrecorded 1 sessions on 1 branches\n",
        "{}",
        out.stderr
    );
    // Outside a CLM repository it refuses.
    std::fs::remove_file(root.join("specs/SPEC.md")).unwrap();
    let out = effort(
        &nowhere,
        &root,
        dir.path(),
        &["record", "--root", &root_arg],
    );
    assert_eq!(out.code, 1);
}

#[test]
fn effort_record_paths_and_streams() {
    // req:effort/commands-in-crate@c08e640f
    let dir = tempfile::tempdir().unwrap();
    let root = record_repo(dir.path());
    // `--cwd`, `.` components and all, `--transcripts` and `--root` are taken from the
    // directory the command runs in, and `--root` overrides the root the call is given.
    let out = effort(
        &dir.path().join("nowhere"),
        dir.path(),
        dir.path(),
        &[
            "record",
            "--branch",
            "impl/a",
            "--cwd",
            "./elsewhere",
            "--transcripts",
            "./projects",
            "--root",
            "repo",
        ],
    );
    assert_eq!(
        out.stdout, "effort impl/a s1 14 tokens 0s\nrecorded 1 sessions on 1 branches\n",
        "{out:?}"
    );
    assert_eq!(out.code, 0, "{out:?}");
    // A warning goes to the error stream with its prefix.
    let out = effort(
        &root,
        dir.path(),
        dir.path(),
        &["record", "--transcripts", "./missing"],
    );
    assert_eq!(out.code, 0, "{out:?}");
    assert_eq!(
        out.stderr,
        format!(
            "warning: no transcripts at {}\n",
            dir.path().join("missing").display()
        ),
        "{out:?}"
    );
    assert!(!out.stdout.contains("warning"), "{out:?}");
    // A failure goes to the error stream after the command's name.
    std::fs::remove_file(root.join("specs/SPEC.md")).unwrap();
    let out = effort(&root, &root, dir.path(), &["record"]);
    assert_eq!(out.code, 1, "{out:?}");
    assert!(out.stderr.starts_with("clm effort: "), "{out:?}");
    assert!(out.stdout.is_empty(), "{out:?}");
}

#[test]
fn arguments_it_cant_parse() {
    // req:effort/commands-in-crate@c08e640f
    let dir = tempfile::tempdir().unwrap();
    let calls: [(Call, &[&str]); 3] = [
        (clm_effort::effort_command, &["record", "--nope"]),
        (clm_effort::capabilities_command, &["--nope"]),
        (clm_effort::architecture_command, &["--nope"]),
    ];
    for (f, args) in calls {
        let out = call(f, dir.path(), dir.path(), dir.path(), args);
        assert_eq!(out.code, 2, "{out:?}");
        assert!(out.stderr.contains("--nope"), "{out:?}");
        assert!(out.stdout.is_empty(), "{out:?}");
        // Help isn't an error: it goes to standard output with the status `clm` exits with.
        let out = call(f, dir.path(), dir.path(), dir.path(), &["--help"]);
        assert_eq!(out.code, 0, "{out:?}");
        assert!(out.stdout.contains("Usage"), "{out:?}");
        assert!(out.stderr.is_empty(), "{out:?}");
    }
    // Reaching here is the scenario's last clause: the test's process goes on.
}

/// A CLM repository with capabilities `p` and `q`, served by node `a`'s two rules.
fn capabilities_repo(dir: &Path) -> PathBuf {
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

fn capabilities(root: &Path, args: &[&str]) -> Out {
    call(clm_effort::capabilities_command, root, root, root, args)
}

#[test]
fn capabilities_cli_lists() {
    // req:effort/capabilities-list@daa12c28
    let dir = tempfile::tempdir().unwrap();
    let out = capabilities(&capabilities_repo(dir.path()), &[]);
    let stdout = &out.stdout;
    assert_eq!(out.code, 0, "{stdout}{}", out.stderr);
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
    let stdout = &out.stdout;
    assert_eq!(out.code, 0, "{stdout}");
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
    let stdout = &out.stdout;
    assert_eq!(out.code, 0, "{stdout}{}", out.stderr);
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
    assert_eq!(out.code, 0, "{}", out.stderr);
    let report: clm_effort::Report = serde_json::from_str(&out.stdout).unwrap();
    assert_eq!(report.capabilities.len(), 2);
    let out = capabilities(&root, &["p", "--format", "json"]);
    let cost: clm_effort::Cost = serde_json::from_str(&out.stdout).unwrap();
    assert_eq!((cost.slug.as_str(), cost.commits), ("p", Some(1)));
}

#[test]
fn capabilities_unknown_slug_exits_1() {
    // req:effort/capability-page@dd387c2f
    let dir = tempfile::tempdir().unwrap();
    let out = capabilities(&capabilities_repo(dir.path()), &["nope"]);
    assert_eq!(out.code, 1);
    assert_eq!(out.stdout, "no capability nope\n");
}

#[test]
fn capabilities_refused_outside_a_clm_repository() {
    // req:effort/capability-page@dd387c2f
    let dir = tempfile::tempdir().unwrap();
    let root = capabilities_repo(dir.path());
    std::fs::remove_file(root.join("specs/SPEC.md")).unwrap();
    let out = capabilities(&root, &[]);
    assert_eq!(out.code, 1);
    assert!(out.stderr.contains("not a CLM repository"));
}

/// `capabilities_repo`'s files in a clone with a bare `origin`, so the ledger can push.
fn decide_repo(dir: &Path) -> PathBuf {
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
fn flag(root: &Path, slug: &str) {
    let (graph, _) = clm_model::load(root);
    let version = graph.requirement("a/y").unwrap().version().to_string();
    std::fs::create_dir_all(root.join("backlog")).unwrap();
    std::fs::write(
        root.join(format!("backlog/{slug}.md")),
        format!("---\ntitle: Y is hard\nkind: friction\ncomponents: [a]\nrequirement: a/y@{version}\n---\n\nIt needed a fake.\n"),
    )
    .unwrap();
}

fn decided(root: &Path) -> Vec<clm_ledger::Record> {
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
    let stdout = &out.stdout;
    assert_eq!(out.code, 0, "{stdout}{}", out.stderr);
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
        out.stdout, "decided capability/q accept (value low), settles 2 flags\n",
        "{}",
        out.stderr
    );
    let records = decided(&root);
    let clm_ledger::Body::Decided { settles, .. } = &records[0].body else {
        panic!("{records:?}");
    };
    assert_eq!(settles, &["f1", "f2"]);
    // A flag filed afterwards is unsettled on `q`.
    flag(&root, "f3");
    let out = capabilities(&root, &["q", "--format", "json"]);
    let cost: clm_effort::Cost = serde_json::from_str(&out.stdout).unwrap();
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
    assert_eq!(out.code, 1);
    assert_eq!(out.stdout, "no capability nope\n");
    assert!(decided(&root).is_empty());
}

#[test]
fn decide_after_parent_options() {
    // req:effort/decide-command@e1ca5b43
    // req:effort/commands-in-crate@c08e640f
    let dir = tempfile::tempdir().unwrap();
    let root = decide_repo(dir.path());
    let given = root.to_string_lossy().into_owned();
    let out = call(
        clm_effort::capabilities_command,
        &dir.path().join("nowhere"),
        dir.path(),
        &root,
        &[
            "--root",
            &given,
            "decide",
            "p",
            "--value",
            "high",
            "--decision",
            "keep",
        ],
    );
    assert_eq!(
        out.stdout, "decided capability/p keep (value high), settles 0 flags\n",
        "{}",
        out.stderr
    );
    // `decide` is never a slug, wherever it stands.
    for args in [&["--format", "json", "decide"][..], &["decide"]] {
        let out = capabilities(&root, args);
        assert_ne!(out.code, 0, "{args:?}");
        assert!(out.stdout.is_empty(), "{args:?}");
    }
}

#[test]
fn decide_own_root_overrides() {
    // req:effort/commands-in-crate@c08e640f
    let dir = tempfile::tempdir().unwrap();
    let root = decide_repo(dir.path());
    // `decide`'s own `--root`, relative to the directory the command runs in, overrides the
    // root the call is given.
    let out = call(
        clm_effort::capabilities_command,
        &dir.path().join("nowhere"),
        dir.path(),
        &root,
        &[
            "decide",
            "q",
            "--value",
            "low",
            "--decision",
            "cut",
            "--root",
            "repo",
        ],
    );
    assert_eq!(
        out.stdout, "decided capability/q cut (value low), settles 0 flags\n",
        "{out:?}"
    );
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
    assert_eq!(out.code, 1);
    assert!(out.stderr.contains("not a CLM repository"));
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
    assert_eq!(out.code, 2, "{out:?}");
    assert!(out.stdout.is_empty());
}

/// `decide_repo` with an architectural constraint `e` enforced by `a/y`.
fn architecture_repo(dir: &Path) -> PathBuf {
    let root = decide_repo(dir);
    std::fs::write(
        root.join("ARCHITECTURE.md"),
        "## Constraints\n\n### Constraint: e\nGit SHALL hold e.\n\n#### Scenario: s\n- ok\n\n#### Enforced by\n- a/y\n\n### Constraint: u\nIt SHALL be a judgement.\n\n#### Scenario: s\n- ok\n\n#### Untested\nA judgement.\n",
    )
    .unwrap();
    common::commit(&root, &[], "architecture");
    root
}

fn architecture(root: &Path, args: &[&str]) -> Out {
    call(clm_effort::architecture_command, root, root, root, args)
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
    let stdout = &out.stdout;
    assert_eq!(out.code, 0, "{stdout}{}", out.stderr);
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
    assert_eq!(out.code, 1);
    assert_eq!(out.stdout, "no architectural constraint nope\n");
    assert!(decided(&root).is_empty());
    // After the parent's options, as `capabilities decide` allows; a slug before `decide`
    // is a usage error, and `decide` is never a slug.
    let given = root.to_string_lossy().into_owned();
    let mut after = vec!["--root", given.as_str()];
    after.extend(args("e"));
    let out = call(
        clm_effort::architecture_command,
        &dir.path().join("nowhere"),
        dir.path(),
        &root,
        &after,
    );
    assert_eq!(out.code, 0, "{out:?}");
    let mut before = vec!["e"];
    before.extend(args("x"));
    let out = architecture(&root, &before);
    assert_eq!(out.code, 2);
    assert!(out.stderr.contains("`decide` takes the slug after it"));
    let out = architecture(&root, &["decide"]);
    assert_ne!(out.code, 0);
    assert!(out.stdout.is_empty());
    // Outside a CLM repository it refuses.
    std::fs::remove_file(root.join("specs/SPEC.md")).unwrap();
    let out = architecture(&root, &args("e"));
    assert_eq!(out.code, 1);
    assert!(out.stderr.contains("not a CLM repository"));
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
    assert_eq!(out.code, 2, "{out:?}");
    assert!(out.stdout.is_empty());
}

#[test]
fn architecture_cli_lists_and_pages() {
    // req:effort/architecture-list@a45a68a0
    let dir = tempfile::tempdir().unwrap();
    let root = architecture_repo(dir.path());
    let out = architecture(&root, &[]);
    let stdout = &out.stdout;
    assert_eq!(out.code, 0, "{stdout}");
    let first = "architecture 2 · 0 held · 0 ⚑ · costs are estimates: an entry's cost is its enforcing rules' share, counted apart from the capabilities'";
    assert_eq!(stdout.lines().next(), Some(first), "{stdout}");
    // Nothing cites `a/y`, so `e` isn't held yet.
    assert!(stdout.contains("held 0/1"), "{stdout}");
    let out = architecture(&root, &["e"]);
    let stdout = &out.stdout;
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
    assert_eq!(out.code, 0);
    let list: Vec<clm_effort::Constraint> = serde_json::from_str(&out.stdout).unwrap();
    let slugs: Vec<&str> = list.iter().map(|c| c.slug.as_str()).collect();
    assert_eq!(slugs, ["e", "u"]);
    assert_eq!(list[0].enforced_by[0].current, Some(false));
    let out = architecture(&root, &["u", "--format", "json"]);
    let one: clm_effort::Constraint = serde_json::from_str(&out.stdout).unwrap();
    assert_eq!(one.untested.as_deref(), Some("A judgement."));
    let out = architecture(&root, &["nope", "--format", "json"]);
    assert_eq!(out.code, 1);
    assert_eq!(out.stdout, "no architectural constraint nope\n");
}

#[test]
fn architecture_help_says_architectural_constraint() {
    // req:effort/architecture-list@a45a68a0
    for args in [&["--help"][..], &["decide", "--help"][..]] {
        let dir = tempfile::tempdir().unwrap();
        let out = architecture(dir.path(), args);
        let help = out.stdout.to_lowercase();
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
    assert_eq!(out.code, 1);
    assert_eq!(out.stdout, "no architectural constraint nope\n");
}
