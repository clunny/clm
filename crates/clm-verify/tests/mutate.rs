//! The mutation pass after a green verification (verify/mutation-pass).

mod common;

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use clm_ledger::Body;
use clm_verify::Call;
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
echo "$CLM_MUTATE_JOBS" >> "$DIR/jobs"
: > "$DIR/diff-copy"
n=0
for p in $(sed -n 's/^diff //p' "$CLM_MUTATE"); do
  n=$((n + 1)); cat "$p" >> "$DIR/diff-copy"; cp "$p" "$DIR/diff-$n"
done
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
    let body = Body::mutated(base, None, vec![], vec![], vec![]);
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
    // req:verify/mutation-pass@1dea303f
    // req:verify/mutated-through-constructor@899e8820
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
    // req:verify/mutation-pass@1dea303f
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
    // req:verify/mutation-pass@1dea303f
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

/// A clone with no `mutated` record whose `[verify]` has no `mutate` yet, with the fake
/// command written, and the version of `a/thing`.
fn unswitched() -> (Fixture, String) {
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
    (f, version)
}

/// A `clm.toml` that sets `mutate`.
fn switched_on(f: &Fixture) -> String {
    format!(
        "[verify]\ncheck = \"{}\"\nmode = \"local\"\ntriggers = []\nevery = {{ lands = 1 }}\nmutate = \"{}\"\n",
        f.path("bin/check").display(),
        f.path("bin/mutate").display()
    )
}

/// The spec of component `a` holding `thing` and each of `more`.
fn spec_of_a(more: &[&str]) -> String {
    let mut reqs = common::UNBUILT.to_string();
    for slug in more {
        reqs.push_str(&format!(
            "\n### Requirement: {slug}\nIt SHALL do the {slug} thing.\n\n\
             #### Scenario: doing\n- GIVEN a\n- WHEN b\n- THEN c\n"
        ));
    }
    format!(
        "---\ntitle: A\nstatus: active\nowners: []\npaths: [\"crates/a/**\"]\n---\n\n\
         ## Contract\nc\n\n## Design\nd\n\n## Requirements\n{reqs}"
    )
}

/// A test file `crates/a/tests/<slug>.rs` whose test `proves_<slug>` cites `a/<slug>` at its
/// current version.
fn test_of(f: &Fixture, slug: &str) -> (String, String) {
    let version = clm_model::load(&f.work())
        .0
        .requirement(&format!("a/{slug}"))
        .unwrap()
        .version()
        .to_string();
    let text = test_file(&version, &format!("let _ = \"{slug}\";"))
        .replace("a/thing", &format!("a/{slug}"))
        .replace("proves_x", &format!("proves_{slug}"));
    (format!("crates/a/tests/{slug}.rs"), text)
}

/// Lands the requirements `more` of `a`, and then a test of each, and of `a/thing` too when
/// `thing`, so each is due.
fn land_due(f: &Fixture, thing: bool, more: &[&str]) {
    f.land(&[("specs/a/SPEC.md", &spec_of_a(more))], "requirements");
    let slugs = thing
        .then_some("thing")
        .into_iter()
        .chain(more.iter().copied());
    let tests: Vec<(String, String)> = slugs.map(|s| test_of(f, s)).collect();
    let files: Vec<(&str, &str)> = tests
        .iter()
        .map(|(p, t)| (p.as_str(), t.as_str()))
        .collect();
    f.land(&files, "tests");
}

/// The newest `mutated` record's lists: killed, unproven, not judged and left.
fn lists(f: &Fixture) -> [Vec<String>; 4] {
    match mutated(f).pop().unwrap() {
        Body::Mutated {
            killed,
            unproven,
            not_judged,
            left,
            ..
        } => [killed, unproven, not_judged, left],
        other => panic!("{other:?}"),
    }
}

