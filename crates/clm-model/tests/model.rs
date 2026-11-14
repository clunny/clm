use std::fs;
use std::path::Path;

use clm_model::{Priority, Violation, content_hash, load};
use tempfile::TempDir;

fn spec(fm: &str, body: &str) -> String {
    format!("---\n{fm}\n---\n\n{body}")
}

fn node(deps: &str) -> String {
    spec(
        &format!("title: T\nstatus: specd\nowners: [\"@a\"]\ndepends_on: [{deps}]"),
        "## Contract\nc\n\n## Design\nd\n\n## Requirements\n",
    )
}

fn repo(files: &[(&str, String)]) -> TempDir {
    let dir = TempDir::new().unwrap();
    for (rel, content) in files {
        let path = dir.path().join("specs").join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }
    dir
}

fn rules(violations: &[Violation]) -> Vec<&str> {
    violations.iter().map(|v| v.rule).collect()
}

fn assert_clean(root: &Path) -> clm_model::Graph {
    let (graph, violations) = load(root);
    assert!(
        violations.is_empty(),
        "unexpected violations: {violations:?}"
    );
    graph
}

#[test]
fn nested_node_gets_dotted_id_and_parent() {
    // req:model/parse-node-specs@f87bf5ee
    let dir = repo(&[
        ("SPEC.md", node("")),
        ("check/SPEC.md", node("")),
        ("check/drift/SPEC.md", node("")),
    ]);
    let graph = assert_clean(dir.path());
    let drift = &graph.nodes["check.drift"];
    assert_eq!(drift.parent.as_deref(), Some("check"));
    assert_eq!(graph.nodes["check"].parent.as_deref(), Some("clm"));
    assert_eq!(graph.nodes["clm"].parent, None);
}

#[test]
fn node_without_parent_spec_is_rejected() {
    // req:model/parse-node-specs@f87bf5ee
    let dir = repo(&[("SPEC.md", node("")), ("a/b/SPEC.md", node(""))]);
    let (_, violations) = load(dir.path());
    assert_eq!(rules(&violations), ["model/parse-node-specs"]);
    assert!(violations[0].message.contains("`a`"));
}

#[test]
fn missing_root_spec_is_rejected() {
    // req:model/parse-node-specs@f87bf5ee
    let dir = repo(&[("a/SPEC.md", node(""))]);
    let (_, violations) = load(dir.path());
    assert!(rules(&violations).contains(&"model/parse-node-specs"));
}

#[test]
fn bad_status_names_file_and_field() {
    // req:model/valid-frontmatter@b4618360
    let dir = repo(&[(
        "SPEC.md",
        spec(
            "title: T\nstatus: done\nowners: [\"@a\"]",
            "## Requirements\n",
        ),
    )]);
    let (_, violations) = load(dir.path());
    assert_eq!(rules(&violations), ["model/valid-frontmatter"]);
    assert_eq!(violations[0].path, Path::new("specs/SPEC.md"));
    assert!(violations[0].message.contains("status"), "{violations:?}");
}

#[test]
fn missing_required_field_is_rejected() {
    // req:model/valid-frontmatter@b4618360
    let dir = repo(&[("SPEC.md", spec("title: T\nstatus: specd", ""))]);
    let (_, violations) = load(dir.path());
    assert_eq!(rules(&violations), ["model/valid-frontmatter"]);
    assert!(violations[0].message.contains("owners"), "{violations:?}");
}

fn budgeted(budget: &str) -> TempDir {
    repo(&[(
        "SPEC.md",
        spec(
            &format!("title: T\nstatus: specd\nowners: [\"@a\"]{budget}"),
            "",
        ),
    )])
}

