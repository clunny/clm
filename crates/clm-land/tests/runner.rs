//! The runner `clm land` and `clm land --check-only` run `[land] check` through
//! (land/shared-check-runner).

mod common;

use std::ffi::OsString;
use std::time::{Duration, Instant};

use clm_land::{CheckEnv, Step, run_check};
use common::{Fixture, refused};

fn passes() -> CheckEnv {
    CheckEnv::default()
}

#[test]
fn a_caller_waits_on_the_check_lock() {
    // req:land/shared-check-runner@05214b4c
    let f = Fixture::new("");
    let work = std::fs::canonicalize(f.work()).unwrap();
    let lock_path = work.join(".clm/check.lock");
    std::fs::create_dir_all(lock_path.parent().unwrap()).unwrap();
    let lock = std::fs::File::create(&lock_path).unwrap();
    lock.lock().unwrap();
    let releaser = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(1));
        drop(lock);
    });
    let log = f.path("runner.log");
    let start = Instant::now();
    let run = run_check("echo ok", &work, None, &passes(), Some(&log)).unwrap();
    assert!(start.elapsed() >= Duration::from_millis(900));
    releaser.join().unwrap();
    assert!(run.passed);
    assert!(run.logged);
    assert_eq!(std::fs::read_to_string(&log).unwrap(), "ok\n");
    // The log is the caller's to keep or remove.
    assert!(log.exists());
}

#[test]
fn the_open_step_returned() {
    // req:land/shared-check-runner@05214b4c
    let f = Fixture::new("");
    let script = "echo '::group::a'; echo x; echo '::group::b'; echo y; echo z; exit 1";
    let run = run_check(script, &f.work(), None, &passes(), None).unwrap();
    assert!(!run.passed);
    assert!(!run.logged);
    assert_eq!(
        run.step,
        Some(Step {
            name: "b".into(),
            first: 3,
            last: 5
        })
    );
    assert_eq!(run.tail, ["::group::a", "x", "::group::b", "y", "z"]);
}

#[test]
fn the_selection_and_the_environment_reach_the_command() {
    // req:land/shared-check-runner@05214b4c
    let f = Fixture::new("");
    let selection = f.path("selection");
    std::fs::write(&selection, "file a\n").unwrap();
    let out = f.path("seen");
    let script = format!(
        "echo \"$CLM_SELECTION|${{CI-unset}}|$X|${{Y-unset}}\" > '{}'",
        out.display()
    );
    let env = CheckEnv {
        set: vec![("X".into(), OsString::from("1"))],
        unset: vec!["CI".into(), "Y".into()],
    };
    let run = run_check(&script, &f.work(), Some(&selection), &env, None).unwrap();
    assert!(run.passed);
    assert_eq!(
        std::fs::read_to_string(&out).unwrap(),
        format!("{}|unset|1|unset\n", selection.display())
    );
    // Without a selection the command sees none, whatever the caller's environment holds.
    let script = format!("echo \"${{CLM_SELECTION-unset}}\" > '{}'", out.display());
    run_check(&script, &f.work(), None, &passes(), None).unwrap();
    assert_eq!(std::fs::read_to_string(&out).unwrap(), "unset\n");
}

#[test]
fn land_unchanged() {
    // req:land/shared-check-runner@05214b4c
    let f = Fixture::new("");
    f.write("check-prints", "for i in $(seq 50); do echo $i; done\n");
    f.write("check-fails", "");
    f.branch("impl/a", &[("b.txt", "b\n")]);
    let why = refused(clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])));
    let lines: Vec<&str> = why.lines().collect();
    let header = format!("last 40 lines of `{}`:", f.check_command());
    assert_eq!(lines[0], header, "{why}");
    let tail: Vec<String> = (11..=50).map(|i| i.to_string()).collect();
    assert_eq!(lines[1..41], tail, "{why}");
    let log = std::fs::canonicalize(f.work())
        .unwrap()
        .join(".clm/land-logs/impl/a.log");
    assert!(
        lines[41].ends_with(&format!("; its output is at {}", log.display())),
        "{why}"
    );
    assert_eq!(lines.len(), 42, "{why}");
}
