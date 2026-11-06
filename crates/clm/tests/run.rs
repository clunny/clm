//! `clm run` with a fake `claude` on PATH, so nothing touches the network or a model.
//! `clm run` prepares the build's branch and prints its prompt; it starts no session.

#[path = "../../clm-orchestrator/tests/common/mod.rs"]
mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use common::*;
use tempfile::TempDir;

const SKILLS: [&str; 3] = [
    "writing-plans",
    "test-driven-development",
    "verification-before-completion",
];

/// A fake `claude` that records that it ran, and a superpowers copy holding the three
/// skills.
struct Fake {
    _dir: TempDir,
    bin: PathBuf,
    out: PathBuf,
    superpowers: PathBuf,
}

fn fake() -> Fake {
    let dir = TempDir::new().unwrap();
    let bin = dir.path().join("bin");
    let out = dir.path().join("out");
    let superpowers = dir.path().join("superpowers");
    for d in [&bin, &out] {
        fs::create_dir_all(d).unwrap();
    }
    for s in SKILLS {
        write(&superpowers, &format!("skills/{s}/SKILL.md"), "skill\n");
    }
    write_exec(&bin.join("claude"), "#!/bin/sh\ntouch \"$FAKE_OUT/ran\"\n");
    Fake {
        _dir: dir,
        bin,
        out,
        superpowers,
    }
}