#[test]
fn size_budget_is_parsed() {
    // req:model/size-budget@b24de718
    for (budget, spec_tokens, code_tokens) in [
        ("\nsize_budget: { spec_tokens: 30000 }", Some(30000), None),
        ("", None, None),
        ("\nsize_budget: {}", None, None),
        (
            "\nsize_budget: { spec_tokens: 1, code_tokens: 5000000000 }",
            Some(1),
            Some(5_000_000_000),
        ),
    ] {
        let graph = assert_clean(budgeted(budget).path());
        let size_budget = &graph.nodes["clm"].frontmatter.size_budget;
        assert_eq!(size_budget.spec_tokens, spec_tokens, "{budget}");
        assert_eq!(size_budget.code_tokens, code_tokens, "{budget}");
    }
}

#[test]
fn bad_size_budget_is_rejected() {
    // req:model/size-budget@b24de718
    for value in [
        "{ spec_tokens: lots }",
        "{ lines: 500 }",
        "{ spec_tokens: 0 }",
        "{ code_tokens: -1 }",
        "{ spec_tokens: 1.5 }",
        "{ spec_tokens: \"30000\" }",
        "{ spec_tokens: null }",
        "{ spec_tokens: 1, code_tokens: 0 }",
        "30000",
        "[1]",
        "null",
    ] {
        let dir = budgeted(&format!("\nsize_budget: {value}"));
        let (graph, violations) = load(dir.path());
        assert_eq!(rules(&violations), ["model/size-budget"], "{value}");
        assert!(graph.nodes.is_empty(), "{value}");
        assert_eq!(violations[0].path, Path::new("specs/SPEC.md"));
        let message = &violations[0].message;
        assert!(message.contains("size_budget"), "{message}");
        assert!(!message.contains('\n'), "{message}");
    }
    // The message names the bad entry, not the whole mapping.
    let dir = budgeted("\nsize_budget: { spec_tokens: 30000, code_token: 9000 }");
    let message = &load(dir.path()).1[0].message;
    assert!(message.contains("code_token: 9000"), "{message}");
    assert!(!message.contains("30000"), "{message}");
}

fn flagged(deps: &str, flag: &str) -> String {
    spec(
        &format!("title: T\nstatus: specd\nowners: [\"@a\"]\ndepends_on: [{deps}]{flag}"),
        "",
    )
}

const OPTIONAL: &str = "\noptional: true";

#[test]
fn bad_optional_value() {
    // req:model/optional-nodes@37ade64d
    for value in ["maybe", "\"true\"", "1"] {
        let dir = repo(&[
            ("SPEC.md", node("")),
            ("a/SPEC.md", flagged("", &format!("\noptional: {value}"))),
        ]);
        let (_, violations) = load(dir.path());
        assert_eq!(rules(&violations), ["model/optional-nodes"], "{value}");
        assert_eq!(violations[0].path, Path::new("specs/a/SPEC.md"));
        assert!(violations[0].message.contains("optional"), "{violations:?}");
    }
}

#[test]
fn optional_root_is_rejected() {
    // req:model/optional-nodes@37ade64d
    let dir = repo(&[("SPEC.md", flagged("", OPTIONAL))]);
    let (_, violations) = load(dir.path());
    assert_eq!(rules(&violations), ["model/optional-nodes"]);
    assert_eq!(violations[0].path, Path::new("specs/SPEC.md"));
    let message = &violations[0].message;
    assert!(
        message.contains("root") && message.contains("optional"),
        "{message}"
    );
    // The rejected root flag doesn't make the tree optional and hide other violations.
    let dir = repo(&[
        ("SPEC.md", flagged("", OPTIONAL)),
        ("a/SPEC.md", flagged("", OPTIONAL)),
        ("b/SPEC.md", node("a")),
    ]);
    let (graph, violations) = load(dir.path());
    assert_eq!(
        rules(&violations),
        ["model/optional-nodes", "model/optional-nodes"]
    );
    assert!(!graph.is_optional("clm"));
    assert!(!graph.is_optional("b"));
    let dir = repo(&[("SPEC.md", flagged("", "\noptional: false"))]);
    assert!(!assert_clean(dir.path()).is_optional("clm"));
}

