//! A refused push to `main`, judged by fetching `origin` again (`land/land-push-retry`), and
//! a `landed` record that didn't reach `origin`.

mod common;

use clm_ledger::Body;
use common::Fixture;

/// Logs the time, in nanoseconds, of each push naming `refs/heads/main` to `main-pushes.log`,
/// and declines it with `declined by test` while `decline` (bash, with `n` the push's number)
/// holds.
fn main_hook(decline: &str) -> String {
    format!(
        r#"
while read -r old new ref; do
  if [ "$ref" = refs/heads/main ]; then
    date +%s%N >> "$DIR/main-pushes.log"
    n=$(wc -l < "$DIR/main-pushes.log")
    if {decline}; then echo "declined by test" >&2; exit 1; fi
  fi
done
exit 0
"#
    )
}

fn landed_commits(f: &Fixture) -> Vec<String> {
    f.landed()
        .into_iter()
        .filter_map(|b| match b {
            Body::Landed { commit, .. } => Some(commit),
            _ => None,
        })
        .collect()
}

#[test]
fn one_transient_refusal() {
    // req:land/land-push-retry@94d61d9c
    let f = Fixture::new("");
    f.hook(Some(&main_hook("[ \"$n\" -eq 1 ]")));
    let head = f.branch("impl/a", &[("b.txt", "b\n")]);
    let old = f.remote("main");
    let out = clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    assert_eq!(f.lines("main-pushes.log").len(), 2);
    assert_eq!(f.lines("check.log").len(), 1);
    let main = f.remote("main");
    assert_eq!(f.parents(&f.path("origin.git"), &main), [old, head]);
    assert_eq!(landed_commits(&f), std::slice::from_ref(&main));
    assert_eq!(out.lines[0], format!("landed {main} code"));
}

#[test]
fn refused_every_time_then_rerun() {
    // req:land/land-push-retry@94d61d9c
    let f = Fixture::new("");
    f.hook(Some(&main_hook("true")));
    f.branch("impl/a", &[("b.txt", "b\n")]);
    let old = f.remote("main");
    let why = match clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])) {
        Err(clm_land::Error::Git(why)) => why,
        other => panic!("expected a git failure, got {other:?}"),
    };
    let times: Vec<u128> = f
        .lines("main-pushes.log")
        .iter()
        .map(|t| t.parse().unwrap())
        .collect();
    assert_eq!(times.len(), 3, "{times:?}");
    const SEC: u128 = 1_000_000_000;
    assert!(times[1] - times[0] >= SEC, "{times:?}");
    assert!(times[2] - times[1] >= 2 * SEC, "{times:?}");
    for needle in ["declined by test", "may be transient", "clm land impl/a"] {
        assert!(why.contains(needle), "no {needle:?} in {why}");
    }
    assert!(!why.contains("keeps moving"), "{why}");
    assert_eq!(f.remote("main"), old);
    assert_eq!(f.checked().len(), 1);
    assert!(f.landed().is_empty());
    f.assert_no_scratch();

    // Scenario: rerun after giving up. The `checked` record stands, so the check doesn't
    // run again.
    f.hook(None);
    let out = clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    assert_eq!(f.lines("check.log").len(), 1);
    let main = f.remote("main");
    assert_eq!(out.lines[0], format!("landed {main} code"));
    assert_eq!(landed_commits(&f), [main]);
}

#[test]
fn ledger_push_refused() {
    // req:land/land-push-retry@94d61d9c
    let f = Fixture::new("");
    f.hook(Some(
        r#"
while read -r old new ref; do
  case "$ref" in refs/heads/clm/ledger/*) echo "no ledgers" >&2; exit 1;; esac
done
exit 0
"#,
    ));
    let head = f.branch("impl/a", &[("b.txt", "b\n")]);
    let out = clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    let main = f.remote("main");
    assert_eq!(f.parents(&f.path("origin.git"), &main)[1], head);
    assert_eq!(out.lines[0], format!("landed {main} code"));
    // The local ledger branch holds the record; `origin` has no ledger.
    assert_eq!(landed_commits(&f), [main]);
    let writer = clm_ledger::writer(&f.work(), &f.env(&[])).unwrap();
    let ledger = format!("clm/ledger/{writer}");
    assert!(
        f.git(&f.path("origin.git"), &["branch", "--list", "clm/ledger/*"])
            .is_empty()
    );
    let warned = format!("the landed record isn't on origin yet: git push origin {ledger}");
    assert!(
        out.warnings.iter().any(|w| w == &warned),
        "{:?}",
        out.warnings
    );
    assert_eq!(
        out.warnings.iter().filter(|w| w.contains("ledger")).count(),
        1
    );
}

#[test]
fn no_host_no_ledger_warning() {
    // The clone's own remote takes every record, so there is no warning.
    let f = Fixture::local();
    f.branch("impl/a", &[("b.txt", "b\n")]);
    let out = clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    assert!(!out.lines[0].contains("(local)"), "{:?}", out.lines);
    assert!(
        !out.warnings.iter().any(|w| w.contains("isn't on origin")),
        "{:?}",
        out.warnings
    );
}
