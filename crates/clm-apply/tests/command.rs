//! `clm apply`, `clm archive` and `clm hashes` as `clm-apply`'s calls: what each writes to
//! its streams and the status it returns (`commands-in-crate`).

use std::fs;
use std::path::Path;

use tempfile::TempDir;

/// What one call wrote to each stream, and the status it returned.
struct Output {
    status: Option<i32>,
    stdout: String,
    stderr: String,
}

/// Calls the command `args[0]` names with the rest of `args`, as the command line gives them
/// after the command's name, in `root`.
fn call(root: &Path, args: &[&str]) -> Output {
    let (name, rest) = args.split_first().unwrap();
    let command = match *name {
        "apply" => clm_apply::apply_command,
        "archive" => clm_apply::archive_command,
        "hashes" => clm_apply::hashes_command,
        other => panic!("clm-apply offers no `{other}`"),
    };
    let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
    let status = command(rest.iter(), root, &|_| None, &mut stdout, &mut stderr);
    Output {
        status: Some(i32::from(status)),
        stdout: String::from_utf8(stdout).unwrap(),
        stderr: String::from_utf8(stderr).unwrap(),
    }
}

/// The command `args` names with `--root <root>`, the root it is given being a directory that
/// holds no repository, so only `--root` can find one.
fn call_with_root(root: &Path, args: &[&str]) -> Output {
    let root = root.to_str().unwrap();
    let mut line = args.to_vec();
    line.extend(["--root", root]);
    call(Path::new("/nonexistent-clm-root"), &line)
}

fn clm(root: &Path, args: &[&str]) -> (Option<i32>, String) {
    let out = call_with_root(root, args);
    (out.status, out.stdout)
}

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

const NODE: &str = "---\ntitle: T\nstatus: specd\nowners: []\n---\n\n## Contract\nc\n\n## Design\nd\n\n## Requirements\n\n### Requirement: r\nR.\n\n#### Scenario: s\n- ok\n";

fn change_repo(delta: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "specs/SPEC.md", NODE);
    write(
        dir.path(),
        "changes/c/change.md",
        "---\nstatus: proposed\n---\n",
    );
    write(dir.path(), "changes/c/deltas/clm.md", delta);
    dir
}

#[test]
fn a_call_writes_what_the_command_prints_and_returns_its_status() {
    // req:apply/commands-in-crate@67d7369f
    let dir = change_repo("---\ncomponent: clm\n---\n");
    // Found from the root the call is given, with no `--root`.
    let out = call(dir.path(), &["apply", "c"]);
    assert_eq!(out.status, Some(0), "{}", out.stdout);
    assert_eq!(
        out.stdout,
        "applied c\n  wrote specs/SPEC.md\n  wrote changes/c/change.md\n"
    );
    assert_eq!(out.stderr, "");

    let out = call(dir.path(), &["hashes", "nope"]);
    assert_eq!(out.status, Some(2));
    assert_eq!(out.stdout, "");
    assert_eq!(out.stderr, "clm hashes: no valid component `nope`\n");

    let out = call(dir.path(), &["archive", "c"]);
    assert_eq!(out.status, Some(1));
    assert_eq!(out.stderr, "");
    assert_eq!(
        out.stdout,
        "changes/c: apply/archive-command: change `c` is Applied; only done or abandoned changes can be archived\n"
    );
}

#[test]
fn a_violation_prints_as_the_models_line() {
    // req:apply/commands-in-crate@67d7369f
    let dir = change_repo(
        "---\ncomponent: clm\nbase: { contract: \"0000000000000000\" }\n---\n## MODIFIED Contract\nc2\n",
    );
    let (graph, _) = clm_model::load(dir.path());
    let change = &graph.changes["c"];
    let expected: String = clm_apply::validate(&graph, change)
        .iter()
        .map(|v| format!("{v}\n"))
        .collect();
    assert!(!expected.is_empty());
    let out = call(dir.path(), &["apply", "c"]);
    assert_eq!(out.status, Some(1));
    assert_eq!(out.stdout, expected);
}

