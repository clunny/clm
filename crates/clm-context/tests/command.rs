//! `clm context` as `clm-context`'s call: what it writes to each stream and the status it
//! returns (`commands-in-crate`).

use std::fs;
use std::path::Path;

use tempfile::TempDir;

/// What one call of `clm context` wrote to each stream, and the status it returned.
struct Output {
    status: Status,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

/// Calls `clm context` with `args` and `--root <root>`, the repository root it is given
/// being a directory that holds no repository, so only `--root` can find one.
fn clm_context(root: &Path, args: &[&str]) -> Output {
    let mut line: Vec<&std::ffi::OsStr> = args.iter().map(std::ffi::OsStr::new).collect();
    line.push("--root".as_ref());
    line.push(root.as_os_str());
    call(Path::new("/nonexistent-clm-root"), &line)
}

fn call(root: &Path, args: &[&std::ffi::OsStr]) -> Output {
    let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
    let status = clm_context::command(
        args.iter().copied(),
        root,
        &|_| None,
        &mut stdout,
        &mut stderr,
    );
    Output {
        status: Status(status),
        stdout,
        stderr,
    }
}

/// The status a call returned, read as a process's exit status is.
struct Status(u8);

impl Status {
    fn code(&self) -> Option<i32> {
        Some(i32::from(self.0))
    }
}

fn stdout(out: &Output) -> String {
    String::from_utf8(out.stdout.clone()).unwrap()
}

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

/// A node spec whose contract and design are `<NAME>-CONTRACT` and `<NAME>-DESIGN`.
fn node(root: &Path, dir: &str, name: &str, deps: &str) {
    let rel = if dir.is_empty() {
        "specs/SPEC.md".to_string()
    } else {
        format!("specs/{dir}/SPEC.md")
    };
    write(
        root,
        &rel,
        &format!(
            "---\ntitle: {name}\nstatus: specd\nowners: []\ndepends_on: [{deps}]\n---\n\n\
             ## Contract\n{name}-CONTRACT\n\n## Design\n{name}-DESIGN\n\n## Requirements\n\n\
             ### Requirement: {lower}-req\n{name}-REQ\n\n#### Scenario: s\n- GIVEN x\n",
            lower = name.to_lowercase()
        ),
    );
}

fn repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "ARCHITECTURE.md", "ARCH-TEXT\n");
    node(dir.path(), "", "ROOT", "");
    node(dir.path(), "a", "A", "c, b");
    node(dir.path(), "a/n", "N", "b");
    node(dir.path(), "b", "B", "");
    node(dir.path(), "c", "C", "");
    write(
        dir.path(),
        "backlog/x.md",
        "---\ntitle: X\nkind: idea\ncomponents: [a]\n---\nBACKLOG-TEXT\n",
    );
    write(
        dir.path(),
        "docs/decisions/d.md",
        "---\ntitle: D\ndate: 2026-10-21\nstatus: accepted\n---\nDECISION-TEXT\n",
    );
    write(
        dir.path(),
        "changes/20261021-x/change.md",
        "---\nstatus: proposed\n---\n\n## Intent\nCHANGE-TEXT\n",
    );
    dir
}

#[test]
fn dependency_contract_only() {
    // req:context/bundle-contents@ffb0697c
    let dir = repo();
    write(dir.path(), "specs/b/assets/bm.html", "BM-CONTENT\n");
    let out = clm_context(dir.path(), &["a"]);
    assert_eq!(out.status.code(), Some(0));
    let text = stdout(&out);
    for present in [
        "ARCH-TEXT",
        "ROOT-CONTRACT",
        "ROOT-DESIGN",
        "A-CONTRACT",
        "A-DESIGN",
        "A-REQ",
        "depends_on: [c, b]",
        "B-CONTRACT",
        "C-CONTRACT",
    ] {
        assert!(text.contains(present), "missing {present}:\n{text}");
    }
    for absent in [
        "B-DESIGN",
        "B-REQ",
        "b-req",
        "ROOT-REQ",
        "N-CONTRACT",
        "BACKLOG-TEXT",
        "DECISION-TEXT",
        "CHANGE-TEXT",
        "bm.html",
        "BM-CONTENT",
        "### Assets",
    ] {
        assert!(!text.contains(absent), "leaked {absent}:\n{text}");
    }
}

