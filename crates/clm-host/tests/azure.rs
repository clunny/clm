//! The `Azure` adapter (host/azure-pull-requests, host/azure-other-operations), against a
//! fake `az` and a fake `gh`.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;
use std::sync::Mutex;

use clm_host::{Azure, Error, Host};
use tempfile::TempDir;

/// The tests put fakes first on the process's `PATH`, one test at a time.
static PATH_LOCK: Mutex<()> = Mutex::new(());

const WEB: &str = "https://dev.azure.com/o/p/_git/r";

/// Writes a fake `program` in `dir`: it records each call's arguments on a line of
/// `<program>.args`, and answers with the text of the first of `answers` whose needle the
/// arguments hold. An answer starting `FAIL:` is printed on stderr and the fake exits 1.
fn fake(dir: &Path, program: &str, answers: &[(&str, &str)]) {
    let mut script = format!(
        "#!/bin/sh\nprintf '%s\\n' \"$*\" >> {}/{program}.args\nall=\"$*\"\n",
        dir.display()
    );
    for (needle, out) in answers {
        script += &match out.strip_prefix("FAIL:") {
            Some(why) => {
                format!("case \"$all\" in *'{needle}'*) echo '{why}' >&2; exit 1;; esac\n")
            }
            None => {
                format!("case \"$all\" in *'{needle}'*) cat <<'EOT'\n{out}\nEOT\nexit 0;; esac\n")
            }
        };
    }
    script += "echo 'unexpected call' >&2\nexit 1\n";
    let path = dir.join(program);
    fs::write(&path, script).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
}

/// Runs `body` with fake `az` and `gh` first on `PATH`, in a git repository on `branch`
/// whose `body.md` holds `body_text`. `body` gets the repository and the fakes' directory.
fn with_fakes<T>(
    az: &[(&str, &str)],
    branch: &str,
    body_text: &str,
    body: impl FnOnce(&Path, &Path) -> T,
) -> T {
    let _one = PATH_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = TempDir::new().unwrap();
    fake(dir.path(), "az", az);
    fake(dir.path(), "gh", &[]);
    let old = std::env::var("PATH").unwrap();
    // SAFETY: every test that touches the environment holds PATH_LOCK.
    unsafe { std::env::set_var("PATH", format!("{}:{old}", dir.path().display())) };
    let repo = TempDir::new().unwrap();
    let init = Command::new("git")
        .args(["init", "-q", "-b", branch])
        .current_dir(repo.path())
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .status()
        .unwrap();
    assert!(init.success());
    fs::write(repo.path().join("body.md"), body_text).unwrap();
    let out = body(repo.path(), dir.path());
    unsafe { std::env::set_var("PATH", old) };
    out
}

fn calls(dir: &Path, program: &str) -> String {
    fs::read_to_string(dir.join(format!("{program}.args"))).unwrap_or_default()
}

