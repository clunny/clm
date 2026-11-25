//! No branch loses a citing test no approved list names (land/deleted-tests-approved).

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
                    and no approved list names it. Keep the test under its name, citing any \
                    requirement, even another than it cited; or rename it in its file to a name \
                    no test carried at the merge base, citing every requirement it cited; or \
                    leave the requirement unbuilt and add a question item";

#[test]
fn a_deleted_test_refuses_the_land() {
    // req:land/deleted-tests-approved@00650f33
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
    // req:land/deleted-tests-approved@00650f33
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
    // req:land/deleted-tests-approved@00650f33
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
    // req:land/deleted-tests-approved@00650f33
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
    // req:land/deleted-tests-approved@00650f33
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
    // req:land/deleted-tests-approved@00650f33
    let f = with_tests();
    f.branch(
        "impl/a",
        &[("tests/t.rs", &test("one")), ("tests/u.rs", &test("two"))],
    );
    clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    assert_eq!(f.landed().len(), 1);
}

#[test]
fn a_recited_test_lands() {
    // req:land/deleted-tests-approved@00650f33
    // `two` keeps its name and cites `b/y` in place of `a/x`: no test is gone.
    let f = with_tests();
    let two = test("two").replace(concat!("req", ":a/x"), concat!("req", ":b/y"));
    f.branch(
        "impl/a",
        &[("tests/t.rs", &format!("{}\n{two}", test("one")))],
    );
    clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    assert_eq!(f.landed().len(), 1);
}

