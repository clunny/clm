use std::fs;
use std::path::Path;
use std::process::Command;

use tempfile::TempDir;

/// `clm`, with a user config dir that holds no `clm/config.toml` and no CLM variables.
fn clm_command() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_clm"));
    cmd.env(
        "XDG_CONFIG_HOME",
        Path::new(env!("CARGO_TARGET_TMPDIR")).join("no-user-config"),
    )
    .env_remove("CLM_AUTOSYNC")
    .env_remove("CLM_DEFECTS")
    .env_remove("CLM_MAX_BUILDS")
    .env_remove("CLM_WRITER");
    cmd
}

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn clm(root: &Path, args: &[&str]) -> (Option<i32>, String) {
    let out = clm_command()
        .args(args)
        .arg("--root")
        .arg(root)
        .output()
        .unwrap();
    (out.status.code(), String::from_utf8(out.stdout).unwrap())
}

#[test]
fn gate_prints_kind_and_blocks_mixed_pr() {
    // req:gate/classify@17132c50
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    let git = |args: &[&str]| {
        let ok = Command::new("git")
            .args([
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@t",
                "-c",
                "commit.gpgsign=false",
            ])
            .args(args)
            .current_dir(d)
            .status()
            .unwrap()
            .success();
        assert!(ok, "git {args:?}");
    };
    git(&["init", "-q", "-b", "main"]);
    write(d, "README.md", "x\n");
    git(&["add", "-A"]);
    git(&["commit", "-qm", "base"]);
    git(&["checkout", "-q", "-b", "pr"]);
    write(d, "specs/a/SPEC.md", "x\n");
    write(d, "src/lib.rs", "x\n");
    git(&["add", "-A"]);
    git(&["commit", "-qm", "mixed"]);

    let out = clm_command()
        .args(["gate", "--base", "main", "--approved", "--root"])
        .arg(d)
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(out.status.code(), Some(1), "{stdout}");
    assert_eq!(stdout.lines().next(), Some("mixed"));
    assert!(stdout.contains("gate/classify"), "{stdout}");
}

#[test]
fn first_line() {
    // req:gate/approval@492f6a25
    // req:gate/needs-review@3276bfca
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    let git = |args: &[&str]| {
        let ok = Command::new("git")
            .args([
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@t",
                "-c",
                "commit.gpgsign=false",
            ])
            .args(args)
            .current_dir(d)
            .status()
            .unwrap()
            .success();
        assert!(ok, "git {args:?}");
    };
    git(&["init", "-q", "-b", "main"]);
    write(d, "backlog/x.md", "---\ntitle: X\nkind: idea\n---\n");
    git(&["add", "-A"]);
    git(&["commit", "-qm", "base"]);
    git(&["checkout", "-q", "-b", "pr"]);
    write(d, "backlog/x.md", "---\ntitle: X2\nkind: idea\n---\n");
    git(&["add", "-A"]);
    git(&["commit", "-qm", "edit item"]);

    let (code, stdout) = clm(d, &["gate", "--base", "main"]);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "spec\n");

    write(d, "ARCHITECTURE.md", "x\n");
    git(&["add", "-A"]);
    git(&["commit", "-qm", "invariants"]);
    let (code, stdout) = clm(d, &["gate", "--base", "main"]);
    assert_eq!(code, Some(1), "{stdout}");
    assert_eq!(stdout.lines().next(), Some("spec review"));
    let violations: Vec<&str> = stdout.lines().skip(1).collect();
    assert_eq!(violations.len(), 1, "{stdout}");
    assert!(
        violations[0].contains("gate/approval") && violations[0].contains("`clm approve <branch>`"),
        "{stdout}"
    );
    let (code, stdout) = clm(d, &["gate", "--base", "main", "--approved"]);
    assert_eq!((code, stdout.as_str()), (Some(0), "spec review\n"));
}

#[test]
fn only_gate_rs_cites_the_gate() {
    // req:gate/binary-tests-apart@671cc207
    fn rs_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
        for e in fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                rs_files(&p, out);
            } else if p.extension().is_some_and(|x| x == "rs") {
                out.push(p);
            }
        }
    }
    let mut files = Vec::new();
    rs_files(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests"),
        &mut files,
    );
    let prefix = concat!("req", ":gate/");
    let mut citing: Vec<String> = files
        .iter()
        .filter(|p| fs::read_to_string(p).unwrap().contains(prefix))
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    citing.sort();
    assert_eq!(citing, ["gate.rs"]);
}
