//! The mutation pass after a green verification (verify/mutation-pass).

mod common;

use clm_ledger::Body;
use common::{Fixture, write_exec};

/// The start of the marker the fixture's test carries, built so the literal never appears
/// whole in this file.
const MARK: &str = concat!("req", ":a/thing@");

/// The pass's fake command, writing what it saw next to the fixture: its plan, its slot, a
/// file when it ran, and the result in `mutate-result`; it waits for `release` while `hold`
/// exists, and exits with `mutate-exit`.
const MUTATE: &str = r#"
echo ran >> "$DIR/mutate-ran"
cp "$CLM_MUTATE" "$DIR/plan"
echo "$CLM_SLOT" > "$DIR/slot"
: > "$DIR/diff-copy"
for p in $(sed -n 's/^diff //p' "$CLM_MUTATE"); do cat "$p" >> "$DIR/diff-copy"; done
echo started > "$DIR/started"
while [ -e "$DIR/hold" ] && [ ! -e "$DIR/release" ]; do sleep 0.05; done
if [ -e "$DIR/mutate-result" ]; then cat "$DIR/mutate-result" > "$CLM_MUTATE_RESULT"; fi
exit "$(cat "$DIR/mutate-exit" 2>/dev/null || echo 0)"
"#;

struct Pass {
    f: Fixture,
    /// The version of `a/thing` now.
    version: String,
}

fn test_file(version: &str, body: &str) -> String {
    format!("#[test]\nfn proves_x() {{\n    // {MARK}{version}\n    {body}\n}}\n")
}

/// A clone whose `main` holds a passing record and a `mutated` record for its head: `mutate`
/// sets the command, and `old_test` puts a test citing `a/thing` at a stale version there.
fn pass(mutate: bool, old_test: bool) -> Pass {
    let f = Fixture::new(Some(
        "mode = \"local\"\ntriggers = []\nevery = { lands = 1 }",
    ));
    let version = clm_model::load(&f.work())
        .0
        .requirement("a/thing")
        .unwrap()
        .version()
        .to_string();
    let script = f.path("bin/mutate");
    write_exec(
        &script,
        &format!(
            "#!/bin/sh\nDIR='{}'\n{MUTATE}",
            f.path("").display().to_string().trim_end_matches('/')
        ),
    );
    let check = f.path("bin/check");
    let extra = if mutate {
        format!("mutate = \"{}\"\n", script.display())
    } else {
        String::new()
    };
    let toml = format!(
        "[verify]\ncheck = \"{}\"\nmode = \"local\"\ntriggers = []\nevery = {{ lands = 1 }}\n{extra}",
        check.display()
    );
    let mut files = vec![
        ("clm.toml", toml),
        ("crates/a/src/lib.rs", "1\n".to_string()),
    ];
    if old_test {
        files.push(("crates/a/tests/x.rs", test_file("11111111", "let _ = 1;")));
    }
    let files: Vec<(&str, &str)> = files.iter().map(|(n, t)| (*n, t.as_str())).collect();
    let base = f.land(&files, "baseline");
    f.verified(&base, true);
    let body = Body::Mutated {
        commit: base,
        since: None,
        killed: vec![],
        unproven: vec![],
        not_judged: vec![],
    };
    clm_ledger::append(&f.work(), body, &f.env()).unwrap();
    Pass { f, version }
}

/// The `mutated` records the clone's ledger holds, oldest first.
fn mutated(f: &Fixture) -> Vec<Body> {
    f.git(&f.work(), &["fetch", "-q", "origin"]);
    let mut all: Vec<_> = clm_ledger::fold(&f.work())
        .unwrap()
        .mutations
        .into_values()
        .flatten()
        .collect();
    all.sort_by(|a, b| a.at.cmp(&b.at));
    all.into_iter().map(|r| r.body).collect()
}

/// Lands a new test of `a/thing` beside a changed `crates/a/src/lib.rs`.
fn land_new_test(p: &Pass) {
    p.f.land(
        &[
            ("crates/a/src/lib.rs", "2\n"),
            ("crates/a/tests/x.rs", &test_file(&p.version, "let _ = 1;")),
        ],
        "a new test",
    );
}

fn said(out: &clm_verify::Outcome, start: &str) -> bool {
    out.notes.iter().any(|n| n.starts_with(start))
}

