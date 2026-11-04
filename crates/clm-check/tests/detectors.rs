use std::fs;
use std::path::Path;

use clm_check::{Detector, detectors, run};
use tempfile::TempDir;

fn detector(name: &str, command: &[&str], hint: Option<&str>) -> Detector {
    Detector {
        name: name.into(),
        command: command.iter().map(|s| s.to_string()).collect(),
        hint: hint.map(Into::into),
        timeout: 600,
    }
}

/// Whether a process with ID `pid` is still running (and not a zombie).
#[cfg(unix)]
fn running(pid: &str) -> bool {
    let status = fs::read_to_string(format!("/proc/{pid}/stat"));
    match status {
        Ok(stat) => !stat.contains(") Z "),
        Err(_) => std::process::Command::new("kill")
            .args(["-0", pid])
            .stderr(std::process::Stdio::null())
            .status()
            .unwrap()
            .success(),
    }
}

#[cfg(unix)]
#[test]
fn a_hanging_detector_is_stopped_with_its_children() {
    // req:check/detectors@8bf99b6d
    let dir = TempDir::new().unwrap();
    let mut hang = detector(
        "hang",
        &[
            "sh",
            "-c",
            "echo started; sleep 30 & echo $! > child.pid; wait",
        ],
        None,
    );
    hang.timeout = 1;
    let list = [hang, detector("bad", &["sh", "-c", "exit 3"], None)];
    let started = std::time::Instant::now();
    let messages: Vec<_> = found(dir.path(), &list)
        .into_iter()
        .map(|(_, _, m)| m)
        .collect();
    assert!(started.elapsed().as_secs() < 10, "{:?}", started.elapsed());
    assert_eq!(messages, ["hang timed out after 1s", "bad failed (exit 3)"]);
    let pid = fs::read_to_string(dir.path().join("child.pid")).unwrap();
    assert!(
        !running(pid.trim()),
        "the detector's child {pid} still runs"
    );
}

/// `(path, rule, message)` of each violation.
fn found(root: &Path, list: &[Detector]) -> Vec<(String, &'static str, String)> {
    detectors(root, list)
        .into_iter()
        .map(|v| (v.path.display().to_string(), v.rule, v.message))
        .collect()
}

fn failing() -> Vec<Detector> {
    vec![
        detector("ok", &["true"], None),
        detector("bad", &["sh", "-c", "echo unused-dep; exit 3"], None),
    ]
}

#[test]
fn reports_each_failing_detector() {
    // req:check/detectors@8bf99b6d
    let dir = TempDir::new().unwrap();
    assert_eq!(
        found(dir.path(), &failing()),
        [(
            "clm.toml".to_string(),
            "check/detectors",
            "bad failed (exit 3)".to_string()
        )]
    );
}

#[test]
fn reports_a_tool_that_cannot_run_with_its_hint() {
    // req:check/detectors@8bf99b6d
    let dir = TempDir::new().unwrap();
    let list = [
        detector(
            "machete",
            &["no-such-tool-xyz"],
            Some("cargo install cargo-machete"),
        ),
        detector("bare", &["no-such-tool-xyz"], None),
    ];
    let found = found(dir.path(), &list);
    assert_eq!(found.len(), 2, "{found:?}");
    let (path, rule, message) = &found[0];
    assert_eq!((path.as_str(), *rule), ("clm.toml", "check/detectors"));
    assert!(message.starts_with("machete could not run: "), "{message}");
    assert!(
        message.ends_with("; install with: cargo install cargo-machete"),
        "{message}"
    );
    assert!(found[1].2.starts_with("bare could not run: "), "{found:?}");
    assert!(!found[1].2.contains("install with"), "{found:?}");
}

#[test]
fn an_empty_command_cannot_run() {
    // req:check/detectors@8bf99b6d
    let dir = TempDir::new().unwrap();
    let found = found(dir.path(), &[detector("empty", &[], None)]);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].2.starts_with("empty could not run: "), "{found:?}");
}

#[test]
fn reports_a_detector_killed_by_a_signal() {
    // req:check/detectors@8bf99b6d
    let dir = TempDir::new().unwrap();
    let list = [detector("killed", &["sh", "-c", "kill -9 $$"], None)];
    assert_eq!(
        found(dir.path(), &list)[0].2,
        "killed failed (killed by a signal)"
    );
}

#[test]
fn runs_from_the_repository_root() {
    // req:check/detectors@8bf99b6d
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("here"), "").unwrap();
    let list = [detector("cwd", &["test", "-f", "here"], None)];
    assert_eq!(found(dir.path(), &list), []);
}

#[test]
fn runs_detectors_in_order_one_after_another() {
    // req:check/detectors@8bf99b6d
    let dir = TempDir::new().unwrap();
    // `second` fails unless `first` has finished writing its file.
    let list = [
        detector(
            "first",
            &["sh", "-c", "sleep 0.2; touch done; exit 1"],
            None,
        ),
        detector("second", &["test", "-f", "done"], None),
    ];
    let names: Vec<_> = found(dir.path(), &list)
        .into_iter()
        .map(|(_, _, message)| message)
        .collect();
    assert_eq!(names, ["first failed (exit 1)"]);
}

#[test]
fn no_detectors_no_violations() {
    // req:check/detectors@8bf99b6d
    let dir = TempDir::new().unwrap();
    assert_eq!(found(dir.path(), &[]), []);
}

#[test]
fn run_skips_detectors() {
    // req:check/detectors@8bf99b6d
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("specs")).unwrap();
    fs::write(
        dir.path().join("specs/SPEC.md"),
        "---\ntitle: R\nstatus: active\nowners: []\n---\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("clm.toml"),
        "[[detectors]]\nname = \"bad\"\ncommand = [\"sh\", \"-c\", \"echo unused-dep; exit 3\"]\n",
    )
    .unwrap();
    let found: Vec<_> = run(dir.path())
        .into_iter()
        .filter(|v| v.rule == "check/detectors")
        .collect();
    assert!(found.is_empty(), "{found:?}");
}