fn ids(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

#[test]
fn the_first_pass_fails_and_the_next_loses_nothing() {
    // req:verify/mutation-pass@1dea303f
    let (f, _) = unswitched();
    let before = f.back(0);
    f.land(&[("clm.toml", &switched_on(&f))], "switch the pass on");
    let (path, test) = test_of(&f, "thing");
    let l = f.land(
        &[("crates/a/src/lib.rs", "changed by L\n"), (&path, &test)],
        "L: a test and a's code",
    );
    f.write("mutate-exit", "3");
    let out = f.due();
    assert!(out.line.starts_with("verify passed "), "{out:?}");
    assert!(f.path("mutate-ran").exists(), "the command never ran");
    let first = mutated(&f);
    assert_eq!(first.len(), 1, "{first:?}");
    match &first[0] {
        Body::Mutated {
            commit,
            since,
            killed,
            left,
            ..
        } => {
            assert_eq!(commit, &l);
            assert_eq!(since.as_deref(), Some(before.as_str()));
            assert!(killed.is_empty());
            assert_eq!(left, &ids(&["a/thing"]));
        }
        other => panic!("{other:?}"),
    }
    // A requirement and a test of it arrive, and the command now works.
    f.land(&[("specs/a/SPEC.md", &spec_of_a(&["other"]))], "another");
    let (path, test) = test_of(&f, "other");
    f.land(&[(&path, &test)], "another test");
    f.write("mutate-exit", "0");
    f.write("mutate-result", "a/thing killed\na/other killed\n");
    f.due();
    let plan = f.lines("plan");
    let at = |line: &str| plan.iter().position(|l| l == line);
    let (thing, other) = (at("requirement a/thing"), at("requirement a/other"));
    assert!(thing.is_some() && other.is_some(), "{plan:?}");
    assert!(thing < other, "the left one goes first: {plan:?}");
    // The carried-over requirement's diff still holds L's change to a's code.
    let diff = std::fs::read_to_string(f.path("diff-1")).unwrap();
    assert!(diff.contains("changed by L"), "{diff}");
    let Body::Mutated { since, .. } = mutated(&f).pop().unwrap() else {
        panic!()
    };
    assert_eq!(since, Some(l));
    assert_eq!(lists(&f)[0], ids(&["a/thing", "a/other"]));
}

#[test]
fn the_command_fails_part_way() {
    // req:verify/mutation-pass@1dea303f
    let p = pass(true, false);
    land_due(&p.f, true, &["other", "third"]);
    p.f.write("mutate-result", "a/thing killed\n");
    p.f.write("mutate-exit", "3");
    let out = p.f.due();
    assert!(out.line.starts_with("verify passed "), "{out:?}");
    assert_eq!(out.code, 0);
    let [killed, unproven, not_judged, left] = lists(&p.f);
    assert_eq!(killed, ids(&["a/thing"]));
    assert!(unproven.is_empty() && not_judged.is_empty());
    // Left in the plan's order, which is ID order here.
    assert_eq!(left, ids(&["a/other", "a/third"]));
    let failed = out
        .notes
        .iter()
        .position(|n| n.starts_with("mutation: ") && n.contains("failed with status 3"));
    let counts = out
        .notes
        .iter()
        .position(|n| n == "mutation: 1 killed, 0 unproven, 0 not judged, 2 left");
    assert!(
        failed.is_some() && counts.is_some() && failed < counts,
        "{:?}",
        out.notes
    );
    assert!(out.notes[failed.unwrap()].contains("bin/mutate"));
}

#[test]
fn a_line_cut_short() {
    // req:verify/mutation-pass@1dea303f
    let p = pass(true, false);
    land_due(&p.f, true, &["other"]);
    // A line that doesn't parse is ignored, and the last line for a requirement holds.
    p.f.write(
        "mutate-result",
        "a/thing survived 2\nnot a verdict at all\na/thing killed\na/other survived 1",
    );
    p.f.write("mutate-exit", "3");
    p.f.due();
    let [killed, unproven, not_judged, left] = lists(&p.f);
    assert_eq!(killed, ids(&["a/thing"]));
    assert!(unproven.is_empty() && not_judged.is_empty(), "{unproven:?}");
    assert_eq!(left, ids(&["a/other"]));
}

#[test]
fn a_not_judged_line_is_not_left() {
    // req:verify/mutation-pass@1dea303f
    let p = pass(true, false);
    land_due(&p.f, true, &["other"]);
    p.f.write("mutate-result", "a/other not-judged\na/thing killed\n");
    let out = p.f.due();
    let [killed, _, not_judged, left] = lists(&p.f);
    assert_eq!(killed, ids(&["a/thing"]));
    assert_eq!(not_judged, ids(&["a/other"]));
    assert!(left.is_empty());
    assert!(
        out.notes
            .contains(&"mutation: 1 killed, 0 unproven, 1 not judged".to_string()),
        "{:?}",
        out.notes
    );
}

#[test]
fn what_is_left_goes_first() {
    // req:verify/mutation-pass@1dea303f
    let p = pass(true, false);
    // `a/thing` has a test at its current version, and the newest record left it.
    let (path, test) = test_of(&p.f, "thing");
    let head = p.f.land(&[(&path, &test)], "a test of thing");
    let base = p.f.back(1);
    let body = Body::mutated(head.clone(), Some(base), vec![], vec![], vec![])
        .with_left(ids(&["a/thing"]));
    clm_ledger::append(&p.f.work(), body, &p.f.env()).unwrap();
    // A land since adds `a/other`, before `a/thing` in ID order.
    land_due(&p.f, false, &["other"]);
    p.f.write("mutate-result", "a/thing killed\na/other killed\n");
    p.f.due();
    let plan = p.f.lines("plan");
    let at = |line: &str| plan.iter().position(|l| l == line);
    let (thing, other) = (at("requirement a/thing"), at("requirement a/other"));
    assert!(thing.is_some() && other.is_some(), "{plan:?}");
    assert!(thing < other, "{plan:?}");
    let [killed, ..] = lists(&p.f);
    assert_eq!(killed, ids(&["a/thing", "a/other"]));
}

/// Appends a `mutated` record for `commit` with `since` and `left`, a little after the last,
/// so records read in the order they were appended.
fn record(f: &Fixture, commit: &str, since: Option<String>, left: &[&str]) {
    std::thread::sleep(Duration::from_millis(5));
    let body =
        Body::mutated(commit.to_string(), since, vec![], vec![], vec![]).with_left(ids(left));
    clm_ledger::append(&f.work(), body, &f.env()).unwrap();
}

#[test]
fn a_carried_diff_runs_from_the_oldest_record_that_left_it() {
    // req:verify/mutation-pass@1dea303f
    let p = pass(true, false);
    let base = p.f.back(0);
    let (path, test) = test_of(&p.f, "thing");
    let x = p.f.land(
        &[
            (&path, &test),
            ("crates/a/src/one.rs", "the first change\n"),
        ],
        "X",
    );
    record(&p.f, &x, Some(base), &["a/thing"]);
    let y =
        p.f.land(&[("crates/a/src/two.rs", "the second change\n")], "Y");
    // Of `Y`'s two records the newer, by `at`, is the one that counts.
    record(&p.f, &y, Some(x.clone()), &[]);
    record(&p.f, &y, Some(x), &["a/thing"]);
    let c = p.f.land(&[("crates/b/src/lib.rs", "b\n")], "C");
    // The newest record is on `C` itself.
    record(&p.f, &c, Some(y), &["a/thing"]);
    p.f.write("mutate-result", "a/thing killed\n");
    p.f.due();
    let plan = p.f.lines("plan");
    assert_eq!(
        plan.first().map(String::as_str),
        Some("requirement a/thing")
    );
    let diff = std::fs::read_to_string(p.f.path("diff-1")).unwrap();
    assert!(diff.contains("the first change"), "{diff}");
    assert!(diff.contains("the second change"), "{diff}");
    assert_eq!(lists(&p.f)[0], ids(&["a/thing"]));
}

#[test]
fn a_record_on_the_verified_commit_carries_its_left() {
    // req:verify/mutation-pass@1dea303f
    let p = pass(true, false);
    let base = p.f.back(0);
    let (path, test) = test_of(&p.f, "thing");
    let x = p.f.land(&[(&path, &test)], "X");
    record(&p.f, &x, Some(base), &[]);
    let c = p.f.land(&[("crates/b/src/lib.rs", "b\n")], "C");
    record(&p.f, &c, Some(x), &["a/thing"]);
    p.f.write("mutate-result", "a/thing killed\n");
    p.f.due();
    assert!(p.f.path("mutate-ran").exists(), "nothing was carried");
    assert_eq!(lists(&p.f)[0], ids(&["a/thing"]));
}

#[test]
fn a_left_requirement_no_longer_cited_is_dropped() {
    // req:verify/mutation-pass@1dea303f
    let p = pass(true, true);
    let head = p.f.back(0);
    let body = Body::mutated(head, None, vec![], vec![], vec![]).with_left(ids(&["a/thing"]));
    clm_ledger::append(&p.f.work(), body, &p.f.env()).unwrap();
    p.f.land(&[("crates/a/src/lib.rs", "5\n")], "code only");
    p.f.due();
    // The only test of `a/thing` cites a stale version, so nothing is due.
    assert!(!p.f.path("mutate-ran").exists());
    assert!(lists(&p.f).iter().all(Vec::is_empty));
}

/// A `clm.toml` for `pass` with `extra` added to its `[verify]`.
fn with_settings(p: &Pass, extra: &str) -> String {
    format!(
        "[verify]\ncheck = \"{}\"\nmode = \"local\"\ntriggers = []\nevery = {{ lands = 1 }}\n\
         mutate = \"{}\"\n{extra}",
        p.f.path("bin/check").display(),
        p.f.path("bin/mutate").display()
    )
}

/// A call with `memory`, a minute of `minute` and a grace of `grace`, stopped by `stop`.
fn call<'a>(
    memory: &'a std::path::Path,
    minute: Duration,
    grace: Duration,
    stop: &'a AtomicBool,
) -> Call<'a> {
    Call {
        memory,
        minute,
        grace,
        stop,
    }
}

