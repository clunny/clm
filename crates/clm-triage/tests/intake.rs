//! `clm intake` against a bare `origin`, a clone and a fake host.

#[path = "../../clm-orchestrator/tests/common/mod.rs"]
mod common;

use std::fs;
use std::path::Path;

use clm_host::{Alert, Error as HostError, Host, Listed};
use clm_triage::{Error, intake};
use common::*;
use tempfile::TempDir;

/// A host that lists `open` and files nothing.
struct Fake {
    open: Result<Vec<Listed>, String>,
    alerts: Result<Vec<Alert>, String>,
}

impl Host for Fake {
    fn page_open(&self, _: &Path, _: &str, _: &str, _: &Path) -> Result<Option<String>, HostError> {
        Ok(None)
    }
    fn page_body(&self, _: &Path, _: &Path) -> Result<Option<String>, HostError> {
        Ok(None)
    }
    fn issue(&self, _: &Path, _: &str, _: &Path, _: &str) -> Result<String, HostError> {
        Err(HostError::Nowhere(String::new()))
    }
    fn list(&self, _: &Path) -> Result<Vec<Listed>, HostError> {
        self.open.clone().map_err(HostError::Git)
    }
    fn alerts(&self, _: &Path) -> Result<Vec<Alert>, HostError> {
        self.alerts.clone().map_err(HostError::Git)
    }
}

fn alert(number: u64) -> Alert {
    Alert {
        number,
        severity: "high".into(),
        ecosystem: "npm".into(),
        package: "vite".into(),
        manifest: "web/package-lock.json".into(),
    }
}

fn issue(number: u64) -> Listed {
    Listed {
        number,
        pull_request: false,
        author: "someone".into(),
    }
}

fn pull(number: u64, author: &str) -> Listed {
    Listed {
        number,
        pull_request: true,
        author: author.into(),
    }
}

fn host(open: Vec<Listed>) -> Fake {
    Fake {
        open: Ok(open),
        alerts: Ok(Vec::new()),
    }
}

fn lines(r: &Repo, open: Vec<Listed>) -> Vec<String> {
    intake(&r.work, &host(open)).unwrap()
}

#[test]
fn scenario_new_imported_and_a_pull_request() {
    // req:orchestrator.triage/intake@5fda2cc9
    let r = repo();
    write(
        &r.work,
        "backlog/port-clash.md",
        "---\ntitle: T\nkind: bug\nissues: [73]\n---\nBody.\n",
    );
    push_main(&r.work, "item");
    assert_eq!(
        lines(&r, vec![issue(92), pull(80, "dana"), issue(73)]),
        [
            "issue 73 imported port-clash",
            "pull 80 dana",
            "issue 92 new"
        ]
    );
}

#[test]
fn imported_names_every_item_sorted() {
    // req:orchestrator.triage/intake@5fda2cc9
    let r = repo();
    for slug in ["zeta", "alpha"] {
        write(
            &r.work,
            &format!("backlog/{slug}.md"),
            "---\ntitle: T\nkind: bug\nissues: [4]\n---\n",
        );
    }
    push_main(&r.work, "items");
    assert_eq!(lines(&r, vec![issue(4)]), ["issue 4 imported alpha zeta"]);
}

#[test]
fn scenario_resolved_by_a_change() {
    // req:orchestrator.triage/intake@5fda2cc9
    let r = repo();
    write(
        &r.work,
        "backlog/fold-declined.md",
        "---\ntitle: T\nkind: bug\nissues: [67]\n---\n",
    );
    push_main(&r.work, "add");
    fs::remove_file(r.work.join("backlog/fold-declined.md")).unwrap();
    write(
        &r.work,
        "changes/archive/20260101-c/change.md",
        &change_md("done", "resolves: [fold-declined]\n"),
    );
    write(
        &r.work,
        "changes/archive/20260301-d/change.md",
        &change_md("done", "from: backlog/fold-declined\n"),
    );
    write(
        &r.work,
        "changes/archive/20260401-e/change.md",
        &change_md("done", "resolves: [other]\n"),
    );
    push_main(&r.work, "delete");
    assert_eq!(
        lines(&r, vec![issue(67)]),
        ["issue 67 resolved fold-declined 20260301-d"]
    );
}

