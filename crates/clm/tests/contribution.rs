//! `clm contribution` (specs/SPEC.md, `clm/contribution-command`), with a fake `gh` first on
//! `PATH`.

#[path = "../../clm-orchestrator/tests/common/mod.rs"]
mod common;

use std::fs;
use std::path::Path;
use std::process::Command;

use common::*;
use tempfile::TempDir;

const PR_JSON: &str = r#"{"author":{"login":"dana"},"state":"OPEN","title":"Add an idea","body":"Please take this.\nThanks","headRepository":{"name":"clm"},"headRepositoryOwner":{"login":"dana"},"headRefName":"idea","maintainerCanModify":true,"files":[{"path":"backlog/x.md"}]}"#;
const FORK_JSON: &str = r#"{"isFork":true,"parent":{"name":"clm","owner":{"login":"clunny"}},"owner":{"login":"dana"},"name":"clm"}"#;

struct Gh {
    dir: TempDir,
}

/// A fake `gh` that logs each call to `$FAKE_OUT/gh`, answers `pr view` with `$FAKE_PR` and
/// `repo view` with `$FAKE_REPO`.
fn fake_gh() -> Gh {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("bin")).unwrap();
    write_exec(
        &dir.path().join("bin/gh"),
        "#!/bin/sh\necho \"$*\" >> \"$FAKE_OUT/gh\"\n\
         case \"$1 $2\" in\n\
         'pr view') printf '%s\\n' \"$FAKE_PR\";;\n\
         'repo view') printf '%s\\n' \"$FAKE_REPO\";;\n\
         'api users/dana') echo '{\"id\":4242}';;\n\
         'pr list') echo '[]';;\n\
         'pr create') echo https://github.com/clunny/clm/pull/9;;\nesac\n",
    );
    Gh { dir }
}

impl Gh {
    fn calls(&self) -> Vec<String> {
        fs::read_to_string(self.dir.path().join("gh"))
            .unwrap_or_default()
            .lines()
            .map(String::from)
            .collect()
    }
}

