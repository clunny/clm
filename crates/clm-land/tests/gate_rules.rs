//! Every merge commit a land pushes names the gate's rules version (land/gate-rules-trailer).

mod common;

use std::time::{Duration, Instant};

use common::Fixture;

fn trailer() -> String {
    format!("Gate-Rules: {}", clm_gate::RULES)
}

/// The message of `rev` on `origin`, and what `%(trailers:key=Gate-Rules,valueonly)` reads.
fn message(f: &Fixture, rev: &str) -> (String, String) {
    let origin = f.path("origin.git");
    let message = f.git(&origin, &["log", "-1", "--format=%B", rev]);
    let value = f.git(
        &origin,
        &[
            "log",
            "-1",
            "--format=%(trailers:key=Gate-Rules,valueonly)",
            rev,
        ],
    );
    (message, value)
}

#[test]
fn a_code_land_ends_with_the_trailer() {
    // req:land/gate-rules-trailer@639a8a6a
    let f = Fixture::new("");
    f.branch("impl/a", &[("src/x.rs", "x\n")]);
    clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    let (message, value) = message(&f, &f.remote("main"));
    assert_eq!(message, format!("Merge impl/a into main\n\n{}", trailer()));
    assert_eq!(value, clm_gate::RULES.to_string());
}

#[test]
fn a_spec_land_puts_it_after_its_change() {
    // req:land/gate-rules-trailer@639a8a6a
    let f = Fixture::new("");
    f.write("gate-out", "spec\n");
    f.branch("spec/x", &[("changes/x/change.md", "x\n")]);
    clm_land::land(&f.work(), Some("spec/x"), &f.env(&[])).unwrap();
    let (message, value) = message(&f, &f.remote("main"));
    assert_eq!(
        message,
        format!("Merge spec/x into main\n\nChange: x\n{}", trailer())
    );
    assert_eq!(value, clm_gate::RULES.to_string());
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
fn lands_taken_together_each_end_with_it_after_checked() {
    // req:land/gate-rules-trailer@639a8a6a
    let f = Fixture::new("");
    for n in ["a", "b"] {
        f.branch(&format!("impl/{n}"), &[(&format!("{n}.txt"), "x\n")]);
    }
    std::fs::create_dir_all(f.work().join(".clm")).unwrap();
    let held = std::fs::File::create(f.work().join(".clm/land.lock")).unwrap();
    held.lock().unwrap();
    let env = f.env(&[]);
    std::thread::scope(|s| {
        let mut runs = Vec::new();
        for (i, n) in ["a", "b"].iter().enumerate() {
            let (env, work) = (&env, f.work());
            let branch = format!("impl/{n}");
            runs.push(s.spawn(move || clm_land::land(&work, Some(&branch), env)));
            let deadline = Instant::now() + Duration::from_secs(30);
            while tickets(&f) < i + 1 {
                assert!(Instant::now() < deadline, "no ticket");
                std::thread::sleep(Duration::from_millis(20));
            }
        }
        drop(held);
        for r in runs {
            r.join().unwrap().unwrap();
        }
    });
    let checks = f.lines("check.log").len();
    assert_eq!(checks, 1, "the two landed on one check");
    let main = f.remote("main");
    for (rev, branch) in [(format!("{main}^"), "impl/a"), (main.clone(), "impl/b")] {
        let (message, value) = message(&f, &rev);
        let lines: Vec<&str> = message.lines().collect();
        assert_eq!(lines[0], format!("Merge {branch} into main"));
        let n = lines.len();
        assert!(lines[n - 2].starts_with("Checked: "), "{message}");
        assert_eq!(lines[n - 1], trailer(), "{message}");
        assert_eq!(value, clm_gate::RULES.to_string());
    }
}