#[test]
fn jobs_follow_memory() {
    // req:verify/mutation-pass@1dea303f
    let p = pass(true, false);
    p.f.write("mutate-result", "a/thing killed\n");
    let stop = AtomicBool::new(false);
    let memory = p.f.path("meminfo");
    let cases = [
        (
            "MemTotal:       98765432 kB\nMemAvailable:    5242880 kB\n",
            "mutate_jobs = 6\n",
        ),
        (
            "MemAvailable:   41943040 kB\nSwapTotal: 0 kB\n",
            "mutate_jobs = 6\n",
        ),
        ("MemTotal:       98765432 kB\n", "mutate_jobs = 6\n"),
        ("MemAvailable:   41943040 kB\n", ""),
    ];
    for (n, (meminfo, extra)) in cases.iter().enumerate() {
        std::fs::write(&memory, meminfo).unwrap();
        let (path, test) = test_of(&p.f, "thing");
        p.f.land(
            &[
                ("clm.toml", &with_settings(&p, extra)),
                (&path, &test.replace("let _", &format!("let _n{n}"))),
            ],
            "a changed test",
        );
        let out = p.f.due_with(&call(
            &memory,
            Duration::from_secs(60),
            Duration::from_secs(10),
            &stop,
        ));
        assert!(out.line.starts_with("verify passed "), "{out:?}");
    }
    assert_eq!(p.f.lines("jobs"), ["2", "6", "1", "1"]);
}