#[test]
fn optional_is_inherited() {
    // req:model/optional-nodes@37ade64d
    let dir = repo(&[
        ("SPEC.md", node("")),
        ("a/SPEC.md", flagged("", OPTIONAL)),
        ("a/x/SPEC.md", node("")),
        ("b/SPEC.md", node("")),
    ]);
    let graph = assert_clean(dir.path());
    assert!(graph.is_optional("a"));
    assert!(graph.is_optional("a.x"));
    assert!(!graph.is_optional("b"));
    assert!(!graph.is_optional("clm"));
}

#[test]
fn required_depends_on_optional_is_rejected() {
    // req:model/optional-nodes@37ade64d
    for (target, files) in [
        ("a", vec![("a/SPEC.md", flagged("", OPTIONAL))]),
        (
            "a.x",
            vec![
                ("a/SPEC.md", flagged("", OPTIONAL)),
                ("a/x/SPEC.md", node("")),
            ],
        ),
    ] {
        let mut files: Vec<(&str, String)> = files;
        files.push(("SPEC.md", node("")));
        files.push(("b/SPEC.md", node(target)));
        let (_, violations) = load(repo(&files).path());
        assert_eq!(rules(&violations), ["model/optional-nodes"], "{target}");
        assert_eq!(violations[0].path, Path::new("specs/b/SPEC.md"));
        let message = &violations[0].message;
        assert!(message.contains("`b`"), "{message}");
        assert!(message.contains(&format!("`{target}`")), "{message}");
    }
}

#[test]
fn optional_may_depend_on_any_node() {
    // req:model/optional-nodes@37ade64d
    let dir = repo(&[
        ("SPEC.md", node("")),
        ("a/SPEC.md", flagged("b, c", OPTIONAL)),
        ("b/SPEC.md", node("")),
        ("c/SPEC.md", flagged("", OPTIONAL)),
    ]);
    let graph = assert_clean(dir.path());
    assert!(graph.is_optional("a"));
    assert!(!graph.is_optional("b"));
}

#[test]
fn dependency_cycle_is_named() {
    // req:model/dependency-dag@12284c3c
    let dir = repo(&[
        ("SPEC.md", node("")),
        ("a/SPEC.md", node("b")),
        ("b/SPEC.md", node("a")),
    ]);
    let (_, violations) = load(dir.path());
    assert_eq!(rules(&violations), ["model/dependency-dag"]);
    assert!(
        violations[0].message.contains("a -> b -> a"),
        "{violations:?}"
    );
}

#[test]
fn unknown_dependency_is_rejected() {
    // req:model/dependency-dag@12284c3c
    let dir = repo(&[("SPEC.md", node("ghost"))]);
    let (_, violations) = load(dir.path());
    assert_eq!(rules(&violations), ["model/dependency-dag"]);
    assert!(violations[0].message.contains("ghost"));
}

#[test]
fn duplicate_requirement_slug_is_rejected() {
    // req:model/unique-requirement-ids@c65dde82
    let body = "## Requirements\n\n### Requirement: foo\nx\n#### Scenario: s\n- y\n\n### Requirement: foo\nx\n#### Scenario: s\n- y\n";
    let dir = repo(&[("SPEC.md", spec("title: T\nstatus: specd\nowners: []", body))]);
    let (_, violations) = load(dir.path());
    assert_eq!(rules(&violations), ["model/unique-requirement-ids"]);
    assert!(violations[0].message.contains("clm/foo"));
}

#[test]
fn requirement_without_scenario_is_rejected() {
    // req:model/unique-requirement-ids@c65dde82
    let body = "## Requirements\n\n### Requirement: foo\nThe thing SHALL work.\n";
    let dir = repo(&[("SPEC.md", spec("title: T\nstatus: specd\nowners: []", body))]);
    let (_, violations) = load(dir.path());
    assert_eq!(rules(&violations), ["model/unique-requirement-ids"]);
}