#[test]
fn resolved_by_a_live_change_naming_it_in_resolves_only() {
    // req:orchestrator.triage/intake@5fda2cc9
    let r = repo();
    write(
        &r.work,
        "backlog/fold-declined.md",
        "---\ntitle: T\nkind: bug\nissues: [67]\n---\n",
    );
    push_main(&r.work, "add");
    fs::remove_file(r.work.join("backlog/fold-declined.md")).unwrap();
    write(
        &r.work,
        "changes/20260101-c/change.md",
        &change_md("applied", "resolves: [fold-declined]\n"),
    );
    push_main(&r.work, "delete");
    assert_eq!(
        lines(&r, vec![issue(67)]),
        ["issue 67 resolved fold-declined 20260101-c"]
    );
}

#[test]
fn scenario_resolved_by_a_code_fix() {
    // req:orchestrator.triage/intake@5fda2cc9
    let r = repo();
    write(
        &r.work,
        "backlog/detector-missing.md",
        "---\ntitle: T\nkind: bug\nnodes: [gone]\nissues: [64]\n---\n",
    );
    push_main(&r.work, "add");
    fs::remove_file(r.work.join("backlog/detector-missing.md")).unwrap();
    push_main(&r.work, "delete");
    let sha = git(&r.work, &["rev-parse", "--short", "HEAD"]);
    assert_eq!(
        lines(&r, vec![issue(64)]),
        [format!("issue 64 resolved detector-missing {}", sha.trim())]
    );
}

#[test]
fn most_recent_deletion_wins_and_unparsable_ones_are_skipped() {
    // req:orchestrator.triage/intake@5fda2cc9
    let r = repo();
    for (slug, text) in [
        ("old", "---\ntitle: T\nkind: bug\nissues: [9]\n---\n"),
        ("new", "---\ntitle: T\nkind: bug\nissues: [9]\n---\n"),
        ("broken", "not frontmatter\n"),
    ] {
        write(&r.work, &format!("backlog/{slug}.md"), text);
        push_main(&r.work, slug);
        fs::remove_file(r.work.join(format!("backlog/{slug}.md"))).unwrap();
        push_main(&r.work, &format!("delete {slug}"));
    }
    let out = lines(&r, vec![issue(9)]);
    assert_eq!(out.len(), 1);
    assert!(out[0].starts_with("issue 9 resolved new "), "{out:?}");
}

#[test]
fn scenario_no_strangers_text() {
    // req:orchestrator.triage/intake@5fda2cc9
    let r = repo();
    // `Listed` holds a number, a kind and a login: a stranger's title or body has nowhere
    // to go, and the lines are built from those fields alone.
    let out = lines(&r, vec![issue(5)]);
    assert_eq!(out, ["issue 5 new"]);
}

#[test]
fn scenario_no_host() {
    // req:orchestrator.triage/intake@5fda2cc9
    let r = repo();
    let out = lines(&r, vec![]);
    assert!(out.is_empty());
}

