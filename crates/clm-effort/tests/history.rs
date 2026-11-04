//! What only git knows (`effort/history`).

#[path = "../../clm-ledger/tests/common/mod.rs"]
mod common;

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

use clm_effort::{Inputs, history, page_text, report};
use clm_model::BacklogKind;
use common::{commit, repo, sh};

/// Commits everything at commit time `at`, returning its hash.
fn commit_at(dir: &Path, at: i64, message: &str) -> String {
    sh(dir, &["add", "-A"]);
    let out = Command::new("git")
        .args(["commit", "-q", "--allow-empty", "-m", message])
        .current_dir(dir)
        .env("HOME", dir)
        .env("GIT_COMMITTER_DATE", format!("@{at} +0000"))
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    sh(dir, &["rev-parse", "HEAD"])
}

fn write(dir: &Path, path: &str, text: &str) {
    let path = dir.join(path);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn node(serves: &str, paths: &str) -> String {
    format!(
        "---\ntitle: T\nstatus: active\nowners: []\npaths: [\"{paths}\"]\nserves: [{serves}]\n---\n\n## Contract\nc\n\n## Requirements\n\n### Requirement: x\nIt SHALL.\n\n#### Scenario: s\n- ok\n"
    )
}

const DAY: i64 = 86_400;

/// Nodes `a` and `b` serving `capability/p`, commits on each one's paths, and a `question`
/// item blocking `a/x`, added on day 1 and deleted on day 4.
fn repository(dir: &Path) -> (String, String) {
    repo(dir, "Ann");
    commit(
        dir,
        &[
            (
                "specs/SPEC.md",
                "---\ntitle: T\nstatus: active\nowners: []\n---\n\n## Contract\nc\n",
            ),
            (
                "CAPABILITIES.md",
                "## Capabilities\n\n### Capability: p\nA person can p.\n\n#### Why\nw\n\n#### Scenario: s\n- ok\n",
            ),
            ("specs/a/SPEC.md", &node("capability/p", "crates/a/**")),
            ("specs/b/SPEC.md", &node("capability/p", "crates/b/**")),
        ],
        "specs",
    );
    write(dir, "crates/a/src/x.rs", "fn a() {}\n");
    let on_a = commit_at(dir, DAY / 2, "a");
    write(
        dir,
        "backlog/q1.md",
        "---\ntitle: Which?\nkind: question\nnodes: [a]\nblocks: [a/x]\n---\n\nWhich one?\n",
    );
    commit_at(dir, DAY, "ask");
    write(dir, "crates/b/src/y.rs", "fn b() {}\n");
    let on_b = commit_at(dir, 2 * DAY, "b");
    std::fs::remove_file(dir.join("backlog/q1.md")).unwrap();
    commit_at(dir, 4 * DAY, "answered");
    (on_a, on_b)
}

#[test]
fn history_counts_commits_and_items() {
    // req:effort/history@809c694b
    let dir = tempfile::tempdir().unwrap();
    let (on_a, on_b) = repository(dir.path());
    let history = history(dir.path()).unwrap();
    // `a`'s commits: the one adding its `SPEC.md`, and its code's.
    let specs = sh(dir.path(), &["rev-parse", "HEAD~4"]);
    let a: Vec<&String> = history.commits["a"].iter().collect();
    let mut want = vec![&specs, &on_a];
    want.sort();
    assert_eq!(a, want);
    assert!(history.commits["b"].contains(&on_b) && !history.commits["b"].contains(&on_a));
    assert_eq!(history.items.len(), 1);
    let item = &history.items[0];
    assert_eq!(
        (item.slug.as_str(), item.kind, item.added, item.deleted),
        ("q1", BacklogKind::Question, DAY, Some(4 * DAY))
    );
    assert_eq!(
        (item.nodes.clone(), item.blocks.clone()),
        (vec!["a".into()], vec!["a/x".into()])
    );
    assert!(history.sizes["a"] > 0 && history.now > 4 * DAY);
}

#[test]
fn page_with_history() {
    // req:effort/history@809c694b
    let dir = tempfile::tempdir().unwrap();
    repository(dir.path());
    let (graph, _) = clm_model::load(dir.path());
    let history = history(dir.path()).unwrap();
    let fold = clm_ledger::fold_records(&[]);
    let report = report(&Inputs {
        graph: &graph,
        fold: &fold,
        history: Some(&history),
        flag_text: &BTreeMap::new(),
    });
    let p = &report.capabilities[0];
    assert_eq!(p.commits, Some(3));
    assert_eq!(p.questions_answered, Some(1));
    assert_eq!(p.blocking, Some((1, 3)));
    let tokens = history.sizes["a"] + history.sizes["b"];
    assert_eq!(p.size_tokens, Some(tokens as f64));
    let page = page_text(p);
    assert!(
        page.contains(&"  3 commits · 0 changes".to_string()),
        "{page:?}"
    );
    assert!(
        page.contains(
            &"  your attention: 0 approvals · 1 questions answered · 0 flags settled".to_string()
        ),
        "{page:?}"
    );
    assert!(
        page.contains(&"  blocking: 1 items held its rules up for 3 days".to_string()),
        "{page:?}"
    );
    assert!(!page.iter().any(|l| l.contains(" - ")), "{page:?}");
}

#[test]
fn blocks_added_later_and_odd_lines_kept() {
    // req:effort/history@809c694b
    // An item added blocking nothing, whose body has a line a patch writes as `+++ ...`,
    // later edited to block `a/x`; and a friction item, whose added date the page shows.
    let dir = tempfile::tempdir().unwrap();
    repository(dir.path());
    let root = dir.path();
    write(
        root,
        "backlog/q2.md",
        "---\ntitle: Later\nkind: human\ncomponents: []\n---\n\n++ not a header\n-- nor this\n",
    );
    commit_at(root, 5 * DAY, "q2");
    write(
        root,
        "backlog/q2.md",
        "---\ntitle: Later\nkind: human\ncomponents: []\nblocks: [a/x]\n---\n\n++ not a header\n",
    );
    commit_at(root, 6 * DAY, "q2 blocks");
    let (graph, _) = clm_model::load(root);
    let version = graph.requirement("a/x").unwrap().version().to_string();
    write(
        root,
        "backlog/fr.md",
        &format!(
            "---\ntitle: Hard\nkind: friction\ncomponents: [a]\nrequirement: a/x@{version}\n---\n\nWhy.\n"
        ),
    );
    commit_at(root, 7 * DAY, "flag");
    let history = history(root).unwrap();
    let q2 = history.items.iter().find(|l| l.slug == "q2").unwrap();
    assert_eq!(
        (q2.kind, q2.added, q2.deleted),
        (BacklogKind::Human, 5 * DAY, None)
    );
    assert_eq!(q2.blocks, ["a/x"]);
    let report = clm_effort::capabilities(root).unwrap();
    let p = &report.capabilities[0];
    assert_eq!(p.blocking.map(|b| b.0), Some(2));
    let page = page_text(p);
    assert!(
        page.contains(&"  ⚑ fr a/x Hard (added 1970-01-08)".to_string()),
        "{page:?}"
    );
    assert!(page.contains(&"    Why.".to_string()), "{page:?}");
}

#[test]
fn unborn_head_has_no_history() {
    // req:effort/history@809c694b
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    sh(root, &["init", "-q", "-b", "main"]);
    write(
        root,
        "specs/SPEC.md",
        "---\ntitle: T\nstatus: active\nowners: []\n---\n\n## Contract\nc\n",
    );
    let history = history(root).unwrap();
    assert!(history.commits.values().all(|c| c.is_empty()) && history.items.is_empty());
}