#[test]
fn assets_listed_not_included() {
    // req:context/bundle-contents@ffb0697c
    let dir = repo();
    write(dir.path(), "specs/a/assets/mockups/m.html", "M-CONTENT\n");
    write(dir.path(), "specs/a/assets/b.png", "B-CONTENT\n");
    let text = stdout(&clm_context(dir.path(), &["a"]));
    let expected = format!(
        "### Assets\n\n- specs/a/assets/b.png {}\n- specs/a/assets/mockups/m.html {}\n",
        clm_model::asset_hash(b"B-CONTENT\n"),
        clm_model::asset_hash(b"M-CONTENT\n"),
    );
    assert!(text.contains(&expected), "{text}");
    assert!(text.find("A-REQ").unwrap() < text.find("### Assets").unwrap());
    assert!(text.find("### Assets").unwrap() < text.find("# Dependencies").unwrap());
    assert!(!text.contains("M-CONTENT"), "{text}");

    let hash = || stdout(&clm_context(dir.path(), &["a", "--hash"]));
    let before = hash();
    write(
        dir.path(),
        "specs/a/assets/mockups/m.html",
        "M-CONTENT-EDITED\n",
    );
    assert_ne!(hash(), before, "an asset's bytes are in the bundle hash");
}

#[test]
fn spine_assets_stay_out() {
    // req:context/bundle-contents@ffb0697c
    let dir = repo();
    write(dir.path(), "specs/a/assets/m.html", "M\n");
    write(dir.path(), "specs/a/n/assets/x.bin", "X\n");
    let text = stdout(&clm_context(dir.path(), &["a.n"]));
    assert!(!text.contains("m.html"), "{text}");
    assert!(
        text.contains(&format!(
            "- specs/a/n/assets/x.bin {}",
            clm_model::asset_hash(b"X\n")
        )),
        "the node path, not the ID:\n{text}"
    );
}

#[test]
fn bundle_from_equals_checkout() {
    // req:context/bundle-from-text@418dfef8
    let dir = repo();
    write(dir.path(), "specs/a/assets/m.html", "M\n");
    let (graph, violations) = clm_model::load(dir.path());
    assert!(violations.is_empty(), "{violations:?}");
    let arch = fs::read_to_string(dir.path().join("ARCHITECTURE.md")).unwrap();
    assert_eq!(
        clm_context::bundle_from(&graph, Some(&arch), "a").unwrap(),
        stdout(&clm_context(dir.path(), &["a"])),
    );

    fs::remove_file(dir.path().join("ARCHITECTURE.md")).unwrap();
    let without = stdout(&clm_context(dir.path(), &["a"]));
    assert!(!without.contains("ARCH-TEXT"), "{without}");
    assert_eq!(
        clm_context::bundle_from(&graph, None, "a").unwrap(),
        without
    );
    assert_eq!(clm_context::bundle_from(&graph, None, "zz"), None);
}

#[test]
fn bundle_from_equals_checkout_with_capabilities() {
    // req:context/bundle-from-text@418dfef8
    // req:context/bundle-contents@ffb0697c
    let dir = repo();
    write(
        dir.path(),
        "CAPABILITIES.md",
        "## Capabilities\n\n### Capability: k\nCAP-K\n\n#### Why\nw\n\n#### Scenario: s\n- ok\n\n\
         ### Capability: u\nCAP-U\n\n#### Why\nw\n\n#### Scenario: s\n- ok\n",
    );
    let spec = dir.path().join("specs/a/SPEC.md");
    let text = fs::read_to_string(&spec).unwrap();
    fs::write(
        &spec,
        text.replace("owners: []\n", "owners: []\nserves: [capability/k]\n"),
    )
    .unwrap();
    let (graph, violations) = clm_model::load(dir.path());
    assert!(violations.is_empty(), "{violations:?}");

    let out = stdout(&clm_context(dir.path(), &["a"]));
    assert!(
        out.contains("# Capabilities served\n\n### Capability: k\nCAP-K"),
        "{out}"
    );
    assert!(!out.contains("CAP-U"), "{out}");
    let arch = fs::read_to_string(dir.path().join("ARCHITECTURE.md")).unwrap();
    assert_eq!(
        clm_context::bundle_from(&graph, Some(&arch), "a").unwrap(),
        out
    );
}

