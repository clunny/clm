//! `clm land` records the landed branch's effort (`land/records-effort`).

mod common;

use clm_ledger::Body;
use common::Fixture;

/// Marks `work` a CLM repository, as `clm effort record` needs, and writes a transcript of a
/// session on `branch` run there under `HOME`'s `.claude/projects`.
fn session(f: &Fixture, branch: &str) {
    let work = clm_repository(f);
    let encoded: String = work
        .to_string_lossy()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let dir = f.path(".claude/projects").join(encoded);
    std::fs::create_dir_all(&dir).unwrap();
    let line = serde_json::json!({
        "sessionId": "s1",
        "gitBranch": branch,
        "cwd": work,
        "timestamp": "2026-10-30T00:00:00.000Z",
        "message": {"id": "m1", "model": "claude-opus-5-5", "usage": {"input_tokens": 10, "output_tokens": 5}},
    });
    std::fs::write(dir.join("s1.jsonl"), format!("{line}\n")).unwrap();
}

/// Marks `work` a CLM repository: `specs/SPEC.md`, untracked, so after `branch`, which
/// commits every file.
fn clm_repository(f: &Fixture) -> std::path::PathBuf {
    let work = f.work();
    std::fs::create_dir_all(work.join("specs")).unwrap();
    std::fs::write(work.join("specs/SPEC.md"), "---\ntitle: T\n---\n").unwrap();
    work
}

fn efforts(f: &Fixture) -> Vec<Body> {
    f.records()
        .into_iter()
        .filter(|r| matches!(r.body, Body::Effort { .. }))
        .map(|r| r.body)
        .collect()
}

/// The writer's ledger branch's last commit adds the `effort` line and then the `landed` one.
fn assert_written_with_the_land(f: &Fixture) {
    let ledger = f.git(
        &f.work(),
        &[
            "for-each-ref",
            "--format=%(refname:short)",
            "refs/heads/clm/ledger/",
        ],
    );
    let diff = f.git(&f.work(), &["show", "--format=", &ledger]);
    // Only the lines the commit adds, not the diff's context.
    let added: String = diff
        .lines()
        .filter(|l| l.starts_with('+') && !l.starts_with("+++"))
        .collect::<Vec<_>>()
        .join("\n");
    let effort = added.find(r#""kind":"effort""#).expect(&added);
    let landed = added.find(r#""kind":"landed""#).expect(&added);
    assert!(effort < landed, "{added}");
}

/// Declines every push to `main` while `decline` (bash, `n` the push's number) holds.
fn main_hook(decline: &str) -> String {
    format!(
        r#"
while read -r old new ref; do
  if [ "$ref" = refs/heads/main ]; then
    echo x >> "$DIR/main-pushes.log"
    n=$(wc -l < "$DIR/main-pushes.log")
    if {decline}; then echo "declined by test" >&2; exit 1; fi
  fi
done
exit 0
"#
    )
}

#[test]
fn land_records_effort() {
    // req:land/records-effort@2fbdf1d7
    let f = Fixture::new("");
    f.branch("impl/a", &[("b.txt", "b\n")]);
    session(&f, "impl/a");
    let out = clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    let main = f.remote("main");
    assert_eq!(out.lines.last().unwrap(), &format!("landed {main} code"));
    let efforts = efforts(&f);
    assert_eq!(efforts.len(), 1, "{:?}", out.warnings);
    assert!(
        matches!(&efforts[0], Body::Effort { branch, session, .. } if branch == "impl/a" && session == "s1")
    );
    assert_written_with_the_land(&f);
}

#[test]
fn land_without_transcripts_lands() {
    // req:land/records-effort@2fbdf1d7
    let f = Fixture::new("");
    f.branch("impl/a", &[("b.txt", "b\n")]);
    clm_repository(&f);
    let out = clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    let main = f.remote("main");
    assert_eq!(out.lines.last().unwrap(), &format!("landed {main} code"));
    assert!(efforts(&f).is_empty());
    assert!(
        out.warnings
            .iter()
            .any(|w| w.starts_with("effort: no transcripts at ")),
        "{:?}",
        out.warnings
    );
}

#[test]
fn refused_push_records_once() {
    // req:land/records-effort@2fbdf1d7
    let f = Fixture::new("");
    f.hook(Some(&main_hook("[ \"$n\" -eq 1 ]")));
    f.branch("impl/a", &[("b.txt", "b\n")]);
    session(&f, "impl/a");
    clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    assert_eq!(f.lines("main-pushes.log").len(), 2);
    assert_eq!(efforts(&f).len(), 1);
    assert_written_with_the_land(&f);
}

#[test]
fn push_never_taken_records_nothing() {
    // req:land/records-effort@2fbdf1d7
    let f = Fixture::new("");
    f.hook(Some(&main_hook("true")));
    f.branch("impl/a", &[("b.txt", "b\n")]);
    session(&f, "impl/a");
    assert!(clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).is_err());
    assert!(efforts(&f).is_empty());
}

#[test]
fn effort_failure_warns_and_lands() {
    // req:land/records-effort@2fbdf1d7
    // Not a CLM repository to `clm effort record`: refused, so a warning, and the land stands.
    let f = Fixture::new("");
    f.branch("impl/a", &[("b.txt", "b\n")]);
    let out = clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    let main = f.remote("main");
    assert_eq!(out.lines.last().unwrap(), &format!("landed {main} code"));
    assert!(
        out.warnings
            .iter()
            .any(|w| w.starts_with("effort not recorded: ")),
        "{:?}",
        out.warnings
    );
    assert_eq!(f.landed().len(), 1);
}
