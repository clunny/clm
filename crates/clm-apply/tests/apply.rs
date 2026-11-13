use std::fs;
use std::path::Path;

use clm_apply::{FileOp, apply, stale_bases, validate};
use clm_model::{Violation, load};
use tempfile::TempDir;

const ROOT: &str = "---\ntitle: Root\nstatus: active\nowners: []\n---\n\n## Contract\nroot\n\n## Design\nroot design\n\n## Requirements\n";

fn node_a(reqs: &[&str]) -> String {
    let blocks: String = reqs
        .iter()
        .map(|s| {
            format!("### Requirement: {s}\nThe {s} SHALL hold.\n\n#### Scenario: {s}\n- ok\n\n")
        })
        .collect();
    format!(
        "---\ntitle: A\nstatus: specd  # frontier\nowners: [\"@a\"]\ndepends_on: []\n---\n\n## Contract\nA contract.\n\n## Design\nA design\nwith two lines.\n\n## Requirements\n\n{blocks}"
    )
}

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn read(root: &Path, rel: &str) -> String {
    fs::read_to_string(root.join(rel)).unwrap()
}

/// A repo with root and node `a` (requirements x, y, z) plus a proposed change `c` with `deltas`.
fn repo(deltas: &[(&str, String)]) -> TempDir {
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    write(d, "specs/a/SPEC.md", &node_a(&["x", "y", "z"]));
    write(
        d,
        "changes/c/change.md",
        "---\nstatus: proposed\n---\n\n# c\n",
    );
    for (node, text) in deltas {
        write(d, &format!("changes/c/deltas/{node}.md"), text);
    }
    dir
}

/// The hash `clm hashes` would report for an element of node `id`.
fn hash(root: &Path, id: &str, element: &str) -> String {
    let (graph, _) = load(root);
    clm_apply::hashes(&graph.nodes[id])
        .into_iter()
        .find(|(e, _)| e == element)
        .unwrap()
        .1
}

fn rules(violations: &[Violation]) -> Vec<&str> {
    violations.iter().map(|v| v.rule).collect()
}

fn assert_check_clean(root: &Path) {
    let (_, violations) = load(root);
    assert!(violations.is_empty(), "{violations:?}");
}

fn req_block(slug: &str, body: &str) -> String {
    format!("### Requirement: {slug}\n{body}\n\n#### Scenario: s\n- ok\n")
}

#[test]
fn stale_delta_blocks_every_write() {
    // req:apply/atomic@bb8aa354
    let dir = repo(&[]);
    let d = dir.path();
    write(
        d,
        "changes/c/deltas/a.md",
        &format!(
            "---\ncomponent: a\n---\n## ADDED Requirements\n{}",
            req_block("new", "New.")
        ),
    );
    write(
        d,
        "changes/c/deltas/clm.md",
        "---\ncomponent: clm\nbase: { contract: \"0000000000000000\" }\n---\n## MODIFIED Contract\nnew root\n",
    );
    let before_a = read(d, "specs/a/SPEC.md");
    let before_root = read(d, "specs/SPEC.md");

    let violations = apply(d, "c").unwrap_err();
    assert_eq!(rules(&violations), ["apply/stale-base"]);
    assert_eq!(read(d, "specs/a/SPEC.md"), before_a);
    assert_eq!(read(d, "specs/SPEC.md"), before_root);
    assert!(read(d, "changes/c/change.md").contains("status: proposed"));
}

#[test]
fn stale_requirement_reports_both_hashes() {
    // req:apply/stale-base@6b3410c6
    let dir = repo(&[]);
    let d = dir.path();
    let current = hash(d, "a", "requirement:y");
    write(
        d,
        "changes/c/deltas/a.md",
        "---\ncomponent: a\n---\n## MODIFIED Requirements\n### Requirement: y <!-- base: 1111111111111111 -->\nChanged.\n\n#### Scenario: s\n- ok\n",
    );
    let (graph, _) = load(d);
    let violations = validate(&graph, &graph.changes["c"]);
    assert_eq!(rules(&violations), ["apply/stale-base"]);
    assert!(violations[0].message.contains("a/y"), "{violations:?}");
    assert!(violations[0].message.contains("1111111111111111"));
    assert!(violations[0].message.contains(&current));
}

/// Repo `repo(&[])` whose `a/y` serves `capability/c`, with capabilities `c` and `d`.
fn repo_with_serves() -> TempDir {
    let dir = repo(&[]);
    let d = dir.path();
    let spec = read(d, "specs/a/SPEC.md").replace(
        "#### Scenario: y\n- ok\n",
        "#### Scenario: y\n- ok\n\n#### Serves\n- capability/c\n",
    );
    write(d, "specs/a/SPEC.md", &spec);
    let entry = |c: &str| {
        format!(
            "### Capability: {c}\nA person can {c}.\n\n#### Why\nW.\n\n#### Scenario: s\n- ok\n\n"
        )
    };
    write(
        d,
        "CAPABILITIES.md",
        &format!(
            "# Capabilities\n\n## Capabilities\n\n{}{}",
            entry("c"),
            entry("d")
        ),
    );
    dir
}

/// Writes a delta modifying `a/y` with `base` and, when `own`, its own `#### Serves`, then
/// edits only `a/y`'s `#### Serves` block on `main`.
fn modify_y_then_edit_serves(d: &Path, own: bool) {
    let base = hash(d, "a", "requirement:y");
    let serves = if own {
        "\n#### Serves\n- capability/c\n"
    } else {
        ""
    };
    write(
        d,
        "changes/c/deltas/a.md",
        &format!(
            "---\ncomponent: a\n---\n## MODIFIED Requirements\n### Requirement: y <!-- base: {base} -->\nThe y SHALL be new.\n\n#### Scenario: y2\n- ok\n{serves}"
        ),
    );
    let spec = read(d, "specs/a/SPEC.md").replace("- capability/c\n", "- capability/d\n");
    write(d, "specs/a/SPEC.md", &spec);
    assert_ne!(hash(d, "a", "requirement:y"), base);
}

#[test]
fn serves_edit_stales_a_delta_with_its_own_serves() {
    // req:apply/stale-base@6b3410c6
    let dir = repo_with_serves();
    let d = dir.path();
    modify_y_then_edit_serves(d, true);
    let (graph, _) = load(d);
    let violations = validate(&graph, &graph.changes["c"]);
    assert_eq!(rules(&violations), ["apply/stale-base"], "{violations:?}");
    assert!(violations[0].message.contains("a/y"), "{violations:?}");
}

#[test]
fn serves_edit_leaves_a_delta_without_serves_valid() {
    // req:apply/stale-base@6b3410c6 req:apply/requirements@b45cdf6f
    let dir = repo_with_serves();
    let d = dir.path();
    modify_y_then_edit_serves(d, false);
    apply(d, "c").unwrap();
    assert_check_clean(d);
    let (graph, _) = load(d);
    let y = &graph.nodes["a"].requirements[1];
    assert!(y.text.contains("SHALL be new"));
    assert_eq!(y.serves, ["capability/d"]);
}

#[test]
fn a_wrong_version_is_stale_without_serves() {
    // req:apply/stale-base@6b3410c6
    let dir = repo_with_serves();
    let d = dir.path();
    let current = hash(d, "a", "requirement:y");
    // The serves tail is right, the version wrong: still stale.
    let base = format!("11111111{}", &current[8..]);
    write(
        d,
        "changes/c/deltas/a.md",
        &format!(
            "---\ncomponent: a\n---\n## MODIFIED Requirements\n### Requirement: y <!-- base: {base} -->\nNew.\n\n#### Scenario: s\n- ok\n"
        ),
    );
    let (graph, _) = load(d);
    let violations = validate(&graph, &graph.changes["c"]);
    assert_eq!(rules(&violations), ["apply/stale-base"], "{violations:?}");
}

#[test]
fn a_bare_version_is_no_base() {
    // req:apply/stale-base@6b3410c6
    let dir = repo_with_serves();
    let d = dir.path();
    let current = hash(d, "a", "requirement:y");
    for base in [current[..8].to_string(), format!("{}zz", &current[..8])] {
        write(
            d,
            "changes/c/deltas/a.md",
            &format!(
                "---\ncomponent: a\n---\n## MODIFIED Requirements\n### Requirement: y <!-- base: {base} -->\nNew.\n\n#### Scenario: s\n- ok\n"
            ),
        );
        let (graph, _) = load(d);
        let violations = validate(&graph, &graph.changes["c"]);
        assert_eq!(rules(&violations), ["apply/stale-base"], "{base}");
    }
}

#[test]
fn removal_compares_the_whole_hash() {
    // req:apply/stale-base@6b3410c6
    let dir = repo_with_serves();
    let d = dir.path();
    let base = hash(d, "a", "requirement:y");
    write(
        d,
        "changes/c/deltas/a.md",
        &format!(
            "---\ncomponent: a\n---\n## REMOVED Requirements\n### Requirement: y <!-- base: {base} -->\n"
        ),
    );
    let spec = read(d, "specs/a/SPEC.md").replace("- capability/c\n", "- capability/d\n");
    write(d, "specs/a/SPEC.md", &spec);
    let (graph, _) = load(d);
    let violations = validate(&graph, &graph.changes["c"]);
    assert_eq!(rules(&violations), ["apply/stale-base"], "{violations:?}");
}

#[test]
fn modification_without_base_is_invalid() {
    // req:apply/stale-base@6b3410c6 req:apply/hash-command@971912c5
    let dir = repo(&[]);
    let d = dir.path();
    write(
        d,
        "changes/c/deltas/a.md",
        "---\ncomponent: a\n---\n## MODIFIED Design\nnew\n\n## REMOVED Requirements\n### Requirement: x\n",
    );
    let (graph, _) = load(d);
    let violations = validate(&graph, &graph.changes["c"]);
    assert_eq!(rules(&violations), ["apply/stale-base", "apply/stale-base"]);
    // The message points to the command by its new name.
    for v in &violations {
        assert!(v.message.contains("`clm hashes a`"), "{violations:?}");
    }
}

