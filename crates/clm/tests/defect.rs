//! `clm defect`: reporting a defect in CLM itself as its `defects` setting says.

use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::sync::{PoisonError, RwLock};

use tempfile::TempDir;

/// A process forked while another thread holds a just-written executable open for writing
/// inherits that descriptor, and running the file then fails with "text file busy". Writing
/// executables holds this lock exclusively, and every spawn shares it.
static FORKS: RwLock<()> = RwLock::new(());

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

/// A temp directory holding a fake `gh` for `PATH`, which records its arguments, one per
/// line, in `args` and its body file in `body`, then runs `then`.
fn fake_gh(then: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    let script = format!(
        "#!/bin/sh\nout={out}\nprintf '%s\\n' \"$@\" > \"$out/args\"\n\
         while [ $# -gt 0 ]; do\n  [ \"$1\" = --body-file ] && cat \"$2\" > \"$out/body\"\n  shift\ndone\n{then}\n",
        out = dir.path().display()
    );
    let gh = dir.path().join("gh");
    let _exclusive = FORKS.write().unwrap_or_else(PoisonError::into_inner);
    fs::write(&gh, script).unwrap();
    fs::set_permissions(&gh, fs::Permissions::from_mode(0o755)).unwrap();
    dir
}

/// Runs `clm defect` with `args` in `cwd`, the report on stdin, no user settings, and `env`.
fn defect(cwd: &Path, args: &[&str], report: &str, env: &[(&str, &str)]) -> Output {
    let config = TempDir::new().unwrap();
    let mut child = {
        let _shared = FORKS.read().unwrap_or_else(PoisonError::into_inner);
        Command::new(env!("CARGO_BIN_EXE_clm"))
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
            .unwrap()
    };
    child
        .stdin
        .take()
        .unwrap()
        .write_all(report.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

fn path_with(gh: &TempDir) -> String {
    format!(
        "{}:{}",
        gh.path().display(),
        std::env::var("PATH").unwrap_or_default()
    )
}

const REPORT: &str = "`clm gate` passed a task whose basis had changed.\n";

#[test]
fn backlog_on_a_spec_branch() {
    // req:plugin.hooks/defect-backlog@064ef91e
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
fn backlog_item_body_and_origin() {
    // req:plugin.hooks/defect-backlog@064ef91e
    let repo = repo("defects = \"backlog\"\n");
    // A report without a final newline ends in exactly one; a title of only punctuation
    // still gets a file name.
    let out = defect(repo.path(), &["--title", "!!!"], "body", &[]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let item = fs::read_to_string(repo.path().join("backlog/defect.md")).unwrap();
    assert!(item.ends_with("\n\nbody\n"), "{item}");
    assert!(item.contains("components: []\n"), "{item}");
    // A spec branch whose change the repository doesn't have (yet, or any more) gives no
    // origin, which `clm check` would reject, and nor does any other branch.
    for (branch, title, file) in [
        ("topic", "Elsewhere", "elsewhere"),
        ("spec/20261023-new", "New change", "new-change"),
        ("impl/20261023-gone/gate", "Gone change", "gone-change"),
    ] {
        git(repo.path(), &["checkout", "-q", "-b", branch]);
        let out = defect(repo.path(), &["--title", title], REPORT, &[]);
        assert_eq!(out.status.code(), Some(0), "{out:?}");
        let item = fs::read_to_string(repo.path().join(format!("backlog/{file}.md"))).unwrap();
        assert!(!item.contains("origin"), "{branch}: {item}");
    }
}

#[test]
fn backlog_on_an_implementation_branch() {
    // req:plugin.hooks/defect-backlog@064ef91e
    let repo = repo("defects = \"backlog\"\n");
    for (branch, title) in [
        ("impl/gate", "On a node"),
        ("impl/archive/20261022-x", "On an archive"),
        ("impl/20261022-x/gate", "On a change item"),
    ] {
        git(repo.path(), &["checkout", "-q", "-B", branch]);
        let out = defect(repo.path(), &["--title", title], REPORT, &[]);
        assert_eq!(out.status.code(), Some(0), "{out:?}");
        let rel = String::from_utf8(out.stdout).unwrap();
        let item = fs::read_to_string(repo.path().join(rel.trim())).unwrap();
        assert!(!item.contains("origin"), "{branch}: {item}");
    }
}

#[test]
fn backlog_node_a_live_change_creates() {
    // req:plugin.hooks/defect-backlog@064ef91e
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
fn backlog_long_title() {
    // req:plugin.hooks/defect-backlog@064ef91e
    let repo = repo("defects = \"backlog\"\n");
    let title = "word ".repeat(80);
    let out = defect(repo.path(), &["--title", &title], REPORT, &[]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let rel = String::from_utf8(out.stdout).unwrap();
    let name = rel.trim().strip_prefix("backlog/").unwrap();
    assert!(name.len() <= 64 + ".md".len(), "{name}");
    assert!(
        name.starts_with("word-word-") && !name.contains("-.md"),
        "{name}"
    );
    let item = fs::read_to_string(repo.path().join(rel.trim())).unwrap();
    assert!(
        item.contains(title.as_str()),
        "the whole title is kept: {item}"
    );
}

#[test]
fn backlog_unknown_node() {
    // req:plugin.hooks/defect-backlog@064ef91e
    let repo = repo("defects = \"backlog\"\n");
    let out = defect(
        repo.path(),
        &["--title", "x", "--component", "nonesuch"],
        REPORT,
        &[],
    );
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("nonesuch"),
        "{out:?}"
    );
    assert!(!repo.path().join("backlog").exists());
}

#[test]
fn backlog_outside_a_clm_repository() {
    // req:plugin.hooks/defect-backlog@064ef91e
    let dir = TempDir::new().unwrap();
    git(dir.path(), &["init", "-q"]);
    let out = defect(
        dir.path(),
        &["--title", "x"],
        REPORT,
        &[("CLM_DEFECTS", "backlog")],
    );
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1, "only .git");
}

#[test]
fn backlog_taken_slug() {
    // req:plugin.hooks/defect-backlog@064ef91e
    let repo = repo("defects = \"backlog\"\n");
    let args = ["--title", "Gate passes a stale task", "--component", "gate"];
    for expected in [
        "backlog/gate-passes-a-stale-task.md",
        "backlog/gate-passes-a-stale-task-2.md",
        "backlog/gate-passes-a-stale-task-3.md",
    ] {
        let out = defect(repo.path(), &args, REPORT, &[]);
        assert_eq!(out.status.code(), Some(0), "{out:?}");
        assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), expected);
        assert!(repo.path().join(expected).is_file());
    }
}

#[test]
fn issue_filed() {
    // req:plugin.hooks/defect-issue@698b188d
    let repo = repo("host = \"github\"\n");
    let gh = fake_gh("echo https://github.com/clunny/clm/issues/7");
    let out = defect(
        repo.path(),
        &["--title", "Sync loses a delta", "--component", "gate"],
        REPORT,
        &[("PATH", &path_with(&gh))],
    );
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        "https://github.com/clunny/clm/issues/7"
    );
    let args = fs::read_to_string(gh.path().join("args")).unwrap();
    let args: Vec<&str> = args.lines().collect();
    assert_eq!(
        args[..6],
        [
            "issue",
            "create",
            "--repo",
            "clunny/clm",
            "--title",
            "Sync loses a delta"
        ]
    );
    let body = fs::read_to_string(gh.path().join("body")).unwrap();
    assert!(body.starts_with("## What happened\n\n"), "{body}");
    assert!(body.contains(REPORT), "{body}");
    assert!(body.contains(env!("CARGO_PKG_VERSION")), "{body}");
    assert!(body.contains(std::env::consts::OS), "{body}");
    assert!(body.contains("- Components: gate\n"), "{body}");
    assert!(!repo.path().join("backlog").exists());
}

#[test]
fn issue_filed_from_a_project_on_another_host() {
    // req:plugin.hooks/defect-issue@698b188d
    for host in ["azure", "git", "none"] {
        let repo = repo(&format!("host = \"{host}\"\n"));
        let gh = fake_gh("echo https://github.com/clunny/clm/issues/7");
        let out = defect(
            repo.path(),
            &["--title", "Sync loses a delta"],
            REPORT,
            &[("PATH", &path_with(&gh))],
        );
        assert_eq!(out.status.code(), Some(0), "{host}: {out:?}");
        assert_eq!(
            String::from_utf8_lossy(&out.stdout).trim(),
            "https://github.com/clunny/clm/issues/7",
            "{host}"
        );
        let args = fs::read_to_string(gh.path().join("args")).unwrap();
        let args: Vec<&str> = args.lines().collect();
        assert_eq!(
            args[..4],
            ["issue", "create", "--repo", "clunny/clm"],
            "{host}"
        );
    }
}

/// The report `clm defect` kept as `name` under `clm/defects/` in `git_dir`.
fn kept(git_dir: &Path, name: &str) -> String {
    fs::read_to_string(git_dir.join("clm/defects").join(name)).unwrap()
}

fn text_of(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

#[test]
fn issue_not_filed_is_kept_without_gh() {
    // req:plugin.hooks/defect-issue@698b188d
    let repo = repo("host = \"git\"\n");
    // A `PATH` that holds `git` and no `gh`.
    let no_gh = TempDir::new().unwrap();
    let git_path = String::from_utf8(
        Command::new("sh")
            .args(["-c", "command -v git"])
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();
    std::os::unix::fs::symlink(git_path.trim(), no_gh.path().join("git")).unwrap();
    let git_dir = repo.path().join(".git").canonicalize().unwrap();
    for name in ["sync-loses-a-delta.md", "sync-loses-a-delta-2.md"] {
        let out = defect(
            repo.path(),
            &["--title", "Sync loses a delta"],
            REPORT,
            &[("PATH", &no_gh.path().display().to_string())],
        );
        assert_eq!(out.status.code(), Some(1), "{out:?}");
        let text = text_of(&out);
        assert!(text.contains(REPORT.trim()), "{text}");
        assert!(text.contains("wasn't filed"), "{text}");
        let file = git_dir.join("clm/defects").join(name);
        assert!(text.contains(&file.display().to_string()), "{text}");
        assert!(text.contains("file it by hand on clunny/clm"), "{text}");
        assert!(kept(&git_dir, name).contains(REPORT.trim()));
    }
}

#[test]
fn issue_without_access() {
    // req:plugin.hooks/defect-issue@698b188d
    let repo = repo("");
    let gh = fake_gh(
        "echo \"GraphQL: Could not resolve to a Repository with the name 'clunny/clm'.\" >&2\nexit 1",
    );
    let out = defect(
        repo.path(),
        &["--title", "Sync loses a delta"],
        REPORT,
        &[("PATH", &path_with(&gh))],
    );
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(text.contains(REPORT.trim()), "{text}");
    assert!(text.contains("Could not resolve to a Repository"), "{text}");
    let git_dir = repo.path().join(".git").canonicalize().unwrap();
    let file = git_dir.join("clm/defects/sync-loses-a-delta.md");
    assert!(text.contains(&file.display().to_string()), "{text}");
    let body = kept(&git_dir, "sync-loses-a-delta.md");
    assert!(body.starts_with("# Sync loses a delta\n\n"), "{body}");
    assert!(body.contains(REPORT), "{body}");
    assert!(body.contains(env!("CARGO_PKG_VERSION")), "{body}");
}

#[test]
fn issue_kept_from_a_linked_worktree() {
    // req:plugin.hooks/defect-issue@698b188d
    let repo = repo("");
    git(
        repo.path(),
        &[
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "commit",
            "-q",
            "--allow-empty",
            "-m",
            "x",
        ],
    );
    let parent = TempDir::new().unwrap();
    let tree = parent.path().join("wt");
    git(
        repo.path(),
        &["worktree", "add", "-q", tree.to_str().unwrap()],
    );
    let gh = fake_gh("echo nope >&2\nexit 1");
    let out = defect(
        &tree,
        &["--title", "Sync loses a delta"],
        REPORT,
        &[("PATH", &path_with(&gh))],
    );
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    git(
        repo.path(),
        &["worktree", "remove", "--force", tree.to_str().unwrap()],
    );
    let git_dir = repo.path().join(".git").canonicalize().unwrap();
    assert!(kept(&git_dir, "sync-loses-a-delta.md").contains(REPORT.trim()));
}

#[test]
fn issue_not_kept_outside_git() {
    // req:plugin.hooks/defect-issue@698b188d
    let dir = TempDir::new().unwrap();
    let gh = fake_gh("echo nope >&2\nexit 1");
    let out = defect(
        dir.path(),
        &["--title", "Sync loses a delta"],
        REPORT,
        &[("PATH", &path_with(&gh))],
    );
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let text = text_of(&out);
    assert!(text.contains(REPORT.trim()), "{text}");
    assert!(text.contains("nope"), "{text}");
    assert!(text.contains("isn't in a git repository"), "{text}");
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[test]
fn issue_unfiled_when_gh_fails_silently_or_is_missing() {
    // req:plugin.hooks/defect-issue@698b188d
    let repo = repo("");
    let silent = fake_gh("exit 3");
    let no_gh = TempDir::new().unwrap();
    for path in [path_with(&silent), no_gh.path().display().to_string()] {
        let out = defect(
            repo.path(),
            &["--title", "Sync loses a delta"],
            REPORT,
            &[("PATH", &path)],
        );
        assert_eq!(out.status.code(), Some(1), "{out:?}");
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(text.contains(REPORT.trim()), "{text}");
        assert!(text.contains("wasn't filed: gh issue create "), "{text}");
    }
    // A `gh` that succeeds without printing a URL hasn't shown the issue was filed.
    let mute = fake_gh("exit 0");
    let out = defect(
        repo.path(),
        &["--title", "Sync loses a delta"],
        REPORT,
        &[("PATH", &path_with(&mute))],
    );
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains(REPORT.trim()),
        "{out:?}"
    );
    let git_dir = repo.path().join(".git").canonicalize().unwrap();
    assert!(kept(&git_dir, "sync-loses-a-delta.md").contains(REPORT.trim()));
}

#[test]
fn reporting_off() {
    // req:plugin.hooks/defect-issue@698b188d
    let repo = repo("");
    let gh = fake_gh("echo https://github.com/clunny/clm/issues/7");
    let out = defect(
        repo.path(),
        &["--title", "x"],
        REPORT,
        &[("PATH", &path_with(&gh)), ("CLM_DEFECTS", "off")],
    );
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert!(!gh.path().join("args").exists());
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("off"),
        "{out:?}"
    );
    assert!(!repo.path().join("backlog").exists());
}

#[test]
fn settings_that_cannot_load() {
    // req:plugin.hooks/defect-issue@698b188d
    let repo = repo("defects = \"always\"\n");
    let out = defect(repo.path(), &["--title", "x"], REPORT, &[]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("clm.toml"),
        "{out:?}"
    );
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