#[test]
fn requirements_and_sections_are_parsed_ignoring_code_fences() {
    // req:model/parse-node-specs@f87bf5ee
    let body = "## Contract\nThe contract.\n\n```markdown\n## Design\n### Requirement: fake\n```\n\n## Design\nThe design.\n\n## Requirements\n\n### Requirement: real\nIt SHALL.\n\n#### Scenario: s\n- ok\n";
    let dir = repo(&[("SPEC.md", spec("title: T\nstatus: specd\nowners: []", body))]);
    let graph = assert_clean(dir.path());
    let root = &graph.nodes["clm"];
    assert!(root.contract.contains("### Requirement: fake"));
    assert_eq!(root.design.trim(), "The design.");
    let ids: Vec<_> = root.requirements.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(ids, ["clm/real"]);
}

#[test]
fn hash_ignores_line_endings_and_trailing_whitespace() {
    // req:model/content-hash@92d37469
    let lf = "### Requirement: x\nIt SHALL.\n";
    assert_eq!(
        content_hash(lf),
        content_hash("### Requirement: x\r\nIt SHALL.\r\n")
    );
    assert_eq!(
        content_hash(lf),
        content_hash("### Requirement: x   \nIt SHALL.\t\n\n\n")
    );
}

#[test]
fn hash_changes_with_content() {
    // req:model/content-hash@92d37469
    let a = "### Requirement: x\nIt SHALL.\n";
    assert_ne!(
        content_hash(a),
        content_hash("### Requirement: x\nIt SHALL NOT.\n")
    );
    assert_ne!(
        content_hash(a),
        content_hash("### Requirement: x\n It SHALL.\n")
    );
}

#[test]
fn every_element_is_hashed() {
    // req:model/content-hash@92d37469
    let body = "## Contract\nc\n\n## Design\nd\n\n## Requirements\n\n### Requirement: r\nx\n#### Scenario: s\n- y\n";
    let dir = repo(&[("SPEC.md", spec("title: T\nstatus: specd\nowners: []", body))]);
    let graph = assert_clean(dir.path());
    let root = &graph.nodes["clm"];
    let hashes = [
        &root.hashes.frontmatter,
        &root.hashes.contract,
        &root.hashes.design,
        &root.requirements[0].hash,
    ];
    for h in hashes {
        assert_eq!(h.len(), 16);
    }
    assert_ne!(root.hashes.contract, root.hashes.design);
}

#[test]
fn version_is_the_first_8_hex_digits_of_the_hash() {
    // req:model/content-hash@92d37469
    let r = clm_model::Requirement {
        id: "a/x".into(),
        text: String::new(),
        scenarios: 1,
        hash: "0123456789abcdef".into(),
        needs: Vec::new(),
        serves: Vec::new(),
        enforced_by: Vec::new(),
        untested: false,
        untested_text: None,
    };
    assert_eq!(r.version(), "01234567");
    let body = "## Requirements\n\n### Requirement: r\nx\n#### Scenario: s\n- y\n";
    let dir = repo(&[("SPEC.md", spec("title: T\nstatus: specd\nowners: []", body))]);
    let graph = assert_clean(dir.path());
    let r = &graph.nodes["clm"].requirements[0];
    assert_eq!(r.version(), &r.hash[..8]);
}

fn with_change(change_md: &str, deltas: &[(&str, &str)]) -> TempDir {
    let dir = repo(&[("SPEC.md", node(""))]);
    let c = dir.path().join("changes/c");
    fs::create_dir_all(c.join("deltas")).unwrap();
    fs::write(c.join("change.md"), change_md).unwrap();
    for (name, text) in deltas {
        fs::write(c.join("deltas").join(name), text).unwrap();
    }
    dir
}

const CHANGE: &str = "---\nstatus: proposed\nauthor: \"@a\"\n---\n\n# Intent\n";