#[test]
fn a_new_test_that_notices() {
    // req:verify/mutation-pass@e561e63f
    let p = pass(true, false);
    p.f.write("mutate-result", "a/thing killed\n");
    land_new_test(&p);
    let out = p.f.due();
    assert!(out.line.starts_with("verify passed ") && out.line.ends_with(" 1 lands"));
    assert_eq!(out.code, 0);
    let plan = p.f.lines("plan");
    let want = [
        "requirement a/thing".to_string(),
        "code crates/a/src/lib.rs".to_string(),
        "code crates/a/tests/x.rs".to_string(),
    ];
    for line in &want {
        assert!(plan.contains(line), "{plan:?}");
    }
    assert!(plan.iter().any(|l| l.starts_with("diff ")), "{plan:?}");
    assert!(
        plan.contains(&"test crates/a/tests/x.rs\tproves_x".to_string()),
        "{plan:?}"
    );
    assert_eq!(plan.last().map(String::as_str), Some("limit 20"));
    // The diff file holds the change to the code file.
    let diff = p.f.lines("diff-copy").join("\n");
    assert!(diff.contains("crates/a/src/lib.rs"), "{diff}");
    let records = mutated(&p.f);
    assert_eq!(records.len(), 2, "{records:?}");
    match &records[1] {
        Body::Mutated {
            killed, unproven, ..
        } => {
            assert_eq!(killed, &["a/thing".to_string()]);
            assert!(unproven.is_empty());
        }
        other => panic!("{other:?}"),
    }
    assert!(said(&out, "mutation: 1 killed, 0 unproven, 0 not judged"));
}

#[test]
fn a_test_that_notices_nothing() {
    // req:verify/mutation-pass@e561e63f
    let p = pass(true, false);
    p.f.write("mutate-result", "a/thing survived 12\n");
    land_new_test(&p);
    let out = p.f.due();
    assert!(out.line.starts_with("verify passed "));
    assert_eq!(out.code, 0);
    let Body::Mutated {
        unproven, killed, ..
    } = mutated(&p.f).pop().unwrap()
    else {
        panic!()
    };
    assert!(killed.is_empty());
    assert_eq!(unproven, [format!("a/thing@{}", p.version)]);
    assert!(said(&out, "mutation: 0 killed, 1 unproven, 0 not judged"));
}

#[test]
fn survived_zero_is_not_judged() {
    // req:verify/mutation-pass@e561e63f
    let p = pass(true, false);
    p.f.write("mutate-result", "a/thing survived 0\n");
    land_new_test(&p);
    let out = p.f.due();
    let Body::Mutated {
        unproven,
        not_judged,
        ..
    } = mutated(&p.f).pop().unwrap()
    else {
        panic!()
    };
    assert!(unproven.is_empty());
    assert_eq!(not_judged, ["a/thing".to_string()]);
    assert!(said(&out, "mutation: 0 killed, 0 unproven, 1 not judged"));
}

#[test]
fn a_test_only_re_cited_is_not_judged() {
    // req:verify/mutation-pass@e561e63f
    let p = pass(true, true);
    p.f.land(
        &[("crates/a/tests/x.rs", &test_file(&p.version, "let _ = 1;"))],
        "re-cite",
    );
    let out = p.f.due();
    assert!(out.line.starts_with("verify passed "));
    assert!(!p.f.path("mutate-ran").exists());
    let records = mutated(&p.f);
    assert_eq!(records.len(), 2);
    let Body::Mutated {
        killed,
        unproven,
        not_judged,
        ..
    } = &records[1]
    else {
        panic!()
    };
    assert!(killed.is_empty() && unproven.is_empty() && not_judged.is_empty());
}

#[test]
fn a_test_changed_in_more_than_its_marker_is_due() {
    // req:verify/mutation-pass@e561e63f
    let p = pass(true, true);
    p.f.write("mutate-result", "a/thing killed\n");
    p.f.land(
        &[(
            "crates/a/tests/x.rs",
            &test_file(&p.version, "assert_eq!(1, 1);"),
        )],
        "a changed body",
    );
    p.f.due();
    assert!(p.f.path("mutate-ran").exists());
    assert!(
        p.f.lines("plan")
            .contains(&"requirement a/thing".to_string())
    );
}

#[test]
fn an_item_resolved_with_its_test_unchanged_is_not_judged_again() {
    // req:verify/mutation-pass@e561e63f
    let p = pass(true, true);
    p.f.land(
        &[("crates/a/tests/x.rs", &test_file(&p.version, "let _ = 1;"))],
        "re-cite",
    );
    p.f.land(
        &[
            ("crates/a/src/lib.rs", "3\n"),
            ("backlog/unproven-a-1234567.md", "gone\n"),
        ],
        "an item",
    );
    // A later land deletes the item and changes the code, leaving the test as it was.
    p.f.git(&p.f.work(), &["rm", "-q", "backlog/unproven-a-1234567.md"]);
    p.f.git(&p.f.work(), &["commit", "-q", "-m", "resolved"]);
    p.f.git(&p.f.work(), &["push", "-q", "origin", "main"]);
    p.f.due();
    assert!(!p.f.path("mutate-ran").exists());
}

