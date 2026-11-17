//! `clm defect`, run as the binary: the tests whose scenario runs `clm check` on the item it
//! wrote, which `clm-hooks` can't call (`commands-live-in-their-crates`), and the root's
//! `component-flags`.

use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

use tempfile::TempDir;

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}: {out:?}");
}

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

/// A CLM repository with a `gate` node and a live change `20261022-x`, on branch
/// `impl/20261022-x/gate`, and `clm.toml`
/// holding `settings`.
fn repo(settings: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    let fm = "---\ntitle: T\nstatus: active\nowners: []\n---\n\n## Contract\nc\n";
    write(dir.path(), "specs/SPEC.md", fm);
    write(dir.path(), "specs/gate/SPEC.md", fm);
    write(dir.path(), "clm.toml", settings);
    // The branch's change, which a backlog item's `origin` must name.
    write(
        dir.path(),
        "changes/20261022-x/change.md",
        "---\nstatus: proposed\n---\n\n# X\n",
    );
    git(dir.path(), &["init", "-q", "-b", "impl/20261022-x/gate"]);
    dir
}

/// Runs `clm defect` with `args` in `cwd`, the report on stdin, no user settings, and `env`.
fn defect(cwd: &Path, args: &[&str], report: &str, env: &[(&str, &str)]) -> Output {
    let config = TempDir::new().unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_clm"))
        .arg("defect")
        .args(args)
        .current_dir(cwd)
        .env("XDG_CONFIG_HOME", config.path())
        .env_remove("CLM_AUTOSYNC")
        .env_remove("CLM_DEFECTS")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .envs(env.iter().copied())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(report.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

const REPORT: &str = "`clm gate` passed a task whose basis had changed.\n";

#[test]
fn backlog_on_a_spec_branch() {
    // req:clm/check-across-components@45a113b3
    // req:plugin.hooks/defect-backlog@06439020
    let repo = repo("defects = \"backlog\"\n");
    git(repo.path(), &["checkout", "-q", "-b", "spec/20261022-x"]);
    let title = "Gate passes a stale task";
    let out = defect(
        repo.path(),
        &["--title", title, "--component", "gate"],
        REPORT,
        &[],
    );
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        "backlog/gate-passes-a-stale-task.md"
    );
    let item = fs::read_to_string(repo.path().join("backlog/gate-passes-a-stale-task.md")).unwrap();
    assert_eq!(
        item,
        format!(
            "---\ntitle: \"{title}\"\nkind: bug\ncomponents: [gate]\norigin: 20261022-x\n---\n\n{REPORT}"
        )
    );
    let check = clm_check(repo.path());
    assert!(check.status.success(), "{check:?}");
    // A spec branch whose change is archived or abandoned has no live change to name.
    write(
        repo.path(),
        "changes/archive/20261021-old/change.md",
        "---\nstatus: done\n---\n\n# Old\n",
    );
    write(
        repo.path(),
        "changes/20261021-dropped/change.md",
        "---\nstatus: abandoned\n---\n\n# Dropped\n",
    );
    for (branch, title) in [
        ("spec/20261021-old", "Old change"),
        ("spec/20261021-dropped", "Dropped change"),
    ] {
        git(repo.path(), &["checkout", "-q", "-b", branch]);
        let out = defect(repo.path(), &["--title", title], REPORT, &[]);
        assert_eq!(out.status.code(), Some(0), "{out:?}");
        let rel = String::from_utf8(out.stdout).unwrap();
        let item = fs::read_to_string(repo.path().join(rel.trim())).unwrap();
        assert!(!item.contains("origin"), "{branch}: {item}");
    }
}

#[test]
fn backlog_node_a_live_change_creates() {
    // req:clm/check-across-components@45a113b3
    // req:plugin.hooks/defect-backlog@06439020
    let repo = repo("defects = \"backlog\"\n");
    write(
        repo.path(),
        "changes/20261022-x/deltas/fresh.md",
        "---\ncomponent: fresh\ncreate: true\n---\n## Frontmatter\n```yaml\ntitle: Fresh\nstatus: specd\nowners: []\n```\n\n## Contract\nc\n",
    );
    let out = defect(
        repo.path(),
        &["--title", "Fresh misfires", "--component", "fresh"],
        REPORT,
        &[],
    );
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        "backlog/fresh-misfires.md"
    );
    let item = fs::read_to_string(repo.path().join("backlog/fresh-misfires.md")).unwrap();
    assert!(item.contains("\ncomponents: [fresh]\n"), "{item}");
    assert!(clm_check(repo.path()).status.success());
}

/// `clm check` on `root`, with no user settings (which may name detectors).
fn clm_check(root: &Path) -> Output {
    let config = TempDir::new().unwrap();
    Command::new(env!("CARGO_BIN_EXE_clm"))
        .args(["check", "--root"])
        .arg(root)
        .env("XDG_CONFIG_HOME", config.path())
        .env_remove("CLM_AUTOSYNC")
        .env_remove("CLM_DEFECTS")
        .output()
        .unwrap()
}

#[test]
fn component_flag_files_the_item() {
    // req:clm/component-flags@4d6d18e7
    let first = repo("defects = \"backlog\"\n");
    let out = defect(
        first.path(),
        &["--title", "Gate misfires", "--component", "gate"],
        REPORT,
        &[],
    );
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let item = fs::read_to_string(first.path().join("backlog/gate-misfires.md")).unwrap();
    assert!(item.contains("components: [gate]"), "{item}");
    // Repeatable: every ID named, in the order given.
    let repo = repo("defects = \"backlog\"\n");
    let fm = "---\ntitle: T\nstatus: active\nowners: []\n---\n\n## Contract\nc\n";
    write(repo.path(), "specs/other/SPEC.md", fm);
    let out = defect(
        repo.path(),
        &[
            "--title",
            "Gate misfires",
            "--component",
            "gate",
            "--component",
            "other",
        ],
        REPORT,
        &[],
    );
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let item = fs::read_to_string(repo.path().join("backlog/gate-misfires.md")).unwrap();
    assert!(item.contains("components: [gate, other]"), "{item}");
    let help = String::from_utf8(defect(repo.path(), &["--help"], "", &[]).stdout).unwrap();
    assert!(help.contains("--component"), "{help}");
}
