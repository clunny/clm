//! The contribution operations (host/contribution-operations), against a fake `gh`.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;
use std::sync::Mutex;

use clm_config::Host as Setting;
use tempfile::TempDir;

/// The tests put a fake `gh` first on the process's `PATH`, one at a time.
static PATH_LOCK: Mutex<()> = Mutex::new(());

/// Runs `body` with a fake `gh` first on `PATH`: it records each call's arguments on a line
/// of `args`, and answers with the text of the first of `answers` whose needle the arguments
/// hold, failing when none does. `body` gets a git repository (`origin` is dana's) and the
/// fake's directory.
fn with_gh<T>(answers: &[(&str, &str)], body: impl FnOnce(&Path, &Path) -> T) -> T {
    let _one = PATH_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = TempDir::new().unwrap();
    let mut script = format!(
        "#!/bin/sh\nprintf '%s\\n' \"$*\" >> {}/args\nall=\"$*\"\n",
        dir.path().display()
    );
    for (needle, out) in answers {
        script +=
            &format!("case \"$all\" in *'{needle}'*) cat <<'EOT'\n{out}\nEOT\nexit 0;; esac\n");
    }
    script += "exit 1\n";
    let gh = dir.path().join("gh");
    fs::write(&gh, script).unwrap();
    fs::set_permissions(&gh, fs::Permissions::from_mode(0o755)).unwrap();
    let old = std::env::var("PATH").unwrap();
    // SAFETY: every test that touches the environment holds PATH_LOCK.
    unsafe { std::env::set_var("PATH", format!("{}:{old}", dir.path().display())) };
    let repo = TempDir::new().unwrap();
    git(repo.path(), &["init", "-q", "-b", "main"]);
    git(
        repo.path(),
        &["remote", "add", "origin", "https://github.com/dana/clm.git"],
    );
    let out = body(repo.path(), dir.path());
    unsafe { std::env::set_var("PATH", old) };
    out
}

fn git(root: &Path, args: &[&str]) -> bool {
    Command::new("git")
        .args(args)
        .current_dir(root)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .output()
        .unwrap()
        .status
        .success()
}

fn calls(dir: &Path) -> String {
    fs::read_to_string(dir.join("args")).unwrap_or_default()
}

const FORK: &str = r#"{"isFork":true,"parent":{"owner":{"login":"clunny"},"name":"clm"},"owner":{"login":"dana"},"name":"clm"}"#;