#[test]
fn nested_node_sees_its_spine_in_order() {
    // req:context/bundle-contents@ffb0697c
    let dir = repo();
    let text = stdout(&clm_context(dir.path(), &["a.n"]));
    let pos = |s: &str| {
        text.find(s)
            .unwrap_or_else(|| panic!("missing {s}:\n{text}"))
    };
    assert!(pos("ROOT-CONTRACT") < pos("ROOT-DESIGN"));
    assert!(pos("ROOT-DESIGN") < pos("A-CONTRACT"));
    assert!(pos("A-DESIGN") < pos("N-CONTRACT"));
    assert!(pos("N-DESIGN") < pos("N-REQ"));
    assert!(!text.contains("A-REQ"), "{text}");
}

#[test]
fn no_architecture_file_no_heading() {
    // req:context/bundle-contents@ffb0697c
    let dir = repo();
    fs::remove_file(dir.path().join("ARCHITECTURE.md")).unwrap();
    let out = clm_context(dir.path(), &["a"]);
    assert_eq!(out.status.code(), Some(0));
    let text = stdout(&out);
    assert!(!text.lines().any(|l| l == "# ARCHITECTURE.md"), "{text}");
    assert!(text.contains("A-CONTRACT"), "{text}");
}

#[test]
fn dependencies_left_after_the_ancestors() {
    // req:context/bundle-contents@ffb0697c
    let dir = repo();
    // `a.n` depends on `a`, its parent, and nothing else: no dependency is left.
    node(dir.path(), "a/n", "N", "a");
    let text = stdout(&clm_context(dir.path(), &["a.n"]));
    assert!(text.lines().any(|l| l == "# Ancestors"), "{text}");
    assert!(!text.lines().any(|l| l == "# Dependencies"), "{text}");
    assert_eq!(text.matches("A-CONTRACT").count(), 1, "{text}");

    // With `b` as well, `b` alone is printed under `# Dependencies`, `a` stays an ancestor.
    node(dir.path(), "a/n", "N", "a, b");
    let text = stdout(&clm_context(dir.path(), &["a.n"]));
    let deps = text.find("\n# Dependencies\n").expect(&text);
    assert!(
        text[deps..].contains("## b: B\n\n### Contract\n\nB-CONTRACT"),
        "{text}"
    );
    assert!(!text[deps..].contains("## a: A"), "{text}");
    assert_eq!(text.matches("A-CONTRACT").count(), 1, "{text}");
}

#[test]
fn identical_runs_and_sorted_dependencies() {
    // req:context/bundle-deterministic@cb7fa43a
    let dir = repo();
    let first = stdout(&clm_context(dir.path(), &["a"]));
    let second = stdout(&clm_context(dir.path(), &["a"]));
    assert_eq!(first, second);
    assert!(first.find("B-CONTRACT").unwrap() < first.find("C-CONTRACT").unwrap());
    assert!(first.ends_with("\n") && !first.ends_with("\n\n"));
}