fn pull_request(id: u32) -> String {
    format!(r#"{{"pullRequestId":{id},"repository":{{"webUrl":"{WEB}"}}}}"#)
}

fn url(id: u32) -> String {
    format!("{WEB}/pullrequest/{id}")
}

#[test]
fn opened() {
    // req:host/azure-pull-requests@03919edc
    let created = pull_request(12);
    let answers = [("pr list", "[]"), ("pr create", created.as_str())];
    with_fakes(&answers, "spec/x", "the body", |root, dir| {
        let got = Azure
            .page_open(root, "spec/x", "x", &root.join("body.md"))
            .unwrap();
        assert_eq!(got, Some(url(12)));
        let az = calls(dir, "az");
        let create = az.lines().find(|l| l.contains("repos pr create")).unwrap();
        for arg in [
            "--source-branch spec/x",
            "--target-branch main",
            "--title x",
            "--detect true",
            "--output json",
        ] {
            assert!(create.contains(arg), "{arg} missing from {create}");
        }
        let body = format!("--description @{}", root.join("body.md").display());
        assert!(create.contains(&body), "{body} missing from {create}");
        let list = az.lines().find(|l| l.contains("repos pr list")).unwrap();
        for arg in [
            "--source-branch spec/x",
            "--target-branch main",
            "--status active",
            "--detect true",
            "--output json",
        ] {
            assert!(list.contains(arg), "{arg} missing from {list}");
        }
        assert_eq!(calls(dir, "gh"), "");
    });
}

#[test]
fn a_relative_body_file_is_made_absolute() {
    // req:host/azure-pull-requests@03919edc
    let created = pull_request(12);
    let answers = [("pr list", "[]"), ("pr create", created.as_str())];
    with_fakes(&answers, "spec/x", "the body", |root, dir| {
        // `az` runs in the repository's root, but the caller's file is relative to the
        // caller's directory, so the argument names it absolutely.
        let cwd = std::env::current_dir().unwrap();
        let name = "azure-test-relative-body.md";
        fs::write(cwd.join(name), "the body").unwrap();
        let out = Azure.page_open(root, "spec/x", "x", Path::new(name));
        fs::remove_file(cwd.join(name)).unwrap();
        out.unwrap();
        let want = format!("--description @{}", cwd.join(name).display());
        assert!(calls(dir, "az").contains(&want), "{}", calls(dir, "az"));
    });
}

#[test]
fn already_open() {
    // req:host/azure-pull-requests@03919edc
    let listed = format!("[{}]", pull_request(12));
    with_fakes(
        &[("pr list", listed.as_str())],
        "spec/x",
        "b",
        |root, dir| {
            let got = Azure
                .page_open(root, "spec/x", "x", &root.join("body.md"))
                .unwrap();
            assert_eq!(got, Some(url(12)));
            assert!(!calls(dir, "az").contains("pr create"));
        },
    );
}

#[test]
fn body_updated() {
    // req:host/azure-pull-requests@03919edc
    let listed = format!("[{}]", pull_request(12));
    let answers = [("pr list", listed.as_str()), ("pr update", "{}")];
    with_fakes(&answers, "spec/x", "new body", |root, dir| {
        let said = Azure.page_body(root, &root.join("body.md")).unwrap();
        assert_eq!(said.as_deref(), Some("updated #12"));
        let az = calls(dir, "az");
        let update = az.lines().find(|l| l.contains("repos pr update")).unwrap();
        let body = format!("--description @{}", root.join("body.md").display());
        assert!(
            update.contains("--id 12") && update.contains(&body),
            "{update}"
        );
        // Nothing else that changes the pull request: the whole call is exactly this.
        assert_eq!(
            update,
            format!("repos pr update --id 12 {body} --detect true --output json")
        );
    });
}

#[test]
fn a_blank_or_missing_body_calls_nothing() {
    // req:host/azure-pull-requests@03919edc
    with_fakes(&[], "spec/x", "  \n", |root, dir| {
        assert!(Azure.page_body(root, &root.join("body.md")).is_err());
        assert!(Azure.page_body(root, &root.join("missing.md")).is_err());
        assert_eq!(calls(dir, "az"), "");
    });
}

#[test]
fn no_active_pull_request_updates_nothing() {
    // req:host/azure-pull-requests@03919edc
    with_fakes(&[("pr list", "[]")], "spec/x", "b", |root, dir| {
        let err = Azure.page_body(root, &root.join("body.md")).unwrap_err();
        assert!(matches!(err, Error::Refused(_)), "{err:?}");
        assert!(!calls(dir, "az").contains("pr update"));
    });
}

#[test]
fn az_not_signed_in() {
    // req:host/azure-pull-requests@03919edc
    let answers = [("repos", "FAIL:Please run 'az login'")];
    with_fakes(&answers, "spec/x", "b", |root, _| {
        let err = Azure
            .page_open(root, "spec/x", "x", &root.join("body.md"))
            .unwrap_err();
        match err {
            Error::Git(message) => assert!(message.contains("az login"), "{message}"),
            other => panic!("{other:?}"),
        }
        let err = Azure.page_body(root, &root.join("body.md")).unwrap_err();
        assert!(format!("{err:?}").contains("az login"), "{err:?}");
    });
}

#[test]
fn issue_has_nowhere_to_go() {
    // req:host/azure-other-operations@1a35e857
    with_fakes(&[], "main", "the report", |root, dir| {
        let err = Azure
            .issue(root, "x", Path::new("body.md"), "clunny/clm")
            .unwrap_err();
        match err {
            Error::Nowhere(report) => assert_eq!(report, "the report"),
            other => panic!("{other:?}"),
        }
        assert_eq!(
            (calls(dir, "az"), calls(dir, "gh")),
            (String::new(), String::new())
        );
    });
}

#[test]
fn contributions_and_alerts_are_not_there() {
    // req:host/azure-other-operations@1a35e857
    with_fakes(&[], "main", "b", |root, dir| {
        let body = root.join("body.md");
        assert_eq!(Azure.parent(root).unwrap(), None);
        assert!(Azure.list(root).unwrap().is_empty());
        assert!(Azure.alerts(root).unwrap().is_empty());
        let no_host = |r: Result<(), Error>| match r {
            Err(Error::Refused(why)) => assert!(why.contains("no host"), "{why}"),
            other => panic!("{other:?}"),
        };
        no_host(Azure.show(root, 7).map(drop));
        no_host(Azure.sync(root));
        no_host(Azure.open_on_parent(root, "spec/x", "x", &body).map(drop));
        no_host(Azure.fetch(root, 7, "pr-7"));
        no_host(Azure.comment(root, 7, &body, true));
        no_host(Azure.label(root, 7, "x"));
        assert_eq!(
            (calls(dir, "az"), calls(dir, "gh")),
            (String::new(), String::new())
        );
    });
}
