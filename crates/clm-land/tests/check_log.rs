//! `[land] check`'s output kept in a log and its tail repeated (land/check-log).

mod common;

use common::{Fixture, refused};

/// The main worktree's log for `impl/a`, as git names the worktree.
fn log(f: &Fixture) -> std::path::PathBuf {
    std::fs::canonicalize(f.work())
        .unwrap()
        .join(".clm/land-logs/impl/a.log")
}

#[test]
fn failing_checks_tail_and_log() {
    // req:land/check-log@6fefb1a1
    let f = Fixture::new("");
    f.write(
        "check-prints",
        "for i in $(seq 100); do if [ $((i % 2)) = 1 ]; then echo $i; else echo $i >&2; fi; done\n",
    );
    f.write("check-fails", "");
    f.branch("impl/a", &[("b.txt", "b\n")]);
    let why = refused(clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])));
    let check = f.check_command();
    let lines: Vec<&str> = why.lines().collect();
    let header = format!("last 40 lines of `{check}`:");
    let at = lines.iter().position(|l| *l == header).expect(&why);
    let tail: Vec<String> = (61..=100).map(|i| i.to_string()).collect();
    assert_eq!(lines[at + 1..at + 41], tail, "{why}");
    let path = log(&f);
    assert_eq!(
        lines.last().unwrap(),
        &format!(
            "`{check}` fails on the merge with main; its output is at {}",
            path.display()
        ),
        "{why}"
    );
    assert_eq!(lines.len(), at + 42, "{why}");
    let all: Vec<String> = (1..=100).map(|i| i.to_string()).collect();
    let text = std::fs::read_to_string(&path).unwrap();
    assert_eq!(text.lines().collect::<Vec<_>>(), all);
}

#[test]
fn log_under_the_main_worktree_from_a_linked_one() {
    // req:land/check-log@6fefb1a1
    let f = Fixture::new("");
    f.write("check-prints", "echo broke\n");
    f.write("check-fails", "");
    f.branch("impl/a", &[("b.txt", "b\n")]);
    let linked = f.linked("linked", "impl/a");
    let why = refused(clm_land::land(&linked, None, &f.env(&[])));
    assert!(why.ends_with(&log(&f).display().to_string()), "{why}");
    assert_eq!(std::fs::read_to_string(log(&f)).unwrap(), "broke\n");
    assert!(!linked.join(".clm").exists());
}

#[test]
fn short_output_repeated_whole() {
    // req:land/check-log@6fefb1a1
    let f = Fixture::new("");
    // The last line has no line break, and still counts.
    f.write("check-prints", "echo one; echo two >&2; printf three\n");
    f.write("check-fails", "");
    f.branch("impl/a", &[("b.txt", "b\n")]);
    let why = refused(clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])));
    let header = format!("last 40 lines of `{}`:", f.check_command());
    let after = why.split_once(&format!("{header}\n")).expect(&why).1;
    let lines: Vec<&str> = after.lines().collect();
    assert_eq!(lines[..3], ["one", "two", "three"], "{why}");
    assert_eq!(lines.len(), 4, "{why}");
    assert!(log(&f).exists());
}

#[test]
fn passing_check_leaves_no_log() {
    // req:land/check-log@6fefb1a1
    let f = Fixture::new("");
    f.write("check-prints", "echo fine\n");
    f.branch("impl/a", &[("b.txt", "b\n")]);
    // A log a failed land left is truncated, then removed once the check passes.
    let path = log(&f);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, "old\n").unwrap();
    clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    assert!(!path.exists());
    assert!(path.parent().unwrap().exists(), "its directories stay");
}

#[test]
fn log_cant_be_created() {
    // req:land/check-log@6fefb1a1
    let f = Fixture::new("");
    f.write("check-fails", "");
    f.write("check-prints", "echo broke\n");
    f.branch("impl/a", &[("b.txt", "b\n")]);
    let home = f.work().join(".clm");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::write(home.join("land-logs"), "a file").unwrap();
    let why = refused(clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])));
    let lines: Vec<&str> = why.lines().collect();
    assert!(lines.contains(&"broke"), "{why}");
    assert_eq!(
        lines.last().unwrap(),
        &format!("`{}` fails on the merge with main", f.check_command()),
        "{why}"
    );
}

/// What a failing land of `impl/a` printed, from the check's output `prints`.
fn fails_printing(prints: &str) -> (Fixture, String) {
    let f = Fixture::new("");
    f.write("check-prints", prints);
    f.write("check-fails", "");
    f.branch("impl/a", &[("b.txt", "b\n")]);
    let why = refused(clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])));
    (f, why)
}

#[test]
fn the_failing_step_named() {
    // req:land/check-log@6fefb1a1
    let (f, why) = fails_printing(
        "echo '::group::unit'; for i in 1 2 3 4 5; do echo u$i; done; echo '::endgroup::'\n\
         echo '::group::e2e'; echo boom; for i in $(seq 60); do echo e$i; done\n",
    );
    let header = format!("last 40 lines of `{}`:", f.check_command());
    let lines: Vec<&str> = why.lines().collect();
    let at = lines.iter().position(|l| *l == header).expect(&why);
    assert_eq!(lines[at + 1], "e21", "{why}");
    assert_eq!(lines[at + 40], "e60", "{why}");
    assert_eq!(
        lines.last().unwrap(),
        &format!(
            "`{}` fails on the merge with main; the step open when it ended is `e2e`, lines 8 \
             to 69 of its output; its output is at {}",
            f.check_command(),
            log(&f).display()
        ),
        "{why}"
    );
    assert_eq!(lines.len(), at + 42, "{why}");
}

#[test]
fn a_step_that_ended_isnt_named() {
    // req:land/check-log@6fefb1a1
    let (f, why) =
        fails_printing("echo '::group::unit'; echo x; echo '::endgroup::  '; echo boom\n");
    assert_eq!(
        why.lines().last().unwrap(),
        format!(
            "`{}` fails on the merge with main; its output is at {}",
            f.check_command(),
            log(&f).display()
        ),
        "{why}"
    );
    assert!(!why.contains("step open"), "{why}");
}

#[test]
fn a_group_marker_split_across_writes() {
    // req:land/check-log@6fefb1a1
    let (_f, why) = fails_printing("printf '::gro'; sleep 0.2; printf 'up::e2e\\n'; echo boom\n");
    assert!(
        why.contains("the step open when it ended is `e2e`, lines 1 to 2 of its output"),
        "{why}"
    );
}

#[test]
fn a_group_line_ends_the_step_before_it() {
    // req:land/check-log@6fefb1a1
    let (_f, why) =
        fails_printing("echo '::group::a'; echo x; echo '::group:: b '; echo y; echo z\n");
    assert!(
        why.contains("the step open when it ended is `b`, lines 3 to 5 of its output"),
        "{why}"
    );
}