#[test]
fn modify_replaces_in_place_and_keeps_order() {
    // req:apply/requirements@b45cdf6f
    let dir = repo(&[]);
    let d = dir.path();
    let base = hash(d, "a", "requirement:y");
    write(
        d,
        "changes/c/deltas/a.md",
        &format!(
            "---\ncomponent: a\n---\n## MODIFIED Requirements\n### Requirement: y <!-- base: {base} -->\nThe y SHALL be new.\n\n#### Scenario: y2\n- ok\n"
        ),
    );
    apply(d, "c").unwrap();
    assert_check_clean(d);
    let (graph, _) = load(d);
    let reqs = &graph.nodes["a"].requirements;
    let ids: Vec<_> = reqs.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(ids, ["a/x", "a/y", "a/z"]);
    assert!(reqs[1].text.contains("SHALL be new"));
    assert!(!read(d, "specs/a/SPEC.md").contains("<!--"));
}

#[test]
fn modify_without_serves_keeps_the_rules_serves() {
    // req:apply/requirements@b45cdf6f
    for (own, kept) in [
        ("", "capability/c"),
        ("\n#### Serves\n- capability/d\n", "capability/d"),
    ] {
        let dir = repo(&[]);
        let d = dir.path();
        let spec = read(d, "specs/a/SPEC.md").replace(
            "#### Scenario: y\n- ok\n",
            "#### Scenario: y\n- ok\n\n#### Serves\n- capability/c\n",
        );
        write(d, "specs/a/SPEC.md", &spec);
        let entry = |c: &str| {
            format!(
                "### Capability: {c}\nA person can {c}.\n\n#### Why\nW.\n\n#### Scenario: s\n- ok\n\n"
            )
        };
        let layer = format!(
            "# Capabilities\n\n## Capabilities\n\n{}{}",
            entry("c"),
            entry("d")
        );
        write(d, "CAPABILITIES.md", &layer);
        let base = hash(d, "a", "requirement:y");
        write(
            d,
            "changes/c/deltas/a.md",
            &format!(
                "---\ncomponent: a\n---\n## MODIFIED Requirements\n### Requirement: y <!-- base: {base} -->\nThe y SHALL be new.\n\n#### Scenario: y2\n- ok\n{own}"
            ),
        );
        apply(d, "c").unwrap();
        let (graph, _) = load(d);
        let y = &graph.nodes["a"].requirements[1];
        assert!(y.text.contains("SHALL be new"), "{own:?}");
        assert_eq!(y.serves, [kept], "{own:?}");
        assert_eq!(
            read(d, "specs/a/SPEC.md").matches("#### Serves").count(),
            1,
            "{own:?}"
        );
    }
}

#[test]
fn kept_serves_stops_at_the_next_heading_and_skips_fences() {
    // req:apply/requirements@b45cdf6f
    let dir = repo(&[]);
    let d = dir.path();
    let entry = |c: &str| {
        format!(
            "### Capability: {c}\nA person can {c}.\n\n#### Why\nW.\n\n#### Scenario: s\n- ok\n\n"
        )
    };
    write(
        d,
        "CAPABILITIES.md",
        &format!("# Capabilities\n\n## Capabilities\n\n{}", entry("c")),
    );
    let spec = read(d, "specs/a/SPEC.md").replace(
        "#### Scenario: y\n- ok\n",
        "#### Scenario: y\n- ok\n\n#### Serves\n- capability/c\n\n#### Scenario: later\n- ok\n",
    );
    write(d, "specs/a/SPEC.md", &spec);
    let base = hash(d, "a", "requirement:y");
    // The delta's text shows a Serves block only inside a fence: it has none of its own.
    write(
        d,
        "changes/c/deltas/a.md",
        &format!(
            "---\ncomponent: a\n---\n## MODIFIED Requirements\n### Requirement: y <!-- base: {base} -->\nThe y SHALL be new, as in:\n\n```\n#### Serves\n- capability/z\n```\n\n#### Scenario: y2\n- ok\n\n#### Scenario: later\n- ok\n"
        ),
    );
    apply(d, "c").unwrap();
    let (graph, _) = load(d);
    let y = &graph.nodes["a"].requirements[1];
    assert_eq!(y.serves, ["capability/c"]);
    let text = read(d, "specs/a/SPEC.md");
    assert_eq!(text.matches("#### Scenario: later").count(), 1, "{text}");
}

#[test]
fn add_appends_and_remove_deletes() {
    // req:apply/requirements@b45cdf6f
    let dir = repo(&[]);
    let d = dir.path();
    let base = hash(d, "a", "requirement:x");
    write(
        d,
        "changes/c/deltas/a.md",
        &format!(
            "---\ncomponent: a\n---\n## ADDED Requirements\n{}\n## REMOVED Requirements\n### Requirement: x <!-- base: {base} -->\n",
            req_block("w", "New w.")
        ),
    );
    apply(d, "c").unwrap();
    assert_check_clean(d);
    let (graph, _) = load(d);
    let ids: Vec<_> = graph.nodes["a"]
        .requirements
        .iter()
        .map(|r| r.id.as_str())
        .collect();
    assert_eq!(ids, ["a/y", "a/z", "a/w"]);
}

#[test]
fn slug_existence_is_checked() {
    // req:apply/requirements@b45cdf6f
    let dir = repo(&[]);
    let d = dir.path();
    write(
        d,
        "changes/c/deltas/a.md",
        &format!(
            "---\ncomponent: a\n---\n## ADDED Requirements\n{}\n## MODIFIED Requirements\n### Requirement: ghost <!-- base: 0000000000000000 -->\nx\n",
            req_block("x", "Dup.")
        ),
    );
    let (graph, _) = load(d);
    let violations = validate(&graph, &graph.changes["c"]);
    assert_eq!(
        rules(&violations),
        ["apply/requirements", "apply/requirements"]
    );
}

#[test]
fn modify_contract_leaves_design_byte_identical() {
    // req:apply/sections@c799ce06
    let dir = repo(&[]);
    let d = dir.path();
    let base = hash(d, "a", "contract");
    write(
        d,
        "changes/c/deltas/a.md",
        &format!(
            "---\ncomponent: a\nbase: {{ contract: \"{base}\" }}\n---\n## MODIFIED Contract\nThe new contract.\n"
        ),
    );
    let design_before = load(d).0.nodes["a"].design.clone();
    apply(d, "c").unwrap();
    assert_check_clean(d);
    let (graph, _) = load(d);
    assert_eq!(graph.nodes["a"].contract.trim(), "The new contract.");
    assert_eq!(graph.nodes["a"].design, design_before);
}

#[test]
fn frontmatter_edit_preserves_other_lines_and_comments() {
    // req:apply/frontmatter@5c9e4b50
    let dir = repo(&[]);
    let d = dir.path();
    let base = hash(d, "a", "frontmatter");
    write(
        d,
        "changes/c/deltas/a.md",
        &format!(
            "---\ncomponent: a\nbase: {{ frontmatter: \"{base}\" }}\n---\n## MODIFIED Frontmatter\n```yaml\ndepends_on: [clm]\npaths: [\"src/**\"]\n```\nReason: rationale is ignored.\n"
        ),
    );
    apply(d, "c").unwrap();
    assert_check_clean(d);
    let text = read(d, "specs/a/SPEC.md");
    assert!(text.contains("status: specd  # frontier\n"), "{text}");
    assert!(text.contains("depends_on: [clm]\n"), "{text}");
    assert!(text.contains("paths: [\"src/**\"]\n---\n"), "{text}");
}

#[test]
fn multi_line_frontmatter_value_is_invalid() {
    // req:apply/frontmatter@5c9e4b50
    let dir = repo(&[]);
    let d = dir.path();
    write(
        d,
        "specs/a/SPEC.md",
        &node_a(&["x"]).replace("depends_on: []", "depends_on:\n  - clm"),
    );
    let base = hash(d, "a", "frontmatter");
    write(
        d,
        "changes/c/deltas/a.md",
        &format!(
            "---\ncomponent: a\nbase: {{ frontmatter: \"{base}\" }}\n---\n## MODIFIED Frontmatter\n```yaml\ndepends_on: []\n```\n"
        ),
    );
    let (graph, _) = load(d);
    let violations = validate(&graph, &graph.changes["c"]);
    assert_eq!(rules(&violations), ["apply/frontmatter"]);
    assert!(
        violations[0].message.contains("spans more than one line"),
        "{violations:?}"
    );
}

fn create_delta(node: &str) -> String {
    format!(
        "---\ncomponent: {node}\ncreate: true\n---\n## Frontmatter\n```yaml\ntitle: New\nstatus: specd\nowners: []\n```\n\n## Contract\nNew contract.\n\n## Design\nNew design.\n\n## ADDED Requirements\n{}",
        req_block("r", "R.")
    )
}

#[test]
fn creates_node_and_child_in_one_change() {
    // req:apply/create-node@78361ffe
    let dir = repo(&[]);
    let d = dir.path();
    write(d, "changes/c/deltas/b.md", &create_delta("b"));
    write(d, "changes/c/deltas/b.k.md", &create_delta("b.k"));
    apply(d, "c").unwrap();
    assert_check_clean(d);
    let (graph, _) = load(d);
    assert_eq!(graph.nodes["b.k"].parent.as_deref(), Some("b"));
    assert_eq!(graph.nodes["b"].contract.trim(), "New contract.");
    assert_eq!(graph.nodes["b.k"].requirements[0].id, "b.k/r");
}

#[test]
fn create_is_invalid_if_node_exists_or_parent_missing() {
    // req:apply/create-node@78361ffe
    let dir = repo(&[]);
    let d = dir.path();
    write(d, "changes/c/deltas/a.md", &create_delta("a"));
    write(d, "changes/c/deltas/q.r.md", &create_delta("q.r"));
    let (graph, _) = load(d);
    let violations = validate(&graph, &graph.changes["c"]);
    assert_eq!(
        rules(&violations),
        ["apply/create-node", "apply/create-node"]
    );
}

#[test]
fn already_applied_change_is_refused() {
    // req:apply/change-status@d7dfb1f4
    let dir = repo(&[]);
    let d = dir.path();
    write(d, "changes/c/change.md", "---\nstatus: applied\n---\n");
    write(
        d,
        "changes/c/deltas/a.md",
        &format!(
            "---\ncomponent: a\n---\n## ADDED Requirements\n{}",
            req_block("n", "N.")
        ),
    );
    let before = read(d, "specs/a/SPEC.md");
    let violations = apply(d, "c").unwrap_err();
    assert_eq!(rules(&violations), ["apply/change-status"]);
    assert_eq!(read(d, "specs/a/SPEC.md"), before);
}