#[test]
fn the_layout() {
    // req:context/bundle-deterministic@cb7fa43a
    // The source's `ARCHITECTURE.md` (an entry holds `#### Carries`), so no carried section;
    // the root's Contract is empty, `a`'s Contract and requirement lines end in whitespace.
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(
        root,
        "ARCHITECTURE.md",
        "## Constraints\n\n### Constraint: x\nIt SHALL x.\n\n#### Scenario: s\n- ok\n\n\
         #### Carries\n- method\n",
    );
    write(
        root,
        "CAPABILITIES.md",
        "## Capabilities\n\n### Capability: k\nCAP-K\n\n#### Why\nw\n\n#### Scenario: s\n- ok\n",
    );
    write(
        root,
        "specs/SPEC.md",
        "---\ntitle: Root\nstatus: active\nowners: []\n---\n\n## Contract\n\n## Design\nROOT-DESIGN\n",
    );
    write(
        root,
        "specs/a/SPEC.md",
        "---\ntitle: A\nstatus: active\nowners: []\ndepends_on: [b]\nserves: [capability/k]\n---\n\n\
         ## Contract\nA-CONTRACT  \nsecond line\t\n\n## Design\nA-DESIGN\n\n## Requirements\n\n\
         ### Requirement: r\nIt SHALL r.   \n\n#### Scenario: s\n- ok\n",
    );
    write(
        root,
        "specs/b/SPEC.md",
        "---\ntitle: B\nstatus: active\nowners: []\n---\n\n## Contract\nB-CONTRACT\n\n\
         ## Design\nB-DESIGN\n",
    );
    write(root, "specs/a/assets/mockups/m.html", "M\n");
    write(root, "specs/a/assets/b.png", "B\n");

    let out = clm_context(root, &["a"]);
    assert_eq!(out.status.code(), Some(0));
    let text = stdout(&out);
    assert!(
        text.lines().all(|l| !l.ends_with([' ', '\t'])),
        "a line ends in whitespace:\n{text}"
    );
    assert!(text.ends_with('\n'), "{text}");
    let fence = "```";
    let expected = format!(
        "# ARCHITECTURE.md\n\n\
         ## Constraints\n\n### Constraint: x\nIt SHALL x.\n\n#### Scenario: s\n- ok\n\n\
         #### Carries\n- method\n\n\
         # Capabilities served\n\n\
         ### Capability: k\nCAP-K\n\n#### Why\nw\n\n#### Scenario: s\n- ok\n\n\
         # Ancestors\n\n\
         ## clm: Root\n\n### Contract\n\n\n\n### Design\n\nROOT-DESIGN\n\n\
         ## a: A\n\n### Frontmatter\n\n\
         {fence}yaml\ntitle: A\nstatus: active\nowners: []\ndepends_on: [b]\n\
         serves: [capability/k]\n{fence}\n\n\
         ### Contract\n\nA-CONTRACT\nsecond line\n\n### Design\n\nA-DESIGN\n\n\
         ### Requirements\n\n### Requirement: r\nIt SHALL r.\n\n#### Scenario: s\n- ok\n\n\
         ### Assets\n\n- specs/a/assets/b.png {b}\n- specs/a/assets/mockups/m.html {m}\n\n\
         # Dependencies\n\n## b: B\n\n### Contract\n\nB-CONTRACT\n",
        b = clm_model::asset_hash(b"B\n"),
        m = clm_model::asset_hash(b"M\n"),
    );
    assert_eq!(text, expected);
}

#[test]
fn hash_follows_the_bundle_only() {
    // req:context/bundle-hash@7010972d
    let dir = repo();
    let hash = || stdout(&clm_context(dir.path(), &["a", "--hash"]));
    let before = hash();
    let bundle = stdout(&clm_context(dir.path(), &["a"]));
    assert_eq!(before.trim().len(), 16, "{before}");
    assert_eq!(before.trim(), clm_model::content_hash(&bundle));

    let b = dir.path().join("specs/b/SPEC.md");
    let source = fs::read_to_string(&b).unwrap();
    fs::write(&b, source.replace("B-DESIGN", "B-DESIGN-EDITED")).unwrap();
    assert_eq!(hash(), before, "dependency design is outside the bundle");

    let source = fs::read_to_string(&b).unwrap();
    fs::write(&b, source.replace("B-CONTRACT", "B-CONTRACT-EDITED")).unwrap();
    assert_ne!(hash(), before, "dependency contract is inside the bundle");
}

#[test]
fn unknown_node_exits_2() {
    // req:context/bundle-errors@ef725bca
    let dir = repo();
    let out = clm_context(dir.path(), &["zz"]);
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(
        String::from_utf8(out.stderr).unwrap(),
        "clm context: no component `zz`\n"
    );
    assert!(out.stdout.is_empty());
}

#[test]
fn invalid_graph_prints_violations_not_a_bundle() {
    // req:context/bundle-errors@ef725bca
    let dir = repo();
    node(dir.path(), "b", "B", "missing");
    let (_, violations) = clm_model::load(dir.path());
    let lines: String = violations.iter().map(|v| format!("{v}\n")).collect();
    assert!(lines.contains("model/dependency-dag"), "{lines}");
    // The same whether or not the component exists.
    for id in ["a", "zz"] {
        let out = clm_context(dir.path(), &[id]);
        let text = stdout(&out);
        assert_eq!(out.status.code(), Some(1), "{text}");
        assert_eq!(text, lines);
        assert!(!text.contains("A-CONTRACT"), "{text}");
        assert!(out.stderr.is_empty(), "{:?}", String::from_utf8(out.stderr));
    }
}

