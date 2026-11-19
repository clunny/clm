//! A code branch loses no citing test no approved list names (land/deleted-tests-approved).

mod common;

use common::{Fixture, refused};

/// A Rust test named `name` citing `a/x`; the marker is built so the literal never appears
/// whole in this file.
fn test(name: &str) -> String {
    format!(
        "#[test]\nfn {name}() {{\n    // {}\n    assert!(true);\n}}\n",
        concat!("req", ":a/x")
    )
}

/// `tests/t.rs` holding `one` and `two` on `main`, pushed.
fn with_tests() -> Fixture {
    let f = Fixture::new("");
    f.commit_files(
        &[("tests/t.rs", &format!("{}\n{}", test("one"), test("two")))],
        "tests",
    );
    f.git(&f.work(), &["push", "-q", "origin", "main"]);
    f
}

fn push_main(f: &Fixture, files: &[(&str, &str)]) {
    f.commit_files(files, "main");
    f.git(&f.work(), &["push", "-q", "origin", "main"]);
}

const LINE: &str = "tests/t.rs: land/deleted-tests-approved: test two, which cited a/x, is gone \
                    and no approved list names it. Keep the test under its name, or under a new \
                    name in its file citing every requirement it cited, or leave the requirement \
                    unbuilt and add a question item";

#[test]
fn a_deleted_test_refuses_the_land() {
    // req:land/deleted-tests-approved@f9eddc3a
    let f = with_tests();
    f.branch("impl/a", &[("tests/t.rs", &test("one"))]);
    let old = f.remote("main");
    let records = f.records().len();
    let why = refused(clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])));
    assert_eq!(why, LINE);
    assert!(f.lines("check.log").is_empty());
    assert!(f.lines("clm-check.log").is_empty());
    assert!(f.lines("gate.log").is_empty());
    assert_eq!(f.remote("main"), old);
    assert_eq!(f.records().len(), records);
}

#[test]
fn a_test_an_approved_list_names_lands() {
    // req:land/deleted-tests-approved@f9eddc3a
    let f = with_tests();
    push_main(
        &f,
        &[(
            "changes/c/deleted-tests.txt",
            "# gone\nDELETE tests/t.rs::two\n",
        )],
    );
    f.branch("impl/a", &[("tests/t.rs", &test("one"))]);
    clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    assert_eq!(f.landed().len(), 1);
}

#[test]
fn a_list_in_the_archive_lands() {
    // req:land/deleted-tests-approved@f9eddc3a
    let f = with_tests();
    push_main(
        &f,
        &[(
            "changes/archive/c/deleted-tests.txt",
            "DELETE tests/t.rs::two\n",
        )],
    );
    f.branch("impl/a", &[("tests/t.rs", &test("one"))]);
    clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    assert_eq!(f.landed().len(), 1);
}

#[test]
fn a_list_on_the_branch_counts_for_nothing() {
    // req:land/deleted-tests-approved@f9eddc3a
    let f = with_tests();
    f.branch(
        "tidy",
        &[
            ("tests/t.rs", &test("one")),
            ("changes/c/deleted-tests.txt", "DELETE tests/t.rs::two\n"),
        ],
    );
    let why = refused(clm_land::land(&f.work(), Some("tidy"), &f.env(&[])));
    assert_eq!(why, LINE);
}

#[test]
fn a_list_naming_another_file_or_test_counts_for_nothing() {
    // req:land/deleted-tests-approved@f9eddc3a
    let f = with_tests();
    push_main(
        &f,
        &[(
            "changes/c/deleted-tests.txt",
            "DELETE tests/u.rs::two\nDELETE tests/t.rs::one\n",
        )],
    );
    f.branch("impl/a", &[("tests/t.rs", &test("one"))]);
    let why = refused(clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])));
    assert_eq!(why, LINE);
}

#[test]
fn a_moved_test_lands() {
    // req:land/deleted-tests-approved@f9eddc3a
    let f = with_tests();
    f.branch(
        "impl/a",
        &[("tests/t.rs", &test("one")), ("tests/u.rs", &test("two"))],
    );
    clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    assert_eq!(f.landed().len(), 1);
}

#[test]
fn a_renamed_test_refuses_the_land() {
    // req:land/deleted-tests-approved@f9eddc3a
    // Renamed with its marker taken off, `two` is gone: the new name cites nothing.
    let f = with_tests();
    let second = "#[test]\nfn second() {\n    assert!(true);\n}\n";
    f.branch(
        "tidy",
        &[("tests/t.rs", &format!("{}\n{second}", test("one")))],
    );
    let why = refused(clm_land::land(&f.work(), Some("tidy"), &f.env(&[])));
    assert_eq!(why, LINE);
}