#[test]
fn a_renamed_test_refuses_the_land() {
    // req:land/deleted-tests-approved@00650f33
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
    // req:land/deleted-tests-approved@00650f33
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

/// `with_tests`' `main` also holding `tests/u.rs` with a test `kept` citing `a/x`, and a branch
/// `tidy` renaming `two` to `kept` in `tests/t.rs`, keeping its marker.
fn renamed_to_a_name_another_file_carries() -> Fixture {
    let f = with_tests();
    push_main(&f, &[("tests/u.rs", &test("kept"))]);
    f.branch(
        "tidy",
        &[("tests/t.rs", &format!("{}\n{}", test("one"), test("kept")))],
    );
    f
}

#[test]
fn a_new_name_another_file_carries_refuses_the_land() {
    // req:land/deleted-tests-approved@00650f33
    let f = renamed_to_a_name_another_file_carries();
    let why = refused(clm_land::land(&f.work(), Some("tidy"), &f.env(&[])));
    assert_eq!(why, LINE);
    // As `clm coverage --deleted-tests` lists it, run on the branch's checkout, given the
    // root spec it loads (untracked, so it changes no tree the land read).
    f.git(&f.work(), &["checkout", "-q", "tidy"]);
    std::fs::create_dir_all(f.work().join("specs")).unwrap();
    std::fs::write(
        f.work().join("specs/SPEC.md"),
        "---\ntitle: R\nstatus: active\nowners: []\n---\n",
    )
    .unwrap();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = clm_check::coverage_command(
        ["--deleted-tests", "origin/main"],
        &f.work(),
        &|_| None,
        &mut out,
        &mut err,
    );
    let out = String::from_utf8(out).unwrap();
    assert_eq!(code, 0, "{out}{}", String::from_utf8_lossy(&err));
    let listed: Vec<&str> = out.lines().filter(|l| l.starts_with("DELETED ")).collect();
    assert_eq!(listed, ["DELETED tests/t.rs::two\ta/x"], "{out}");
}

#[test]
fn the_pre_check_refuses_a_new_name_another_file_carries() {
    // req:land/deleted-tests-approved@00650f33
    let f = renamed_to_a_name_another_file_carries();
    f.git(&f.work(), &["checkout", "-q", "tidy"]);
    let why = refused(clm_land::check_only(&f.work(), None, &f.env(&[])));
    assert_eq!(why, LINE);
    assert!(f.lines("check.log").is_empty());
}

#[test]
fn a_deleted_file_names_each_test_it_held() {
    // req:land/deleted-tests-approved@00650f33
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

const ROOT_SPEC: &str = "---\ntitle: Root\nstatus: active\nowners: []\n---\n\n## Contract\nroot\n\n## Design\nroot design\n\n## Requirements\n";
const A_SPEC: &str = "---\ntitle: A\nstatus: specd\nowners: [\"@a\"]\n---\n\n## Contract\nc\n\n## Design\nd\n\n## Requirements\n\n### Requirement: x\nX SHALL hold.\n\n#### Scenario: holds\n- ok\n";

#[test]
fn a_spec_branch_is_not_judged() {
    // req:land/deleted-tests-approved@00650f33
    // `spec/r` holds only the committed result of `clm rename r`, which rewrote the marker in
    // a test's title: a replay, which this rule leaves alone.
    let f = Fixture::new("");
    push_main(
        &f,
        &[
            ("specs/SPEC.md", ROOT_SPEC),
            ("specs/a/SPEC.md", A_SPEC),
            ("a.test.ts", &ts(&["one", "two"])),
        ],
    );
    let work = f.work();
    f.git(&work, &["checkout", "-q", "-b", "spec/r"]);
    let change = "---\nstatus: proposed\nrenames:\n  - requirement: a/x\n    to: y\n---\n\n## Intent\nRename.\n";
    std::fs::create_dir_all(work.join("changes/r")).unwrap();
    std::fs::write(work.join("changes/r/change.md"), change).unwrap();
    f.git(&work, &["add", "-A"]);
    clm_rename::rename(&work, "r").unwrap();
    f.git(&work, &["add", "-A"]);
    f.git(&work, &["commit", "-q", "-m", "rename"]);
    let text = std::fs::read_to_string(work.join("a.test.ts")).unwrap();
    assert!(text.contains(concat!("req", ":a/y")), "{text}");
    f.git(&work, &["push", "-q", "origin", "spec/r"]);
    f.git(&work, &["checkout", "-q", "main"]);
    f.write("gate-out", "spec\n");
    clm_land::land(&work, Some("spec/r"), &f.env(&[])).unwrap();
    assert_eq!(f.landed().len(), 1);
}

#[test]
fn a_spec_branch_that_deletes_a_test() {
    // req:land/deleted-tests-approved@00650f33
    // A spec branch is judged as any other, and so is a branch whose only other change is an
    // approval edit, which vouches for no test.
    let f = Fixture::new("approval = \"other\"");
    push_main(&f, &[("docs/t.test.ts", &ts(&["one", "two"]))]);
    f.branch("spec/s", &[("docs/t.test.ts", &ts(&["one"]))]);
    let toml = std::fs::read_to_string(f.work().join("clm.toml"))
        .unwrap()
        .replace("approval = \"other\"", "approval = \"any\"");
    f.branch(
        "t",
        &[("docs/t.test.ts", &ts(&["one"])), ("clm.toml", &toml)],
    );
    f.write("gate-out", "spec\n");
    let old = f.remote("main");
    for branch in ["spec/s", "t"] {
        let why = refused(clm_land::land(&f.work(), Some(branch), &f.env(&[])));
        assert!(
            why.starts_with("docs/t.test.ts: land/deleted-tests-approved: test two "),
            "{branch}: {why}"
        );
    }
    // The pre-check, in a checkout of `spec/s`, before its check command runs.
    f.git(&f.work(), &["checkout", "-q", "spec/s"]);
    let why = refused(clm_land::check_only(&f.work(), None, &f.env(&[])));
    assert!(
        why.starts_with("docs/t.test.ts: land/deleted-tests-approved: test two "),
        "{why}"
    );
    assert!(f.lines("check.log").is_empty());
    assert_eq!(f.remote("main"), old);
    assert!(f.landed().is_empty());
}

#[test]
fn what_it_doesnt_read() {
    // req:land/deleted-tests-approved@00650f33
    // A branch touching no file that can hold a test, a binary among them.
    let f = with_tests();
    let work = f.work();
    f.git(&work, &["checkout", "-q", "-b", "docs"]);
    std::fs::create_dir_all(work.join("assets")).unwrap();
    std::fs::write(
        work.join("assets/logo.png"),
        [0x89, b'P', b'N', b'G', 0xff, 0],
    )
    .unwrap();
    f.commit_files(&[("README.md", "read me\n")], "docs");
    f.git(&work, &["push", "-q", "origin", "docs"]);
    f.git(&work, &["checkout", "-q", "main"]);
    clm_land::land(&work, Some("docs"), &f.env(&[])).unwrap();
    // A test file that isn't UTF-8 holds no test, so deleting it loses none.
    let g = Fixture::new("");
    let mut bytes = test("two").into_bytes();
    bytes.extend([0xff, 0xfe, b'\n']);
    std::fs::create_dir_all(g.work().join("tests")).unwrap();
    std::fs::write(g.work().join("tests/bin.rs"), bytes).unwrap();
    push_main(&g, &[]);
    let work = g.work();
    g.git(&work, &["checkout", "-q", "-b", "tidy"]);
    g.git(&work, &["rm", "-q", "tests/bin.rs"]);
    g.git(&work, &["commit", "-q", "-m", "drop"]);
    g.git(&work, &["push", "-q", "origin", "tidy"]);
    g.git(&work, &["checkout", "-q", "main"]);
    clm_land::land(&work, Some("tidy"), &g.env(&[])).unwrap();
    assert_eq!(f.landed().len() + g.landed().len(), 2);
}

#[test]
fn the_pre_check_prints_the_same_line() {
    // req:land/deleted-tests-approved@00650f33
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
    // req:land/deleted-tests-approved@00650f33
    let f = with_tests();
    f.branch("impl/a", &[("tests/t.rs", &test("one"))]);
    f.git(&f.work(), &["checkout", "-q", "--detach", "impl/a"]);
    let why = refused(clm_land::check_only(&f.work(), None, &f.env(&[])));
    assert_eq!(why, LINE);
    assert!(f.lines("check.log").is_empty());
}

#[test]
fn the_pre_check_passes_a_renamed_test_keeping_its_marker() {
    // req:land/deleted-tests-approved@00650f33
    let f = with_tests();
    f.branch(
        "tidy",
        &[(
            "tests/t.rs",
            &format!("{}\n{}", test("one"), test("second")),
        )],
    );
    f.git(&f.work(), &["checkout", "-q", "tidy"]);
    let out = clm_land::check_only(&f.work(), None, &f.env(&[])).unwrap();
    assert_eq!(out.lines.last().unwrap(), "checks pass");
}

#[test]
fn the_pre_check_with_no_origin_main_judges_nothing() {
    // req:land/deleted-tests-approved@00650f33
    let f = with_tests();
    f.branch("impl/a", &[("tests/t.rs", &test("one"))]);
    f.git(&f.work(), &["checkout", "-q", "impl/a"]);
    f.git(&f.work(), &["update-ref", "-d", "refs/remotes/origin/main"]);
    let out = clm_land::check_only(&f.work(), None, &f.env(&[])).unwrap();
    assert_eq!(out.lines.last().unwrap(), "checks pass");
}

#[test]
fn a_root_in_a_subdirectory_reads_paths_from_the_root() {
    // req:land/deleted-tests-approved@00650f33
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
    // req:land/deleted-tests-approved@00650f33
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
    // req:land/deleted-tests-approved@00650f33
    // req:land/batched-lands@ba7cbebe
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