/// `clm <args>` in `dir` with the fake `gh` first on `PATH`: exit code, stdout and stderr.
fn clm(dir: &Path, gh: &Gh, args: &[&str]) -> (Option<i32>, String, String) {
    let path = format!(
        "{}:{}",
        gh.dir.path().join("bin").display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let out = output(
        Command::new(env!("CARGO_BIN_EXE_clm"))
            .args(args)
            .current_dir(dir)
            .env("PATH", path)
            .env("XDG_CONFIG_HOME", gh.dir.path())
            .env("FAKE_OUT", gh.dir.path())
            .env("FAKE_PR", PR_JSON)
            .env("FAKE_REPO", FORK_JSON),
    );
    (
        out.status.code(),
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
    )
}

/// A checkout whose `clm.toml` sets `host`, holding `c.md`.
fn checkout(host: &str) -> Repo {
    let r = repo();
    write(&r.work, "clm.toml", &format!("host = \"{host}\"\n"));
    write(&r.work, "c.md", "A comment\n");
    r
}

#[test]
fn scenario_show() {
    // req:clm/contribution-command@84f0a2d6
    let (r, gh) = (checkout("github"), fake_gh());
    let (code, out, err) = clm(&r.work, &gh, &["contribution", "show", "7"]);
    assert_eq!(code, Some(0), "{err}");
    let lines: Vec<&str> = out.lines().collect();
    for want in [
        "author: dana",
        "state: OPEN",
        "title: Add an idea",
        "head repository: https://github.com/dana/clm",
        "head branch: idea",
        "maintainers can edit: true",
        "file: backlog/x.md",
    ] {
        assert!(lines.contains(&want), "{want} missing from {out}");
    }
    let body = out.find("Please take this.\nThanks").expect(&out);
    assert!(out[body..].trim_end().ends_with("Thanks"), "{out}");
    assert!(
        lines.iter().position(|l| *l == "file: backlog/x.md") < Some(body_line(&out)),
        "{out}"
    );
}

/// The index of the line the body starts on: the first line after the `body:` label.
fn body_line(out: &str) -> usize {
    out.lines().position(|l| l == "body:").expect(out) + 1
}

#[test]
fn scenario_no_parent() {
    // req:clm/contribution-command@84f0a2d6
    let (r, gh) = (checkout("none"), fake_gh());
    let (code, out, err) = clm(&r.work, &gh, &["contribution", "parent"]);
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(out, "none\n");
    assert!(gh.calls().is_empty(), "{:?}", gh.calls());
}

#[test]
fn parent_of_a_fork_is_printed() {
    // req:clm/contribution-command@84f0a2d6
    let (r, gh) = (checkout("github"), fake_gh());
    let (code, out, err) = clm(&r.work, &gh, &["contribution", "parent"]);
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(out, "clunny/clm\n");
}

#[test]
fn scenario_no_host() {
    // req:clm/contribution-command@84f0a2d6
    let (r, gh) = (checkout("none"), fake_gh());
    let (code, _, err) = clm(
        &r.work,
        &gh,
        &["contribution", "comment", "7", "--body-file", "c.md"],
    );
    assert_eq!(code, Some(1), "{err}");
    assert!(err.contains("no host"), "{err}");
    assert!(gh.calls().is_empty(), "{:?}", gh.calls());
}

#[test]
fn every_other_operation_reports_no_host() {
    // req:clm/contribution-command@84f0a2d6
    let cases: [&[&str]; 5] = [
        &["sync"],
        &["open", "spec/x", "--title", "t", "--body-file", "c.md"],
        &["show", "7"],
        &["fetch", "7", "pr-7"],
        &["label", "7", "idea"],
    ];
    for args in cases {
        let (r, gh) = (checkout("git"), fake_gh());
        let mut full = vec!["contribution"];
        full.extend(args);
        let (code, out, err) = clm(&r.work, &gh, &full);
        assert_eq!(code, Some(1), "{args:?}: {err}");
        assert!(out.is_empty(), "{args:?}: {out}");
        assert!(err.contains("no host"), "{args:?}: {err}");
        assert!(gh.calls().is_empty(), "{args:?}: {:?}", gh.calls());
    }
}

#[test]
fn comment_close_and_label_reach_the_host() {
    // req:clm/contribution-command@84f0a2d6
    let (r, gh) = (checkout("github"), fake_gh());
    let (code, _, err) = clm(
        &r.work,
        &gh,
        &[
            "contribution",
            "comment",
            "7",
            "--body-file",
            "c.md",
            "--close",
        ],
    );
    assert_eq!(code, Some(0), "{err}");
    let (code, _, err) = clm(&r.work, &gh, &["contribution", "label", "7", "idea"]);
    assert_eq!(code, Some(0), "{err}");
    let calls = gh.calls();
    assert!(calls.contains(&"issue comment 7 --body-file c.md".to_string()));
    assert!(calls.contains(&"issue close 7".to_string()), "{calls:?}");
    assert!(
        calls.contains(&"issue edit 7 --add-label idea".to_string()),
        "{calls:?}"
    );
}

#[test]
fn open_prints_the_parents_pull_request() {
    // req:clm/contribution-command@84f0a2d6
    let (r, gh) = (checkout("github"), fake_gh());
    let (code, out, err) = clm(
        &r.work,
        &gh,
        &[
            "contribution",
            "open",
            "spec/x",
            "--title",
            "t",
            "--body-file",
            "c.md",
        ],
    );
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(out, "https://github.com/clunny/clm/pull/9\n");
}

#[test]
fn a_failing_operation_exits_1_with_its_reason() {
    // req:clm/contribution-command@84f0a2d6
    let (r, gh) = (checkout("github"), fake_gh());
    write_exec(
        &gh.dir.path().join("bin/gh"),
        "#!/bin/sh\necho 'gh exploded' >&2\nexit 1\n",
    );
    for args in [
        &["contribution", "label", "7", "idea"][..],
        &["contribution", "show", "7"][..],
        &["contribution", "sync"][..],
    ] {
        let (code, _, err) = clm(&r.work, &gh, args);
        assert_eq!(code, Some(1), "{args:?}: {err}");
        assert!(err.contains("gh exploded"), "{args:?}: {err}");
    }
}

#[test]
fn comment_without_close_leaves_it_open() {
    // req:clm/contribution-command@84f0a2d6
    let (r, gh) = (checkout("github"), fake_gh());
    let (code, _, err) = clm(
        &r.work,
        &gh,
        &["contribution", "comment", "7", "--body-file", "c.md"],
    );
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(gh.calls(), vec!["issue comment 7 --body-file c.md"]);
}

#[test]
fn sync_reaches_the_host() {
    // req:clm/contribution-command@84f0a2d6
    let (r, gh) = (checkout("github"), fake_gh());
    let (code, _, err) = clm(&r.work, &gh, &["contribution", "sync"]);
    assert_eq!(code, Some(0), "{err}");
    let calls = gh.calls();
    assert!(
        calls.iter().any(|c| c.starts_with("repo sync dana/clm")),
        "{calls:?}"
    );
}