#[test]
fn a_forks_parent() {
    // req:host/contribution-operations@6134fd80
    with_gh(&[("repo view", FORK)], |root, _| {
        let host = clm_host::adapter(Setting::Github);
        assert_eq!(host.parent(root).unwrap().as_deref(), Some("clunny/clm"));
    });
    with_gh(
        &[("repo view", r#"{"isFork":false,"parent":null}"#)],
        |root, _| {
            let host = clm_host::adapter(Setting::Github);
            assert_eq!(host.parent(root).unwrap(), None);
        },
    );
}

#[test]
fn sync_brings_the_fork_up_without_force() {
    // req:host/contribution-operations@6134fd80
    with_gh(&[("repo view", FORK), ("repo sync", "")], |root, dir| {
        clm_host::adapter(Setting::Github).sync(root).unwrap();
        let log = calls(dir);
        assert!(log.contains("repo sync dana/clm --branch main"), "{log}");
        assert!(!log.contains("--force"), "{log}");
    });
}

#[test]
fn opening_on_the_parent() {
    // req:host/contribution-operations@6134fd80
    let answers = [
        ("repo view", FORK),
        ("pr list", "[]"),
        ("pr create", "https://github.com/clunny/clm/pull/9"),
    ];
    with_gh(&answers, |root, dir| {
        let body = root.join("body.md");
        fs::write(&body, "b").unwrap();
        let host = clm_host::adapter(Setting::Github);
        let url = host.open_on_parent(root, "spec/x", "x", &body).unwrap();
        assert_eq!(url, "https://github.com/clunny/clm/pull/9");
        let log = calls(dir);
        let create = log.lines().find(|l| l.starts_with("pr create")).unwrap();
        for part in ["--repo clunny/clm", "--base main", "--head dana:spec/x"] {
            assert!(create.contains(part), "{create}");
        }
    });
}

#[test]
fn an_open_pull_request_on_the_parent_is_returned() {
    // req:host/contribution-operations@6134fd80
    let open = r#"[{"url":"https://github.com/clunny/clm/pull/4"}]"#;
    with_gh(&[("repo view", FORK), ("pr list", open)], |root, dir| {
        let host = clm_host::adapter(Setting::Github);
        let url = host
            .open_on_parent(root, "spec/x", "x", Path::new("body.md"))
            .unwrap();
        assert_eq!(url, "https://github.com/clunny/clm/pull/4");
        assert!(!calls(dir).contains("pr create"));
    });
}

#[test]
fn a_pull_requests_files() {
    // req:host/contribution-operations@6134fd80
    let pr = r#"{"author":{"login":"dana"},"state":"OPEN","title":"t","body":"b","headRepository":{"name":"clm"},"headRepositoryOwner":{"login":"dana"},"headRefName":"spec/x","maintainerCanModify":true,"files":[{"path":"backlog/x.md"},{"path":"crates/a/src/lib.rs"}]}"#;
    with_gh(&[("pr view 7", pr)], |root, _| {
        let shown = shown(root, 7);
        assert!(shown.pull_request);
        assert_eq!(shown.author, "dana");
        assert_eq!(shown.files, ["backlog/x.md", "crates/a/src/lib.rs"]);
        assert_eq!(
            shown.head_repo_url.as_deref(),
            Some("https://github.com/dana/clm")
        );
        assert_eq!(shown.head_branch.as_deref(), Some("spec/x"));
        assert!(shown.maintainers_can_edit);
    });
}

fn shown(root: &Path, number: u64) -> clm_host::Shown {
    clm_host::adapter(Setting::Github)
        .show(root, number)
        .unwrap()
}

#[test]
fn an_issue_is_shown_when_no_pull_request_has_the_number() {
    // req:host/contribution-operations@6134fd80
    let issue = r#"{"author":{"login":"sam"},"state":"OPEN","title":"idea","body":"do it"}"#;
    with_gh(&[("issue view 3", issue)], |root, _| {
        let shown = shown(root, 3);
        assert!(!shown.pull_request);
        assert_eq!(shown.author, "sam");
        assert_eq!(shown.title, "idea");
        assert!(shown.files.is_empty());
    });
}

#[test]
fn closing_with_a_comment_and_labelling() {
    // req:host/contribution-operations@6134fd80
    let answers = [("comment", ""), ("close", ""), ("edit", "")];
    with_gh(&answers, |root, dir| {
        let body = root.join("c.md");
        fs::write(&body, "thanks").unwrap();
        let host = clm_host::adapter(Setting::Github);
        host.comment(root, 7, &body, true).unwrap();
        host.label(root, 7, "accepted").unwrap();
        let log = calls(dir);
        assert!(log.contains("comment 7 --body-file"), "{log}");
        assert!(log.contains("close 7"), "{log}");
        assert!(log.contains("edit 7 --add-label accepted"), "{log}");
    });
}

#[test]
fn fetch_makes_a_branch_and_refuses_an_existing_one() {
    // req:host/contribution-operations@6134fd80
    with_gh(&[], |root, _| {
        let commit = [
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "commit",
            "-q",
            "--allow-empty",
            "-m",
            "i",
        ];
        assert!(git(root, &commit));
        let remote = TempDir::new().unwrap();
        let remote = remote.path().display().to_string();
        assert!(git(root, &["clone", "-q", "--bare", ".", &remote]));
        assert!(git(root, &["remote", "set-url", "origin", &remote]));
        assert!(git(root, &["update-ref", "refs/pull/7/head", "HEAD"]));
        assert!(git(root, &["push", "-q", "origin", "refs/pull/7/head"]));
        let host = clm_host::adapter(Setting::Github);
        host.fetch(root, 7, "contrib/7").unwrap();
        assert!(git(
            root,
            &["rev-parse", "--verify", "refs/heads/contrib/7"]
        ));
        assert!(host.fetch(root, 7, "contrib/7").is_err());
        assert!(host.fetch(root, 7, "main").is_err());
    });
}

#[test]
fn no_host_runs_no_gh() {
    // req:host/contribution-operations@6134fd80
    for setting in [Setting::None, Setting::Git] {
        with_gh(&[("", "{}")], |root, dir| {
            let host = clm_host::adapter(setting);
            let file = Path::new("c.md");
            assert_eq!(host.parent(root).unwrap(), None);
            assert!(host.show(root, 7).is_err());
            assert!(host.sync(root).is_err());
            assert!(host.open_on_parent(root, "b", "t", file).is_err());
            assert!(host.fetch(root, 7, "b").is_err());
            assert!(host.comment(root, 7, file, true).is_err());
            assert!(host.label(root, 7, "l").is_err());
            assert_eq!(calls(dir), "");
        });
    }
}