#[test]
fn change_and_deltas_are_parsed() {
    // req:model/parse-changes@d4cfec66
    let delta = "---\ncomponent: clm\nbase: { contract: \"abc\" }\n---\n## MODIFIED Contract\nnew\n\n## ADDED Requirements\n### Requirement: r\nx\n#### Scenario: s\n- y\n\n## REMOVED Requirements\n### Requirement: old <!-- base: def -->\n";
    let dir = with_change(CHANGE, &[("clm.md", delta)]);
    let graph = assert_clean(dir.path());
    let change = &graph.changes["c"];
    assert!(change.frontmatter.status.is_pending());
    let d = &change.deltas[0];
    assert_eq!(d.node, "clm");
    assert_eq!(d.base.contract.as_deref(), Some("abc"));
    assert_eq!(d.modified_contract.as_deref().map(str::trim), Some("new"));
    assert_eq!(d.added[0].slug, "r");
    assert_eq!(d.removed[0].slug, "old");
    assert_eq!(d.removed[0].base.as_deref(), Some("def"));
}

#[test]
fn unknown_delta_section_is_rejected() {
    // req:model/parse-changes@d4cfec66
    let delta = "---\ncomponent: clm\n---\n## RENAMED Requirements\nx\n";
    let dir = with_change(CHANGE, &[("clm.md", delta)]);
    let (_, violations) = load(dir.path());
    assert_eq!(rules(&violations), ["model/parse-changes"]);
    assert_eq!(violations[0].path, Path::new("changes/c/deltas/clm.md"));
    assert!(violations[0].message.contains("RENAMED Requirements"));
}

#[test]
fn bad_change_status_and_missing_node_are_rejected() {
    // req:model/parse-changes@d4cfec66
    let dir = with_change(
        &CHANGE.replace("proposed", "merged"),
        &[("x.md", "---\ncreate: true\n---\n")],
    );
    let (_, violations) = load(dir.path());
    assert_eq!(rules(&violations), ["model/parse-changes"]);
    let dir = with_change(CHANGE, &[("x.md", "---\ncreate: true\n---\n")]);
    let (_, violations) = load(dir.path());
    assert_eq!(rules(&violations), ["model/parse-changes"]);
    assert!(
        violations[0].message.contains("component"),
        "{violations:?}"
    );
}

#[test]
fn live_change_may_resolve_deleted_item() {
    // req:model/parse-changes@d4cfec66
    let change = CHANGE.replace("author", "resolves: [gone]\nauthor");
    let dir = with_change(&change, &[]);
    let graph = assert_clean(dir.path());
    assert!(!graph.changes["c"].archived);
    assert_eq!(graph.changes["c"].frontmatter.resolves, ["gone"]);
}

#[test]
fn change_priority() {
    // req:model/parse-changes@d4cfec66
    let dir = repo(&[("SPEC.md", node(""))]);
    for (id, priority) in [("x", "priority: P2\n"), ("y", ""), ("z", "priority: P0\n")] {
        let c = dir.path().join("changes").join(id);
        fs::create_dir_all(&c).unwrap();
        fs::write(
            c.join("change.md"),
            format!("---\nstatus: proposed\n{priority}---\n"),
        )
        .unwrap();
    }
    let (graph, violations) = load(dir.path());
    assert_eq!(graph.changes["x"].frontmatter.priority, Priority::P2);
    assert_eq!(graph.changes["y"].frontmatter.priority, Priority::P5);
    assert!(!graph.changes.contains_key("z"));
    assert_eq!(rules(&violations), ["model/parse-changes"]);
    assert_eq!(violations[0].path, Path::new("changes/z/change.md"));
    assert!(violations[0].message.contains("P0"), "{violations:?}");
}

#[test]
fn priority_order_display_and_caps() {
    // req:model/parse-changes@d4cfec66
    let all = [
        Priority::P1,
        Priority::P2,
        Priority::P3,
        Priority::P4,
        Priority::P5,
    ];
    assert!(all.is_sorted_by(|a, b| a < b));
    assert_eq!(Priority::default(), Priority::P5);
    assert_eq!(all.map(|p| p.to_string()), ["P1", "P2", "P3", "P4", "P5"]);
    assert_eq!(
        all.map(Priority::cap),
        [Some(1), Some(2), Some(3), Some(4), None]
    );
}

