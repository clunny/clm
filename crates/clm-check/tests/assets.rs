//! `asset-references`, through `clm_check::run`.

use std::fs;
use std::path::Path;

use clm_check::run;
use tempfile::TempDir;

const ROOT: &str = "---\ntitle: R\nstatus: active\nowners: []\n---\n";

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

/// Node `a` whose `SPEC.md` body (after its frontmatter) is `body`.
fn node(body: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "specs/SPEC.md", ROOT);
    write(
        dir.path(),
        "specs/a/SPEC.md",
        &format!("---\ntitle: A\nstatus: active\nowners: []\n---\n\n{body}\n"),
    );
    dir
}

/// Node `a` whose Design is `design`.
fn repo(design: &str) -> TempDir {
    node(&format!("## Design\n\n{design}"))
}

/// Each violation `run` reports, formatted, after checking they are all
/// `check/asset-references` (so a broken fixture can't pass a test that expects none).
fn found(root: &Path) -> Vec<String> {
    let found: Vec<String> = run(root).iter().map(clm_check::format).collect();
    assert!(
        found
            .iter()
            .all(|v| v.contains(": check/asset-references: ")),
        "{found:?}"
    );
    found
}

// req:check/asset-references@3f70faba
#[test]
fn missing_mockup_is_reported() {
    let dir = repo("See `assets/m.html`, and again `assets/m.html`.");
    assert_eq!(
        found(dir.path()),
        [
            "specs/a/SPEC.md: check/asset-references: component `a` mentions `assets/m.html`, which isn't in specs/a/assets/"
        ]
    );
}

// req:check/asset-references@3f70faba
#[test]
fn contract_mention_is_reported_once_with_design() {
    let dir = node("## Contract\n\nSee `assets/m.html`.\n\n## Design\n\nAlso `assets/m.html`.");
    let found = found(dir.path());
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("`assets/m.html`"), "{found:?}");
}

// req:check/asset-references@3f70faba
#[test]
fn present_asset_passes() {
    let dir = repo("See `assets/mockups/m.html`.");
    write(dir.path(), "specs/a/assets/mockups/m.html", "<p>m</p>\n");
    assert_eq!(found(dir.path()), Vec::<String>::new());
}

// req:check/asset-references@3f70faba
#[test]
fn placeholders_and_directories_pass() {
    let dir = node(
        "## Contract\n\nAssets live at `assets/<path>`, `assets/<node-id>/**`, `assets/mockups/*.html`, `assets/mockups/` and `assets/`.",
    );
    assert_eq!(found(dir.path()), Vec::<String>::new());
}

// req:check/asset-references@3f70faba
#[test]
fn scenario_examples_pass() {
    let dir = node(
        "## Design\n\nNo assets.\n\n## Requirements\n\n### Requirement: x\nIt SHALL.\n\n#### Scenario: s\n- GIVEN `assets/m.html`\n- THEN ok\n",
    );
    assert_eq!(found(dir.path()), Vec::<String>::new());
}

// req:check/asset-references@3f70faba
#[test]
fn requirement_text_and_frontmatter_pass() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "specs/SPEC.md", ROOT);
    write(
        dir.path(),
        "specs/a/SPEC.md",
        "---\ntitle: \"A `assets/f.html`\"\nstatus: active\nowners: []\n---\n\n## Requirements\n\n### Requirement: x\nIt SHALL keep `assets/r.html`.\n\n#### Scenario: s\n- ok\n",
    );
    assert_eq!(found(dir.path()), Vec::<String>::new());
}

// req:check/asset-references@3f70faba
#[test]
fn fenced_code_is_not_a_code_span() {
    let dir = repo(
        "```markdown\nSee `assets/a.html`.\n\nAnd `assets/b.html`.\n```\n\n~~~\n`assets/c.html`\n~~~\n\nThen `assets/m.html`.",
    );
    let found = found(dir.path());
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("`assets/m.html`"), "{found:?}");
}

#[test]
fn stray_backticks_do_not_pair_across_paragraphs() {
    let dir =
        repo("A lone ` here.\n\nThen `assets/m.html`.\n\nAn escaped \\` and `assets/n.html`.");
    let found = found(dir.path());
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(found[0].contains("`assets/m.html`"), "{found:?}");
    assert!(found[1].contains("`assets/n.html`"), "{found:?}");
}

#[test]
fn backslash_does_not_escape_inside_a_span() {
    let dir = repo("Use `C:\\` then see `assets/m.html`.");
    let found = found(dir.path());
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("`assets/m.html`"), "{found:?}");
}

#[test]
fn backslash_escapes_one_backtick_of_a_run() {
    let dir = repo("a \\`` b `assets/x.html` c``.\n\nAnd \\\\`assets/y.html`.");
    let found = found(dir.path());
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("`assets/y.html`"), "{found:?}");
}

#[test]
fn shorter_fence_does_not_close_a_longer_one() {
    let dir = repo(
        "````markdown\n```text\n`assets/a.html`\n```\n````\n\n~~~\n~~~ not a close\n`assets/b.html`\n~~~\n\nThen `assets/m.html`.",
    );
    let found = found(dir.path());
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("`assets/m.html`"), "{found:?}");
}

#[test]
fn span_padding_is_one_space_each_side() {
    let dir = repo("`` assets/m.html `` and ` assets/n.html`.");
    let found = found(dir.path());
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("`assets/m.html`"), "{found:?}");
}

#[test]
fn only_spans_starting_with_assets_count() {
    let dir = repo("Not `x/assets/m.html`, `assets` or assets/m.html without backticks.");
    assert_eq!(found(dir.path()), Vec::<String>::new());
}

// req:check/says-component@8d7d2aad
#[test]
fn asset_that_isnt_there_says_component() {
    let dir = repo("See `assets/mockups/m.html`.");
    let found = found(dir.path());
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].contains("component `a` mentions `assets/mockups/m.html`"),
        "{found:?}"
    );
}