#[test]
fn success_marks_change_applied() {
    // req:apply/change-status@d7dfb1f4
    let dir = repo(&[]);
    let d = dir.path();
    write(
        d,
        "changes/c/deltas/a.md",
        &format!(
            "---\ncomponent: a\n---\n## ADDED Requirements\n{}",
            req_block("n", "N.")
        ),
    );
    let written = apply(d, "c").unwrap();
    assert!(read(d, "changes/c/change.md").contains("status: applied\n"));
    assert!(
        written
            .iter()
            .any(|op| op.path().ends_with("specs/a/SPEC.md"))
    );
    assert!(
        written
            .iter()
            .any(|op| op.path().ends_with("changes/c/change.md"))
    );
}

#[test]
fn hashes_match_what_validation_compares() {
    // req:apply/hash-command@971912c5
    let dir = repo(&[]);
    let d = dir.path();
    let (graph, _) = load(d);
    let elements: Vec<String> = clm_apply::hashes(&graph.nodes["a"])
        .into_iter()
        .map(|(e, _)| e)
        .collect();
    assert_eq!(
        elements,
        [
            "frontmatter",
            "contract",
            "design",
            "requirement:x",
            "requirement:y",
            "requirement:z"
        ]
    );
    let (fm, contract, design, z) = (
        hash(d, "a", "frontmatter"),
        hash(d, "a", "contract"),
        hash(d, "a", "design"),
        hash(d, "a", "requirement:z"),
    );
    write(
        d,
        "changes/c/deltas/a.md",
        &format!(
            "---\ncomponent: a\nbase: {{ frontmatter: \"{fm}\", contract: \"{contract}\", design: \"{design}\" }}\n---\n## MODIFIED Frontmatter\n```yaml\ntitle: A2\n```\n\n## MODIFIED Contract\nc2\n\n## MODIFIED Design\nd2\n\n## MODIFIED Requirements\n### Requirement: z <!-- base: {z} -->\nZ2.\n\n#### Scenario: s\n- ok\n"
        ),
    );
    let (graph, _) = load(d);
    assert!(validate(&graph, &graph.changes["c"]).is_empty());
}

/// A repo whose node `a` has asset `m.html` ("old"), and change `c` with an `a` delta
/// (`base` recorded for `m.html` when given, plus `extra` sections) and `assets` files.
fn asset_repo(base: Option<&str>, extra: &str, assets: &[(&str, &[u8])]) -> TempDir {
    let dir = repo(&[]);
    let d = dir.path();
    write(d, "specs/a/assets/m.html", "old\n");
    let base = base.map_or(String::new(), |h| {
        format!("base: {{ assets: {{ \"m.html\": \"{h}\" }} }}\n")
    });
    write(
        d,
        "changes/c/deltas/a.md",
        &format!("---\ncomponent: a\n{base}---\n{extra}"),
    );
    for (path, bytes) in assets {
        let full = d.join("changes/c/assets/a").join(path);
        fs::create_dir_all(full.parent().unwrap()).unwrap();
        fs::write(full, bytes).unwrap();
    }
    dir
}

#[test]
fn modified_asset_is_written() {
    // req:apply/apply-assets@786adb62
    let current = clm_model::asset_hash(b"old\n");
    let dir = asset_repo(Some(&current), "", &[("m.html", b"new\n")]);
    let d = dir.path();
    // The base recorded is what `clm hashes a` prints for the asset.
    assert_eq!(hash(d, "a", "asset:m.html"), current);
    let written = apply(d, "c").unwrap();
    assert_eq!(read(d, "specs/a/assets/m.html"), "new\n");
    assert!(written.contains(&FileOp::Write(
        "specs/a/assets/m.html".into(),
        b"new\n".to_vec()
    )));
    assert_check_clean(d);
}

#[test]
fn stale_asset_writes_nothing() {
    // req:apply/apply-assets@786adb62
    let base = clm_model::asset_hash(b"old\n");
    let dir = asset_repo(
        Some(&base),
        &format!("## ADDED Requirements\n{}", req_block("n", "N.")),
        &[("m.html", b"new\n")],
    );
    let d = dir.path();
    // `main` edits the asset after the delta was written.
    write(d, "specs/a/assets/m.html", "edited on main\n");
    let spec_before = read(d, "specs/a/SPEC.md");

    let violations = apply(d, "c").unwrap_err();
    assert_eq!(rules(&violations), ["apply/stale-base"]);
    assert!(violations[0].message.contains("`a`"), "{violations:?}");
    assert!(violations[0].message.contains("m.html"), "{violations:?}");
    assert_eq!(read(d, "specs/a/assets/m.html"), "edited on main\n");
    assert_eq!(read(d, "specs/a/SPEC.md"), spec_before);
    assert!(read(d, "changes/c/change.md").contains("status: proposed"));
}

#[test]
fn modified_asset_without_base_is_stale() {
    // req:apply/apply-assets@786adb62
    let dir = asset_repo(None, "", &[("m.html", b"new\n")]);
    let d = dir.path();
    let violations = apply(d, "c").unwrap_err();
    assert_eq!(rules(&violations), ["apply/stale-base"]);
    assert!(violations[0].message.contains("m.html"), "{violations:?}");
    assert_eq!(read(d, "specs/a/assets/m.html"), "old\n");
}

#[test]
fn removed_asset_is_deleted() {
    // req:apply/apply-assets@786adb62
    let dir = asset_repo(None, "", &[]);
    let d = dir.path();
    write(d, "specs/a/assets/img/old.png", "png");
    let hash = clm_model::asset_hash(b"png");
    write(
        d,
        "changes/c/deltas/a.md",
        &format!(
            "---\ncomponent: a\n---\n## REMOVED Assets\n- `img/old.png` <!-- base: {hash} -->\n"
        ),
    );
    let written = apply(d, "c").unwrap();
    assert!(!d.join("specs/a/assets/img/old.png").exists());
    // The directory it empties goes too, as `clm sync`'s regenerated tree has none.
    assert!(!d.join("specs/a/assets/img").exists());
    assert!(written.contains(&FileOp::Remove("specs/a/assets/img/old.png".into())));
    assert_eq!(read(d, "specs/a/assets/m.html"), "old\n");
    assert_check_clean(d);
}

#[test]
fn a_recorded_base_for_a_missing_asset_is_stale() {
    // req:apply/apply-assets@786adb62
    // `main` deleted `m.html` after the delta recorded its hash.
    let base = clm_model::asset_hash(b"old\n");
    let dir = asset_repo(Some(&base), "", &[("m.html", b"new\n")]);
    let d = dir.path();
    fs::remove_file(d.join("specs/a/assets/m.html")).unwrap();
    let violations = apply(d, "c").unwrap_err();
    assert_eq!(rules(&violations), ["apply/stale-base"]);
    assert!(violations[0].message.contains("m.html"), "{violations:?}");
    assert!(!d.join("specs/a/assets/m.html").exists());
}

#[test]
fn an_asset_both_written_and_removed_or_removed_twice_is_invalid() {
    // req:apply/apply-assets@786adb62
    let base = clm_model::asset_hash(b"old\n");
    let entry = format!("- `m.html` <!-- base: {base} -->\n");
    let dir = asset_repo(
        Some(&base),
        &format!("## REMOVED Assets\n{entry}"),
        &[("m.html", b"new\n")],
    );
    let d = dir.path();
    let violations = apply(d, "c").unwrap_err();
    assert_eq!(rules(&violations), ["apply/apply-assets"]);
    assert_eq!(read(d, "specs/a/assets/m.html"), "old\n");

    let dir = asset_repo(None, &format!("## REMOVED Assets\n{entry}{entry}"), &[]);
    let d = dir.path();
    let violations = apply(d, "c").unwrap_err();
    assert_eq!(rules(&violations), ["apply/apply-assets"]);
    assert_eq!(read(d, "specs/a/assets/m.html"), "old\n");
}

#[test]
fn root_assets_are_invalid() {
    // req:apply/apply-assets@786adb62
    let dir = repo(&[]);
    let d = dir.path();
    write(d, "changes/c/deltas/clm.md", "---\ncomponent: clm\n---\n");
    write(d, "changes/c/assets/clm/x.png", "png");
    let violations = apply(d, "c").unwrap_err();
    assert_eq!(rules(&violations), ["apply/apply-assets"]);
    assert!(!d.join("specs/assets").exists());
}

#[test]
fn removing_a_missing_or_changed_asset_is_stale() {
    // req:apply/apply-assets@786adb62
    let dir = asset_repo(
        None,
        "## REMOVED Assets\n- `ghost.png` <!-- base: 0000000000000000 -->\n- `m.html` <!-- base: 1111111111111111 -->\n",
        &[],
    );
    let d = dir.path();
    let violations = apply(d, "c").unwrap_err();
    assert_eq!(rules(&violations), ["apply/stale-base", "apply/stale-base"]);
    assert!(
        violations[0].message.contains("ghost.png"),
        "{violations:?}"
    );
    assert!(violations[1].message.contains("1111111111111111"));
    assert_eq!(read(d, "specs/a/assets/m.html"), "old\n");
}

#[test]
fn new_asset_needs_no_base_and_binary_is_copied_byte_for_byte() {
    // req:apply/apply-assets@786adb62
    let bytes: &[u8] = &[0xff, 0x00, 0xfe, b'\n'];
    let dir = asset_repo(None, "", &[("img/new.bin", bytes)]);
    let d = dir.path();
    apply(d, "c").unwrap();
    assert_eq!(
        fs::read(d.join("specs/a/assets/img/new.bin")).unwrap(),
        bytes
    );
    assert_eq!(read(d, "specs/a/assets/m.html"), "old\n");
    assert_check_clean(d);
}

#[test]
fn created_node_gets_its_assets() {
    // req:apply/apply-assets@786adb62
    let dir = repo(&[]);
    let d = dir.path();
    write(d, "changes/c/deltas/b.k.md", &create_delta("b.k"));
    write(d, "changes/c/deltas/b.md", &create_delta("b"));
    write(d, "changes/c/assets/b.k/m.html", "<p>k</p>\n");
    apply(d, "c").unwrap();
    assert_eq!(read(d, "specs/b/k/assets/m.html"), "<p>k</p>\n");
    assert_check_clean(d);
    let (graph, _) = load(d);
    assert!(graph.nodes["b.k"].assets.contains_key("m.html"));
}