#[test]
fn archived_changes_may_resolve_deleted_items() {
    // req:model/parse-changes@d4cfec66
    let dir = repo(&[("SPEC.md", node(""))]);
    let c = dir.path().join("changes/archive/c");
    fs::create_dir_all(&c).unwrap();
    fs::write(
        c.join("change.md"),
        "---\nstatus: done\nresolves: [gone]\n---\n",
    )
    .unwrap();
    let graph = assert_clean(dir.path());
    assert_eq!(graph.changes["c"].frontmatter.resolves, ["gone"]);
}

const ASSET_DELTA: &str = "---\ncomponent: clm\nbase: { assets: { \"m.html\": \"abc\" } }\n---\n## REMOVED Assets\n- `old.png` <!-- base: 0123456789abcdef -->\n";

#[test]
fn change_assets() {
    // req:model/parse-changes@d4cfec66
    let dir = with_change(CHANGE, &[("clm.md", ASSET_DELTA)]);
    write(dir.path(), "changes/c/assets/clm/m.html", "<p>new</p>");
    write(dir.path(), "changes/c/assets/clm/sub/n.svg", "<svg/>");
    let graph = assert_clean(dir.path());
    let change = &graph.changes["c"];
    let assets = &change.assets["clm"];
    assert_eq!(assets["m.html"], b"<p>new</p>");
    assert_eq!(assets["sub/n.svg"], b"<svg/>");
    let d = &change.deltas[0];
    assert_eq!(d.base.assets["m.html"], "abc");
    assert_eq!(
        d.removed_assets,
        [("old.png".to_string(), "0123456789abcdef".to_string())]
    );
}

#[test]
fn removed_asset_without_a_base() {
    // req:model/parse-changes@d4cfec66
    let delta = "---\ncomponent: clm\n---\n## REMOVED Assets\n- `old.png`\n";
    let dir = with_change(CHANGE, &[("clm.md", delta)]);
    let (_, violations) = load(dir.path());
    assert_eq!(rules(&violations), ["model/parse-changes"]);
    assert_eq!(violations[0].path, Path::new("changes/c/deltas/clm.md"));
    assert!(
        violations[0].message.contains("`old.png`"),
        "{violations:?}"
    );
}

#[test]
fn removed_asset_outside_the_assets_dir() {
    // req:model/parse-changes@d4cfec66
    for path in ["../SPEC.md", "/etc/x", "", "a/./b", "a//b"] {
        let delta = format!(
            "---\ncomponent: clm\n---\n## REMOVED Assets\n- `{path}` <!-- base: 0123456789abcdef -->\n"
        );
        let dir = with_change(CHANGE, &[("clm.md", &delta)]);
        let (_, violations) = load(dir.path());
        assert_eq!(rules(&violations), ["model/parse-changes"], "{path}");
    }
}

#[test]
fn change_asset_without_a_delta() {
    // req:model/parse-changes@d4cfec66
    let dir = with_change(CHANGE, &[]);
    write(dir.path(), "changes/c/assets/zzz/m.html", "x");
    write(dir.path(), "changes/c/assets/loose.png", "x");
    let (_, violations) = load(dir.path());
    assert_eq!(
        rules(&violations),
        ["model/parse-changes", "model/parse-changes"]
    );
    assert_eq!(violations[0].path, Path::new("changes/c/assets/loose.png"));
    assert!(violations[1].message.contains("`zzz`"), "{violations:?}");
}

const REMOVAL: &str = "---\ncomponent: clm\nremove: true\nbase: { contract: \"abc\" }\n---\n";

