//! `clm view <name>`: the spec, flight and blockers views from the working tree and the
//! local refs, never fetching.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

use tempfile::TempDir;

const REQ: &str = concat!("req", ":");

/// `clm <args> --root <root>`, with no user settings.
fn clm(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_clm"))
        .env(
            "XDG_CONFIG_HOME",
            Path::new(env!("CARGO_TARGET_TMPDIR")).join("no-user-config"),
        )
        .args(args)
        .arg("--root")
        .arg(root)
        .output()
        .unwrap()
}

fn stdout(out: &Output) -> String {
    String::from_utf8(out.stdout.clone()).unwrap()
}

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

const ROOT: &str = "---\ntitle: R\nstatus: active\nowners: []\n---\n";

/// A `specd` node depending on nothing, each requirement `(slug, needs)` needing those listed.
fn needs(reqs: &[(&str, &[&str])]) -> String {
    let mut s = node("", &[]);
    for (r, needed) in reqs {
        let list: String = needed.iter().map(|n| format!("- {n}\n")).collect();
        s.push_str(&format!(
            "\n### Requirement: {r}\nIt SHALL.\n\n#### Needs\n{list}\n#### Scenario: s\n- ok\n"
        ));
    }
    s
}

/// A `specd` node depending on `deps` with requirements `reqs`.
fn node(deps: &str, reqs: &[&str]) -> String {
    let mut s = format!(
        "---\ntitle: T\nstatus: specd\nowners: []\ndepends_on: [{deps}]\n---\n\n## Requirements\n"
    );
    for r in reqs {
        s.push_str(&format!(
            "\n### Requirement: {r}\nIt SHALL.\n\n#### Scenario: s\n- ok\n"
        ));
    }
    s
}

/// Nodes `a` and `b`, `b` depending on `a`.
fn two_nodes() -> TempDir {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "specs/SPEC.md", ROOT);
    write(dir.path(), "specs/a/SPEC.md", &node("", &["x"]));
    write(dir.path(), "specs/b/SPEC.md", &node("a", &["y"]));
    dir
}

fn git_env(dir: &Path, args: &[&str], env: &[(&str, &str)]) -> String {
    let out = Command::new("git")
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .envs(env.iter().copied())
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}: {out:?}");
    String::from_utf8(out.stdout).unwrap()
}

fn git(dir: &Path, args: &[&str]) -> String {
    git_env(dir, args, &[])
}

/// Makes `root` a repository with one commit on `main`.
fn init(root: &Path) {
    git(root, &["init", "-q", "-b", "main"]);
    git(root, &["add", "-A"]);
    git(root, &["commit", "-qm", "base"]);
}

/// A remote-tracking ref `origin/impl/<node>` on a commit made `hours` hours ago, as a
/// fetch would leave it; nothing is fetched.
fn claim_ref(root: &Path, node: &str, hours: u64) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let date = format!("@{} +0000", now - hours * 3600);
    let tree = git(root, &["rev-parse", "HEAD^{tree}"]);
    let head = git(root, &["rev-parse", "HEAD"]);
    let sha = git_env(
        root,
        &["commit-tree", tree.trim(), "-p", head.trim(), "-m", "claim"],
        &[("GIT_COMMITTER_DATE", &date), ("GIT_AUTHOR_DATE", &date)],
    );
    git(
        root,
        &[
            "update-ref",
            &format!("refs/remotes/origin/impl/{node}"),
            sha.trim(),
        ],
    );
}

fn json(out: &Output) -> serde_json::Value {
    serde_json::from_str(&stdout(out)).unwrap_or_else(|e| panic!("{e}: {}", stdout(out)))
}

#[test]
fn scenario_plain_chart() {
    // req:view/view-command@0fb30947
    let dir = two_nodes();
    let out = clm(dir.path(), &["view", "spec"]);
    let text = stdout(&out);
    assert_eq!(out.status.code(), Some(0), "{text}");
    assert!(text.contains(" a ") && text.contains(" b "), "{text}");
    assert!(!text.contains('\u{1b}'), "{text}");
    assert!(text.contains("└─") || text.contains("├─"), "{text}");
}