#[test]
fn a_failing_list_is_refused_with_its_reason() {
    // req:orchestrator.triage/intake@5fda2cc9
    let r = repo();
    let fake = Fake {
        open: Err("gh: not logged in".into()),
        alerts: Ok(Vec::new()),
    };
    match intake(&r.work, &fake) {
        Err(Error::Refused(why)) => assert!(why.contains("not logged in"), "{why}"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn git_failure_is_a_git_error() {
    // req:orchestrator.triage/intake@5fda2cc9
    let dir = TempDir::new().unwrap();
    assert!(matches!(
        intake(dir.path(), &host(vec![issue(1)])),
        Err(Error::Git(_))
    ));
}

fn with_alerts(r: &Repo, open: Vec<Listed>, alerts: Result<Vec<Alert>, String>) -> Vec<String> {
    let fake = Fake {
        open: Ok(open),
        alerts,
    };
    intake(&r.work, &fake).unwrap()
}

#[test]
fn scenario_alerts_after_the_inbox() {
    // req:orchestrator.triage/intake@5fda2cc9
    let r = repo();
    write(
        &r.work,
        "backlog/bump-x.md",
        "---\ntitle: T\nkind: bug\nalerts: [2]\n---\n",
    );
    push_main(&r.work, "item");
    assert_eq!(
        with_alerts(&r, vec![issue(92)], Ok(vec![alert(4), alert(2)])),
        [
            "issue 92 new",
            "alert 2 imported bump-x",
            "alert 4 new high npm vite web/package-lock.json"
        ]
    );
}

#[test]
fn scenario_alert_resolved_by_a_code_fix() {
    // req:orchestrator.triage/intake@5fda2cc9
    let r = repo();
    write(
        &r.work,
        "backlog/bump-y.md",
        "---\ntitle: T\nkind: bug\nalerts: [7]\n---\n",
    );
    push_main(&r.work, "add");
    fs::remove_file(r.work.join("backlog/bump-y.md")).unwrap();
    push_main(&r.work, "delete");
    let sha = git(&r.work, &["rev-parse", "--short", "HEAD"]);
    assert_eq!(
        with_alerts(&r, vec![], Ok(vec![alert(7)])),
        [format!("alert 7 resolved bump-y {}", sha.trim())]
    );
}

#[test]
fn an_alert_resolved_by_a_change_names_that_change() {
    // req:orchestrator.triage/intake@5fda2cc9
    let r = repo();
    write(
        &r.work,
        "backlog/bump-z.md",
        "---\ntitle: T\nkind: bug\nalerts: [8]\n---\n",
    );
    push_main(&r.work, "add");
    fs::remove_file(r.work.join("backlog/bump-z.md")).unwrap();
    write(
        &r.work,
        "changes/archive/20260101-c/change.md",
        &change_md("done", "resolves: [bump-z]\n"),
    );
    push_main(&r.work, "delete");
    assert_eq!(
        with_alerts(&r, vec![], Ok(vec![alert(8)])),
        ["alert 8 resolved bump-z 20260101-c"]
    );
}

#[test]
fn issue_and_alert_numbers_never_match_each_others_items() {
    // req:orchestrator.triage/intake@5fda2cc9
    let r = repo();
    write(
        &r.work,
        "backlog/by-issue.md",
        "---\ntitle: T\nkind: bug\nissues: [3]\n---\n",
    );
    write(
        &r.work,
        "backlog/by-alert.md",
        "---\ntitle: T\nkind: bug\nalerts: [5]\n---\n",
    );
    write(
        &r.work,
        "backlog/gone-by-issue.md",
        "---\ntitle: T\nkind: bug\nissues: [6]\n---\n",
    );
    push_main(&r.work, "items");
    fs::remove_file(r.work.join("backlog/gone-by-issue.md")).unwrap();
    push_main(&r.work, "delete");
    let sha = git(&r.work, &["rev-parse", "--short", "HEAD"]);
    assert_eq!(
        with_alerts(
            &r,
            vec![issue(5), issue(6), issue(2)],
            Ok(vec![alert(3), alert(6)])
        ),
        [
            "issue 2 new".to_string(),
            "issue 5 new".to_string(),
            format!("issue 6 resolved gone-by-issue {}", sha.trim()),
            "alert 3 new high npm vite web/package-lock.json".to_string(),
            "alert 6 new high npm vite web/package-lock.json".to_string()
        ]
    );
}

#[test]
fn scenario_alerts_refused() {
    // req:orchestrator.triage/intake@5fda2cc9
    let r = repo();
    assert_eq!(
        with_alerts(&r, vec![issue(92)], Err("gh: 403".into())),
        ["issue 92 new", "alerts unavailable"]
    );
}

#[test]
fn alerts_without_issues_and_a_refusal_without_issues() {
    // req:orchestrator.triage/intake@5fda2cc9
    let r = repo();
    assert_eq!(
        with_alerts(&r, vec![], Ok(vec![alert(1)])),
        ["alert 1 new high npm vite web/package-lock.json"]
    );
    assert_eq!(
        with_alerts(&r, vec![], Err("gh: 403".into())),
        ["alerts unavailable"]
    );
}