#[test]
fn a_renamed_test_keeping_its_marker_lands() {
    // req:land/deleted-tests-approved@f9eddc3a
    let f = with_tests();
    f.branch(
        "tidy",
        &[(
            "tests/t.rs",
            &format!("{}\n{}", test("one"), test("second")),
        )],
    );
    clm_land::land(&f.work(), Some("tidy"), &f.env(&[])).unwrap();
    assert_eq!(f.landed().len(), 1);
}

#[test]
fn a_deleted_file_names_each_test_it_held() {
    // req:land/deleted-tests-approved@f9eddc3a
    let f = with_tests();
    f.git(&f.work(), &["checkout", "-q", "-b", "impl/a"]);
    f.git(&f.work(), &["rm", "-q", "tests/t.rs"]);
    f.git(&f.work(), &["commit", "-q", "-m", "drop"]);
    f.git(&f.work(), &["push", "-q", "origin", "impl/a"]);
    f.git(&f.work(), &["checkout", "-q", "main"]);
    let why = refused(clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])));
    let lines: Vec<&str> = why.lines().collect();
    assert_eq!(lines.len(), 2, "{why}");
    assert!(lines[0].starts_with("tests/t.rs: land/deleted-tests-approved: test one,"));
    assert_eq!(lines[1], LINE);
}

/// `docs/t.test.ts` holding two tests citing `a/x`: a file under a spec path, so a `spec/*`
/// branch may change it.
fn ts(names: &[&str]) -> String {
    names
        .iter()
        .map(|n| format!("test(\"{n} {}\", () => {{}});\n", concat!("req", ":a/x")))
        .collect()
}

#[test]
fn a_spec_branch_is_not_judged() {
    // req:land/deleted-tests-approved@f9eddc3a
    let f = Fixture::new("");
    push_main(&f, &[("docs/t.test.ts", &ts(&["one", "two"]))]);
    // The marker in a title rewritten, as `clm rename` rewrites it, keeps the test's name.
    let renamed = ts(&["one", "two"]).replace(concat!("req", ":a/x"), concat!("req", ":a/y"));
    f.branch("spec/r", &[("docs/t.test.ts", &renamed)]);
    clm_land::land(&f.work(), Some("spec/r"), &f.env(&[])).unwrap();
    // A spec branch that loses a test isn't judged either; the same edit on another name is.
    let g = Fixture::new("");
    push_main(&g, &[("docs/t.test.ts", &ts(&["one", "two"]))]);
    g.branch("spec/s", &[("docs/t.test.ts", &ts(&["one"]))]);
    g.branch("tidy", &[("docs/t.test.ts", &ts(&["one"]))]);
    clm_land::land(&g.work(), Some("spec/s"), &g.env(&[])).unwrap();
    let why = refused(clm_land::land(&g.work(), Some("tidy"), &g.env(&[])));
    assert!(
        why.starts_with("docs/t.test.ts: land/deleted-tests-approved: test two "),
        "{why}"
    );
}

#[test]
fn the_pre_check_prints_the_same_line() {
    // req:land/deleted-tests-approved@f9eddc3a
    let f = with_tests();
    f.branch("impl/a", &[("tests/t.rs", &test("one"))]);
    f.git(&f.work(), &["checkout", "-q", "impl/a"]);
    let why = refused(clm_land::check_only(&f.work(), None, &f.env(&[])));
    assert_eq!(why, LINE);
    assert!(f.lines("check.log").is_empty());
    assert!(f.lines("clm-check.log").is_empty());
}

#[test]
fn the_pre_check_judges_a_detached_head() {
    // req:land/deleted-tests-approved@f9eddc3a
    let f = with_tests();
    f.branch("impl/a", &[("tests/t.rs", &test("one"))]);
    f.git(&f.work(), &["checkout", "-q", "--detach", "impl/a"]);
    let why = refused(clm_land::check_only(&f.work(), None, &f.env(&[])));
    assert_eq!(why, LINE);
    assert!(f.lines("check.log").is_empty());
}

#[test]
fn the_pre_check_on_a_spec_branch_judges_nothing() {
    // req:land/deleted-tests-approved@f9eddc3a
    let f = with_tests();
    f.branch("spec/a", &[("tests/t.rs", &test("one"))]);
    f.git(&f.work(), &["checkout", "-q", "spec/a"]);
    let out = clm_land::check_only(&f.work(), None, &f.env(&[])).unwrap();
    assert_eq!(out.lines.last().unwrap(), "checks pass");
}