#[test]
fn scenario_coloured_chart() {
    // req:view/view-command@0fb30947
    let dir = two_nodes();
    let plain = stdout(&clm(dir.path(), &["view", "spec"]));
    let ansi = stdout(&clm(dir.path(), &["view", "spec", "--format", "ansi"]));
    assert!(ansi.contains("\u{1b}["), "{ansi}");
    let mut stripped = String::new();
    let mut chars = ansi.chars();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            for c in chars.by_ref() {
                if c == 'm' {
                    break;
                }
            }
        } else {
            stripped.push(c);
        }
    }
    assert_eq!(stripped, plain);
}

#[test]
fn scenario_width() {
    // req:view/view-command@0fb30947
    let dir = TempDir::new().unwrap();
    let long = "n".repeat(60);
    write(dir.path(), "specs/SPEC.md", ROOT);
    write(dir.path(), "specs/a/SPEC.md", &node("", &["x"]));
    write(
        dir.path(),
        &format!("specs/{long}/SPEC.md"),
        &node("a", &["y"]),
    );
    let out = clm(dir.path(), &["view", "spec", "--width", "40"]);
    let text = stdout(&out);
    assert_eq!(out.status.code(), Some(0), "{text}");
    for line in text.lines() {
        assert!(line.chars().count() <= 40, "{line:?}");
    }
    assert!(text.contains('…'), "{text}");
    // The default is 100 columns: a longer line is cut there.
    let longer = "m".repeat(120);
    write(
        dir.path(),
        &format!("specs/{longer}/SPEC.md"),
        &node("a", &["y"]),
    );
    let text = stdout(&clm(dir.path(), &["view", "spec"]));
    let widest = text.lines().map(|l| l.chars().count()).max().unwrap();
    assert_eq!(widest, 100, "{text}");
    assert!(text.contains(&long) && !text.contains(&longer), "{text}");
}

#[test]
fn scenario_svg() {
    // req:view/view-command@0fb30947
    let dir = two_nodes();
    let out = clm(dir.path(), &["view", "spec", "--format", "svg"]);
    let text = stdout(&out);
    assert_eq!(out.status.code(), Some(0), "{text}");
    let doc = roxmltree::Document::parse(&text).unwrap_or_else(|e| panic!("{e}: {text}"));
    assert_eq!(doc.root_element().tag_name().name(), "svg");
    // The labels: the text of the `<text>` elements, not the embedded stylesheet.
    let texts: Vec<String> = doc
        .descendants()
        .filter(|n| n.has_tag_name("text"))
        .map(|n| n.descendants().filter_map(|t| t.text()).collect::<String>())
        .collect();
    assert!(texts.iter().any(|t| t.starts_with("a ")), "{texts:?}");
    assert!(texts.iter().any(|t| t.starts_with("b ")), "{texts:?}");
    for view in ["flight", "blockers"] {
        let text = stdout(&clm(dir.path(), &["view", view, "--format", "svg"]));
        let doc = roxmltree::Document::parse(&text).unwrap_or_else(|e| panic!("{e}: {text}"));
        assert_eq!(doc.root_element().tag_name().name(), "svg");
    }
}

#[test]
fn scenario_offline() {
    // req:view/view-command@0fb30947
    let dir = TempDir::new().unwrap();
    write(dir.path(), "specs/SPEC.md", ROOT);
    write(dir.path(), "specs/x/SPEC.md", &node("", &["p"]));
    init(dir.path());
    git(
        dir.path(),
        &[
            "remote",
            "add",
            "origin",
            "https://127.0.0.1:9/unreachable.git",
        ],
    );
    claim_ref(dir.path(), "x", 1);
    let out = clm(dir.path(), &["view", "flight"]);
    let text = stdout(&out);
    assert_eq!(out.status.code(), Some(0), "{text}");
    let line = text
        .lines()
        .find(|l| l.starts_with("  x "))
        .unwrap_or_else(|| panic!("{text}"));
    assert!(line.contains("claimed"), "{text}");
    let v = json(&clm(dir.path(), &["view", "flight", "--format", "json"]));
    assert_eq!(v["nodes"][0]["state"], "claimed");
}

