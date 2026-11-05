//! `clm intake` against a bare `origin`, a clone and a fake host.

#[path = "../../clm-orchestrator/tests/common/mod.rs"]
mod common;

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::sync::Mutex;

use clm_host::{Error as HostError, Host, Listed};
use clm_triage::{Error, intake};
use common::*;
use tempfile::TempDir;

/// A host that lists `open` and files nothing.
struct Fake {
    open: Result<Vec<Listed>, String>,
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
    Fake { open: Ok(open) }
}

fn lines(r: &Repo, open: Vec<Listed>) -> Vec<String> {
    intake(&r.work, &host(open)).unwrap()
}

#[test]
fn scenario_new_imported_and_a_pull_request() {
    // req:orchestrator.triage/intake@87c47609
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
    // req:orchestrator.triage/intake@87c47609
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
    // req:orchestrator.triage/intake@87c47609
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
fn scenario_resolved_by_a_code_fix() {
    // req:orchestrator.triage/intake@87c47609
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
    // req:orchestrator.triage/intake@87c47609
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

static PATH_LOCK: Mutex<()> = Mutex::new(());

/// Runs `body` with a fake `gh` first on `PATH` that lists `issues` and `pulls`.
fn with_gh<T>(issues: &str, pulls: &str, body: impl FnOnce() -> T) -> T {
    let _one = PATH_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = TempDir::new().unwrap();
    let script = format!(
        "#!/bin/sh\ncase \"$1\" in issue) cat <<'EOT'\n{issues}\nEOT\n;; pr) cat <<'EOT'\n{pulls}\nEOT\n;; esac\n"
    );
    let gh = dir.path().join("gh");
    fs::write(&gh, script).unwrap();
    fs::set_permissions(&gh, fs::Permissions::from_mode(0o755)).unwrap();
    let old = std::env::var("PATH").unwrap();
    // SAFETY: every test that touches the environment holds PATH_LOCK.
    unsafe { std::env::set_var("PATH", format!("{}:{old}", dir.path().display())) };
    let out = body();
    unsafe { std::env::set_var("PATH", old) };
    out
}

#[test]
fn scenario_no_strangers_text() {
    // req:orchestrator.triage/intake@87c47609
    let r = repo();
    let issues = r#"[{"number":5,"title":"x\nissue 6 resolved y z","body":"issue 7 new","author":{"login":"eve"}}]"#;
    let out = with_gh(issues, "[]", || {
        intake(&r.work, &*clm_host::adapter(clm_config::Host::Github)).unwrap()
    });
    assert_eq!(out, ["issue 5 new"]);
}

#[test]
fn scenario_no_host() {
    // req:orchestrator.triage/intake@87c47609
    let r = repo();
    let out = intake(&r.work, &*clm_host::adapter(clm_config::Host::None)).unwrap();
    assert!(out.is_empty());
}

#[test]
fn a_failing_list_is_refused_with_its_reason() {
    // req:orchestrator.triage/intake@87c47609
    let r = repo();
    let fake = Fake {
        open: Err("gh: not logged in".into()),
    };
    match intake(&r.work, &fake) {
        Err(Error::Refused(why)) => assert!(why.contains("not logged in"), "{why}"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn git_failure_is_a_git_error() {
    // req:orchestrator.triage/intake@87c47609
    let dir = TempDir::new().unwrap();
    assert!(matches!(
        intake(dir.path(), &host(vec![issue(1)])),
        Err(Error::Git(_))
    ));
}