/// Whether the process `pid` still runs: a zombie, which has ended and waits only to be
/// reaped, doesn't.
fn running(pid: &str) -> bool {
    std::fs::read_to_string(format!("/proc/{}/stat", pid.trim())).is_ok_and(|stat| {
        stat.rsplit(')')
            .next()
            .is_some_and(|s| !s.trim_start().starts_with('Z'))
    })
}

/// Waits up to ten seconds for `path` to exist.
fn wait_for(path: &std::path::Path) -> bool {
    for _ in 0..200 {
        if path.exists() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    path.exists()
}

#[test]
fn the_time_limit() {
    // req:verify/mutation-pass@1dea303f
    let p = pass(true, false);
    land_due(&p.f, true, &["other"]);
    p.f.land(
        &[("clm.toml", &with_settings(&p, "mutate_minutes = 1\n"))],
        "a limit",
    );
    let dir = p.f.path("");
    write_exec(
        &p.f.path("bin/mutate"),
        &format!(
            "#!/bin/sh\nDIR='{}'\n\
             echo 'a/thing killed' >> \"$CLM_MUTATE_RESULT\"\n\
             sleep 20 &\necho $! > \"$DIR/child\"\necho $$ > \"$DIR/leader\"\n\
             trap '' TERM\nsleep 20\n",
            dir.display().to_string().trim_end_matches('/')
        ),
    );
    let stop = AtomicBool::new(false);
    let memory = p.f.path("meminfo");
    let started = std::time::Instant::now();
    let out = p.f.due_with(&call(
        &memory,
        Duration::from_secs(1),
        Duration::from_secs(1),
        &stop,
    ));
    // The fake ends by itself after 20 seconds, so a pass that fails to end it fails here
    // rather than hanging.
    assert!(
        started.elapsed() < Duration::from_secs(15),
        "{:?}",
        started.elapsed()
    );
    assert!(out.line.starts_with("verify passed "), "{out:?}");
    assert_eq!(out.code, 0);
    let (leader, child) = (p.f.lines("leader"), p.f.lines("child"));
    assert!(!running(&leader[0]), "the command still runs");
    assert!(!running(&child[0]), "its child still runs");
    let [killed, _, _, left] = lists(&p.f);
    assert_eq!(killed, ids(&["a/thing"]));
    assert_eq!(left, ids(&["a/other"]));
    assert!(
        out.notes
            .contains(&"mutation: stopped at its limit of 1 minutes".to_string()),
        "{:?}",
        out.notes
    );
    assert!(!out.notes.iter().any(|n| n.contains("failed with status")));
}

#[test]
fn a_stop_request() {
    // req:verify/mutation-pass@1dea303f
    let p = pass(true, false);
    land_due(&p.f, true, &["other"]);
    let dir = p.f.path("");
    write_exec(
        &p.f.path("bin/mutate"),
        &format!(
            "#!/bin/sh\nDIR='{}'\necho $$ > \"$DIR/leader\"\n\
             echo 'a/thing killed' >> \"$CLM_MUTATE_RESULT\"\n: > \"$DIR/written\"\n\
             i=0\nwhile [ ! -e \"$DIR/never\" ] && [ $i -lt 400 ]; do sleep 0.05; i=$((i + 1)); done\n",
            dir.display().to_string().trim_end_matches('/')
        ),
    );
    let stop = AtomicBool::new(false);
    let memory = p.f.path("meminfo");
    // The fake gives up waiting after about 20 seconds, so a pass that ignores the stop
    // request fails here rather than hanging.
    let started = std::time::Instant::now();
    let out = std::thread::scope(|s| {
        let verifying = s.spawn(|| {
            p.f.due_with(&call(
                &memory,
                Duration::from_secs(60),
                Duration::from_secs(10),
                &stop,
            ))
        });
        assert!(wait_for(&p.f.path("written")), "the line was never written");
        stop.store(true, Ordering::SeqCst);
        verifying.join().unwrap()
    });
    assert!(
        started.elapsed() < Duration::from_secs(15),
        "{:?}",
        started.elapsed()
    );
    assert!(out.line.starts_with("verify passed "), "{out:?}");
    assert!(!running(&p.f.lines("leader")[0]), "the command still runs");
    let [killed, _, _, left] = lists(&p.f);
    assert_eq!(killed, ids(&["a/thing"]));
    assert_eq!(left, ids(&["a/other"]));
    assert!(
        out.notes.contains(&"mutation: stopped".to_string()),
        "{:?}",
        out.notes
    );
}

#[test]
fn an_empty_setting_ends_the_run() {
    // req:verify/mutation-pass@1dea303f
    let (f, version) = unswitched();
    f.write("mutate-result", "a/thing killed\n");
    let set = format!("mutate = \"{}\"", f.path("bin/mutate").display());
    let empty = switched_on(&f).replace(&set, "mutate = \"\"");
    let end = f.land(&[("clm.toml", &empty)], "an empty setting");
    f.land(&[("clm.toml", &switched_on(&f))], "switch the pass on");
    f.land(
        &[("crates/a/tests/x.rs", &test_file(&version, "let _ = 1;"))],
        "a test",
    );
    f.due();
    let Body::Mutated { since, .. } = mutated(&f).pop().unwrap() else {
        panic!()
    };
    assert_eq!(since, Some(end));
}

#[test]
fn the_pass_switched_on_from_the_first_commit() {
    // req:verify/mutation-pass@1dea303f
    let (f, version) = unswitched();
    f.write("mutate-result", "a/thing killed\n");
    f.restart(
        &[
            ("clm.toml", &switched_on(&f)),
            ("crates/a/tests/x.rs", &test_file(&version, "let _ = 1;")),
        ],
        "the first commit",
    );
    let out = f.due();
    assert!(out.line.starts_with("verify passed "), "{out:?}");
    let plan = f.lines("plan");
    assert!(
        plan.contains(&"requirement a/thing".to_string()),
        "{plan:?}"
    );
    assert!(!plan.iter().any(|l| l.starts_with("diff ")), "{plan:?}");
    let Body::Mutated { since, killed, .. } = mutated(&f).pop().unwrap() else {
        panic!()
    };
    assert_eq!((since, killed), (None, vec!["a/thing".to_string()]));
}

#[test]
fn a_test_only_re_cited_is_not_judged() {
    // req:verify/mutation-pass@1dea303f
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
    // req:verify/mutation-pass@1dea303f
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
    // req:verify/mutation-pass@1dea303f
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
    // req:verify/mutation-pass@1dea303f
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
    // req:verify/mutation-pass@1dea303f
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
fn the_pass_starts_from_the_newest_mutated_record() {
    // req:verify/mutation-pass@1dea303f
    let p = pass(true, false);
    p.f.write("mutate-result", "a/thing killed\n");
    let middle = p.f.land(&[("crates/a/src/lib.rs", "9\n")], "middle");
    let body = Body::mutated(middle.clone(), None, vec![], vec![], vec![]);
    clm_ledger::append(&p.f.work(), body, &p.f.env()).unwrap();
    land_new_test(&p);
    p.f.due();
    let Body::Mutated { since, commit, .. } = mutated(&p.f).pop().unwrap() else {
        panic!()
    };
    assert_eq!(since, Some(middle));
    assert_eq!(commit, p.f.back(0));
}

#[test]
fn the_pass_has_a_slot_of_its_own() {
    // req:verify/mutation-pass@1dea303f
    let p = pass(true, false);
    p.f.write("mutate-result", "a/thing killed\n");
    std::fs::write(p.f.work().join("clm.local.toml"), "max_builds = 4\n").unwrap();
    land_new_test(&p);
    p.f.due();
    assert_eq!(p.f.lines("slot"), ["5"]);
}

#[test]
fn unset_runs_no_pass() {
    // req:verify/mutation-pass@1dea303f
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
    // req:verify/mutation-pass@1dea303f
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
    // req:verify/mutation-pass@1dea303f
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