#[test]
fn scenario_unknown_view() {
    // req:view/view-command@0fb30947
    let dir = two_nodes();
    let out = clm(dir.path(), &["view", "everything"]);
    assert_eq!(out.status.code(), Some(2));
    let err = String::from_utf8(out.stderr).unwrap();
    for name in ["spec", "flight", "blockers", "queue"] {
        assert!(err.contains(name), "{err}");
    }
    for format in ["pdf", "png"] {
        let out = clm(dir.path(), &["view", "spec", "--format", format]);
        assert_eq!(out.status.code(), Some(2), "{format}");
    }
    let out = clm(dir.path(), &["view", "queue", "--format", "svg"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8(out.stderr).unwrap().contains("svg"));
    let out = clm(dir.path(), &["view", "spec", "--width", "0"]);
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn violations_exit_1() {
    // req:view/view-command@0fb30947
    let dir = two_nodes();
    write(dir.path(), "specs/c/SPEC.md", "---\ntitle: C\n---\n");
    let out = clm(dir.path(), &["view", "spec"]);
    let text = stdout(&out);
    assert_eq!(out.status.code(), Some(1), "{text}");
    assert!(text.contains("model/valid-frontmatter"), "{text}");
}

#[test]
fn scenario_nested_nodes() {
    // req:view/spec-view@380d5f8a
    let dir = TempDir::new().unwrap();
    write(dir.path(), "specs/SPEC.md", ROOT);
    write(dir.path(), "specs/check/SPEC.md", &node("", &["c"]));
    write(
        dir.path(),
        "specs/check/drift/SPEC.md",
        &node("model", &["x", "y"]).replace("owners: []", "owners: [\"@a\"]"),
    );
    write(dir.path(), "specs/model/SPEC.md", &node("", &["m"]));
    let (graph, _) = clm_model::load(dir.path());
    let x = graph.requirement("check.drift/x").unwrap().version();
    write(
        dir.path(),
        "tests/t.rs",
        &format!("// {REQ}check.drift/x@{x}\n// {REQ}check.drift/y@00000000\n"),
    );
    let v = json(&clm(dir.path(), &["view", "spec", "--format", "json"]));
    assert_eq!(v["view"], "spec");
    let nodes = v["nodes"].as_array().unwrap();
    let find = |id: &str| nodes.iter().find(|n| n["node"] == id).unwrap();
    assert_eq!(find("check.drift")["parent"], "check");
    assert_eq!(find("check")["parent"], "clm");
    assert_eq!(find("check.drift")["owners"], serde_json::json!(["@a"]));
    assert_eq!(
        find("check.drift")["depends_on"],
        serde_json::json!(["model"])
    );
    for (key, n) in [
        ("current", 1),
        ("behind", 1),
        ("unbuilt", 0),
        ("obsolete", 0),
        ("total", 2),
    ] {
        assert_eq!(find("check.drift")[key], n, "{key}");
    }
    assert_eq!(find("check.drift").get("specd"), None);
    // Scenario: tree chart.
    let text = stdout(&clm(dir.path(), &["view", "spec"]));
    let lines: Vec<&str> = text.lines().collect();
    let check = lines.iter().position(|l| l.contains("check ")).unwrap();
    let drift = lines
        .iter()
        .position(|l| l.contains("check.drift"))
        .unwrap();
    assert!(drift > check, "{text}");
    let indent = |l: &str| l.find(|c: char| c.is_alphanumeric()).unwrap();
    assert!(indent(lines[drift]) > indent(lines[check]), "{text}");
    for fact in ["█▓", "1/2", "@a", "model"] {
        assert!(lines[drift].contains(fact), "{fact} missing from {text}");
    }
}

#[test]
fn scenario_node_detail_from_the_command() {
    // req:view/component-detail@5158adfc
    let dir = two_nodes();
    write(
        dir.path(),
        "specs/a/SPEC.md",
        &node("", &["x"]).replace(
            "\n## Requirements",
            "\n## Contract\nAlpha does X.\n\n## Requirements",
        ),
    );
    write(
        dir.path(),
        "changes/20260101-c/change.md",
        "---\nstatus: applied\n---\n",
    );
    write(
        dir.path(),
        "changes/20260101-c/deltas/a.md",
        "---\ncomponent: a\n---\n## ADDED Requirements\n### Requirement: x\nIt SHALL.\n\n#### Scenario: s\n- ok\n",
    );
    write(
        dir.path(),
        "backlog/q.md",
        "---\ntitle: Ask\nkind: question\ncomponents: [a]\n---\n",
    );
    let v = json(&clm(dir.path(), &["view", "spec", "--format", "json"]));
    let nodes = v["nodes"].as_array().unwrap();
    let find = |id: &str| nodes.iter().find(|n| n["node"] == id).unwrap();
    let a = find("a");
    assert_eq!(a["contract"], "Alpha does X.", "{a:#}");
    assert_eq!(
        a["changes"],
        serde_json::json!([{ "change": "20260101-c", "status": "applied",
                             "current": 0, "total": 1, "complete": false }])
    );
    assert_eq!(
        a["backlog"],
        serde_json::json!([{ "slug": "q", "kind": "question", "title": "Ask" }])
    );
    let b = find("b");
    assert_eq!(b["changes"], serde_json::json!([]));
    assert_eq!(b["backlog"], serde_json::json!([]));
    assert_eq!(b["contract"], "");
}

#[test]
fn scenario_oversized_on_the_chart() {
    // req:view/spec-view@380d5f8a
    let dir = two_nodes();
    // `a`'s spec is over the repository's spec budget; it sets no `size_budget`.
    write(
        dir.path(),
        "specs/a/SPEC.md",
        &format!("{}\n{}\n", node("", &["x"]), "x".repeat(800)),
    );
    write(
        dir.path(),
        "clm.toml",
        "[component_size]\nspec_tokens = 150\n",
    );
    init(dir.path());
    let out = clm(dir.path(), &["view", "spec", "--format", "json"]);
    let v = json(&out);
    let find = |id: &str| {
        v["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|n| n["node"] == id)
            .unwrap()
            .clone()
    };
    assert_eq!(find("a")["oversized"], true, "{v:#}");
    assert_eq!(find("b")["oversized"], false, "{v:#}");
    let text = stdout(&clm(dir.path(), &["view", "spec"]));
    let line = |id: &str| {
        text.lines()
            .find(|l| {
                l.trim_start_matches(|c: char| !c.is_alphanumeric())
                    .starts_with(&format!("{id} "))
            })
            .unwrap_or_else(|| panic!("no {id} in {text}"))
    };
    assert!(line("a").contains("⚠ oversized"), "{text}");
    assert!(!line("b").contains('⚠'), "{text}");
}

#[test]
fn sizes_that_cant_be_checked_mark_nothing() {
    // req:view/spec-view@380d5f8a
    let dir = two_nodes();
    write(
        dir.path(),
        "clm.toml",
        "[component_size]\nspec_tokens = \"many\"\n",
    );
    let out = clm(dir.path(), &["view", "spec", "--format", "json"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let stderr = String::from_utf8(out.stderr.clone()).unwrap();
    assert!(stderr.starts_with("size not checked: "), "{stderr}");
    for n in json(&out)["nodes"].as_array().unwrap() {
        assert_eq!(n.get("oversized"), None, "{n:#}");
    }
}

#[test]
fn scenario_a_removed_nodes_markers() {
    // req:view/spec-view@380d5f8a
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    write(
        d,
        "specs/a/SPEC.md",
        &node("", &[]).replace("depends_on", "paths: [\"src/a/**\"]\ndepends_on"),
    );
    write(d, "changes/d/change.md", "---\nstatus: applied\n---\n");
    write(
        d,
        "changes/d/deltas/gone.md",
        "---\ncomponent: gone\nremove: true\n---\n## REMOVED Requirements\n\n\
         ### Requirement: x <!-- base: 0123456789abcdef -->\n",
    );
    write(d, "src/a/t.rs", &format!("// {REQ}gone/x\n"));
    let out = clm(d, &["view", "spec", "--format", "json"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let v = json(&out);
    let nodes = v["nodes"].as_array().unwrap();
    let a = nodes.iter().find(|n| n["node"] == "a").unwrap();
    assert_eq!(a["obsolete"], 1, "{v:#}");
    assert!(nodes.iter().all(|n| n["node"] != "gone"), "{v:#}");
    let text = stdout(&clm(d, &["view", "spec"]));
    let line = text
        .lines()
        .find(|l| {
            l.trim_start_matches(|c: char| !c.is_alphanumeric())
                .starts_with("a ")
        })
        .unwrap_or_else(|| panic!("no a in {text}"));
    assert!(line.contains("✕1"), "{text}");
}

#[test]
fn spec_chart_shows_the_queue_state_from_the_claim_refs() {
    // req:view/spec-view@380d5f8a
    let dir = two_nodes();
    init(dir.path());
    claim_ref(dir.path(), "a", 1);
    let text = stdout(&clm(dir.path(), &["view", "spec"]));
    // The line whose ID, after the tree prefix, is `id`.
    let line = |id: &str| {
        text.lines()
            .find(|l| {
                l.trim_start_matches(|c: char| !c.is_alphanumeric())
                    .starts_with(&format!("{id} "))
            })
            .unwrap_or_else(|| panic!("no {id} in {text}"))
    };
    assert!(line("a").contains("claimed"), "{text}");
    // `b` depends on `a`, but needs nothing of it, so it is ready (`needs-alone-order`).
    assert!(line("b").contains("ready"), "{text}");
    // The JSON is the model alone, with no state.
    let v = json(&clm(dir.path(), &["view", "spec", "--format", "json"]));
    for n in v["nodes"].as_array().unwrap() {
        assert_eq!(n.get("state"), None, "{n:#}");
    }
    assert!(!v.to_string().contains("claimed"), "{v:#}");
}

#[test]
fn scenario_claimed_node_and_command_matches_library() {
    // req:view/flight-view@d01ad49e
    // req:view/view-library@b5ceab63
    let dir = two_nodes();
    init(dir.path());
    claim_ref(dir.path(), "a", 1);
    let out = clm(dir.path(), &["view", "flight", "--format", "json"]);
    let v = json(&out);
    let a = v["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["node"] == "a")
        .unwrap();
    assert_eq!(a["state"], "claimed");
    assert_eq!(a["claim"]["pushed_hours_ago"], 1);
    assert_eq!(a["claim"]["lapses_in_hours"], 3);
    assert_eq!(a["pending"].as_array().unwrap().len(), 1);
    assert_eq!(a["pending"][0]["state"], "unbuilt");

    let (graph, violations) = clm_model::load(dir.path());
    assert!(violations.is_empty());
    let markers = clm_check::scan(dir.path()).req;
    let refs = clm_orchestrator::claim_refs(dir.path());
    assert_eq!(refs.len(), 1);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let model = clm_view::flight_view(
        &graph,
        &markers,
        &refs,
        now,
        clm_orchestrator::DEFAULT_EXPIRY_HOURS,
    );
    // The same once the command's `checkout` is removed.
    let mut without = v.clone();
    without.as_object_mut().unwrap().remove("checkout");
    assert_eq!(without, serde_json::to_value(&model).unwrap());
    let checkout = clm_view::Head::Checkout(clm_view::Checkout {
        commit: head7(dir.path()),
        behind: None,
    });
    assert_eq!(
        stdout(&out),
        clm_view::render(
            &clm_view::View::Flight(model),
            clm_view::Format::Json,
            100,
            Some(&checkout)
        )
    );
}

/// `HEAD`'s hash cut to its first 7 hex digits.
fn head7(root: &Path) -> String {
    git(root, &["rev-parse", "HEAD"])[..7].to_string()
}

/// `HEAD` `n` commits behind `origin/main`, as a fetch would leave it, with an `origin`
/// that can't be reached.
fn behind_origin(root: &Path, n: usize) {
    let base = git(root, &["rev-parse", "HEAD"]);
    for i in 0..n {
        git(
            root,
            &["commit", "-q", "--allow-empty", "-m", &format!("later {i}")],
        );
    }
    git(root, &["update-ref", "refs/remotes/origin/main", "HEAD"]);
    git(root, &["reset", "-q", "--hard", base.trim()]);
    git(
        root,
        &[
            "remote",
            "add",
            "origin",
            "https://127.0.0.1:9/unreachable.git",
        ],
    );
}

#[test]
fn scenario_checkout_behind_on_the_chart() {
    // req:view/view-command@0fb30947
    let dir = two_nodes();
    let d = dir.path();
    init(d);
    behind_origin(d, 3);
    let out = clm(d, &["view", "flight"]);
    let text = stdout(&out);
    assert_eq!(out.status.code(), Some(0), "{text}");
    let first = text.lines().next().unwrap();
    assert!(first.starts_with("commit "), "{text}");
    assert!(first.ends_with("⚠ 3 behind origin/main"), "{text}");
    let out = clm(d, &["view", "flight", "--format", "svg"]);
    let svg = stdout(&out);
    assert_eq!(out.status.code(), Some(0), "{svg}");
    let doc = roxmltree::Document::parse(&svg).unwrap_or_else(|e| panic!("{e}: {svg}"));
    let texts: String = doc
        .descendants()
        .filter(|n| n.has_tag_name("text"))
        .flat_map(|n| n.descendants().filter_map(|t| t.text()))
        .collect();
    assert!(texts.contains("3 behind origin/main"), "{svg}");
    let out = clm(d, &["view", "flight", "--format", "json"]);
    assert_eq!(out.status.code(), Some(0));
    let v = json(&out);
    assert_eq!(v["checkout"]["behind"], 3, "{v:#}");
    assert_eq!(v["checkout"]["commit"], head7(d), "{v:#}");
    // The spec view's SVG holds it too, above its boxes.
    let svg = stdout(&clm(d, &["view", "spec", "--format", "svg"]));
    roxmltree::Document::parse(&svg).unwrap_or_else(|e| panic!("{e}: {svg}"));
    assert!(svg.contains("3 behind origin/main"), "{svg}");
}

#[test]
fn scenario_checkout_current() {
    // req:view/view-command@0fb30947
    let dir = two_nodes();
    let d = dir.path();
    init(d);
    git(d, &["update-ref", "refs/remotes/origin/main", "HEAD"]);
    let text = stdout(&clm(d, &["view", "spec"]));
    assert_eq!(
        text.lines().next(),
        Some(format!("commit {}", head7(d)).as_str()),
        "{text}"
    );
    let out = clm(d, &["view", "spec", "--format", "json"]);
    assert_eq!(out.status.code(), Some(0));
    let v = json(&out);
    assert_eq!(v["checkout"]["behind"], 0, "{v:#}");
    assert_eq!(v["checkout"]["commit"], head7(d), "{v:#}");
    // Without a repository there is no commit to name, and no `checkout`.
    let bare = two_nodes();
    let text = stdout(&clm(bare.path(), &["view", "spec"]));
    assert!(!text.contains("commit "), "{text}");
    let v = json(&clm(bare.path(), &["view", "spec", "--format", "json"]));
    assert_eq!(v.get("checkout"), None, "{v:#}");
}

#[test]
fn scenario_change_progress() {
    // req:view/flight-view@d01ad49e
    let dir = TempDir::new().unwrap();
    write(dir.path(), "specs/SPEC.md", ROOT);
    write(dir.path(), "specs/n/SPEC.md", &node("", &["x", "y"]));
    write(
        dir.path(),
        "changes/d/change.md",
        "---\nstatus: applied\n---\n",
    );
    write(
        dir.path(),
        "changes/d/deltas/n.md",
        "---\ncomponent: n\n---\n## ADDED Requirements\n### Requirement: x\nIt SHALL.\n\n#### Scenario: s\n- ok\n\n### Requirement: y\nIt SHALL.\n\n#### Scenario: s\n- ok\n",
    );
    let (graph, _) = clm_model::load(dir.path());
    let x = graph.requirement("n/x").unwrap().version();
    write(
        dir.path(),
        "tests/t.rs",
        &format!("// {REQ}n/x@{x}\n// {REQ}n/y@00000000\n"),
    );
    let v = json(&clm(dir.path(), &["view", "flight", "--format", "json"]));
    assert_eq!(v["changes"][0]["status"], "applied");
    let d = &v["changes"][0]["changes"][0];
    assert_eq!(d["change"], "d");
    assert_eq!(d["current"], 1);
    assert_eq!(d["total"], 2);
    assert_eq!(d["complete"], false);
    assert_eq!(v["totals"]["requirements_pending"], 1);
}

#[test]
fn scenario_requirement_removed_later() {
    // req:view/flight-view@d01ad49e
    let dir = TempDir::new().unwrap();
    write(dir.path(), "specs/SPEC.md", ROOT);
    write(dir.path(), "specs/n/SPEC.md", &node("", &["x"]));
    write(
        dir.path(),
        "changes/d/change.md",
        "---\nstatus: applied\n---\n",
    );
    write(
        dir.path(),
        "changes/d/deltas/n.md",
        "---\ncomponent: n\n---\n## ADDED Requirements\n### Requirement: x\nIt SHALL.\n\n#### Scenario: s\n- ok\n\n### Requirement: y\nIt SHALL.\n\n#### Scenario: s\n- ok\n",
    );
    let (graph, _) = clm_model::load(dir.path());
    let x = graph.requirement("n/x").unwrap().version();
    write(dir.path(), "tests/t.rs", &format!("// {REQ}n/x@{x}\n"));
    let v = json(&clm(dir.path(), &["view", "flight", "--format", "json"]));
    let d = &v["changes"][0]["changes"][0];
    assert_eq!(d["change"], "d");
    assert_eq!(d["current"], 1);
    assert_eq!(d["total"], 1);
    assert_eq!(d["obsolete"], serde_json::json!([]));
    assert_eq!(d["complete"], true);
}

#[test]
fn scenario_removed_requirement_still_cited() {
    // req:view/flight-view@d01ad49e
    let dir = TempDir::new().unwrap();
    write(dir.path(), "specs/SPEC.md", ROOT);
    write(dir.path(), "specs/n/SPEC.md", &node("", &["y"]));
    write(
        dir.path(),
        "changes/d/change.md",
        "---\nstatus: applied\n---\n",
    );
    write(
        dir.path(),
        "changes/d/deltas/n.md",
        "---\ncomponent: n\n---\n## REMOVED Requirements\n### Requirement: x\n",
    );
    write(dir.path(), "tests/t.rs", &format!("//\n//\n// {REQ}n/x\n"));
    let v = json(&clm(dir.path(), &["view", "flight", "--format", "json"]));
    let d = &v["changes"][0]["changes"][0];
    assert_eq!(d["change"], "d", "{v:#}");
    assert_eq!(d["obsolete"], serde_json::json!(["tests/t.rs:3"]), "{v:#}");
    assert_eq!(d["complete"], false);
    let status = stdout(&clm(dir.path(), &["status"]));
    let row = status
        .lines()
        .find(|l| l.trim_start().starts_with("d "))
        .unwrap_or_else(|| panic!("no row for d in {status}"));
    assert!(!row.contains("complete"), "{row}");
}

#[test]
fn scenario_order() {
    // req:view/flight-view@d01ad49e
    let dir = TempDir::new().unwrap();
    write(dir.path(), "specs/SPEC.md", ROOT);
    write(dir.path(), "specs/a/SPEC.md", &node("", &["x"]));
    write(dir.path(), "specs/z/SPEC.md", &node("", &["x"]));
    let delta = |n: &str| {
        format!(
            "---\ncomponent: {n}\n---\n## ADDED Requirements\n### Requirement: x\nIt SHALL.\n\n#### Scenario: s\n- ok\n"
        )
    };
    write(
        dir.path(),
        "changes/a1/change.md",
        "---\nstatus: applied\n---\n",
    );
    write(dir.path(), "changes/a1/deltas/a.md", &delta("a"));
    write(
        dir.path(),
        "changes/z9/change.md",
        "---\nstatus: implementing\npriority: P2\n---\n",
    );
    write(dir.path(), "changes/z9/deltas/z.md", &delta("z"));
    let text = stdout(&clm(dir.path(), &["view", "flight"]));
    let pos = |s: &str| text.find(s).unwrap_or_else(|| panic!("no {s} in {text}"));
    assert!(pos("\n  z ") < pos("\n  a "), "{text}");
    assert!(pos("z9") < pos("a1"), "{text}");
}

#[test]
fn scenario_chain_weight() {
    // req:view/blockers-view@3950c690
    let dir = TempDir::new().unwrap();
    write(dir.path(), "specs/SPEC.md", ROOT);
    write(dir.path(), "specs/a/SPEC.md", &node("", &["x"]));
    write(dir.path(), "specs/b/SPEC.md", &needs(&[("y", &["a/x"])]));
    write(dir.path(), "specs/c/SPEC.md", &needs(&[("x", &["b/y"])]));
    init(dir.path());
    claim_ref(dir.path(), "a", 1);
    let v = json(&clm(dir.path(), &["view", "blockers", "--format", "json"]));
    let roots = v["roots"].as_array().unwrap();
    assert_eq!(roots.len(), 1, "{v:#}");
    assert_eq!(roots[0]["root"], "a");
    assert_eq!(roots[0]["weight"], 2);
    assert_eq!(roots[0]["holds"][0]["node"], "b");
    assert_eq!(roots[0]["holds"][1]["node"], "c");
    // `b/y` is blocked, so `c` waits on its blocker `a/x` directly.
    assert_eq!(roots[0]["holds"][1]["via"], serde_json::json!([]));
}

#[test]
fn scenario_human_first() {
    // req:view/blockers-view@3950c690
    let dir = TempDir::new().unwrap();
    write(dir.path(), "specs/SPEC.md", ROOT);
    write(dir.path(), "specs/a/SPEC.md", &node("", &["x"]));
    write(dir.path(), "specs/d/SPEC.md", &node("", &["x"]));
    for n in ["e", "f", "g", "h"] {
        write(
            dir.path(),
            &format!("specs/{n}/SPEC.md"),
            &needs(&[("x", &["d/x"])]),
        );
    }
    write(
        dir.path(),
        "backlog/q.md",
        "---\ntitle: Q\nkind: question\nblocks: [a/x]\n---\n",
    );
    init(dir.path());
    claim_ref(dir.path(), "d", 1);
    let text = stdout(&clm(dir.path(), &["view", "blockers"]));
    let pos = |s: &str| text.find(s).unwrap_or_else(|| panic!("no {s} in {text}"));
    assert!(pos("backlog/q") < pos("\nd "), "{text}");
    assert!(text.contains("holds up 4"), "{text}");
    assert!(text.contains("waits on you"), "{text}");
    let v = json(&clm(dir.path(), &["view", "blockers", "--format", "json"]));
    assert_eq!(v["roots"][0]["root"], "backlog/q", "{v:#}");
    assert_eq!(v["roots"][0]["weight"], 1, "{v:#}");
    assert_eq!(v["roots"][1]["root"], "d", "{v:#}");
    assert_eq!(v["roots"][1]["weight"], 4, "{v:#}");
}

#[test]
fn scenario_dependency_holds_nothing() {
    // `b` depends on `a`, but its own requirement is unblocked, so it is ready and held by
    // nothing.
    // req:view/blockers-view@3950c690
    let dir = TempDir::new().unwrap();
    write(dir.path(), "specs/SPEC.md", ROOT);
    write(dir.path(), "specs/a/SPEC.md", &node("", &["x"]));
    write(dir.path(), "specs/b/SPEC.md", &node("a", &["x"]));
    write(
        dir.path(),
        "backlog/q.md",
        "---\ntitle: Q\nkind: question\nblocks: [a/x]\n---\n",
    );
    let v = json(&clm(dir.path(), &["view", "blockers", "--format", "json"]));
    let held: Vec<&str> = v["roots"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|r| r["holds"].as_array().unwrap())
        .map(|h| h["node"].as_str().unwrap())
        .collect();
    assert_eq!(held, ["a"], "{v:#}");
    let flight = json(&clm(dir.path(), &["view", "flight", "--format", "json"]));
    let b = flight["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["node"] == "b")
        .unwrap();
    assert_eq!(b["state"], "ready", "{flight:#}");
}

#[test]
fn scenario_need_reaches_the_needed_requirements_item() {
    // req:view/blockers-view@3950c690
    let dir = TempDir::new().unwrap();
    write(dir.path(), "specs/SPEC.md", ROOT);
    write(
        dir.path(),
        "specs/a/SPEC.md",
        &node("", &[]).replace(
            "## Requirements\n",
            "## Requirements\n\n### Requirement: x\nIt SHALL.\n\n#### Needs\n- b/y\n\n\
             #### Scenario: s\n- ok\n",
        ),
    );
    write(dir.path(), "specs/b/SPEC.md", &node("", &["y", "z"]));
    write(
        dir.path(),
        "backlog/q.md",
        "---\ntitle: Q\nkind: question\nblocks: [b/y]\n---\n",
    );
    let v = json(&clm(dir.path(), &["view", "blockers", "--format", "json"]));
    let roots = v["roots"].as_array().unwrap();
    assert_eq!(roots.len(), 1, "{v:#}");
    assert_eq!(roots[0]["root"], "backlog/q");
    assert_eq!(roots[0]["weight"], 1);
    assert_eq!(roots[0]["holds"][0]["node"], "a");
}

#[test]
fn scenario_nothing_blocked() {
    // req:view/blockers-root-lines@8d9bf009
    let dir = two_nodes();
    // `b` depends on `a`: with `a` converged, `b` is ready.
    let (graph, _) = clm_model::load(dir.path());
    let x = graph.requirement("a/x").unwrap().version();
    write(dir.path(), "tests/t.rs", &format!("// {REQ}a/x@{x}\n"));
    let out = clm(dir.path(), &["view", "blockers"]);
    let text = stdout(&out);
    assert_eq!(out.status.code(), Some(0), "{text}");
    let lines: Vec<&str> = text.lines().collect();
    let at = lines.iter().position(|l| *l == "blockers").expect(&text);
    assert_eq!(&lines[at + 1..], ["nothing is blocked"], "{text}");
}