#[test]
fn removal_delta_is_parsed() {
    // req:model/parse-changes@d4cfec66
    let delta = format!(
        "{REMOVAL}## REMOVED Requirements\n### Requirement: old <!-- base: def -->\n\n## REMOVED Assets\n- `old.png` <!-- base: 0123456789abcdef -->\n"
    );
    let dir = with_change(CHANGE, &[("clm.md", &delta)]);
    let graph = assert_clean(dir.path());
    let d = &graph.changes["c"].deltas[0];
    assert!(d.remove && !d.create);
    assert_eq!(d.removed[0].slug, "old");
    assert_eq!(d.removed_assets[0].0, "old.png");

    let dir = with_change(CHANGE, &[("clm.md", REMOVAL)]);
    let graph = assert_clean(dir.path());
    assert!(graph.changes["c"].deltas[0].remove);
}

#[test]
fn create_and_remove_is_rejected() {
    // req:model/parse-changes@d4cfec66
    let delta = "---\ncomponent: clm\ncreate: true\nremove: true\n---\n";
    let dir = with_change(CHANGE, &[("clm.md", delta)]);
    let (graph, violations) = load(dir.path());
    assert_eq!(rules(&violations), ["model/parse-changes"]);
    assert_eq!(violations[0].path, Path::new("changes/c/deltas/clm.md"));
    let m = &violations[0].message;
    assert!(m.contains("`create`") && m.contains("`remove`"), "{m}");
    assert!(graph.changes["c"].deltas.is_empty());
}

#[test]
fn removal_with_a_modified_section() {
    // req:model/parse-changes@d4cfec66
    for section in ["MODIFIED Design", "ADDED Requirements", "Contract"] {
        let delta = format!("{REMOVAL}## {section}\nx\n");
        let dir = with_change(CHANGE, &[("clm.md", &delta)]);
        let (graph, violations) = load(dir.path());
        assert_eq!(rules(&violations), ["model/parse-changes"], "{section}");
        assert_eq!(violations[0].path, Path::new("changes/c/deltas/clm.md"));
        assert!(
            violations[0].message.contains(&format!("`## {section}`")),
            "{violations:?}"
        );
        assert!(graph.changes["c"].deltas.is_empty());
    }
}

#[test]
fn removal_with_change_assets() {
    // req:model/parse-changes@d4cfec66
    let dir = with_change(CHANGE, &[("clm.md", REMOVAL)]);
    write(dir.path(), "changes/c/assets/clm/m.html", "<p>x</p>");
    let (_, violations) = load(dir.path());
    assert_eq!(rules(&violations), ["model/parse-changes"]);
    assert_eq!(violations[0].path, Path::new("changes/c/assets/clm"));
    assert!(violations[0].message.contains("`clm`"), "{violations:?}");
    assert!(violations[0].message.contains("remov"), "{violations:?}");

    // Whatever order the deltas naming the node sort in.
    let modify = "---\ncomponent: clm\n---\n";
    let dir = with_change(CHANGE, &[("a.md", modify), ("b.md", REMOVAL)]);
    write(dir.path(), "changes/c/assets/clm/m.html", "<p>x</p>");
    let (_, violations) = load(dir.path());
    assert_eq!(rules(&violations), ["model/parse-changes"]);
    assert!(violations[0].message.contains("remov"), "{violations:?}");
}

#[test]
fn removal_with_an_unknown_section() {
    // req:model/parse-changes@d4cfec66
    let delta = format!("{REMOVAL}## REMOVED Requirement\nx\n");
    let dir = with_change(CHANGE, &[("clm.md", &delta)]);
    let (_, violations) = load(dir.path());
    assert_eq!(rules(&violations), ["model/parse-changes"]);
    let m = &violations[0].message;
    assert!(
        m.contains("unknown delta section `## REMOVED Requirement`"),
        "{m}"
    );
}