#[test]
fn hashes_of_the_capability_node() {
    // req:apply/hash-command@971912c5
    let dir = repo(&[]);
    let d = dir.path();
    write(
        d,
        "CAPABILITIES.md",
        "## Capabilities\n\n### Capability: a\nOne can.\n\n#### Why\nw\n\n#### Scenario: s\n- ok\n",
    );
    write(
        d,
        "specs/capability/SPEC.md",
        "---\ntitle: Capabilities\nstatus: specd\nowners: []\ndepends_on: []\n---\n\n## Contract\nc\n\n## Design\nd\n",
    );
    let (graph, violations) = load(d);
    assert!(violations.is_empty(), "{violations:?}");
    let lines = clm_apply::layer_hashes(&graph, clm_model::Layer::Capability);
    let elements: Vec<_> = lines.iter().map(|(e, _)| e.as_str()).collect();
    assert_eq!(
        elements,
        ["frontmatter", "contract", "design", "capability:a"]
    );
    assert_eq!(lines[3].1, graph.entry("capability/a").unwrap().hash);
    let design = &lines[2].1;
    write(
        d,
        "changes/c/deltas/capability.md",
        &format!(
            "---\ncomponent: capability\nbase: {{ design: \"{design}\" }}\n---\n## MODIFIED Design\nnew\n"
        ),
    );
    let (graph, _) = load(d);
    assert_eq!(validate(&graph, &graph.changes["c"]), []);
}

#[test]
fn hashes_of_a_layer_without_its_node() {
    // req:apply/hash-command@971912c5
    let dir = repo(&[]);
    let d = dir.path();
    write(
        d,
        "ARCHITECTURE.md",
        "## Constraints\n\n### Constraint: x\nIt SHALL.\n\n#### Scenario: s\n- ok\n",
    );
    let (graph, _) = load(d);
    let lines = clm_apply::layer_hashes(&graph, clm_model::Layer::Architecture);
    let entry = graph.entry("architecture/x").unwrap();
    assert_eq!(lines, [("requirement:x".to_string(), entry.hash.clone())]);
    assert_eq!(
        clm_apply::layer_hashes(&graph, clm_model::Layer::Capability),
        []
    );
}

#[test]
fn hashes_lists_assets_after_requirements() {
    // req:apply/hash-command@971912c5 req:apply/apply-assets@786adb62
    let dir = repo(&[]);
    let d = dir.path();
    write(d, "specs/a/assets/z.txt", "z\n");
    write(d, "specs/a/assets/m.html", "<p>m</p>\n");
    let (graph, _) = load(d);
    let hashes = clm_apply::hashes(&graph.nodes["a"]);
    let tail: Vec<_> = hashes[hashes.len() - 2..].to_vec();
    assert_eq!(
        tail,
        [
            (
                "asset:m.html".to_string(),
                clm_model::asset_hash(b"<p>m</p>\n")
            ),
            ("asset:z.txt".to_string(), clm_model::asset_hash(b"z\n")),
        ]
    );
}

/// A `remove: true` delta for `id` built from its current hashes, leaving out the
/// requirements and assets named in `skip`.
fn removal(root: &Path, id: &str, skip: &[&str]) -> String {
    let (graph, _) = load(root);
    let mut bases = Vec::new();
    let mut reqs = String::new();
    let mut assets = String::new();
    for (element, hash) in clm_apply::hashes(&graph.nodes[id]) {
        if let Some(slug) = element.strip_prefix("requirement:") {
            if !skip.contains(&slug) {
                reqs.push_str(&format!("### Requirement: {slug} <!-- base: {hash} -->\n"));
            }
        } else if let Some(path) = element.strip_prefix("asset:") {
            if !skip.contains(&path) {
                assets.push_str(&format!("- `{path}` <!-- base: {hash} -->\n"));
            }
        } else {
            bases.push(format!("{element}: \"{hash}\""));
        }
    }
    format!(
        "---\ncomponent: {id}\nremove: true\nbase: {{ {} }}\n---\n## REMOVED Requirements\n{reqs}\n## REMOVED Assets\n{assets}",
        bases.join(", ")
    )
}

/// Every file under `rel`, sorted, so a refused apply can be shown to write nothing.
fn snapshot(root: &Path, rel: &str) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    let mut stack = vec![root.join(rel)];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else {
                let name = path.strip_prefix(root).unwrap().display().to_string();
                out.push((name, fs::read(&path).unwrap()));
            }
        }
    }
    out.sort();
    out
}