#[test]
fn arguments_it_cant_parse() {
    // req:apply/commands-in-crate@67d7369f
    let dir = change_repo("---\ncomponent: clm\n---\n");
    for name in ["apply", "archive", "hashes"] {
        let out = call(dir.path(), &[name, "c", "--nosuchoption"]);
        assert_eq!(out.status, Some(2), "{name}");
        assert_eq!(out.stdout, "", "{name}");
        assert!(
            out.stderr.contains("unexpected argument '--nosuchoption'"),
            "{name}: {}",
            out.stderr
        );
        assert!(
            out.stderr.contains(&format!("clm {name}")),
            "{}",
            out.stderr
        );
    }
    // The test's process went on, and nothing was applied.
    assert!(
        fs::read_to_string(dir.path().join("changes/c/change.md"))
            .unwrap()
            .contains("proposed")
    );
}

#[test]
fn hashes_prints_elements_usable_as_bases() {
    // req:apply/hash-command@971912c5
    let dir = change_repo("---\ncomponent: clm\n---\n");
    let (code, out) = clm(dir.path(), &["hashes", "clm"]);
    assert_eq!(code, Some(0));
    let elements: Vec<_> = out.lines().map(|l| l.split(' ').next().unwrap()).collect();
    assert_eq!(
        elements,
        ["frontmatter", "contract", "design", "requirement:r"]
    );
    let hash = |n: usize| out.lines().nth(n).unwrap().split(' ').nth(1).unwrap();
    let (contract, r) = (hash(1), hash(3));
    write(
        dir.path(),
        "changes/c/deltas/clm.md",
        &format!(
            "---\ncomponent: clm\nbase: {{ contract: \"{contract}\" }}\n---\n## MODIFIED Contract\nc2\n\n## MODIFIED Requirements\n### Requirement: r <!-- base: {r} -->\nR2.\n\n#### Scenario: s\n- ok\n"
        ),
    );
    let (code, out) = clm(dir.path(), &["apply", "c"]);
    assert_eq!(code, Some(0), "{out}");
}

#[test]
fn hashes_prints_asset_lines() {
    // req:apply/hash-command@971912c5 req:apply/apply-assets@786adb62
    let dir = change_repo("---\ncomponent: clm\n---\n");
    write(dir.path(), "specs/a/SPEC.md", NODE);
    write(dir.path(), "specs/a/assets/m.html", "<p>m</p>\n");
    let (code, out) = clm(dir.path(), &["hashes", "a"]);
    assert_eq!(code, Some(0));
    let expected = format!("asset:m.html {}", clm_model::asset_hash(b"<p>m</p>\n"));
    assert_eq!(out.lines().last(), Some(expected.as_str()), "{out}");
}

#[test]
fn hashes_fails_on_an_unknown_node() {
    // req:apply/hash-command@971912c5
    let dir = change_repo("---\ncomponent: clm\n---\n");
    let out = call_with_root(dir.path(), &["hashes", "nope"]);
    let stderr = out.stderr;
    assert_eq!(out.status, Some(2));
    assert!(
        stderr.contains("clm hashes: no valid component `nope`"),
        "{stderr}"
    );
}

#[test]
fn apply_reports_and_exits_1_writing_nothing() {
    // req:apply/atomic@bb8aa354
    let dir = change_repo(
        "---\ncomponent: clm\nbase: { contract: \"0000000000000000\" }\n---\n## MODIFIED Contract\nc2\n",
    );
    let before = fs::read_to_string(dir.path().join("specs/SPEC.md")).unwrap();
    let (code, out) = clm(dir.path(), &["apply", "c"]);
    assert_eq!(code, Some(1));
    assert!(
        out.contains("changes/c/deltas/clm.md: apply/stale-base: "),
        "{out}"
    );
    assert_eq!(
        fs::read_to_string(dir.path().join("specs/SPEC.md")).unwrap(),
        before
    );
}

#[test]
fn apply_exits_1_on_a_stale_asset_and_reports_removals() {
    // req:apply/apply-assets@786adb62
    let dir = change_repo("---\ncomponent: clm\n---\n");
    let d = dir.path();
    write(d, "specs/a/SPEC.md", NODE);
    write(d, "specs/a/assets/m.html", "main's edit\n");
    write(d, "specs/a/assets/old.png", "png");
    write(
        d,
        "changes/c/deltas/a.md",
        "---\ncomponent: a\nbase: { assets: { \"m.html\": \"0000000000000000\" } }\n---\n",
    );
    write(d, "changes/c/assets/a/m.html", "new\n");
    let (code, out) = clm(d, &["apply", "c"]);
    assert_eq!(code, Some(1));
    assert!(
        out.contains("apply/stale-base: ") && out.contains("m.html"),
        "{out}"
    );
    assert_eq!(
        fs::read_to_string(d.join("specs/a/assets/m.html")).unwrap(),
        "main's edit\n"
    );

    let old = clm_model::asset_hash(b"png");
    let m = clm_model::asset_hash(b"main's edit\n");
    write(
        d,
        "changes/c/deltas/a.md",
        &format!(
            "---\ncomponent: a\nbase: {{ assets: {{ \"m.html\": \"{m}\" }} }}\n---\n## REMOVED Assets\n- `old.png` <!-- base: {old} -->\n"
        ),
    );
    let (code, out) = clm(d, &["apply", "c"]);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("  wrote specs/a/assets/m.html\n"), "{out}");
    assert!(out.contains("  removed specs/a/assets/old.png\n"), "{out}");
}