#[test]
fn the_call_writes_what_the_command_prints() {
    // req:context/commands-in-crate@d15a10ad
    // req:context/bundle-components@29ea8d19
    let dir = repo();
    let (graph, _) = clm_model::load(dir.path());
    let bundle = clm_context::bundle(&graph, dir.path(), "a").unwrap();
    let run = |args: &[&str]| {
        let args: Vec<&std::ffi::OsStr> = args.iter().map(std::ffi::OsStr::new).collect();
        call(dir.path(), &args)
    };

    let out = run(&["a"]);
    assert_eq!(out.status.0, 0);
    assert_eq!(stdout(&out), bundle);
    assert!(out.stderr.is_empty());

    let out = run(&["a", "--hash"]);
    assert_eq!(out.status.0, 0);
    assert_eq!(stdout(&out), format!("{}\n", clm_context::hash(&bundle)));

    let out = run(&["zz"]);
    assert_eq!(out.status.0, 2);
    assert!(out.stdout.is_empty());
    assert_eq!(
        String::from_utf8(out.stderr).unwrap(),
        "clm context: no component `zz`\n"
    );

    node(dir.path(), "b", "B", "missing");
    let (_, violations) = clm_model::load(dir.path());
    let lines: String = violations.iter().map(|v| format!("{v}\n")).collect();
    let out = run(&["a"]);
    assert_eq!(out.status.0, 1);
    assert_eq!(stdout(&out), lines);
    assert!(out.stderr.is_empty());
}

#[test]
fn bundle_gives_the_text_whatever_the_graph_violates() {
    // req:context/bundle-components@29ea8d19
    let dir = repo();
    let (clean, _) = clm_model::load(dir.path());
    let before = clm_context::bundle(&clean, dir.path(), "a").unwrap();

    // `c` now depends on a component that doesn't exist: the command prints no bundle, but
    // `bundle` still gives the text for the graph it is given.
    node(dir.path(), "c", "C", "missing");
    let (graph, violations) = clm_model::load(dir.path());
    assert!(!violations.is_empty());
    assert_eq!(clm_context(dir.path(), &["a"]).status.code(), Some(1));
    let arch = fs::read_to_string(dir.path().join("ARCHITECTURE.md")).unwrap();
    let text = clm_context::bundle(&graph, dir.path(), "a").unwrap();
    assert_eq!(text, before);
    assert_eq!(
        Some(text),
        clm_context::bundle_from(&graph, Some(&arch), "a")
    );
    assert_eq!(clm_context::bundle(&graph, dir.path(), "zz"), None);
    assert_eq!(clm_context::bundle(&clean, dir.path(), "zz"), None);
}

#[test]
fn help_writes_the_usage_to_the_output_stream() {
    // req:context/commands-in-crate@d15a10ad
    let dir = repo();
    let out = call(dir.path(), &["--help".as_ref()]);
    assert_eq!(out.status.0, 0);
    assert!(out.stderr.is_empty());
    let text = stdout(&out);
    assert!(text.contains("Usage: clm context"), "{text}");
}

/// A stream every write to fails.
struct Closed;

impl std::io::Write for Closed {
    fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
        Err(std::io::ErrorKind::BrokenPipe.into())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn a_stream_it_cant_write_returns_2() {
    // req:context/commands-in-crate@d15a10ad
    let dir = repo();
    let run = |args: &[&str], out: &mut dyn std::io::Write, err: &mut dyn std::io::Write| {
        let args: Vec<&std::ffi::OsStr> = args.iter().map(std::ffi::OsStr::new).collect();
        clm_context::command(args, dir.path(), &|_| None, out, err)
    };
    let mut sink = Vec::new();
    // The bundle, its hash and the usage go to a closed output stream.
    assert_eq!(run(&["a"], &mut Closed, &mut sink), 2);
    assert_eq!(run(&["a", "--hash"], &mut Closed, &mut sink), 2);
    assert_eq!(run(&["--help"], &mut Closed, &mut sink), 2);
    // The unknown component's line and the usage error go to a closed error stream.
    assert_eq!(run(&["zz"], &mut sink, &mut Closed), 2);
    assert_eq!(run(&["--no-such-option"], &mut sink, &mut Closed), 2);
    assert!(sink.is_empty());
}

#[test]
fn arguments_it_cant_parse_return_2() {
    // req:context/commands-in-crate@d15a10ad
    let dir = repo();
    let args: Vec<&std::ffi::OsStr> = ["a", "--no-such-option"]
        .iter()
        .map(std::ffi::OsStr::new)
        .collect();
    let out = call(dir.path(), &args);
    assert_eq!(out.status.0, 2);
    assert!(out.stdout.is_empty());
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(err.contains("--no-such-option"), "{err}");
    assert!(err.contains("Usage: clm context"), "{err}");
}