fn write(root: &Path, rel: &str, text: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

#[test]
fn order_keys_are_accepted_and_ignored() {
    // req:model/parse-changes@d4cfec66
    let dir = repo(&[("SPEC.md", node(""))]);
    write(
        dir.path(),
        "changes/x/change.md",
        "---\nstatus: proposed\nafter: [x]\nprefer_after: [y]\n---\n",
    );
    write(
        dir.path(),
        "changes/archive/w/change.md",
        "---\nstatus: done\nafter: [nope]\n---\n",
    );
    let graph = assert_clean(dir.path());
    assert!(graph.changes.contains_key("x"));
    assert!(graph.changes["w"].archived);
}

#[test]
fn archived_changes_may_name_unknown_changes() {
    // req:model/parse-changes@d4cfec66
    let dir = repo(&[("SPEC.md", node(""))]);
    write(
        dir.path(),
        "changes/archive/x/change.md",
        "---\nstatus: done\nafter: [gone]\nprefer_after: [x]\n---\n",
    );
    assert_clean(dir.path());
}

#[test]
fn live_change_may_hold_a_plan() {
    // req:model/parse-changes@d4cfec66
    let dir = repo(&[("SPEC.md", node(""))]);
    write(
        dir.path(),
        "changes/x/change.md",
        "---\nstatus: applied\n---\n",
    );
    write(dir.path(), "changes/x/plan.md", "# Plan\n");
    write(dir.path(), "changes/x/tasks/a.md", "notes\n");
    // Archived changes keep theirs, in whatever format they were written in.
    write(
        dir.path(),
        "changes/archive/w/change.md",
        "---\nstatus: done\n---\n",
    );
    write(
        dir.path(),
        "changes/archive/w/plan.md",
        "not even frontmatter\n",
    );

    let graph = assert_clean(dir.path());
    assert!(graph.changes.contains_key("x"));
    assert!(graph.changes["w"].archived);
}

fn change_at(root: &Path, dir: &str, fm: &str) {
    let c = root.join("changes").join(dir);
    fs::create_dir_all(&c).unwrap();
    fs::write(c.join("change.md"), format!("---\n{fm}---\n")).unwrap();
}

#[test]
fn burdens_parsed() {
    // req:model/parse-changes@d4cfec66
    let dir = repo(&[("SPEC.md", node(""))]);
    change_at(
        dir.path(),
        "x",
        "status: proposed\nburdens: [a paid model run on every land]\n",
    );
    let graph = assert_clean(dir.path());
    assert_eq!(
        graph.changes["x"].frontmatter.burdens,
        ["a paid model run on every land"]
    );
}

#[test]
fn burdens_default_empty() {
    // req:model/parse-changes@d4cfec66
    let dir = repo(&[("SPEC.md", node(""))]);
    change_at(dir.path(), "y", "status: proposed\n");
    let graph = assert_clean(dir.path());
    assert!(graph.changes["y"].frontmatter.burdens.is_empty());
}

#[test]
fn live_gate_rejected() {
    // req:model/parse-changes@d4cfec66
    let dir = repo(&[("SPEC.md", node(""))]);
    change_at(dir.path(), "x", "gate: HARD\nstatus: proposed\n");
    let (_, violations) = load(dir.path());
    assert_eq!(rules(&violations), ["model/parse-changes"]);
    assert_eq!(violations[0].path, Path::new("changes/x/change.md"));
    assert!(violations[0].message.contains("gate"), "{violations:?}");
}

#[test]
fn archived_gate_ignored() {
    // req:model/parse-changes@d4cfec66
    let dir = repo(&[("SPEC.md", node(""))]);
    change_at(dir.path(), "archive/w", "gate: HARD\nstatus: done\n");
    let graph = assert_clean(dir.path());
    assert!(graph.changes["w"].archived);
}

#[test]
fn legacy_significant_is_ignored() {
    // req:model/valid-frontmatter@b4618360
    for value in ["true", "false", "maybe"] {
        let dir = repo(&[(
            "SPEC.md",
            spec(
                &format!("title: T\nstatus: specd\nowners: [\"@a\"]\nsignificant: {value}"),
                "",
            ),
        )]);
        assert_clean(dir.path());
    }
}
