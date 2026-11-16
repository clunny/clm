//! `clm page`, `clm branch delete` and `clm issue` (specs/host/SPEC.md), with a fake `gh`
//! first on `PATH`.

#[path = "../../clm-orchestrator/tests/common/mod.rs"]
mod common;

use std::fs;
use std::path::Path;
use std::process::Command;

use common::*;
use tempfile::TempDir;

/// A fake `gh` whose `pr view` prints `$FAKE_PR_VIEW` (failing when it is unset), whose
/// `pr create` and `issue create` print a URL, or exit `$FAKE_CREATE_EXIT` when set; every
/// call is logged to `$FAKE_OUT/gh`, one per line.
struct Gh {
    dir: TempDir,
}

fn fake_gh() -> Gh {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("bin")).unwrap();
    write_exec(
        &dir.path().join("bin/gh"),
        "#!/bin/sh\necho \"$*\" >> \"$FAKE_OUT/gh\"\n\
         if [ \"$2\" = create ] && [ -n \"$FAKE_CREATE_EXIT\" ]; then \
         echo 'create failed' >&2; exit \"$FAKE_CREATE_EXIT\"; fi\n\
         case \"$1 $2\" in\n\
         'pr view') [ -z \"$FAKE_VIEW_ERROR\" ] || { echo \"$FAKE_VIEW_ERROR\" >&2; exit 1; }; \
         [ -n \"$FAKE_PR_VIEW\" ] || { echo 'no pull requests found' >&2; exit 1; }; \
         printf '%s\\n' \"$FAKE_PR_VIEW\";;\n\
         'pr create') while [ $# -gt 0 ]; do \
         [ \"$1\" = --body-file ] && cp \"$2\" \"$FAKE_OUT/body\"; shift; done; \
         echo 'Creating pull request'; echo https://github.com/o/r/pull/9;;\n\
         'issue create') echo https://github.com/o/r/issues/3;;\nesac\n",
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

/// `clm <args>` in `dir` with `gh` first on `PATH` and no user settings: exit code, stdout
/// and stderr.
fn clm(dir: &Path, gh: &Gh, args: &[&str], env: &[(&str, &str)]) -> (Option<i32>, String, String) {
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
            .envs(env.iter().copied()),
    );
    (
        out.status.code(),
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
    )
}

/// A checkout whose `clm.toml` sets `host`, on branch `spec/x`, holding `body.md`.
fn checkout(host: &str) -> Repo {
    let r = repo();
    write(&r.work, "clm.toml", &format!("host = \"{host}\"\n"));
    write(&r.work, "body.md", "The body\n");
    git(&r.work, &["checkout", "-q", "-b", "spec/x"]);
    r
}

const OPEN_ARGS: [&str; 7] = [
    "pr",
    "open",
    "spec/x",
    "--title",
    "x",
    "--body-file",
    "body.md",
];

const PR_7: &str = r#"{"number":7,"state":"OPEN","headRefName":"spec/x"}"#;
const BODY_ARGS: [&str; 3] = ["pr", "body", "body.md"];

#[test]
fn unloadable_settings_exit_2() {
    let (r, gh) = (checkout("github"), fake_gh());
    write(&r.work, "clm.toml", "host = \"nowhere\"\n");
    let (code, _, err) = clm(
        &r.work,
        &gh,
        &["branch", "delete", "impl/a", "--lease", "abc"],
        &[],
    );
    assert_eq!(code, Some(2), "{err}");
    assert!(err.contains("clm.toml"), "{err}");
}

#[test]
fn scenario_pr_with_no_host() {
    // req:clm/renamed-commands@164ad075
    let (r, gh) = (checkout("none"), fake_gh());
    let (code, out, err) = clm(&r.work, &gh, &OPEN_ARGS, &[]);
    assert_eq!((code, out.as_str()), (Some(0), "none\n"), "{err}");
    assert!(gh.calls().is_empty(), "{:?}", gh.calls());
    // A failure names the command as it was run.
    let (r, gh) = (checkout("github"), fake_gh());
    let (_, _, err) = clm(&r.work, &gh, &OPEN_ARGS, &[("FAKE_CREATE_EXIT", "1")]);
    assert!(err.contains("clm pr open"), "{err}");
    let env = [("FAKE_PR_VIEW", PR_7)];
    let (code, out, err) = clm(&r.work, &gh, &BODY_ARGS, &env);
    assert_eq!((code, out.as_str()), (Some(0), "updated #7\n"), "{err}");
    fs::remove_file(r.work.join("body.md")).unwrap();
    let (_, _, err) = clm(&r.work, &gh, &BODY_ARGS, &env);
    assert!(err.contains("clm pr body"), "{err}");
}
