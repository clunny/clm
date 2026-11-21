//! `clm init`, `clm survey` and `clm guide` as calls of `clm-init` (`commands-in-crate`):
//! what each writes to the two streams and the status it returns, with the process going on.

use std::path::Path;
use std::process::Command;

use tempfile::TempDir;

mod common;

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

/// Calls `clm <args>` in `cwd` with no environment but git's own config kept out and git
/// looking no higher than `cwd`'s parent: the status and both buffers' text.
fn run(cwd: &Path, args: &[&str]) -> (u8, String, String) {
    let ceiling = cwd.parent().unwrap().to_str().unwrap();
    let [global, nosystem] = common::ISOLATED_GIT;
    let vars = [global, nosystem, ("GIT_CEILING_DIRECTORIES", ceiling)];
    let out = common::call(cwd, args, &|_| None, &vars);
    (
        u8::try_from(out.status.code().unwrap()).unwrap(),
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
    )
}

#[test]
fn init_writes_what_it_prints_to_the_buffers() {
    // req:init/commands-in-crate@a3067317
    let tmp = TempDir::new().unwrap();
    git(tmp.path(), &["init", "-q", "-b", "main"]);
    let (code, out, err) = run(tmp.path(), &["init", "--lang", "rust"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(err, "");
    assert!(out.contains("wrote clm.toml"), "{out}");
    assert!(out.contains("Next steps (yours):"), "{out}");
    assert!(tmp.path().join("clm.toml").is_file());

    // Outside a git repository: the prefixed error on the error stream, and 1.
    let plain = TempDir::new().unwrap();
    let (code, out, err) = run(plain.path(), &["init"]);
    assert_eq!((code, out.as_str()), (1, ""), "{err}");
    assert!(
        err.starts_with("clm init: not in a git repository"),
        "{err}"
    );
}

#[test]
fn an_option_init_does_not_have() {
    // req:init/commands-in-crate@a3067317
    let tmp = TempDir::new().unwrap();
    let (code, out, err) = run(tmp.path(), &["init", "--no-such-option"]);
    assert_eq!((code, out.as_str()), (2, ""), "{err}");
    assert!(
        err.contains("unexpected argument '--no-such-option'"),
        "{err}"
    );
    assert!(err.contains("Usage: clm init"), "{err}");
    // The test's process goes on: the same call parses its own options after it.
    let (code, ..) = run(
        tmp.path(),
        &["init", "--check-permissions", "--lang", "rust"],
    );
    assert_eq!(code, 2);
}

#[test]
fn survey_and_guide_refuse_what_they_do_not_have() {
    // req:init/commands-in-crate@a3067317
    let tmp = TempDir::new().unwrap();
    let (code, out, err) = run(tmp.path(), &["survey", "--nope"]);
    assert_eq!((code, out.as_str()), (2, ""), "{err}");
    assert!(err.contains("Usage: clm survey"), "{err}");
    let (code, out, err) = run(tmp.path(), &["guide", "a", "b"]);
    assert_eq!((code, out.as_str()), (2, ""), "{err}");
    assert!(err.contains("Usage: clm guide"), "{err}");
}

#[test]
fn survey_writes_its_map_and_its_error_with_the_prefix() {
    // req:init/commands-in-crate@a3067317
    let tmp = TempDir::new().unwrap();
    git(tmp.path(), &["init", "-q", "-b", "main"]);
    std::fs::write(tmp.path().join("a.rs"), "fn a() {}\n").unwrap();
    git(tmp.path(), &["add", "a.rs"]);
    let (code, out, err) = run(tmp.path(), &["survey"]);
    assert_eq!((code, err.as_str()), (0, ""), "{out}");
    assert!(out.starts_with("# directories\n"), "{out}");
    let (code, out, err) = run(tmp.path(), &["survey", "--components", "missing.yaml"]);
    assert_eq!((code, out.as_str()), (2, ""), "{err}");
    assert!(err.starts_with("clm survey: missing.yaml: "), "{err}");
}

#[test]
fn guide_writes_the_page_or_the_unknown_page_error() {
    // req:init/commands-in-crate@a3067317
    let tmp = TempDir::new().unwrap();
    let (code, out, err) = run(tmp.path(), &["guide", "constraints"]);
    assert_eq!((code, err.as_str()), (0, ""));
    assert!(out.starts_with("# CLM's constraints\n"), "{out}");
    let (code, out, err) = run(tmp.path(), &["guide", "no-such-page"]);
    assert_eq!((code, out.as_str()), (1, ""));
    assert!(err.starts_with("no guide page no-such-page\n"), "{err}");
}

#[test]
fn survey_maps_the_root_it_is_given_or_its_root_option() {
    // req:init/commands-in-crate@a3067317
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("repo");
    std::fs::create_dir_all(repo.join("tool/src")).unwrap();
    git(&repo, &["init", "-q", "-b", "main"]);
    std::fs::write(repo.join("top.rs"), "fn t() {}\n").unwrap();
    std::fs::write(repo.join("tool/src/a.rs"), "fn a() {}\n").unwrap();
    git(&repo, &["add", "-A"]);
    let map = |cwd: &Path, root: Option<&Path>, args: &[&str]| {
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = clm_init::survey_command(
            args.iter().copied(),
            root,
            cwd,
            &|_| None,
            &mut out,
            &mut err,
        );
        assert_eq!(code, 0, "{}", String::from_utf8_lossy(&err));
        String::from_utf8(out).unwrap()
    };
    let whole = map(&repo, None, &[]);
    assert!(whole.contains("\ntool/src  rust"), "{whole}");
    // The root `crates/clm` found is mapped, not the repository's top.
    let found = map(&repo, Some(&repo.join("tool")), &[]);
    assert!(found.contains("\nsrc  rust"), "{found}");
    assert!(!found.contains("tool/src"), "{found}");
    // `--root`, relative to the directory it runs in, overrides the root it is given.
    assert_eq!(map(&repo, Some(&repo), &["--root", "tool"]), found);
}

#[test]
fn help_asked_of_each_call() {
    // req:init/commands-in-crate@a3067317
    let tmp = TempDir::new().unwrap();
    let helps = [
        (
            "init",
            "Set up CLM's tooling in the CLM repository found from the current directory, or at the top of the git repository when there is none, and print the steps only a human can do",
            "Usage: clm init [OPTIONS]",
        ),
        (
            "survey",
            "Print a compact map of the repository's code, for drafting its spec graph",
            "Usage: clm survey [OPTIONS]",
        ),
        (
            "guide",
            "Print CLM's guide: with no page, the index of its pages; with one, that page",
            "Usage: clm guide [PAGE]",
        ),
    ];
    for (command, about, usage) in helps {
        let (code, out, err) = run(tmp.path(), &[command, "--help"]);
        assert_eq!((code, err.as_str()), (0, ""), "{command}: {out}");
        let first: Vec<&str> = out.lines().take(3).collect();
        assert_eq!(first, [about, "", usage], "{command}: {out}");
    }
}

/// An output stream that refuses every write.
struct Closed;

impl std::io::Write for Closed {
    fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
        Err(std::io::ErrorKind::BrokenPipe.into())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Err(std::io::ErrorKind::BrokenPipe.into())
    }
}

#[test]
fn a_closed_output_stream() {
    // req:init/commands-in-crate@a3067317
    let tmp = TempDir::new().unwrap();
    git(tmp.path(), &["init", "-q", "-b", "main"]);
    std::fs::write(tmp.path().join("a.rs"), "fn a() {}\n").unwrap();
    git(tmp.path(), &["add", "a.rs"]);
    let env = |_: &str| None;
    let none: [&str; 0] = [];
    let mut err = Vec::new();
    let init = clm_init::init_command(
        ["--host", "git"],
        None,
        tmp.path(),
        &env,
        &mut Closed,
        &mut err,
    );
    assert_eq!(init, 2, "{}", String::from_utf8_lossy(&err));
    assert!(tmp.path().join("clm.toml").is_file());
    let survey = clm_init::survey_command(none, None, tmp.path(), &env, &mut Closed, &mut err);
    let guide = clm_init::guide_command(none, None, tmp.path(), &env, &mut Closed, &mut err);
    assert_eq!((survey, guide), (2, 2));
}
