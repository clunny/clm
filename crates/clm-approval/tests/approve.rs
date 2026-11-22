mod common;

use std::fs;
use std::path::Path;

use clm_ledger::{Body, Record};
use common::{Fixture, line, refused};

#[test]
fn approve_a_spec_branch() {
    // req:land.approval/approve@9ba68567
    let f = Fixture::new("");
    f.branch("spec/x", &[("changes/x/change.md", "x\n")]);
    let fp = f.fingerprint("spec/x");
    let out = clm_approval::approve(&f.work(), Some("spec/x"), &f.env(&[])).unwrap();
    assert_eq!(out.lines, [format!("approved {fp} spec/x")]);
    assert!(out.warnings.is_empty(), "{:?}", out.warnings);
    let records = f.records();
    match &records[..] {
        [r] => {
            assert!(r.writer.starts_with("christopher-lunny/"), "{}", r.writer);
            assert_eq!(
                r.body,
                Body::approved(fp, "spec/x".into(), Some("x".into())).with_clears(Vec::new())
            );
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn approve_without_a_change() {
    // req:land.approval/approve@9ba68567
    let f = Fixture::new("");
    f.branch("spec/x", &[("specs/x/SPEC.md", "x\n")]);
    clm_approval::approve(&f.work(), Some("spec/x"), &f.env(&[])).unwrap();
    match &f.records()[..] {
        [r] => assert!(matches!(&r.body, Body::Approved { change: None, .. })),
        other => panic!("{other:?}"),
    }
}

#[test]
fn nothing_to_approve() {
    // req:land.approval/approve@9ba68567
    let f = Fixture::new("");
    let work = f.work();
    f.git(&work, &["checkout", "-q", "-b", "spec/x"]);
    f.git(&work, &["push", "-q", "origin", "spec/x"]);
    refused(clm_approval::approve(&work, None, &f.env(&[])));
    assert!(f.records().is_empty());
}

#[test]
fn main_refused() {
    // req:land.approval/approve@9ba68567
    let f = Fixture::new("");
    let why = refused(clm_approval::approve(&f.work(), None, &f.env(&[])));
    assert!(why.contains("`main`"), "{why}");
    assert!(f.records().is_empty());
}

#[test]
fn refused_under_codex() {
    // req:land.approval/approve-refuses-unconfirming-agent-harness@93b62f91
    let f = Fixture::new("");
    f.branch("spec/x", &[("changes/x/change.md", "x\n")]);
    let env = f.env(&[(clm_agent_harness::CODEX_MARKER, "t1")]);
    let why = refused(clm_approval::approve(&f.work(), Some("spec/x"), &env));
    for word in ["codex", "approval-policy", "own terminal"] {
        assert!(why.contains(word), "{why}");
    }
    assert!(f.records().is_empty());
}

#[test]
fn refusal_comes_before_any_git_work() {
    // req:land.approval/approve-refuses-unconfirming-agent-harness@93b62f91
    let f = Fixture::new("");
    let env = f.env(&[(clm_agent_harness::CODEX_MARKER, "t1")]);
    let missing = f.path("no-such-root");
    let why = refused(clm_approval::approve(&missing, Some("spec/none"), &env));
    assert!(why.contains("codex"), "{why}");
}

#[test]
fn approves_in_the_humans_terminal() {
    // req:land.approval/approve-refuses-unconfirming-agent-harness@93b62f91
    let f = Fixture::new("");
    f.branch("spec/x", &[("changes/x/change.md", "x\n")]);
    clm_approval::approve(&f.work(), Some("spec/x"), &f.env(&[])).unwrap();
    assert_eq!(f.records().len(), 1);
}

#[test]
fn approves_under_claude_code() {
    // req:land.approval/approve-refuses-unconfirming-agent-harness@93b62f91
    let f = Fixture::new("");
    f.branch("spec/x", &[("changes/x/change.md", "x\n")]);
    clm_approval::approve(&f.work(), Some("spec/x"), &f.env(&[("CLAUDECODE", "1")])).unwrap();
    assert_eq!(f.records().len(), 1);
}

/// What `clm approve <args>` wrote on standard output and standard error, and returned, run in
/// `root`.
fn command(f: &Fixture, root: &Path, args: &[&str]) -> (u8, String, String) {
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = clm_approval::approve_command(
        args.iter().copied(),
        root,
        root,
        &f.env(&[]),
        &mut out,
        &mut err,
    );
    let text = |b: Vec<u8>| String::from_utf8(b).unwrap();
    (code, text(out), text(err))
}

/// The `approved` records the work clone sees.
fn approvals(f: &Fixture) -> Vec<Record> {
    f.records()
        .into_iter()
        .filter(|r| matches!(r.body, Body::Approved { .. }))
        .collect()
}

#[test]
fn a_branch_that_cant_be_approved() {
    // req:land.approval/approve@9ba68567
    let f = Fixture::new("");
    let work = f.work();
    f.branch("spec/x", &[("changes/x/change.md", "x\n")]);
    let refuses = |args: &[&str], names: &str| {
        let (code, out, err) = command(&f, &work, args);
        assert_eq!(code, 1, "{args:?}: {err}");
        assert_eq!(out, "", "{args:?}");
        assert!(err.starts_with("clm approve: "), "{args:?}: {err}");
        assert!(err.contains(names), "{args:?}: {err} lacks {names}");
        assert!(f.records().is_empty(), "{args:?} appended");
    };
    refuses(&["main"], "`main`");
    refuses(&[], "`main`");
    f.git(&work, &["checkout", "-q", "--detach", "spec/x"]);
    refuses(&[], "name the branch");
    f.git(&work, &["checkout", "-q", "main"]);
    refuses(&["spec/nope"], "spec/nope");
    // An `origin` with no `main`, approving a branch with a diff.
    let origin = f.path("origin.git");
    f.git(&origin, &["config", "receive.denyDeleteCurrent", "ignore"]);
    f.git(
        &f.path("other"),
        &["push", "-q", "origin", "--delete", "main"],
    );
    refuses(&["spec/x"], "origin/main");
}

#[test]
fn a_branch_origin_deleted() {
    // req:land.approval/approve@9ba68567
    let f = Fixture::new("");
    let work = f.work();
    f.branch("spec/x", &[("changes/x/change.md", "x\n")]);
    let c1 = f.fingerprint("spec/x");
    // `origin` moves the branch on to a descendant, the clone tracks it, then `origin`
    // deletes it.
    let other = f.path("other");
    f.git(&other, &["fetch", "-q", "origin"]);
    f.git(&other, &["checkout", "-q", "-B", "spec/x", "origin/spec/x"]);
    fs::write(other.join("y.txt"), "y\n").unwrap();
    f.git(&other, &["add", "-A"]);
    f.git(&other, &["commit", "-q", "-m", "c2"]);
    f.git(&other, &["push", "-q", "origin", "spec/x"]);
    f.git(&work, &["fetch", "-q", "origin"]);
    let c2 = f.git(&work, &["rev-parse", "origin/spec/x"]);
    assert_ne!(c2, f.git(&work, &["rev-parse", "spec/x"]));
    f.git(&other, &["push", "-q", "origin", "--delete", "spec/x"]);
    clm_approval::approve(&work, Some("spec/x"), &f.env(&[])).unwrap();
    let [record] = &approvals(&f)[..] else {
        panic!("not one approval")
    };
    assert!(
        matches!(&record.body, Body::Approved { fingerprint, .. } if *fingerprint == c1),
        "{record:?}"
    );
    let tracking = f.git(&work, &["for-each-ref", "refs/remotes/origin/spec/x"]);
    assert_eq!(tracking, "", "the stale tracking ref is kept");
}

#[test]
fn a_branch_with_two_changes() {
    // req:land.approval/approve@9ba68567
    let f = Fixture::new("");
    f.branch(
        "spec/x",
        &[
            ("changes/b/change.md", "b\n"),
            ("changes/a/change.md", "a\n"),
        ],
    );
    clm_approval::approve(&f.work(), Some("spec/x"), &f.env(&[])).unwrap();
    let [record] = &approvals(&f)[..] else {
        panic!("not one approval")
    };
    assert!(
        matches!(&record.body, Body::Approved { change: Some(c), .. } if c == "a"),
        "{record:?}"
    );
}

/// Pushes a ledger branch for `writer` holding `lines` to `origin`, from the `other` clone.
fn push_ledger(f: &Fixture, writer: &str, lines: &[String]) {
    let other = f.path("other");
    f.git(&other, &["checkout", "-q", "--orphan", "ledger"]);
    f.git(&other, &["rm", "-rq", "--cached", "."]);
    fs::write(other.join("records.jsonl"), lines.join("\n") + "\n").unwrap();
    f.git(&other, &["add", "records.jsonl"]);
    f.git(&other, &["commit", "-q", "-m", "ledger"]);
    let to = format!("HEAD:refs/heads/clm/ledger/{writer}");
    f.git(&other, &["push", "-q", "origin", &to]);
    f.git(&other, &["checkout", "-q", "-f", "main"]);
    f.git(&other, &["branch", "-q", "-D", "ledger"]);
}

fn request(id: &str, writer: &str, branch: &str, fingerprint: &str) -> String {
    let body = Body::ChangesRequested {
        branch: branch.into(),
        fingerprint: fingerprint.into(),
        text: "Name the rule".into(),
    };
    line(id, writer, "2026-11-17T10:00:00.000Z", body, None)
}

/// The `clears` of the last `approved` record for `branch` the work clone sees.
fn clears(f: &Fixture, branch: &str) -> Option<Vec<String>> {
    approvals(f)
        .into_iter()
        .rev()
        .find_map(|r| match r.body {
            Body::Approved {
                branch: b, clears, ..
            } if b == branch => Some(clears),
            _ => None,
        })
        .expect("an approval of the branch")
}

#[test]
fn approve_over_a_request() {
    // req:land.approval/approve@9ba68567
    let f = Fixture::new("");
    f.branch("spec/x", &[("changes/x/change.md", "x\n")]);
    f.branch("spec/z", &[("changes/z/change.md", "z\n")]);
    let fp = f.fingerprint("spec/x");
    // On `origin` only: none of them is in the clone before `clm approve`'s fetch.
    push_ledger(
        &f,
        "christopher-lunny/web",
        &[
            request("r1", "christopher-lunny/web", "spec/x", &fp),
            request("r3", "christopher-lunny/web", "spec/x", "other"),
            request("r4", "christopher-lunny/web", "spec/y", &fp),
        ],
    );
    push_ledger(&f, "dana/web", &[request("r2", "dana/web", "spec/x", &fp)]);
    let work = f.work();
    clm_approval::approve(&work, Some("spec/x"), &f.env(&[])).unwrap();
    let approver = approvals(&f)[0].writer.clone();
    assert!(approver.starts_with("christopher-lunny/"), "{approver}");
    assert_eq!(clears(&f, "spec/x"), Some(vec!["r1".to_string()]));
    // Named again, though an earlier approval cleared it.
    clm_approval::approve(&work, Some("spec/x"), &f.env(&[])).unwrap();
    assert_eq!(clears(&f, "spec/x"), Some(vec!["r1".to_string()]));
    clm_approval::approve(&work, Some("spec/z"), &f.env(&[])).unwrap();
    assert_eq!(clears(&f, "spec/z"), Some(Vec::new()));
}

#[test]
fn an_approval_not_yet_on_origin() {
    // req:land.approval/approve@9ba68567
    use std::os::unix::fs::PermissionsExt as _;
    let f = Fixture::new("");
    f.branch("spec/x", &[("changes/x/change.md", "x\n")]);
    let fp = f.fingerprint("spec/x");
    let hook = f.path("origin.git/hooks/pre-receive");
    let script = "#!/bin/sh\nwhile read old new ref; do\n  case \"$ref\" in \
                  refs/heads/clm/ledger/*) exit 1;; esac\ndone\n";
    fs::write(&hook, script).unwrap();
    fs::set_permissions(&hook, fs::Permissions::from_mode(0o755)).unwrap();
    let work = f.work();
    let (code, out, err) = command(&f, &work, &["spec/x"]);
    assert_eq!(code, 0, "{err}");
    assert_eq!(out, format!("approved {fp} spec/x\n"));
    // The local ledger branch holds it, and `origin` doesn't.
    let fold = clm_ledger::fold(&work).unwrap();
    let [record] = &fold.records[..] else {
        panic!("not one record: {:?}", fold.records)
    };
    assert!(matches!(record.body, Body::Approved { .. }), "{record:?}");
    let on_origin = f.git(&work, &["ls-remote", "origin", "refs/heads/clm/ledger/*"]);
    assert_eq!(on_origin, "");
    let warning = err
        .lines()
        .find(|l| l.starts_with("clm approve: warning: "))
        .unwrap_or_else(|| panic!("no warning: {err}"));
    let push = format!("git push origin clm/ledger/{}", record.writer);
    assert!(warning.contains(&push), "{warning}");
}