#[test]
fn every_citing_test_is_in_the_plan_and_a_stale_citation_is_none() {
    // req:verify/mutation-pass@e561e63f
    let p = pass(true, false);
    p.f.write("mutate-result", "a/thing killed\n");
    let two = format!(
        "{}{}",
        test_file(&p.version, "let _ = 1;"),
        test_file(&p.version, "let _ = 2;").replace("proves_x", "proves_y")
    );
    p.f.land(
        &[
            ("crates/a/tests/x.rs", &two),
            ("crates/a/tests/y.rs", &test_file(&p.version, "let _ = 3;")),
            // A test citing the requirement at an old version proves nothing of it.
            (
                "crates/a/tests/stale.rs",
                &test_file("11111111", "let _ = 4;").replace("proves_x", "proves_old"),
            ),
        ],
        "tests",
    );
    p.f.due();
    let plan = p.f.lines("plan");
    for test in [
        "test crates/a/tests/x.rs\tproves_x",
        "test crates/a/tests/x.rs\tproves_y",
        "test crates/a/tests/y.rs\tproves_x",
    ] {
        assert!(plan.contains(&test.to_string()), "{plan:?}");
    }
    assert!(!plan.iter().any(|l| l.contains("proves_old")), "{plan:?}");
}

#[test]
fn a_stale_citation_alone_makes_nothing_due() {
    // req:verify/mutation-pass@e561e63f
    let p = pass(true, false);
    p.f.land(
        &[(
            "crates/a/tests/stale.rs",
            &test_file("11111111", "let _ = 4;"),
        )],
        "stale",
    );
    p.f.due();
    assert!(!p.f.path("mutate-ran").exists());
}

#[test]
fn the_pass_has_a_slot_of_its_own() {
    // req:verify/mutation-pass@e561e63f
    let p = pass(true, false);
    p.f.write("mutate-result", "a/thing killed\n");
    std::fs::write(p.f.work().join("clm.local.toml"), "max_builds = 4\n").unwrap();
    land_new_test(&p);
    p.f.due();
    assert_eq!(p.f.lines("slot"), ["5"]);
}

#[test]
fn a_failing_command_records_nothing() {
    // req:verify/mutation-pass@e561e63f
    let p = pass(true, false);
    p.f.write("mutate-exit", "3");
    land_new_test(&p);
    let out = p.f.due();
    assert!(out.line.starts_with("verify passed "));
    assert_eq!(out.code, 0);
    assert_eq!(mutated(&p.f).len(), 1);
    assert!(
        out.notes
            .iter()
            .any(|n| n.starts_with("mutation: ") && n.contains("failed with status 3")),
        "{:?}",
        out.notes
    );
}

#[test]
fn unset_runs_no_pass() {
    // req:verify/mutation-pass@e561e63f
    let p = pass(false, false);
    land_new_test(&p);
    let out = p.f.due();
    assert!(out.line.starts_with("verify passed "));
    assert_eq!(mutated(&p.f).len(), 1);
    assert!(!said(&out, "mutation:"));
    assert!(!p.f.path("mutate-ran").exists());
}

#[test]
fn a_red_check_runs_no_pass() {
    // req:verify/mutation-pass@e561e63f
    let p = pass(true, false);
    p.f.write("check-fails", "");
    land_new_test(&p);
    let out = p.f.due();
    assert_eq!(out.code, 1);
    assert!(!p.f.path("mutate-ran").exists());
    assert_eq!(mutated(&p.f).len(), 1);
}

#[test]
fn lands_do_not_wait_for_the_pass() {
    // req:verify/mutation-pass@e561e63f
    let p = pass(true, false);
    p.f.write("hold", "");
    land_new_test(&p);
    std::thread::scope(|s| {
        let verifying = s.spawn(|| p.f.due());
        for _ in 0..400 {
            if p.f.path("started").exists() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        assert!(p.f.path("started").exists(), "the pass never started");
        // The pass is waiting for `release`; a land's check runs now.
        let run = clm_land::run_check(
            "true",
            &p.f.work(),
            None,
            &clm_land::CheckEnv::default(),
            None,
        )
        .unwrap();
        assert!(run.passed);
        assert!(!p.f.path("release").exists());
        p.f.write("release", "");
        verifying.join().unwrap();
    });
}