/// The archive half of the root's test of that name; its backlog half, which runs `clm
/// backlog`, is `check`'s, proven by `clm-check`'s `backlog_lists_each_item`.
#[test]
fn backlog_and_archive_commands() {
    let dir = change_repo("---\ncomponent: clm\n---\n");
    // req:apply/archive-command@c68462bb
    let (code, _) = clm(dir.path(), &["archive", "c"]);
    assert_eq!(code, Some(1), "proposed change must not archive");
    write(
        dir.path(),
        "changes/c/change.md",
        "---\nstatus: abandoned\n---\n",
    );
    let (code, out) = clm(dir.path(), &["archive", "c"]);
    assert_eq!(code, Some(0), "{out}");
    assert!(dir.path().join("changes/archive/c/change.md").is_file());
}

#[test]
fn hashes_of_layers() {
    // req:apply/hash-command@971912c5
    let dir = change_repo("---\ncomponent: clm\n---\n");
    let entry = |slug: &str| {
        format!("### Capability: {slug}\nOne can.\n\n#### Why\nw\n\n#### Scenario: s\n- ok\n")
    };
    // File order, not sorted: `b` first.
    let (a, b) = (entry("b"), entry("a"));
    write(
        dir.path(),
        "CAPABILITIES.md",
        &format!("## Capabilities\n\n{a}\n{b}"),
    );
    let x = "### Constraint: x\nIt SHALL.\n\n#### Scenario: s\n- ok\n";
    write(
        dir.path(),
        "ARCHITECTURE.md",
        &format!("## Constraints\n\n{x}"),
    );
    let hash = |text: &str| clm_model::content_hash(text);
    let (code, out) = clm(dir.path(), &["hashes", "capability"]);
    assert_eq!(code, Some(0), "{out}");
    // Each block runs to the next heading: the first holds the blank line before the second.
    let expected = format!(
        "capability:b {}\ncapability:a {}\n",
        hash(&format!("{a}\n")),
        hash(&b)
    );
    assert_eq!(out, expected);
    let (code, out) = clm(dir.path(), &["hashes", "architecture"]);
    assert_eq!(code, Some(0), "{out}");
    assert_eq!(out, format!("requirement:x {}\n", hash(x)));
}

#[test]
fn hashes_of_the_capability_node_come_first() {
    // req:apply/hash-command@971912c5
    let dir = change_repo("---\ncomponent: clm\n---\n");
    write(
        dir.path(),
        "CAPABILITIES.md",
        "## Capabilities\n\n### Capability: a\nOne can.\n\n#### Why\nw\n\n#### Scenario: s\n- ok\n",
    );
    write(
        dir.path(),
        "specs/capability/SPEC.md",
        "---\ntitle: Capabilities\nstatus: specd\nowners: []\ndepends_on: []\n---\n\n## Contract\nc\n\n## Design\nd\n",
    );
    let (code, out) = clm(dir.path(), &["hashes", "capability"]);
    assert_eq!(code, Some(0), "{out}");
    let elements: Vec<_> = out.lines().map(|l| l.split(' ').next().unwrap()).collect();
    assert_eq!(
        elements,
        ["frontmatter", "contract", "design", "capability:a"]
    );
    let design = out.lines().nth(2).unwrap().split(' ').nth(1).unwrap();
    write(
        dir.path(),
        "changes/c/deltas/clm.md",
        &format!(
            "---\ncomponent: capability\nbase: {{ design: \"{design}\" }}\n---\n## MODIFIED Design\nnew\n"
        ),
    );
    let (code, out) = clm(dir.path(), &["apply", "c"]);
    assert_eq!(code, Some(0), "{out}");
}