fn messages(violations: &[Violation]) -> String {
    violations
        .iter()
        .map(|v| v.message.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn node_removed() {
    // req:apply/remove-node@668c4ecf
    let dir = repo(&[]);
    let d = dir.path();
    write(d, "specs/a/assets/m.html", "m\n");
    write(d, "changes/c/deltas/a.md", &removal(d, "a", &[]));
    let ops = apply(d, "c").unwrap();
    assert!(!d.join("specs/a").exists());
    assert!(ops.contains(&FileOp::Remove("specs/a/SPEC.md".into())));
    assert!(ops.contains(&FileOp::Remove("specs/a/assets/m.html".into())));
    assert!(read(d, "changes/c/change.md").contains("status: applied"));
    assert_check_clean(d);
}

#[test]
fn removal_leaving_out_a_requirement_or_asset_is_invalid() {
    // req:apply/remove-node@668c4ecf
    let dir = repo(&[]);
    let d = dir.path();
    write(d, "specs/a/assets/m.html", "m\n");
    write(
        d,
        "changes/c/deltas/a.md",
        &removal(d, "a", &["y", "m.html"]),
    );
    let before = snapshot(d, "");
    let violations = apply(d, "c").unwrap_err();
    let text = messages(&violations);
    assert!(text.contains("a/y"), "{text}");
    assert!(text.contains("m.html"), "{text}");
    assert!(!text.contains("a/x"), "{text}");
    assert_eq!(snapshot(d, ""), before);
    assert!(read(d, "changes/c/change.md").contains("status: proposed"));
}

#[test]
fn removal_needs_current_bases_for_frontmatter_contract_and_design() {
    // req:apply/remove-node@668c4ecf
    let dir = repo(&[]);
    let d = dir.path();
    let full = removal(d, "a", &[]);
    let base = |element: &str| format!("{element}: \"{}\"", hash(d, "a", element));
    let x = hash(d, "a", "requirement:x");
    // Each of frontmatter, contract and design in turn without a base, then stale, with the
    // others current.
    for element in ["frontmatter", "contract", "design"] {
        for (replacement, expected) in [
            (
                format!("{element}: \"2222222222222222\""),
                "2222222222222222",
            ),
            (format!("{element}: null"), "without a recorded base"),
        ] {
            let edited = full.replace(&base(element), &replacement);
            assert_ne!(edited, full);
            write(d, "changes/c/deltas/a.md", &edited);
            let violations = apply(d, "c").unwrap_err();
            let text = messages(&violations);
            assert_eq!(violations.len(), 1, "{text}");
            assert!(text.contains(&format!("{element} of `a`")), "{text}");
            assert!(text.contains(expected), "{text}");
        }
    }
    // A requirement listed with a stale base, or none.
    for replacement in ["3333333333333333 -->", "-->"] {
        let edited = full.replace(&format!("{x} -->"), replacement);
        write(d, "changes/c/deltas/a.md", &edited);
        let text = messages(&apply(d, "c").unwrap_err());
        assert!(text.contains("requirement `a/x`"), "{text}");
    }
    assert!(d.join("specs/a/SPEC.md").exists());
    assert!(read(d, "changes/c/change.md").contains("status: proposed"));
}

#[test]
fn removing_a_missing_node_is_invalid() {
    // req:apply/remove-node@668c4ecf
    let dir = repo(&[]);
    let d = dir.path();
    write(
        d,
        "changes/c/deltas/gone.md",
        "---\ncomponent: gone\nremove: true\n---\n## REMOVED Requirements\n",
    );
    let violations = apply(d, "c").unwrap_err();
    assert!(messages(&violations).contains("`gone`"), "{violations:?}");
}

#[test]
fn removing_a_node_with_a_child_left_behind_is_invalid() {
    // req:apply/remove-node@668c4ecf
    let dir = repo(&[]);
    let d = dir.path();
    write(d, "specs/a/b/SPEC.md", &node_a(&["q"]));
    write(d, "changes/c/deltas/a.md", &removal(d, "a", &[]));
    let before = snapshot(d, "");
    let violations = apply(d, "c").unwrap_err();
    assert!(messages(&violations).contains("`a.b`"), "{violations:?}");
    assert_eq!(snapshot(d, ""), before);
}

#[test]
fn removing_the_root_is_invalid() {
    // req:apply/remove-node@668c4ecf
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    write(
        d,
        "changes/c/change.md",
        "---\nstatus: proposed\n---\n\n# c\n",
    );
    write(d, "changes/c/deltas/clm.md", &removal(d, "clm", &[]));
    let before = snapshot(d, "");
    let violations = apply(d, "c").unwrap_err();
    assert!(messages(&violations).contains("`clm`"), "{violations:?}");
    assert_eq!(snapshot(d, ""), before);
}

#[test]
fn removing_a_node_the_change_creates_a_child_of_is_invalid() {
    // req:apply/remove-node@668c4ecf
    let dir = repo(&[]);
    let d = dir.path();
    write(d, "changes/c/deltas/a.md", &removal(d, "a", &[]));
    write(d, "changes/c/deltas/a.c.md", &create_delta("a.c"));
    let before = snapshot(d, "");
    let violations = apply(d, "c").unwrap_err();
    let text = messages(&violations);
    assert!(text.contains("`a`") && text.contains("`a.c`"), "{text}");
    assert_eq!(snapshot(d, ""), before);
}

#[test]
fn parent_and_child_removed_together() {
    // req:apply/remove-node@668c4ecf
    for order in [["a", "a.b"], ["a.b", "a"]] {
        let dir = repo(&[]);
        let d = dir.path();
        write(d, "specs/a/b/SPEC.md", &node_a(&["q"]));
        write(d, "specs/a/b/assets/i.png", "png");
        for id in order {
            write(
                d,
                &format!("changes/c/deltas/{id}.md"),
                &removal(d, id, &[]),
            );
        }
        apply(d, "c").unwrap();
        assert!(!d.join("specs/a").exists());
        assert_check_clean(d);
    }
}

#[test]
fn removal_keeps_a_directory_holding_other_files() {
    // req:apply/remove-node@668c4ecf
    let dir = repo(&[]);
    let d = dir.path();
    write(d, "specs/a/assets/m.html", "m\n");
    write(d, "specs/a/notes.txt", "kept\n");
    write(d, "changes/c/deltas/a.md", &removal(d, "a", &[]));
    apply(d, "c").unwrap();
    assert!(!d.join("specs/a/SPEC.md").exists());
    assert!(!d.join("specs/a/assets").exists());
    assert_eq!(read(d, "specs/a/notes.txt"), "kept\n");
}

/// A change's `change.md` with `status`, declaring `keys`.
fn change_md(status: &str, keys: &str) -> String {
    format!("---\nstatus: {status}\nfrontmatter_keys: {keys}\n---\n\n# c\n")
}

/// A delta for `a` whose `## MODIFIED Frontmatter` sets `colour: blue`, with the correct base.
fn colour_delta(root: &Path) -> String {
    let base = hash(root, "a", "frontmatter");
    format!(
        "---\ncomponent: a\nbase: {{ frontmatter: \"{base}\" }}\n---\n## MODIFIED Frontmatter\n```yaml\ncolour: blue\n```\n"
    )
}

/// A delta creating `b` with `colour: red` in its `## Frontmatter`.
fn colour_create() -> String {
    create_delta("b").replace("owners: []\n", "owners: []\ncolour: red\n")
}

#[test]
fn key_the_same_change_declares() {
    // req:apply/declared-frontmatter-keys@70002c38
    let dir = repo(&[]);
    let d = dir.path();
    write(d, "changes/c/change.md", &change_md("proposed", "[colour]"));
    write(d, "changes/c/deltas/a.md", &colour_delta(d));
    apply(d, "c").unwrap();
    let text = read(d, "specs/a/SPEC.md");
    assert!(text.contains("\ncolour: blue\n---\n"), "{text}");
    assert_check_clean(d);
}

#[test]
fn key_another_live_change_declares() {
    // req:apply/declared-frontmatter-keys@70002c38
    let dir = repo(&[]);
    let d = dir.path();
    write(d, "changes/y/change.md", &change_md("applied", "[colour]"));
    write(d, "changes/c/deltas/b.md", &colour_create());
    apply(d, "c").unwrap();
    let text = read(d, "specs/b/SPEC.md");
    assert!(text.contains("\ncolour: red\n"), "{text}");
    assert_check_clean(d);
}

#[test]
fn undeclared_key_is_a_violation_and_writes_nothing() {
    // req:apply/declared-frontmatter-keys@70002c38
    let dir = repo(&[]);
    let d = dir.path();
    // An abandoned change isn't live, so its keys aren't declared.
    write(
        d,
        "changes/y/change.md",
        &change_md("abandoned", "[colour]"),
    );
    write(d, "changes/c/deltas/a.md", &colour_delta(d));
    write(d, "changes/c/deltas/b.md", &colour_create());
    let before = snapshot(d, "");
    let violations = apply(d, "c").unwrap_err();
    assert_eq!(
        rules(&violations),
        ["apply/frontmatter", "apply/create-node"]
    );
    for (v, node) in violations.iter().zip(["`a`", "`b`"]) {
        assert!(v.message.contains(node), "{violations:?}");
        assert!(v.message.contains("`colour`"), "{violations:?}");
    }
    assert_eq!(snapshot(d, ""), before);
}

#[test]
fn a_known_key_is_still_validated() {
    // req:apply/declared-frontmatter-keys@70002c38
    let dir = repo(&[]);
    let d = dir.path();
    write(
        d,
        "changes/c/deltas/a.md",
        &colour_delta(d).replace("colour: blue", "optional: maybe"),
    );
    let violations = apply(d, "c").unwrap_err();
    assert_eq!(rules(&violations), ["apply/frontmatter"]);
    assert!(
        violations[0].message.contains("`optional`"),
        "{violations:?}"
    );
}

#[test]
fn optional_root_is_a_violation_and_writes_nothing() {
    // req:apply/declared-frontmatter-keys@70002c38
    let dir = repo(&[]);
    let d = dir.path();
    let base = hash(d, "clm", "frontmatter");
    write(
        d,
        "changes/c/deltas/clm.md",
        &format!(
            "---\ncomponent: clm\nbase: {{ frontmatter: \"{base}\" }}\n---\n## MODIFIED Frontmatter\n```yaml\noptional: true\n```\n"
        ),
    );
    let before = snapshot(d, "");
    let violations = apply(d, "c").unwrap_err();
    assert_eq!(rules(&violations), ["apply/frontmatter"]);
    assert!(violations[0].message.contains("`clm`"), "{violations:?}");
    assert!(
        violations[0].message.contains("`optional`"),
        "{violations:?}"
    );
    assert_eq!(snapshot(d, ""), before);
}

#[test]
fn outside_off_the_root_is_a_violation_on_both_paths() {
    // req:apply/declared-frontmatter-keys@70002c38
    // "Any other failure of that check": the node-ID check's other root rule,
    // `outside-globs`, refuses `outside` on a non-root node, modified or created.
    let dir = repo(&[]);
    let d = dir.path();
    write(
        d,
        "changes/c/deltas/a.md",
        &colour_delta(d).replace("colour: blue", "outside: [\"x/**\"]"),
    );
    write(
        d,
        "changes/c/deltas/b.md",
        &create_delta("b").replace("owners: []\n", "owners: []\noutside: [\"x/**\"]\n"),
    );
    let before = snapshot(d, "");
    let violations = apply(d, "c").unwrap_err();
    assert_eq!(
        rules(&violations),
        ["apply/frontmatter", "apply/create-node"]
    );
    for (v, node) in violations.iter().zip(["`a`", "`b`"]) {
        assert!(v.message.contains(node), "{violations:?}");
        assert!(v.message.contains("`outside`"), "{violations:?}");
    }
    assert_eq!(snapshot(d, ""), before);
}

/// Node `b`, depending on `deps`, with requirement `q`.
fn node_b(deps: &str) -> String {
    node_a(&["q"]).replace("depends_on: []", &format!("depends_on: {deps}"))
}

/// A delta for `id` whose `## MODIFIED Frontmatter` is `yaml`, with the correct base.
fn frontmatter_delta(root: &Path, id: &str, yaml: &str) -> String {
    let base = hash(root, id, "frontmatter");
    format!(
        "---\ncomponent: {id}\nbase: {{ frontmatter: \"{base}\" }}\n---\n## MODIFIED Frontmatter\n```yaml\n{yaml}\n```\n"
    )
}

#[test]
fn required_node_depends_on_a_node_made_optional() {
    // req:apply/resulting-graph@b3caad6a
    let dir = repo(&[]);
    let d = dir.path();
    write(d, "specs/b/SPEC.md", &node_b("[a]"));
    write(
        d,
        "changes/c/deltas/a.md",
        &frontmatter_delta(d, "a", "optional: true"),
    );
    let before = snapshot(d, "");
    let violations = apply(d, "c").unwrap_err();
    assert_eq!(rules(&violations), ["apply/resulting-graph"]);
    let v = &violations[0];
    assert_eq!(v.path, Path::new("specs/b/SPEC.md"));
    assert!(
        v.message
            .starts_with("change `c` would leave: model/optional-nodes: "),
        "{v:?}"
    );
    assert!(
        v.message.contains("`b`") && v.message.contains("`a`"),
        "{v:?}"
    );
    assert_eq!(snapshot(d, ""), before);
}

#[test]
fn dependant_left_behind() {
    // req:apply/resulting-graph@b3caad6a req:apply/remove-node@668c4ecf
    let dir = repo(&[]);
    let d = dir.path();
    write(d, "specs/b/SPEC.md", &node_b("[a]"));
    write(d, "changes/c/deltas/a.md", &removal(d, "a", &[]));
    let before = snapshot(d, "");
    let violations = apply(d, "c").unwrap_err();
    assert_eq!(rules(&violations), ["apply/resulting-graph"]);
    let v = &violations[0];
    assert_eq!(v.path, Path::new("specs/b/SPEC.md"));
    assert!(
        v.message
            .starts_with("change `c` would leave: model/dependency-dag: "),
        "{v:?}"
    );
    assert!(v.message.contains("`a`"), "{v:?}");
    assert!(d.join("specs/a/SPEC.md").exists());
    assert_eq!(snapshot(d, ""), before);
}

#[test]
fn dependant_edited_in_the_same_change() {
    // req:apply/remove-node@668c4ecf req:apply/resulting-graph@b3caad6a
    let dir = repo(&[]);
    let d = dir.path();
    write(d, "specs/b/SPEC.md", &node_b("[a]"));
    write(d, "changes/c/deltas/a.md", &removal(d, "a", &[]));
    write(
        d,
        "changes/c/deltas/b.md",
        &frontmatter_delta(d, "b", "depends_on: []"),
    );
    apply(d, "c").unwrap();
    assert!(!d.join("specs/a").exists());
    let (graph, violations) = load(d);
    assert!(violations.is_empty(), "{violations:?}");
    assert!(graph.nodes["b"].frontmatter.depends_on.is_empty());
}

#[test]
fn backlog_blocks_a_removed_requirement() {
    // req:apply/resulting-graph@b3caad6a
    let dir = repo(&[]);
    let d = dir.path();
    write(
        d,
        "backlog/wait.md",
        "---\ntitle: Wait\nkind: question\ncomponents: [a]\nblocks: [a/y]\n---\n\nWhy?\n",
    );
    let y = hash(d, "a", "requirement:y");
    write(
        d,
        "changes/c/deltas/a.md",
        &format!(
            "---\ncomponent: a\n---\n## REMOVED Requirements\n### Requirement: y <!-- base: {y} -->\n"
        ),
    );
    assert_check_clean(d);
    let before = snapshot(d, "");
    let violations = apply(d, "c").unwrap_err();
    assert_eq!(rules(&violations), ["apply/resulting-graph"]);
    let v = &violations[0];
    assert_eq!(v.path, Path::new("backlog/wait.md"));
    assert!(
        v.message
            .starts_with("change `c` would leave: model/parse-backlog: "),
        "{v:?}"
    );
    assert!(v.message.contains("a/y"), "{v:?}");
    assert_eq!(snapshot(d, ""), before);
}

/// `backlog/<slug>.md` of `kind` blocking `blocks`, and change `c` resolving `slug` that removes
/// `a/y` with its base.
fn resolving_and_removing_y(d: &Path, slug: &str, kind: &str, blocks: &str) {
    write(
        d,
        &format!("backlog/{slug}.md"),
        &format!("---\ntitle: T\nkind: {kind}\ncomponents: [a]\nblocks: [{blocks}]\n---\n\nbody\n"),
    );
    resolving(d, &[slug]);
    write(d, "changes/c/deltas/a.md", &remove_y(d));
}

#[test]
fn answered_question_blocks_a_removed_requirement() {
    // req:apply/resulting-graph@b3caad6a
    let dir = repo(&[]);
    let d = dir.path();
    resolving_and_removing_y(d, "q", "question", "a/y");
    let (graph, violations) = load(d);
    assert!(violations.is_empty(), "{violations:?}");
    assert!(
        validate(&graph, &graph.changes["c"]).is_empty(),
        "check reports the change"
    );
    apply(d, "c").unwrap();
    assert!(!d.join("backlog/q.md").exists());
    assert!(!read(d, "specs/a/SPEC.md").contains("Requirement: y"));
    assert_check_clean(d);
}

#[test]
fn resolved_bug_blocks_a_removed_requirement() {
    // req:apply/resulting-graph@b3caad6a
    let dir = repo(&[]);
    let d = dir.path();
    resolving_and_removing_y(d, "b", "bug", "a/y");
    let before = snapshot(d, "");
    let violations = apply(d, "c").unwrap_err();
    assert_eq!(rules(&violations), ["apply/resulting-graph"]);
    let v = &violations[0];
    assert_eq!(v.path, Path::new("backlog/b.md"));
    assert!(
        v.message
            .starts_with("change `c` would leave: model/parse-backlog: "),
        "{v:?}"
    );
    assert!(v.message.contains("a/y"), "{v:?}");
    assert!(d.join("backlog/b.md").exists());
    assert_eq!(snapshot(d, ""), before);
}

#[test]
fn answered_question_that_blocks_nothing_is_no_violation() {
    // req:apply/resulting-graph@b3caad6a
    let dir = repo(&[]);
    let d = dir.path();
    write(
        d,
        "backlog/q.md",
        "---\ntitle: T\nkind: question\ncomponents: [a]\nblocks: [a/gone]\n---\n\nbody\n",
    );
    resolving(d, &["q"]);
    valid_delta(d);
    apply(d, "c").unwrap();
    assert!(!d.join("backlog/q.md").exists());
    assert_check_clean(d);
}

#[test]
fn violation_already_present_is_not_reported_again() {
    // req:apply/resulting-graph@b3caad6a
    let dir = repo(&[]);
    let d = dir.path();
    write(d, "specs/b/SPEC.md", &node_b("[gone]"));
    let base = hash(d, "a", "design");
    write(
        d,
        "changes/c/deltas/a.md",
        &format!(
            "---\ncomponent: a\nbase: {{ design: \"{base}\" }}\n---\n## MODIFIED Design\nNew.\n"
        ),
    );
    let (graph, current) = load(d);
    assert_eq!(rules(&current), ["model/dependency-dag"]);
    assert_eq!(validate(&graph, &graph.changes["c"]), []);
}

#[test]
fn needs_and_backlog_nodes_naming_a_removed_node_are_new_violations() {
    // req:apply/resulting-graph@b3caad6a req:apply/remove-node@668c4ecf
    let dir = repo(&[]);
    let d = dir.path();
    write(
        d,
        "specs/b/SPEC.md",
        &node_b("[]").replace("#### Scenario: q", "#### Needs\n- a/x\n\n#### Scenario: q"),
    );
    write(
        d,
        "backlog/later.md",
        "---\ntitle: Later\nkind: deferred\ncomponents: [a]\n---\n\nLater.\n",
    );
    write(d, "changes/c/deltas/a.md", &removal(d, "a", &[]));
    assert_check_clean(d);
    let before = snapshot(d, "");
    let violations = apply(d, "c").unwrap_err();
    assert_eq!(
        rules(&violations),
        ["apply/resulting-graph", "apply/resulting-graph"]
    );
    let text = messages(&violations);
    assert!(
        text.contains("model/requirement-needs: ") && text.contains("a/x"),
        "{text}"
    );
    assert!(
        text.contains("model/parse-backlog: ") && text.contains("`a`"),
        "{text}"
    );
    assert_eq!(snapshot(d, ""), before);
}

#[test]
fn not_checked_while_a_delta_is_invalid() {
    // req:apply/resulting-graph@b3caad6a
    // Removing `a` is valid, but `b`'s delta, which would drop its dependency on `a`, is
    // stale: only the stale base is reported, not the graph left without `b`'s edit.
    let dir = repo(&[]);
    let d = dir.path();
    write(d, "specs/b/SPEC.md", &node_b("[a]"));
    write(d, "changes/c/deltas/a.md", &removal(d, "a", &[]));
    write(
        d,
        "changes/c/deltas/b.md",
        &frontmatter_delta(d, "b", "depends_on: []")
            .replace(&hash(d, "b", "frontmatter"), "4444444444444444"),
    );
    let violations = apply(d, "c").unwrap_err();
    assert_eq!(rules(&violations), ["apply/stale-base"]);
}

#[test]
fn a_second_copy_of_a_violation_already_present_is_new() {
    // req:apply/resulting-graph@b3caad6a
    let dir = repo(&[]);
    let d = dir.path();
    write(d, "specs/b/SPEC.md", &node_b("[gone]"));
    write(
        d,
        "changes/c/deltas/b.md",
        &frontmatter_delta(d, "b", "depends_on: [gone, gone]"),
    );
    let (graph, _) = load(d);
    let violations = validate(&graph, &graph.changes["c"]);
    assert_eq!(rules(&violations), ["apply/resulting-graph"]);
    assert!(violations[0].message.contains("`gone`"), "{violations:?}");
}

#[test]
fn a_parse_violation_the_written_node_already_has_is_not_reported_again() {
    // req:apply/resulting-graph@b3caad6a
    // `a/y` has no scenario, which the node parser reports on `a`'s current text and on
    // the text the change writes alike.
    let dir = repo(&[]);
    let d = dir.path();
    let text = node_a(&["x", "y"]).replace("#### Scenario: y\n- ok\n", "");
    write(d, "specs/a/SPEC.md", &text);
    let base = hash(d, "a", "design");
    write(
        d,
        "changes/c/deltas/a.md",
        &format!(
            "---\ncomponent: a\nbase: {{ design: \"{base}\" }}\n---\n## MODIFIED Design\nNew.\n"
        ),
    );
    let (graph, current) = load(d);
    assert_eq!(rules(&current), ["model/unique-requirement-ids"]);
    assert_eq!(validate(&graph, &graph.changes["c"]), []);
}

#[test]
fn a_violation_whose_message_the_change_alters_is_new() {
    // req:apply/resulting-graph@b3caad6a
    // `a` -> `b` -> `a` is a cycle already; making `b` depend on `k`, which depends on
    // `a`, turns it into a longer one, whose message names `k`.
    let dir = repo(&[]);
    let d = dir.path();
    write(
        d,
        "specs/a/SPEC.md",
        &node_a(&["x"]).replace("depends_on: []", "depends_on: [b]"),
    );
    write(d, "specs/b/SPEC.md", &node_b("[a]"));
    write(d, "specs/k/SPEC.md", &node_b("[a]"));
    write(
        d,
        "changes/c/deltas/b.md",
        &frontmatter_delta(d, "b", "depends_on: [k]"),
    );
    let (graph, current) = load(d);
    assert_eq!(rules(&current), ["model/dependency-dag"]);
    let violations = validate(&graph, &graph.changes["c"]);
    assert_eq!(rules(&violations), ["apply/resulting-graph"]);
    let message = &violations[0].message;
    assert!(
        message.starts_with("change `c` would leave: model/dependency-dag: dependency cycle: ")
            && message.contains('k'),
        "{violations:?}"
    );
}

#[test]
fn checked_whatever_the_status_but_not_by_reapply() {
    // req:apply/resulting-graph@b3caad6a
    let dir = repo(&[]);
    let d = dir.path();
    write(d, "specs/b/SPEC.md", &node_b("[a]"));
    write(d, "changes/c/change.md", &change_md("applied", "[]"));
    write(d, "changes/c/deltas/a.md", &removal(d, "a", &[]));
    let (graph, _) = load(d);
    let change = &graph.changes["c"];
    assert_eq!(
        rules(&validate(&graph, change)),
        ["apply/change-status", "apply/resulting-graph"]
    );
    assert!(clm_apply::reapply(&graph, change).is_ok());
}

#[test]
fn a_created_node_is_checked_at_its_path() {
    // req:apply/resulting-graph@b3caad6a
    let dir = repo(&[]);
    let d = dir.path();
    write(
        d,
        "changes/c/deltas/n.md",
        &create_delta("n").replace("owners: []\n", "owners: []\ndepends_on: [nowhere]\n"),
    );
    let violations = apply(d, "c").unwrap_err();
    assert_eq!(rules(&violations), ["apply/resulting-graph"]);
    assert_eq!(violations[0].path, Path::new("specs/n/SPEC.md"));
    assert!(
        violations[0].message.contains("`nowhere`"),
        "{violations:?}"
    );
}

#[test]
fn violation_the_change_fixes() {
    // req:apply/loaded-graph@311a0b6b req:apply/atomic@bb8aa354
    let dir = repo(&[]);
    let d = dir.path();
    write(d, "specs/b/SPEC.md", &node_b("[gone]"));
    assert!(!load(d).1.is_empty());
    write(
        d,
        "changes/c/deltas/b.md",
        &frontmatter_delta(d, "b", "depends_on: []"),
    );
    apply(d, "c").unwrap();
    let (graph, violations) = load(d);
    assert!(violations.is_empty(), "{violations:?}");
    assert!(graph.nodes["b"].frontmatter.depends_on.is_empty());
}

#[test]
fn violation_the_change_leaves() {
    // req:apply/loaded-graph@311a0b6b req:apply/atomic@bb8aa354
    let dir = repo(&[]);
    let d = dir.path();
    write(d, "specs/b/SPEC.md", &node_b("[gone]"));
    write(
        d,
        "changes/c/deltas/a.md",
        &frontmatter_delta(d, "a", "optional: true"),
    );
    let before = snapshot(d, "");
    let violations = apply(d, "c").unwrap_err();
    assert_eq!(rules(&violations), ["model/dependency-dag"]);
    assert_eq!(violations[0].path, Path::new("specs/b/SPEC.md"));
    assert_eq!(snapshot(d, ""), before);
}

#[test]
fn entry_removed_with_the_rules_that_served_it() {
    // req:apply/loaded-graph@311a0b6b req:apply/requirements@b45cdf6f
    let dir = repo(&[]);
    let d = dir.path();
    let spec = read(d, "specs/a/SPEC.md").replace(
        "#### Scenario: y\n- ok\n",
        "#### Scenario: y\n- ok\n\n#### Serves\n- capability/gone\n",
    );
    write(d, "specs/a/SPEC.md", &spec);
    let base = hash(d, "a", "requirement:y");
    // `CAPABILITIES.md` no longer holds `gone`: the branch cut it with the delta below.
    write(
        d,
        "CAPABILITIES.md",
        "# Capabilities\n\n## Capabilities\n\n### Capability: kept\nA person can keep.\n\n#### Why\nW.\n\n#### Scenario: s\n- ok\n",
    );
    assert_eq!(rules(&load(d).1), ["model/serves"]);
    write(
        d,
        "changes/c/deltas/a.md",
        &format!(
            "---\ncomponent: a\n---\n## MODIFIED Requirements\n### Requirement: y <!-- base: {base} -->\nThe y SHALL be new.\n\n#### Scenario: y2\n- ok\n\n#### Serves\n- capability/kept\n"
        ),
    );
    apply(d, "c").unwrap();
    let (graph, violations) = load(d);
    assert!(violations.is_empty(), "{violations:?}");
    assert_eq!(graph.nodes["a"].requirements[1].serves, ["capability/kept"]);
}

#[test]
fn a_file_the_model_cant_parse_blocks() {
    // req:apply/loaded-graph@311a0b6b
    let dir = repo(&[]);
    let d = dir.path();
    write(d, "changes/other/change.md", "---\nstatus: nonsense\n---\n");
    write(
        d,
        "changes/c/deltas/a.md",
        &frontmatter_delta(d, "a", "optional: true"),
    );
    let before = snapshot(d, "");
    let violations = apply(d, "c").unwrap_err();
    assert_eq!(rules(&violations), ["model/parse-changes"]);
    assert!(
        violations[0].path.starts_with("changes/other"),
        "{violations:?}"
    );
    assert_eq!(snapshot(d, ""), before);
}

#[test]
fn invalid_delta_and_a_broken_graph() {
    // req:apply/loaded-graph@311a0b6b req:apply/atomic@bb8aa354
    let dir = repo(&[]);
    let d = dir.path();
    write(d, "specs/b/SPEC.md", &node_b("[gone]"));
    write(
        d,
        "changes/c/deltas/b.md",
        &frontmatter_delta(d, "b", "depends_on: []"),
    );
    // `b`'s frontmatter moves on after the delta was written, so its base is stale.
    let moved = read(d, "specs/b/SPEC.md")
        .replace("depends_on: [gone]", "depends_on: [gone]\nsuccessor: \"\"");
    write(d, "specs/b/SPEC.md", &moved);
    let before = snapshot(d, "");
    let violations = apply(d, "c").unwrap_err();
    let rules = rules(&violations);
    assert!(rules.contains(&"apply/stale-base"), "{violations:?}");
    assert!(rules.contains(&"model/dependency-dag"), "{violations:?}");
    assert_eq!(snapshot(d, ""), before);
}

/// A delta for `a` removing `y` with its base.
fn remove_y(root: &Path) -> String {
    let base = hash(root, "a", "requirement:y");
    format!(
        "---\ncomponent: a\n---\n## REMOVED Requirements\n### Requirement: y <!-- base: {base} -->\n"
    )
}

#[test]
fn resulting_graph_of_a_removal() {
    // req:apply/resulting-graph-call@3288507d
    let dir = repo(&[]);
    let d = dir.path();
    write(d, "changes/c/deltas/a.md", &remove_y(d));
    let (graph, _) = load(d);
    let before = snapshot(d, "");
    let after = clm_apply::resulting(&graph, &graph.changes["c"]).unwrap();
    assert_eq!(snapshot(d, ""), before);
    let ids: Vec<_> = after.nodes["a"]
        .requirements
        .iter()
        .map(|r| r.id.as_str())
        .collect();
    assert_eq!(ids, ["a/x", "a/z"]);
    assert_eq!(after.nodes["clm"], graph.nodes["clm"]);
    assert_eq!(after.changes, graph.changes);
    assert_eq!(after.layers, graph.layers);
    assert_eq!(after.backlog, graph.backlog);
    assert_eq!(after.decisions, graph.decisions);
    let ops = apply(d, "c").unwrap();
    let written = ops
        .iter()
        .find_map(|op| match op {
            FileOp::Write(path, bytes) if path.ends_with("a/SPEC.md") => Some(bytes.clone()),
            _ => None,
        })
        .unwrap();
    assert_eq!(after.nodes["a"].source.as_bytes(), written.as_slice());
}

#[test]
fn resulting_drops_a_removed_node() {
    // req:apply/resulting-graph-call@3288507d
    let dir = repo(&[]);
    let d = dir.path();
    write(d, "changes/c/deltas/a.md", &removal(d, "a", &[]));
    let (graph, _) = load(d);
    let after = clm_apply::resulting(&graph, &graph.changes["c"]).unwrap();
    assert!(!after.nodes.contains_key("a"));
    assert!(after.nodes.contains_key("clm"));
    assert!(d.join("specs/a/SPEC.md").exists());
}

#[test]
fn resulting_is_none_for_a_stale_delta() {
    // req:apply/resulting-graph-call@3288507d
    let dir = repo(&[]);
    let d = dir.path();
    write(
        d,
        "changes/c/deltas/a.md",
        "---\ncomponent: a\n---\n## REMOVED Requirements\n### Requirement: y <!-- base: 1111111111111111 -->\n",
    );
    let (graph, _) = load(d);
    assert_eq!(clm_apply::resulting(&graph, &graph.changes["c"]), None);
}

#[test]
fn resulting_leaves_out_the_items_apply_deletes() {
    // req:apply/resulting-graph-call@3288507d
    let dir = repo(&[]);
    let d = dir.path();
    item(d, "q", "question");
    item(d, "b", "bug");
    item(d, "h", "human");
    item(d, "i", "idea");
    promoted(d, "backlog/i", &["q", "b", "h"]);
    valid_delta(d);
    let (graph, _) = load(d);
    let before = snapshot(d, "");
    let after = clm_apply::resulting(&graph, &graph.changes["c"]).unwrap();
    let kept: Vec<_> = after.backlog.keys().map(String::as_str).collect();
    assert_eq!(kept, ["b", "h"]);
    assert_eq!(graph.backlog.len(), 4);
    assert_eq!(snapshot(d, ""), before);
}

#[test]
fn resulting_keeps_a_promoted_human_task() {
    // req:apply/resulting-graph-call@3288507d
    let dir = repo(&[]);
    let d = dir.path();
    item(d, "h", "human");
    promoted(d, "backlog/h", &[]);
    valid_delta(d);
    let (graph, _) = load(d);
    let after = clm_apply::resulting(&graph, &graph.changes["c"]).unwrap();
    assert!(after.backlog.contains_key("h"));
}

#[test]
fn resulting_ignores_the_change_status() {
    // req:apply/resulting-graph-call@3288507d
    let dir = repo(&[]);
    let d = dir.path();
    write(d, "changes/c/change.md", &change_md("applied", "[]"));
    write(d, "changes/c/deltas/a.md", &remove_y(d));
    let (graph, _) = load(d);
    let after = clm_apply::resulting(&graph, &graph.changes["c"]).unwrap();
    assert_eq!(after.nodes["a"].requirements.len(), 2);
}

#[test]
fn refused_status_and_a_broken_graph() {
    // req:apply/loaded-graph@311a0b6b
    let dir = repo(&[]);
    let d = dir.path();
    write(d, "specs/b/SPEC.md", &node_b("[gone]"));
    write(d, "changes/c/change.md", &change_md("applied", "[]"));
    write(
        d,
        "changes/c/deltas/b.md",
        &frontmatter_delta(d, "b", "depends_on: []"),
    );
    let before = snapshot(d, "");
    let violations = apply(d, "c").unwrap_err();
    assert_eq!(
        rules(&violations),
        ["apply/change-status", "model/dependency-dag"]
    );
    assert_eq!(snapshot(d, ""), before);
}

#[test]
fn stale_bases_names_what_validate_calls_stale() {
    // req:apply/stale-elements@94ee9040
    let dir = repo_with_serves();
    let d = dir.path();
    let (bx, by) = (hash(d, "a", "requirement:x"), hash(d, "a", "requirement:y"));
    let block = |s: &str, base: &str| {
        format!(
            "### Requirement: {s} <!-- base: {base} -->\nThe {s} SHALL be new.\n\n#### Scenario: n\n- ok\n"
        )
    };
    write(
        d,
        "changes/c/deltas/a.md",
        &format!(
            "---\ncomponent: a\n---\n## MODIFIED Requirements\n{}\n{}",
            block("y", &by),
            block("x", &bx)
        ),
    );
    // Only y's `#### Serves` is edited (a version-only base stays valid); x loses a line.
    let spec = read(d, "specs/a/SPEC.md")
        .replace("- capability/c\n", "- capability/d\n")
        .replace("The x SHALL hold.", "The x SHALL differ.");
    write(d, "specs/a/SPEC.md", &spec);
    let (graph, _) = load(d);
    let delta = &graph.changes["c"].deltas[0];
    assert_eq!(
        stale_bases(graph.nodes.get("a"), delta),
        [("requirement:x".to_string(), bx)]
    );
    let violations = validate(&graph, &graph.changes["c"]);
    assert_eq!(rules(&violations), ["apply/stale-base"], "{violations:?}");
    assert!(violations[0].message.contains("a/x"), "{violations:?}");
}

#[test]
fn stale_bases_orders_held_before_lacking_and_skips_untouched() {
    // req:apply/stale-elements@94ee9040
    let dir = repo_with_serves();
    let d = dir.path();
    // z (removed, stale) is listed before x (modified with own Serves, stale) and w (absent);
    // the frontmatter base and the unmodified Contract's base are left out.
    write(
        d,
        "changes/c/deltas/a.md",
        "---\ncomponent: a\nbase:\n  frontmatter: f1\n  contract: c1\n  design: d1\n---\n## MODIFIED Design\nnew\n\n## MODIFIED Requirements\n### Requirement: w <!-- base: w1 -->\nThe w SHALL.\n\n#### Scenario: n\n- ok\n\n### Requirement: x <!-- base: x1 -->\nThe x SHALL.\n\n#### Scenario: n\n- ok\n\n#### Serves\n- capability/c\n\n## REMOVED Requirements\n### Requirement: z <!-- base: z1 -->\n",
    );
    let (graph, _) = load(d);
    let delta = &graph.changes["c"].deltas[0];
    let names: Vec<_> = stale_bases(graph.nodes.get("a"), delta)
        .into_iter()
        .map(|(e, _)| e)
        .collect();
    assert_eq!(
        names,
        ["design", "requirement:x", "requirement:z", "requirement:w"]
    );
}

#[test]
fn stale_bases_of_a_gone_node_lists_every_element() {
    // req:apply/stale-elements@94ee9040
    let dir = repo(&[]);
    let d = dir.path();
    write(
        d,
        "changes/c/deltas/a.md",
        "---\ncomponent: a\nbase:\n  design: h1\n---\n## MODIFIED Design\nnew\n\n## REMOVED Requirements\n### Requirement: z <!-- base: h2 -->\n",
    );
    let (graph, _) = load(d);
    let delta = &graph.changes["c"].deltas[0];
    assert_eq!(
        stale_bases(None, delta),
        [
            ("design".to_string(), "h1".to_string()),
            ("requirement:z".to_string(), "h2".to_string())
        ]
    );
}

fn item(d: &Path, slug: &str, kind: &str) {
    write(
        d,
        &format!("backlog/{slug}.md"),
        &format!("---\ntitle: T\nkind: {kind}\ncomponents: [a]\n---\n\nbody\n"),
    );
}

fn resolving(d: &Path, slugs: &[&str]) {
    write(
        d,
        "changes/c/change.md",
        &format!(
            "---\nstatus: proposed\nresolves: [{}]\n---\n\n# c\n",
            slugs.join(", ")
        ),
    );
}

fn valid_delta(d: &Path) {
    write(
        d,
        "changes/c/deltas/a.md",
        &format!(
            "---\ncomponent: a\n---\n## ADDED Requirements\n{}",
            req_block("new", "New.")
        ),
    );
}

#[test]
fn answered_question_is_deleted() {
    // req:apply/answered-questions@e2932b53
    let dir = repo(&[]);
    let d = dir.path();
    item(d, "q", "question");
    resolving(d, &["q"]);
    valid_delta(d);
    let ops = apply(d, "c").unwrap();
    assert!(!d.join("backlog/q.md").exists());
    assert!(ops.contains(&FileOp::Remove("backlog/q.md".into())));
}

#[test]
fn other_kinds_stay() {
    // req:apply/answered-questions@e2932b53
    let dir = repo(&[]);
    let d = dir.path();
    item(d, "b", "bug");
    item(d, "h", "human");
    resolving(d, &["b", "h"]);
    valid_delta(d);
    apply(d, "c").unwrap();
    assert!(d.join("backlog/b.md").exists() && d.join("backlog/h.md").exists());
}

#[test]
fn refused_change_keeps_its_question() {
    // req:apply/answered-questions@e2932b53
    let dir = repo(&[]);
    let d = dir.path();
    item(d, "q", "question");
    resolving(d, &["q"]);
    write(
        d,
        "changes/c/deltas/clm.md",
        "---\ncomponent: clm\nbase: { contract: \"0000000000000000\" }\n---\n## MODIFIED Contract\nnew root\n",
    );
    apply(d, "c").unwrap_err();
    assert!(d.join("backlog/q.md").exists());
}

#[test]
fn already_deleted_question_is_no_violation() {
    // req:apply/answered-questions@e2932b53
    let dir = repo(&[]);
    let d = dir.path();
    resolving(d, &["q"]);
    valid_delta(d);
    let ops = apply(d, "c").unwrap();
    assert!(
        !ops.iter()
            .any(|o| matches!(o, FileOp::Remove(p) if p.starts_with("backlog")))
    );
}

fn promoted(d: &Path, from: &str, resolves: &[&str]) {
    write(
        d,
        "changes/c/change.md",
        &format!(
            "---\nstatus: proposed\nfrom: {from}\nresolves: [{}]\n---\n\n# c\n",
            resolves.join(", ")
        ),
    );
}

fn removals(ops: &[FileOp], path: &str) -> usize {
    ops.iter()
        .filter(|o| **o == FileOp::Remove(path.into()))
        .count()
}

#[test]
fn promoted_idea_is_deleted() {
    // req:apply/promoted-item@5992a1d8
    let dir = repo(&[]);
    let d = dir.path();
    item(d, "i", "idea");
    promoted(d, "backlog/i", &[]);
    valid_delta(d);
    let ops = apply(d, "c").unwrap();
    assert!(!d.join("backlog/i.md").exists());
    assert_eq!(removals(&ops, "backlog/i.md"), 1);
}

#[test]
fn promoted_item_of_any_kind_is_deleted() {
    // req:apply/promoted-item@5992a1d8
    let dir = repo(&[]);
    let d = dir.path();
    item(d, "b", "bug");
    promoted(d, "backlog/b", &[]);
    valid_delta(d);
    apply(d, "c").unwrap();
    assert!(!d.join("backlog/b.md").exists());
}

#[test]
fn promoted_human_task_stays() {
    // req:apply/promoted-item@5992a1d8
    let dir = repo(&[]);
    let d = dir.path();
    item(d, "h", "human-task");
    promoted(d, "backlog/h", &[]);
    valid_delta(d);
    apply(d, "c").unwrap();
    assert!(d.join("backlog/h.md").exists());
}

#[test]
fn refused_change_keeps_its_promoted_item() {
    // req:apply/promoted-item@5992a1d8
    let dir = repo(&[]);
    let d = dir.path();
    item(d, "i", "idea");
    promoted(d, "backlog/i", &[]);
    write(
        d,
        "changes/c/deltas/clm.md",
        "---\ncomponent: clm\nbase: { contract: \"0000000000000000\" }\n---\n## MODIFIED Contract\nnew root\n",
    );
    apply(d, "c").unwrap_err();
    assert!(d.join("backlog/i.md").exists());
}

#[test]
fn already_deleted_promoted_item_is_no_violation() {
    // req:apply/promoted-item@5992a1d8
    let dir = repo(&[]);
    let d = dir.path();
    promoted(d, "backlog/i", &[]);
    valid_delta(d);
    let ops = apply(d, "c").unwrap();
    assert!(
        !ops.iter()
            .any(|o| matches!(o, FileOp::Remove(p) if p.starts_with("backlog")))
    );
}

#[test]
fn from_without_the_backlog_prefix_deletes_nothing() {
    // req:apply/promoted-item@5992a1d8
    let dir = repo(&[]);
    let d = dir.path();
    item(d, "i", "idea");
    promoted(d, "i", &[]);
    valid_delta(d);
    apply(d, "c").unwrap();
    assert!(d.join("backlog/i.md").exists());
}

#[test]
fn item_named_in_from_and_resolves_is_removed_once() {
    // req:apply/promoted-item@5992a1d8
    let dir = repo(&[]);
    let d = dir.path();
    item(d, "q", "question");
    promoted(d, "backlog/q", &["q"]);
    valid_delta(d);
    let ops = apply(d, "c").unwrap();
    assert!(!d.join("backlog/q.md").exists());
    assert_eq!(removals(&ops, "backlog/q.md"), 1);
}

#[test]
fn workaround_flag_on_a_rewritten_requirement() {
    // req:apply/resulting-graph@b3caad6a
    let dir = repo(&[]);
    let d = dir.path();
    let version = {
        let (graph, _) = load(d);
        graph.requirement("a/x").unwrap().version().to_string()
    };
    write(
        d,
        "backlog/workaround-a-x-1.md",
        &format!(
            "---\ntitle: Hard\nkind: workaround\ncomponents: [a]\nrequirement: a/x@{version}\n---\n\nHard.\n"
        ),
    );
    let base = hash(d, "a", "requirement:x");
    write(
        d,
        "changes/c/deltas/a.md",
        &format!(
            "---\ncomponent: a\n---\n## MODIFIED Requirements\n### Requirement: x <!-- base: {base} -->\nThe x SHALL be new.\n\n#### Scenario: x2\n- ok\n"
        ),
    );
    assert_check_clean(d);
    let (graph, _) = load(d);
    assert_eq!(validate(&graph, &graph.changes["c"]), []);
    let kept = clm_apply::resulting(&graph, &graph.changes["c"]).unwrap();
    assert_eq!(kept.backlog.len(), graph.backlog.len());
    apply(d, "c").unwrap();
    assert!(read(d, "specs/a/SPEC.md").contains("The x SHALL be new."));
    let (graph, after) = load(d);
    assert_eq!(rules(&after), ["model/parse-backlog"]);
    assert_eq!(
        after[0].path,
        Path::new("backlog/workaround-a-x-1.md"),
        "{after:?}"
    );
    assert!(
        after[0]
            .message
            .contains(graph.requirement("a/x").unwrap().version()),
        "{after:?}"
    );
}