#[test]
fn the_pre_check_with_no_origin_main_judges_nothing() {
    // req:land/deleted-tests-approved@f9eddc3a
    let f = with_tests();
    f.branch("impl/a", &[("tests/t.rs", &test("one"))]);
    f.git(&f.work(), &["checkout", "-q", "impl/a"]);
    f.git(&f.work(), &["update-ref", "-d", "refs/remotes/origin/main"]);
    let out = clm_land::check_only(&f.work(), None, &f.env(&[])).unwrap();
    assert_eq!(out.lines.last().unwrap(), "checks pass");
}

#[test]
fn a_root_in_a_subdirectory_reads_paths_from_the_root() {
    // req:land/deleted-tests-approved@f9eddc3a
    let f = Fixture::nested();
    let proj = f.work();
    f.commit_files(
        &[
            ("tests/t.rs", &format!("{}\n{}", test("one"), test("two"))),
            ("changes/c/deleted-tests.txt", "DELETE tests/t.rs::two\n"),
        ],
        "tests",
    );
    f.git(&proj, &["push", "-q", "origin", "main"]);
    f.branch("impl/a", &[("tests/t.rs", &test("one"))]);
    clm_land::land(&proj, Some("impl/a"), &f.env(&[])).unwrap();
    assert_eq!(f.landed().len(), 1);
}

#[test]
fn a_root_in_a_subdirectory_names_the_file_from_the_root() {
    // req:land/deleted-tests-approved@f9eddc3a
    let f = Fixture::nested();
    let proj = f.work();
    f.commit_files(
        &[("tests/t.rs", &format!("{}\n{}", test("one"), test("two")))],
        "tests",
    );
    f.git(&proj, &["push", "-q", "origin", "main"]);
    f.branch("impl/a", &[("tests/t.rs", &test("one"))]);
    let why = refused(clm_land::land(&proj, Some("impl/a"), &f.env(&[])));
    assert_eq!(why, LINE);
}

/// The tickets in the land lock's queue.
fn tickets(f: &Fixture) -> usize {
    std::fs::read_dir(f.work().join(".clm/land.queue"))
        .map(|dir| {
            dir.filter_map(Result::ok)
                .filter(|e| {
                    let name = e.file_name();
                    let name = name.to_string_lossy();
                    name.contains("-ordinary-p") || name.contains("-priority-p")
                })
                .count()
        })
        .unwrap_or(0)
}

#[test]
fn a_waiting_land_that_lost_a_test_is_not_taken_with_another() {
    // req:land/deleted-tests-approved@f9eddc3a
    let f = with_tests();
    f.branch("impl/a", &[("a.txt", "a2\n")]);
    f.branch("impl/b", &[("b.txt", "b\n")]);
    std::fs::create_dir_all(f.work().join(".clm")).unwrap();
    let held = std::fs::File::create(f.work().join(".clm/land.lock")).unwrap();
    held.lock().unwrap();
    let env = f.env(&[]);
    let outs: Vec<_> = std::thread::scope(|s| {
        let mut runs = Vec::new();
        for (i, n) in ["a", "b"].iter().enumerate() {
            let (env, work) = (&env, f.work());
            let branch = format!("impl/{n}");
            runs.push(s.spawn(move || clm_land::land(&work, Some(&branch), env)));
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
            while tickets(&f) < i + 1 {
                assert!(std::time::Instant::now() < deadline, "no ticket");
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
        }
        // While both wait, `impl/b` loses a citing test: the holder judges it as it takes it.
        f.git(&f.work(), &["checkout", "-q", "impl/b"]);
        f.commit_files(&[("tests/t.rs", &test("one"))], "lose two");
        f.git(&f.work(), &["push", "-q", "origin", "impl/b"]);
        f.git(&f.work(), &["checkout", "-q", "main"]);
        drop(held);
        runs.into_iter().map(|r| r.join().unwrap()).collect()
    });
    let mut outs = outs.into_iter();
    outs.next().unwrap().unwrap();
    assert_eq!(refused(outs.next().unwrap()), LINE);
    f.git(&f.work(), &["fetch", "-q", "origin"]);
    let subjects = f.git(
        &f.work(),
        &["log", "--first-parent", "--format=%s", "origin/main"],
    );
    assert!(subjects.contains("Merge impl/a into main"), "{subjects}");
    assert!(!subjects.contains("impl/b"), "{subjects}");
}