/// What a `clm run` printed.
struct Ran {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

fn clm_run(r: &Repo, f: &Fake, node: &str, env: &[(&str, &Path)]) -> Ran {
    clm_run_in(&r.work, &r.work, f, node, env)
}

fn clm_run_in(cwd: &Path, root: &Path, f: &Fake, node: &str, env: &[(&str, &Path)]) -> Ran {
    clm_run_with(cwd, root, f, &[node], env)
}

/// `clm run <args>` in `cwd`.
fn clm_run_with(cwd: &Path, root: &Path, f: &Fake, args: &[&str], env: &[(&str, &Path)]) -> Ran {
    let path = format!(
        "{}:{}",
        f.bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    // The user's settings and a `CARGO_TARGET_DIR` from the test's own environment stay
    // out, unless a test sets them.
    let out = output(
        Command::new(env!("CARGO_BIN_EXE_clm"))
            .arg("run")
            .args(args)
            .arg("--root")
            .arg(root)
            .current_dir(cwd)
            .env("PATH", path)
            .env("FAKE_OUT", &f.out)
            .env("CLM_SUPERPOWERS_DIR", &f.superpowers)
            .env("XDG_CONFIG_HOME", &f.out)
            .env_remove("CARGO_TARGET_DIR")
            .envs(env.iter().map(|(k, v)| (*k, v.as_os_str()))),
    );
    Ran {
        code: out.status.code(),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

/// What a `clm run` that must succeed printed, the launcher, and the prompt it wrote to
/// `.clm/prompts/<node>.md` under `root`, which the launcher names by its absolute path.
fn launched_in(
    cwd: &Path,
    root: &Path,
    f: &Fake,
    node: &str,
    env: &[(&str, &Path)],
) -> (String, String) {
    let ran = clm_run_in(cwd, root, f, node, env);
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    let launcher = ran.stdout;
    assert!(launcher.lines().count() <= 5, "{launcher}");
    let file = cwd
        .join(root)
        .canonicalize()
        .unwrap()
        .join(format!(".clm/prompts/{node}.md"));
    assert!(file.is_absolute());
    assert!(
        launcher.contains(&file.display().to_string()),
        "{} missing from {launcher}",
        file.display()
    );
    assert!(
        launcher.contains(&format!("git switch impl/{node}")),
        "{launcher}"
    );
    (launcher, fs::read_to_string(&file).unwrap())
}

/// The prompt of a `clm run` that must succeed.
fn prompt(r: &Repo, f: &Fake, node: &str, env: &[(&str, &Path)]) -> String {
    launched_in(&r.work, &r.work, f, node, env).1
}

/// A `clm run` that must refuse with `code`, printing nothing on stdout.
fn refused(r: &Repo, f: &Fake, node: &str, env: &[(&str, &Path)], code: i32) -> String {
    let ran = clm_run(r, f, node, env);
    assert_eq!(ran.code, Some(code), "{}", ran.stderr);
    assert_eq!(ran.stdout, "", "nothing on stdout");
    ran.stderr
}

fn ran(f: &Fake) -> bool {
    f.out.join("ran").exists()
}

fn local_tip(r: &Repo, branch: &str) -> Option<String> {
    let out = output(
        Command::new("git")
            .args([
                "rev-parse",
                "--verify",
                "-q",
                &format!("refs/heads/{branch}"),
            ])
            .current_dir(&r.work),
    );
    out.status
        .success()
        .then(|| String::from_utf8(out.stdout).unwrap().trim().to_string())
}

/// Whether some worktree has `branch` checked out.
fn checked_out(r: &Repo, branch: &str) -> bool {
    git(&r.work, &["worktree", "list", "--porcelain"])
        .lines()
        .any(|l| l == format!("branch refs/heads/{branch}"))
}

#[test]
fn scenario_node_prepared() {
    // req:orchestrator.run/run@0691bb50
    let (r, f) = (repo(), fake());
    let (launcher, p) = launched_in(&r.work, &r.work, &f, "b", &[]);
    let bundle = clm_context::bundle(&load(&r.work), &r.work, "b").unwrap();
    assert!(!launcher.contains(bundle.trim()), "{launcher}");
    assert!(launcher.contains("before anything else"), "{launcher}");

    let tip = remote_tip(&r, "impl/b").expect("origin/impl/b exists");
    assert_eq!(
        git(&r.origin, &["log", "-1", "--format=%s", &tip]).trim(),
        "Claim b"
    );
    assert_eq!(local_tip(&r, "impl/b").as_deref(), Some(tip.as_str()));
    assert_eq!(
        git(&r.work, &["rev-parse", "--abbrev-ref", "impl/b@{upstream}"]).trim(),
        "origin/impl/b"
    );
    assert!(!checked_out(&r, "impl/b"), "no worktree has impl/b");
    assert!(!ran(&f), "claude must not start");
    assert_eq!(git(&r.work, &["branch", "--show-current"]).trim(), "main");
    assert_eq!(git(&r.work, &["status", "--porcelain"]), "");
    let exclude = fs::read_to_string(r.work.join(".git/info/exclude")).unwrap();
    assert!(exclude.lines().any(|l| l == ".clm/"), "{exclude}");

    let first = p.lines().next().unwrap();
    assert!(first.contains("git switch impl/b"), "{first}");
    assert!(first.contains("git branch --show-current"), "{first}");
    for needle in [
        "You are running unattended",
        "confirm a tool call",
        "`landed <sha>`",
        "stopped: <reason>",
        ".clm/plans/b.md",
        "executing-plans",
        "/code-review",
        "spec-auditor",
        "## Review",
        "clm rebase",
        "spec changed",
        "clm pending b",
        "## Sync",
        "Then land with `clm land`",
        "`b/r` unbuilt",
    ] {
        assert!(p.contains(needle), "{needle} missing from {p}");
    }
    for s in SKILLS {
        let path = f.superpowers.join(format!("skills/{s}/SKILL.md"));
        assert!(
            p.contains(&format!("`superpowers:{s}`, at `{}`", path.display())),
            "{s} with its path missing from {p}"
        );
    }
    assert!(p.contains("read and follow that file"), "{p}");
    assert!(
        !p.contains("already exists"),
        "a fresh claim isn't a resume: {p}"
    );
    assert!(p.ends_with(&bundle), "{p}");
}

#[test]
fn an_earlier_prompt_file_is_replaced() {
    // req:orchestrator.run/run@0691bb50
    let (r, f) = (repo(), fake());
    write(&r.work, ".clm/prompts/b.md", "an earlier run's prompt\n");
    let p = prompt(&r, &f, "b", &[]);
    assert!(!p.contains("an earlier run's prompt"), "{p}");
    assert!(p.starts_with("Run `git switch impl/b`"), "{p}");
}

#[test]
fn a_refused_run_leaves_an_earlier_prompt_file_alone() {
    // req:orchestrator.run/run@0691bb50
    let (r, f) = (repo(), fake());
    write(&r.work, ".clm/prompts/b.md", "an earlier run's prompt\n");
    push_claim(&r.work, "b", 1);
    refused(&r, &f, "b", &[], 1);
    assert_eq!(
        fs::read_to_string(r.work.join(".clm/prompts/b.md")).unwrap(),
        "an earlier run's prompt\n"
    );
}

#[test]
fn prompts_directory_that_cant_be_made_refuses_before_claiming() {
    // req:orchestrator.run/run@0691bb50
    let (r, f) = (repo(), fake());
    write(&r.work, ".clm", "a file, not a directory\n");
    let stderr = refused(&r, &f, "b", &[], 1);
    assert!(stderr.contains(".clm/prompts"), "{stderr}");
    assert!(remote_tip(&r, "impl/b").is_none(), "no claim");
}

#[test]
fn prompt_file_that_cant_be_written_exits_1_naming_it() {
    // req:orchestrator.run/run@0691bb50
    let (r, f) = (repo(), fake());
    fs::create_dir_all(r.work.join(".clm/prompts/b.md")).unwrap();
    let stderr = refused(&r, &f, "b", &[], 1);
    assert!(stderr.contains(".clm/prompts/b.md"), "{stderr}");
    // The claim stays, for the next `clm run` to resume.
    assert!(remote_tip(&r, "impl/b").is_some());
    assert!(local_tip(&r, "impl/b").is_some());
}

#[test]
fn scenario_check_command_named() {
    // req:orchestrator.run/prompt-check-command@6b68f4d5
    let (r, f) = (repo(), fake());
    write(
        &r.work,
        "clm.toml",
        "[land]\ncheck = \"scripts/check.sh\"\n",
    );
    let p = prompt(&r, &f, "b", &[]);
    for needle in [
        "run `clm land --check-only`, which runs `scripts/check.sh`, the command `clm.toml` \
         names, and then `clm check`",
        "from the worktree's top level as one plain command, exactly as written, adding no \
         `cd`",
        "pass the repo's checks again with `clm land --check-only`, which runs \
         `scripts/check.sh` and then `clm check`",
        "report the refusal with `clm defect`",
        "`## CLM defects`",
        "land with `clm land`, which runs the whole command",
    ] {
        assert!(p.contains(needle), "{needle} missing from {p}");
    }
    assert!(!p.contains("run `scripts/check.sh`"), "{p}");
    assert!(!p.contains("with `scripts/check.sh`"), "{p}");
}

#[test]
fn scenario_no_check_command() {
    // req:orchestrator.run/prompt-check-command@6b68f4d5
    let (r, f) = (repo(), fake());
    write(&r.work, "clm.toml", "host = \"none\"\n");
    let p = prompt(&r, &f, "b", &[]);
    assert!(!p.contains("one plain command"), "{p}");
    assert!(!p.contains("--check-only"), "{p}");
}

#[test]
fn scenario_unreadable_settings() {
    // req:orchestrator.run/prompt-check-command@6b68f4d5
    let (r, f) = (repo(), fake());
    write(&r.work, "clm.toml", "[land\ncheck = \n");
    let stderr = refused(&r, &f, "b", &[], 1);
    assert!(stderr.contains("clm.toml"), "{stderr}");
    assert!(remote_tip(&r, "impl/b").is_none(), "no claim");
}

#[test]
fn scenario_no_target_directory() {
    // req:orchestrator.run/prompt-check-command@6b68f4d5
    for target in [None, Some("/elsewhere")] {
        let (r, f) = (repo(), fake());
        let linked = r.work.parent().unwrap().join("linked");
        let args = ["worktree", "add", "-q", "--detach"];
        let args = [&args[..], &[linked.to_str().unwrap(), "main"]].concat();
        git(&r.work, &args);
        write(
            &linked,
            "clm.toml",
            "[land]\ncheck = \"scripts/check.sh\"\n",
        );
        let env: Vec<(&str, &Path)> = target
            .iter()
            .map(|t| ("CARGO_TARGET_DIR", Path::new(t)))
            .collect();
        let (_, p) = launched_in(&linked, &linked, &f, "b", &env);
        assert!(p.contains("`scripts/check.sh`"), "{p}");
        assert!(!p.contains("CARGO_TARGET_DIR"), "{p}");
        assert!(!p.contains("target/build"), "{p}");
    }
}

#[test]
fn scenario_prompt_commands_and_pr_body() {
    // req:orchestrator.run/prompt-commands@9008204d
    // req:orchestrator.run/prompt-pr-body@ac48f44f
    let (r, f) = (repo(), fake());
    let p = prompt(&r, &f, "b", &[]);
    for needle in [
        "rebase ok",
        "rebase failed",
        "remote rejected",
        "stale info",
        "clm pr open impl/b",
        "--body-file .clm/pr-body.md",
        "clm pr body .clm/pr-body.md",
        "Then land with `clm land`",
    ] {
        assert!(p.contains(needle), "{needle} missing from {p}");
    }
    for forbidden in [
        "one command per Bash call",
        "exits 0",
        "exits 2",
        "gh pr create",
        "--body ",
        "gh pr edit",
        "clm watch",
        "gh run rerun",
    ] {
        assert!(!p.contains(forbidden), "{forbidden} in {p}");
    }
}

#[test]
fn scenario_capability_nodes_prompt() {
    // req:orchestrator.run/prompt-serves@bf9d2c22
    let (r, f) = (repo(), fake());
    write(
        &r.work,
        "CAPABILITIES.md",
        "## Capabilities\n\n### Capability: k\nA person can k.\n\n#### Why\nw\n\n\
         #### Scenario: s\n- ok\n",
    );
    write(&r.work, "specs/capability/SPEC.md", &node("", &[]));
    push_main(&r.work, "capabilities");
    let p = prompt(&r, &f, "capability", &[]);
    for needle in ["# Capabilities served", "public boundary"] {
        assert!(p.contains(needle), "{needle} missing from {p}");
    }
    let p = prompt(&r, &f, "b", &[]);
    assert!(!p.contains("public boundary"), "{p}");
}

#[test]
fn root_builds_the_capability_tests_without_a_capability_node() {
    // req:orchestrator.run/prompt-serves@bf9d2c22
    let (r, f) = (repo(), fake());
    write(
        &r.work,
        "CAPABILITIES.md",
        "## Capabilities\n\n### Capability: k\nA person can k.\n\n#### Why\nw\n\n\
         #### Scenario: s\n- ok\n",
    );
    push_main(&r.work, "capabilities");
    let p = prompt(&r, &f, "clm", &[]);
    assert!(p.contains("public boundary"), "{p}");
}

/// The prompt `clm run <args>` writes for node `b`, and its paragraphs that name
/// `spec-auditor`.
fn review_prompt(args: &[&str]) -> (String, Vec<String>) {
    let (r, f) = (repo(), fake());
    let ran = clm_run_with(&r.work, &r.work, &f, args, &[]);
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    let p = fs::read_to_string(r.work.join(".clm/prompts/b.md")).unwrap();
    let auditor = p
        .split("\n\n")
        .filter(|para| para.contains("spec-auditor"))
        .map(str::to_string)
        .collect();
    (p, auditor)
}

#[test]
fn scenario_review_model_given() {
    // req:orchestrator.run/review-model@601bdd45
    let (p, auditor) = review_prompt(&["b", "--review-model", "opus"]);
    assert!(
        auditor.iter().any(|para| para.contains("model: \"opus\"")),
        "{auditor:?}"
    );
    assert!(p.contains("model: \"sonnet\""), "{p}");
}

#[test]
fn scenario_no_review_model() {
    // req:orchestrator.run/review-model@601bdd45
    let (p, auditor) = review_prompt(&["b"]);
    assert!(
        auditor.iter().any(|para| para.contains("your own model")),
        "{auditor:?}"
    );
    assert!(
        auditor.iter().all(|para| !para.contains("model: \"")),
        "{auditor:?}"
    );
    assert!(p.contains("model: \"sonnet\""), "{p}");
    assert!(!p.contains("model: \"opus\""), "{p}");
    assert!(
        p.contains("never launch a subagent without naming its model"),
        "{p}"
    );
}

/// Node `b` whose `b/x` moved from `11111111` (an earlier commit's text) to the current
/// text, `b/y` uncited, and `t.rs:9` citing `b/gone`, which a live change removed.
fn behind_and_obsolete() -> Repo {
    let r = repo();
    let w = &r.work;
    write(
        w,
        "specs/b/SPEC.md",
        &node_with("", "", &[("x", "It SHALL x."), ("y", "It SHALL y.")]),
    );
    push_main(w, "b at first");
    let first = load(w).requirement("b/x").unwrap().version().to_string();
    write(
        w,
        "specs/b/SPEC.md",
        &node_with("", "", &[("x", "It SHALL x, more."), ("y", "It SHALL y.")]),
    );
    write(w, "changes/g/change.md", "---\nstatus: applied\n---\n");
    write(
        w,
        "changes/g/deltas/b.md",
        "---\ncomponent: b\n---\n## REMOVED Requirements\n\n### Requirement: gone <!-- base: 0000000000000000 -->\n",
    );
    let mut t = String::new();
    for _ in 0..8 {
        t.push_str("//\n");
    }
    t.push_str(&format!("// {}:b/gone\n", "req"));
    t.push_str(&format!("// {}:b/x@{first}\n", "req"));
    // A version `specs/b/SPEC.md` never had.
    t.push_str(&format!("// {}:b/x@deadbeef\n", "req"));
    // A bare marker, which cites no version.
    t.push_str(&format!("// {}:b/x\n", "req"));
    write(w, "t.rs", &t);
    // A marker at the current version, which the prompt doesn't name.
    let now = load(w).requirement("b/x").unwrap().version().to_string();
    write(w, "crates/b/t.rs", &format!("// {}:b/x@{now}\n", "req"));
    push_main(w, "b moved, gone removed");
    r
}

#[test]
fn scenario_behind_and_obsolete() {
    // req:orchestrator.run/pending-prompt@dea4856a
    let (r, f) = (behind_and_obsolete(), fake());
    let p = prompt(&r, &f, "b", &[]);
    let current = load(&r.work)
        .requirement("b/x")
        .unwrap()
        .version()
        .to_string();
    assert!(p.contains(&format!("`b/x` behind `{current}`")), "{p}");
    assert!(p.contains("-It SHALL x.\n+It SHALL x, more."), "{p}");
    assert!(
        p.contains("version `deadbeef` wasn't found in the history of `specs/b/SPEC.md`"),
        "{p}"
    );
    assert!(p.contains("`b/y` unbuilt"), "{p}");
    assert!(p.contains("`b/gone` obsolete"), "{p}");
    assert!(p.contains("t.rs:9"), "{p}");
    assert!(p.contains("Re-cite each marker"), "{p}");
    assert!(p.contains("t.rs:10, t.rs:11, t.rs:12"), "{p}");
    assert!(!p.contains("crates/b/t.rs:1"), "{p}");
    for needle in ["req:<id>@<version>", "## Left pending", "clm pending b"] {
        assert!(p.contains(needle), "{needle} missing from {p}");
    }
}

#[test]
fn pending_requirements_are_listed_most_urgent_first() {
    // req:orchestrator.run/pending-prompt@dea4856a
    let (r, f) = (repo(), fake());
    // `b/r` comes from `x` at no priority; `u` adds `b/u` at P1 and `t` adds `b/t` at P3.
    write(&r.work, "specs/b/SPEC.md", &node("", &["r", "t", "u"]));
    write(
        &r.work,
        "changes/u/change.md",
        "---\nstatus: applied\npriority: P1\n---\n",
    );
    write(&r.work, "changes/u/deltas/b.md", &delta("b", &["u"]));
    write(
        &r.work,
        "changes/t/change.md",
        "---\nstatus: applied\npriority: P3\n---\n",
    );
    write(&r.work, "changes/t/deltas/b.md", &delta("b", &["t"]));
    push_main(&r.work, "u and t");
    let p = prompt(&r, &f, "b", &[]);
    let at = |id: &str| {
        p.find(&format!("- `{id}` unbuilt"))
            .unwrap_or_else(|| panic!("{id}: {p}"))
    };
    assert!(at("b/u") < at("b/t") && at("b/t") < at("b/r"), "{p}");
}

#[test]
fn scenario_blocked_requirement_left_out() {
    // req:orchestrator.run/pending-prompt@dea4856a
    let (r, f) = (repo(), fake());
    write(
        &r.work,
        "specs/b/SPEC.md",
        &node_with("", "", &[("x", "It SHALL x."), ("y", "It SHALL y.")]),
    );
    write(
        &r.work,
        "backlog/q.md",
        "---\ntitle: Q\nkind: question\nblocks: [b/y]\n---\n",
    );
    push_main(&r.work, "b and q");
    let p = prompt(&r, &f, "b", &[]);
    assert!(p.contains("- `b/x` unbuilt"), "{p}");
    assert!(!p.contains("- `b/y` unbuilt"), "{p}");
    assert!(p.contains("`b/y`, blocked by `backlog/q`"), "{p}");
}

#[test]
fn scenario_needing_requirement_left_out() {
    // req:orchestrator.run/pending-prompt@dea4856a
    let (r, f) = (repo(), fake());
    write(
        &r.work,
        "specs/b/SPEC.md",
        &node_with(
            "",
            "",
            &[
                ("x", "It SHALL x."),
                ("y", "It SHALL y.\n\n#### Needs\n- c/s"),
            ],
        ),
    );
    push_main(&r.work, "b/y needs c/s");
    let p = prompt(&r, &f, "b", &[]);
    assert!(p.contains("- `b/x` unbuilt"), "{p}");
    assert!(!p.contains("- `b/y` unbuilt"), "{p}");
    assert!(p.contains("`b/y`, blocked by `c/s`"), "{p}");
}

#[test]
fn pinned_superpowers_comes_from_the_cache() {
    // req:orchestrator.run/run@0691bb50
    let (r, f) = (repo(), fake());
    let cache = TempDir::new().unwrap();
    let pinned = cache
        .path()
        .join("clm/superpowers")
        .join(clm_run::SUPERPOWERS_COMMIT);
    for s in SKILLS {
        write(&pinned, &format!("skills/{s}/SKILL.md"), "skill\n");
    }
    // An empty override counts as unset.
    let env = [
        ("XDG_CACHE_HOME", cache.path()),
        ("CLM_SUPERPOWERS_DIR", Path::new("")),
    ];
    let p = prompt(&r, &f, "b", &env);
    let path = pinned.join("skills/writing-plans/SKILL.md");
    assert!(p.contains(&path.display().to_string()), "{p}");
}

#[test]
fn missing_skill_refuses_before_claiming() {
    // req:orchestrator.run/run@0691bb50
    let (r, f) = (repo(), fake());
    fs::remove_dir_all(f.superpowers.join("skills/writing-plans")).unwrap();
    let stderr = refused(&r, &f, "b", &[], 1);
    assert!(stderr.contains("writing-plans"), "{stderr}");
    assert!(remote_tip(&r, "impl/b").is_none(), "no claim");
}

#[test]
fn scenario_claimed_elsewhere() {
    // req:orchestrator.run/run@0691bb50
    let (r, f) = (repo(), fake());
    push_claim(&r.work, "b", 1);
    refused(&r, &f, "b", &[], 1);
    assert!(!ran(&f), "claude must not start");
    assert!(local_tip(&r, "impl/b").is_none());
}

#[test]
fn claim_failing_in_git_exits_2() {
    // req:orchestrator.run/run@0691bb50
    let (r, f) = (repo(), fake());
    pre_receive(&r, "echo 'declined by test' >&2\nexit 1\n");
    refused(&r, &f, "b", &[], 2);
    assert!(local_tip(&r, "impl/b").is_none());
}

#[test]
fn claim_released_when_the_branch_cant_be_made() {
    // req:orchestrator.run/run@0691bb50
    let (r, f) = (repo(), fake());
    // A stale lock makes `git branch` fail after the claim is pushed.
    write(&r.work, ".git/refs/heads/impl/b.lock", "");
    refused(&r, &f, "b", &[], 2);
    assert!(remote_tip(&r, "impl/b").is_none(), "the claim is released");
}

#[test]
fn main_checkout_on_the_branch_is_refused() {
    // req:orchestrator.run/run@0691bb50
    let (r, f) = (repo(), fake());
    prompt(&r, &f, "b", &[]);
    git(&r.work, &["checkout", "-q", "impl/b"]);
    let stderr = refused(&r, &f, "b", &[], 1);
    assert!(
        stderr.contains(&r.work.canonicalize().unwrap().display().to_string()),
        "{stderr}"
    );
    assert_eq!(git(&r.work, &["branch", "--show-current"]).trim(), "impl/b");
}

#[test]
fn relative_root() {
    let (r, f) = (repo(), fake());
    let parent = r.work.parent().unwrap();
    // The launcher still names the prompt by its absolute path.
    launched_in(parent, Path::new("work"), &f, "b", &[]);
    assert!(local_tip(&r, "impl/b").is_some());
}

/// A local `impl/b` with a commit neither `origin/main` nor `origin/impl/b` has.
fn unpushed_branch(r: &Repo) -> String {
    git(&r.work, &["branch", "impl/b", "origin/main"]);
    git(&r.work, &["checkout", "-q", "impl/b"]);
    write(&r.work, "src/unpushed.rs", "u\n");
    git(&r.work, &["add", "-A"]);
    git(&r.work, &["commit", "-qm", "unpushed"]);
    git(&r.work, &["checkout", "-q", "main"]);
    local_tip(r, "impl/b").unwrap()
}

#[test]
fn scenario_unpushed_leftover_branch() {
    // req:orchestrator.run/run@0691bb50
    for lapsed_claim in [true, false] {
        let (r, f) = (repo(), fake());
        let claim = lapsed_claim.then(|| push_claim(&r.work, "b", 5));
        let before = unpushed_branch(&r);
        let stderr = refused(&r, &f, "b", &[], 1);
        assert!(stderr.contains("impl/b"), "{stderr}");
        assert_eq!(remote_tip(&r, "impl/b"), claim, "no claim pushed");
        assert_eq!(local_tip(&r, "impl/b"), Some(before), "the commit stays");
    }
}

#[test]
fn scenario_landed_leftover_branch() {
    // req:orchestrator.run/run@0691bb50
    let (r, f) = (repo(), fake());
    git(&r.work, &["branch", "impl/b", "origin/main"]);
    prompt(&r, &f, "b", &[]);
    let tip = remote_tip(&r, "impl/b").expect("claimed");
    assert_eq!(local_tip(&r, "impl/b"), Some(tip));
}

/// A local `impl/b`, checked out nowhere, at a commit whose parent is `origin/main` and whose
/// tree is `origin/main`'s: a claim this clone made and never pushed.
fn unpushed_claim(r: &Repo) -> String {
    let tree = git(&r.work, &["rev-parse", "origin/main^{tree}"]);
    let main = git(&r.work, &["rev-parse", "origin/main"]);
    let sha = git(
        &r.work,
        &[
            "commit-tree",
            tree.trim(),
            "-p",
            main.trim(),
            "-m",
            "unpushed claim",
        ],
    );
    git(&r.work, &["branch", "impl/b", sha.trim()]);
    sha.trim().to_string()
}

#[test]
fn scenario_interrupted_after_the_push() {
    // req:orchestrator.run/run@0691bb50
    let (r, f) = (repo(), fake());
    prompt(&r, &f, "b", &[]);
    let claim = remote_tip(&r, "impl/b").expect("claimed");
    fs::remove_file(r.work.join(".clm/prompts/b.md")).unwrap();
    let p = prompt(&r, &f, "b", &[]);
    assert!(p.contains("`impl/b` already exists"), "{p}");
    assert_eq!(
        remote_tip(&r, "impl/b"),
        Some(claim.clone()),
        "no new claim"
    );
    assert_eq!(local_tip(&r, "impl/b"), Some(claim));
    assert!(r.work.join(".clm/prompts/b.md").exists());
}

#[test]
fn scenario_interrupted_before_the_push() {
    // req:orchestrator.run/run@0691bb50
    let (r, f) = (repo(), fake());
    let leftover = unpushed_claim(&r);
    assert!(remote_tip(&r, "impl/b").is_none());
    prompt(&r, &f, "b", &[]);
    let claim = remote_tip(&r, "impl/b").expect("claimed");
    assert_ne!(claim, leftover, "a new claim commit");
    assert_eq!(
        git(&r.work, &["rev-parse", &format!("{claim}^")]).trim(),
        git(&r.work, &["rev-parse", "origin/main"]).trim()
    );
    assert_eq!(local_tip(&r, "impl/b"), Some(claim));
}

#[test]
fn scenario_another_clone_claimed_meanwhile() {
    // req:orchestrator.run/run@0691bb50
    let (r, f) = (repo(), fake());
    unpushed_claim(&r);
    let theirs = push_claim(&r.work, "b", 1);
    refused(&r, &f, "b", &[], 1);
    assert_eq!(remote_tip(&r, "impl/b"), Some(theirs));
}

/// The worktree a build of `b` would have had: `.claude/worktrees/agent-1` in the main
/// worktree, where Claude Code puts it, on the local `impl/b` an earlier `clm run b` left.
fn build_worktree(r: &Repo, f: &Fake) -> PathBuf {
    prompt(r, f, "b", &[]);
    let wt = r.work.join(".claude/worktrees/agent-1");
    git(
        &r.work,
        &["worktree", "add", "-q", wt.to_str().unwrap(), "impl/b"],
    );
    wt
}

/// A build worktree with a commit pushed over the claim.
fn resumable(r: &Repo, f: &Fake) -> PathBuf {
    let wt = build_worktree(r, f);
    write(&wt, "src/b.rs", "b\n");
    git(&wt, &["add", "-A"]);
    git(&wt, &["commit", "-qm", "work"]);
    git(&wt, &["push", "-q", "origin", "HEAD:impl/b"]);
    wt
}

#[test]
fn scenario_resume_with_unlanded_commits() {
    // req:orchestrator.run/run@0691bb50
    let (r, f) = (repo(), fake());
    let wt = resumable(&r, &f);
    let pushed = remote_tip(&r, "impl/b");
    let p = prompt(&r, &f, "b", &[]);
    assert_eq!(remote_tip(&r, "impl/b"), pushed, "no new claim commit");
    assert!(!wt.exists(), "the build's worktree is gone");
    assert!(!checked_out(&r, "impl/b"));
    assert_eq!(local_tip(&r, "impl/b"), pushed, "the commit stays");
    for s in [
        "`impl/b` already exists",
        "Start with `clm rebase`",
        "land the same branch",
    ] {
        assert!(p.contains(s), "{s}: {p}");
    }
}

/// An earlier build's branch `impl/b`, cut when `b/y` needed uncited `c/z`, pushed with a
/// commit; then a commit on `origin/main` citing `c/z`, and a checkout left at the older
/// commit. With `conflict`, the two commits change the same line of `shared.txt`.
fn resumed_behind_main(r: &Repo, f: &Fake, conflict: bool) {
    let w = &r.work;
    write(
        w,
        "specs/b/SPEC.md",
        &node_with(
            "",
            "",
            &[
                ("x", "It SHALL x."),
                ("y", "It SHALL y.\n\n#### Needs\n- c/z"),
            ],
        ),
    );
    write(w, "specs/c/SPEC.md", &node("", &["z"]));
    write(w, "shared.txt", "one\n");
    push_main(w, "b/y needs c/z");
    let wt = build_worktree(r, f);
    cite(&wt, "b.rs", &["b/x"]);
    if conflict {
        write(&wt, "shared.txt", "branch\n");
    }
    git(&wt, &["add", "-A"]);
    git(&wt, &["commit", "-qm", "work"]);
    git(&wt, &["push", "-q", "origin", "HEAD:impl/b"]);
    cite(w, "c.rs", &["c/z"]);
    if conflict {
        write(w, "shared.txt", "main\n");
    }
    push_main(w, "c/z built");
    git(w, &["reset", "-q", "--hard", "HEAD~1"]);
}

#[test]
fn scenario_resumed_branch_behind_main() {
    // req:orchestrator.run/pending-prompt@dea4856a
    let (r, f) = (repo(), fake());
    resumed_behind_main(&r, &f, false);
    let p = prompt(&r, &f, "b", &[]);
    assert!(p.contains("- `b/y` unbuilt"), "{p}");
    assert!(!p.contains("blocked by"), "{p}");
    assert!(!p.contains("conflicts with it"), "{p}");
    // The merge holds the branch's own commit, which built `b/x`.
    assert!(!p.contains("- `b/x` unbuilt"), "{p}");
}

#[test]
fn scenario_resumed_branch_that_conflicts_with_main() {
    // req:orchestrator.run/pending-prompt@dea4856a
    let (r, f) = (repo(), fake());
    resumed_behind_main(&r, &f, true);
    // The branch and `origin/main` change the same line, so their merge conflicts.
    assert_eq!(
        output(Command::new("git").current_dir(&r.work).args([
            "merge-tree",
            "--write-tree",
            "origin/main",
            "impl/b"
        ]))
        .status
        .code(),
        Some(1),
        "the setup must conflict"
    );
    let p = prompt(&r, &f, "b", &[]);
    assert!(p.contains("- `b/y` unbuilt"), "{p}");
    assert!(
        p.contains("read from `origin/main` because the branch conflicts with it"),
        "{p}"
    );
    assert!(
        p.contains("`clm pending b` after the first `clm rebase`"),
        "{p}"
    );
}

#[test]
fn resume_of_a_bare_claim() {
    // req:orchestrator.run/run@0691bb50
    let (r, f) = (repo(), fake());
    build_worktree(&r, &f);
    let claim = remote_tip(&r, "impl/b");
    let p = prompt(&r, &f, "b", &[]);
    assert!(p.contains("`impl/b` already exists"), "{p}");
    assert_eq!(remote_tip(&r, "impl/b"), claim, "resumed without claiming");
}

#[test]
fn resume_refuses_once_the_claim_is_lost() {
    // req:orchestrator.run/run@0691bb50
    let (r, f) = (repo(), fake());
    build_worktree(&r, &f);
    // Someone takes over the claim with a commit the local branch doesn't have.
    git(&r.origin, &["branch", "-f", "impl/b", "main"]);
    let theirs = push_claim(&r.work, "b", 0);
    let stderr = refused(&r, &f, "b", &[], 1);
    assert!(stderr.contains("impl/b"), "{stderr}");
    assert_eq!(remote_tip(&r, "impl/b"), Some(theirs));
}

#[test]
fn dirty_worktree_on_resume_is_refused() {
    // req:orchestrator.run/run@0691bb50
    let (r, f) = (repo(), fake());
    let wt = resumable(&r, &f);
    fs::write(wt.join("notes.txt"), "mine\n").unwrap();
    let pushed = remote_tip(&r, "impl/b");
    let stderr = refused(&r, &f, "b", &[], 1);
    assert!(stderr.contains("agent-1"), "{stderr}");
    assert!(wt.join("notes.txt").exists());
    assert_eq!(remote_tip(&r, "impl/b"), pushed);
}

/// A clean, locked build worktree on `impl/b` at its claim, whose claim then landed on
/// `origin/main` with a merge commit, `origin/impl/b` still at that tip.
fn landed_leftover(r: &Repo, f: &Fake) -> (PathBuf, String) {
    let wt = build_worktree(r, f);
    git(&r.work, &["worktree", "lock", wt.to_str().unwrap()]);
    merge_claim(&r.work, "b");
    (wt, remote_tip(r, "impl/b").unwrap())
}

#[test]
fn scenario_landed_leftover_worktree() {
    // req:orchestrator.run/run@0691bb50
    let (r, f) = (repo(), fake());
    let (wt, old) = landed_leftover(&r, &f);
    prompt(&r, &f, "b", &[]);
    assert!(!wt.exists(), "the landed worktree is gone");
    let tip = remote_tip(&r, "impl/b").unwrap();
    assert_ne!(tip, old, "a new claim commit");
    let main = git(&r.origin, &["rev-parse", "main"]);
    assert_eq!(git(&r.origin, &["rev-parse", &format!("{tip}^")]), main);
    assert_eq!(local_tip(&r, "impl/b"), Some(tip));
}

#[test]
fn scenario_dirty_leftover() {
    // req:orchestrator.run/run@0691bb50
    let (r, f) = (repo(), fake());
    let (wt, old) = landed_leftover(&r, &f);
    let notes = wt.join("notes.txt");
    fs::write(&notes, "mine\n").unwrap();
    let stderr = refused(&r, &f, "b", &[], 1);
    assert!(stderr.contains(".claude/worktrees/agent-1"), "{stderr}");
    assert_eq!(remote_tip(&r, "impl/b").unwrap(), old, "no claim pushed");
    assert!(notes.exists());
}

#[test]
fn scenario_node_that_writes_a_protected_path() {
    // req:orchestrator.run/run@0691bb50
    // req:orchestrator/next@1f34c883
    let (r, f) = (repo(), fake());
    write(
        &r.work,
        "specs/w/SPEC.md",
        &node_with(
            "",
            "paths: [\"web/**\"]\n",
            &[("x", "It SHALL write `web/.npmrc`.")],
        ),
    );
    push_main(&r.work, "w");
    assert_eq!(
        clm_orchestrator::next(&r.work, 4, 9).unwrap(),
        ["a", "b", "c", "w"]
    );
    let p = prompt(&r, &f, "w", &[]);
    assert!(p.starts_with("Run `git switch impl/w`"), "{p}");
    assert!(p.contains("`w/x` unbuilt"), "{p}");
}

/// The names of the clone's places.
fn place_names(r: &Repo) -> Vec<String> {
    clm_orchestrator::places(&r.work, 4)
        .unwrap()
        .into_iter()
        .map(|p| p.name)
        .collect()
}

/// Takes a build place `node` for `impl/<node>`, as `clm run` would.
fn take(r: &Repo, node: &str) {
    let placed = clm_orchestrator::take_place(
        &r.work,
        node,
        clm_orchestrator::PlaceKind::Build,
        &[format!("impl/{node}")],
        3,
        4,
    )
    .unwrap();
    assert_eq!(placed, clm_orchestrator::Placed::New);
}

#[test]
fn scenario_cap_reached() {
    // req:orchestrator.run/run-takes-place@2c1f76ba
    let (r, f) = (repo(), fake());
    push_claim(&r.work, "a", 0);
    git(&r.work, &["fetch", "-q", "origin"]);
    take(&r, "a");
    let stderr = refused(&r, &f, "b", &[("CLM_MAX_BUILDS", Path::new("1"))], 3);
    assert!(stderr.contains("1 of max_builds = 1 in use"), "{stderr}");
    assert!(remote_tip(&r, "impl/b").is_none(), "no claim");
    assert!(local_tip(&r, "impl/b").is_none(), "no branch");
    assert_eq!(place_names(&r), ["a"]);
    let exclude = fs::read_to_string(r.work.join(".git/info/exclude")).unwrap_or_default();
    assert!(!exclude.lines().any(|l| l == ".clm/"), "{exclude}");
    assert!(!r.work.join(".clm/prompts").exists());
}

#[test]
fn scenario_place_taken() {
    // req:orchestrator.run/run-takes-place@2c1f76ba
    let (r, f) = (repo(), fake());
    prompt(&r, &f, "b", &[]);
    let file = fs::read_to_string(r.work.join(".git/clm/builds/b")).unwrap();
    let lines: Vec<&str> = file.lines().collect();
    assert!(lines.contains(&"kind build"), "{file}");
    assert!(lines.contains(&"branch impl/b"), "{file}");
}

#[test]
fn scenario_resume_keeps_its_place() {
    // req:orchestrator.run/run-takes-place@2c1f76ba
    let (r, f) = (repo(), fake());
    let one = [("CLM_MAX_BUILDS", Path::new("1"))];
    resumable(&r, &f);
    assert_eq!(place_names(&r), ["b"]);
    let claim = remote_tip(&r, "impl/b");
    let p = prompt(&r, &f, "b", &one);
    assert!(p.contains("`impl/b` already exists"), "{p}");
    assert_eq!(remote_tip(&r, "impl/b"), claim, "resumed without claiming");
    assert_eq!(place_names(&r), ["b"]);
}

#[test]
fn scenario_refused_claim_frees_the_place() {
    // req:orchestrator.run/run-takes-place@2c1f76ba
    let (r, f) = (repo(), fake());
    push_claim(&r.work, "b", 1);
    refused(&r, &f, "b", &[], 1);
    assert!(place_names(&r).is_empty(), "{:?}", place_names(&r));
}

#[test]
fn a_run_that_fails_after_claiming_frees_its_new_place() {
    // req:orchestrator.run/run-takes-place@2c1f76ba
    let (r, f) = (repo(), fake());
    fs::create_dir_all(r.work.join(".clm/prompts/b.md")).unwrap();
    refused(&r, &f, "b", &[], 1);
    assert!(place_names(&r).is_empty(), "{:?}", place_names(&r));
}

#[test]
fn a_refused_resume_keeps_its_reused_place() {
    // req:orchestrator.run/run-takes-place@2c1f76ba
    let (r, f) = (repo(), fake());
    resumable(&r, &f);
    // Someone takes over the claim, so the resume is refused after the place is reused.
    git(&r.origin, &["branch", "-f", "impl/b", "main"]);
    push_claim(&r.work, "b", 0);
    refused(&r, &f, "b", &[], 1);
    assert_eq!(place_names(&r), ["b"]);
}

#[test]
fn scenario_landed_leftover_gone_before_the_place() {
    // req:orchestrator.run/run-takes-place@2c1f76ba
    let (r, f) = (repo(), fake());
    push_claim(&r.work, "a", 0);
    git(&r.work, &["fetch", "-q", "origin"]);
    take(&r, "a");
    git(&r.work, &["branch", "impl/b", "origin/main"]);
    assert!(remote_tip(&r, "impl/b").is_none());
    let stderr = refused(&r, &f, "b", &[("CLM_MAX_BUILDS", Path::new("1"))], 3);
    assert!(stderr.contains("max_builds"), "{stderr}");
    assert!(
        local_tip(&r, "impl/b").is_none(),
        "the landed branch is gone"
    );
    assert_eq!(place_names(&r), ["a"]);
}

#[test]
fn landed_leftover_worktree_gone_before_the_place() {
    // req:orchestrator.run/run-takes-place@2c1f76ba
    let (r, f) = (repo(), fake());
    let (wt, _) = landed_leftover(&r, &f);
    clm_orchestrator::free_place(&r.work, "b").unwrap();
    push_claim(&r.work, "a", 0);
    git(&r.work, &["fetch", "-q", "origin"]);
    take(&r, "a");
    refused(&r, &f, "b", &[("CLM_MAX_BUILDS", Path::new("1"))], 3);
    assert!(!wt.exists(), "the landed worktree is gone");
    assert_eq!(place_names(&r), ["a"]);
}

#[test]
fn dirty_worktree_takes_no_place() {
    // req:orchestrator.run/run-takes-place@2c1f76ba
    for landed in [true, false] {
        let (r, f) = (repo(), fake());
        let wt = if landed {
            landed_leftover(&r, &f).0
        } else {
            resumable(&r, &f)
        };
        clm_orchestrator::free_place(&r.work, "b").unwrap();
        // A full clone: a run that judged the place first would exit 3.
        push_claim(&r.work, "a", 0);
        git(&r.work, &["fetch", "-q", "origin"]);
        take(&r, "a");
        fs::write(wt.join("notes.txt"), "mine\n").unwrap();
        let stderr = refused(&r, &f, "b", &[("CLM_MAX_BUILDS", Path::new("1"))], 1);
        assert!(stderr.contains(".claude/worktrees/agent-1"), "{stderr}");
        assert_eq!(place_names(&r), ["a"]);
        assert!(wt.join("notes.txt").exists());
    }
}

/// What `clm run --finish <node>` printed.
fn clm_finish(r: &Repo, node: &str) -> Ran {
    clm_finish_in(r, &r.work, node)
}

/// `clm run --finish <node>` run in the worktree at `dir`.
fn clm_finish_in(r: &Repo, dir: &Path, node: &str) -> Ran {
    let out = output(
        Command::new(env!("CARGO_BIN_EXE_clm"))
            .args(["run", "--finish", node, "--root"])
            .arg(dir)
            .current_dir(dir)
            .env("XDG_CONFIG_HOME", r.work.join(".no-config")),
    );
    Ran {
        code: out.status.code(),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

/// A `clm run --finish` that must exit 0; its stdout.
fn finished(r: &Repo, node: &str) -> String {
    let ran = clm_finish(r, node);
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    ran.stdout
}

/// The landed build of "landed build tidied": a place `b`, a clean, locked worktree at
/// `.claude/worktrees/agent-1` on local `impl/b`, whose tip is the second parent of a merge
/// on `origin/main`, and no `origin/impl/b`. Returns the worktree's canonical path.
fn landed_build(r: &Repo, f: &Fake) -> PathBuf {
    let wt = build_worktree(r, f).canonicalize().unwrap();
    git(&r.work, &["worktree", "lock", wt.to_str().unwrap()]);
    merge_claim(&r.work, "b");
    git(&r.origin, &["branch", "-q", "-D", "impl/b"]);
    assert_eq!(place_names(r), ["b"]);
    wt
}

#[test]
fn scenario_landed_build_tidied() {
    // req:orchestrator.run/run-finish@62f7802d
    let (r, f) = (repo(), fake());
    let wt = landed_build(&r, &f);
    let out = finished(&r, "b");
    assert!(place_names(&r).is_empty(), "{:?}", place_names(&r));
    assert!(!wt.exists(), "the landed worktree is gone");
    assert!(out.contains("freed b"), "{out}");
    let removed = format!("removed {}", wt.display());
    assert!(out.contains(&removed), "{out}");
    let landed = out.find("landed impl/b").expect(&out);
    assert!(out.find("freed b").unwrap() < landed, "{out}");
    assert!(landed < out.find(&removed).unwrap(), "{out}");
}

#[test]
fn scenario_stopped_build_kept() {
    // req:orchestrator.run/run-finish@62f7802d
    let (r, f) = (repo(), fake());
    let wt = resumable(&r, &f);
    let pushed = remote_tip(&r, "impl/b");
    let out = finished(&r, "b");
    assert!(place_names(&r).is_empty(), "{:?}", place_names(&r));
    assert!(wt.exists(), "the worktree stays");
    assert!(!out.contains("removed"), "{out}");
    assert!(!out.lines().any(|l| l.starts_with("landed")), "{out}");
    assert_eq!(remote_tip(&r, "impl/b"), pushed, "origin/impl/b unchanged");
    assert_eq!(local_tip(&r, "impl/b"), pushed, "impl/b unchanged");
    let p = prompt(&r, &f, "b", &[]);
    assert!(p.contains("`impl/b` already exists"), "{p}");
    assert_eq!(remote_tip(&r, "impl/b"), pushed, "resumed without claiming");
}

#[test]
fn scenario_dirty_worktree_kept() {
    // req:orchestrator.run/run-finish@62f7802d
    let (r, f) = (repo(), fake());
    let wt = landed_build(&r, &f);
    fs::write(wt.join("notes.txt"), "mine\n").unwrap();
    let out = finished(&r, "b");
    assert!(wt.join("notes.txt").exists());
    assert!(out.contains("landed impl/b"), "{out}");
    assert!(out.find("landed impl/b") < out.find("kept"), "{out}");
    assert!(
        out.contains(&format!("kept {}: uncommitted changes", wt.display())),
        "{out}"
    );
}

#[test]
fn scenario_checkout_outside_the_worktrees_directory() {
    // req:orchestrator.run/run-finish@62f7802d
    let (r, f) = (repo(), fake());
    prompt(&r, &f, "b", &[]);
    let other = r.work.parent().unwrap().join("other");
    git(
        &r.work,
        &["worktree", "add", "-q", other.to_str().unwrap(), "impl/b"],
    );
    merge_claim(&r.work, "b");
    let out = finished(&r, "b");
    assert!(other.exists(), "a worktree elsewhere is never touched");
    assert!(!out.contains("removed"), "{out}");
}

#[test]
fn finish_prints_landed_with_no_worktree() {
    // req:orchestrator.run/run-finish@62f7802d
    let (r, f) = (repo(), fake());
    let wt = landed_build(&r, &f);
    git(&r.work, &["worktree", "unlock", wt.to_str().unwrap()]);
    git(&r.work, &["worktree", "remove", wt.to_str().unwrap()]);
    let out = finished(&r, "b");
    assert!(out.contains("landed impl/b"), "{out}");
}

#[test]
fn scenario_nothing_to_finish() {
    // req:orchestrator.run/run-finish@62f7802d
    let r = repo();
    let out = finished(&r, "b");
    assert!(!out.contains("freed"), "{out}");
    assert!(!out.lines().any(|l| l.starts_with("landed")), "{out}");
}

#[test]
fn finish_from_a_linked_worktree_looks_under_the_main_one() {
    // req:orchestrator.run/run-finish@62f7802d
    let (r, f) = (repo(), fake());
    let wt = landed_build(&r, &f);
    let session = r.work.parent().unwrap().join("session");
    let args = ["worktree", "add", "-q", "--detach"];
    git(
        &r.work,
        &[&args[..], &[session.to_str().unwrap(), "main"]].concat(),
    );
    let ran = clm_finish_in(&r, &session, "b");
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    assert!(!wt.exists(), "{}", ran.stdout);
}

#[test]
fn finish_never_removes_the_worktree_it_runs_in() {
    // req:orchestrator.run/run-finish@62f7802d
    let (r, f) = (repo(), fake());
    let wt = landed_build(&r, &f);
    let ran = clm_finish_in(&r, &wt, "b");
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    assert!(wt.exists(), "the session's own worktree stays");
    assert!(!ran.stdout.contains("removed"), "{}", ran.stdout);
}

#[test]
fn finish_keeps_a_branch_landed_only_locally() {
    // req:orchestrator.run/run-finish@62f7802d
    let (r, f) = (repo(), fake());
    let wt = landed_build(&r, &f);
    // `origin/impl/b` gets a commit `origin/main` lacks, so not every tip has landed.
    let main = git(&r.work, &["rev-parse", "origin/main"]);
    let tree = git(&r.work, &["rev-parse", "origin/main^{tree}"]);
    let sha = git(
        &r.work,
        &["commit-tree", tree.trim(), "-p", main.trim(), "-m", "more"],
    );
    git(
        &r.work,
        &[
            "push",
            "-q",
            "origin",
            &format!("{}:refs/heads/impl/b", sha.trim()),
        ],
    );
    let out = finished(&r, "b");
    assert!(wt.exists(), "{out}");
    assert!(!out.lines().any(|l| l.starts_with("landed")), "{out}");
}

#[test]
fn finish_fails_in_git_with_exit_2() {
    // req:orchestrator.run/run-finish@62f7802d
    let r = repo();
    git(&r.work, &["remote", "set-url", "origin", "/no/such/origin"]);
    assert_eq!(clm_finish(&r, "b").code, Some(2));
}

#[test]
fn scenario_a_build_prompt_says_component() {
    // req:orchestrator.run/says-component@49a0408e
    let (r, f) = (behind_and_obsolete(), fake());
    let p = prompt(&r, &f, "b", &[]);
    let lists = p.split("\nThe context bundle for").next().unwrap();
    assert!(
        lists.contains("Bring component `b`'s code up to its spec."),
        "{lists}"
    );
    assert!(lists.contains("which claims component `b`"), "{lists}");
    for old in ["node", "Node", "spine", "Spine"] {
        assert!(!lists.contains(old), "{old} in {lists}");
    }
}

#[test]
fn refusals_say_component() {
    // req:orchestrator.run/says-component@49a0408e
    let (r, f) = (repo(), fake());
    let err = refused(&r, &f, "a/b", &[], 1);
    assert!(err.contains("`a/b` isn't a component ID"), "{err}");
}
