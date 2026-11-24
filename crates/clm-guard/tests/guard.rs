//! The guard's rules: which commands it asks the human about or blocks, judged in process,
//! and `clm hook guard` called as `clm-guard`'s call: what it prints and exits with, and the
//! `gh` it asks through.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;
use std::sync::{PoisonError, RwLock};
use std::time::Duration;

use clm_guard::{Budgets, Decision};
use serde_json::Value;
use tempfile::TempDir;

/// A child forked while another test thread writes an executable inherits the write fd
/// until it execs, and exec'ing that file meanwhile fails with ETXTBSY. So writers of
/// executables hold this lock exclusively, and every spawn, the guard's own included, shares
/// it.
static FORKS: RwLock<()> = RwLock::new(());

fn executable(path: &Path, script: &str) {
    let _exclusive = FORKS.write().unwrap_or_else(PoisonError::into_inner);
    fs::write(path, script).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

/// The variables that keep the developer's own git config out of the guard's queries,
/// which inherit this process's environment: a global `push.default=matching` would block
/// every push the tests expect allowed.
const NO_USER_CONFIG: [(&str, &str); 2] = [
    ("GIT_CONFIG_GLOBAL", "/dev/null"),
    ("GIT_CONFIG_NOSYSTEM", "1"),
];

/// `clm_guard::guard_hook` with no user or system git config, holding the spawn lock
/// exclusively while it sets them, since the guard may ask git.
fn guard_hook(
    agent_harness: clm_agent_harness::AgentHarness,
    event: &str,
) -> clm_agent_harness::Rendered {
    let _exclusive = FORKS.write().unwrap_or_else(PoisonError::into_inner);
    let _set = ProcessEnv::set(NO_USER_CONFIG.iter());
    clm_guard::guard_hook(agent_harness, event)
}

/// `clm_guard::guard` with no user or system git config, holding the spawn lock
/// exclusively while it sets them, since the guard may ask git.
fn guard(event: &str) -> Decision {
    let _exclusive = FORKS.write().unwrap_or_else(PoisonError::into_inner);
    let _set = ProcessEnv::set(NO_USER_CONFIG.iter());
    clm_guard::guard(event)
}

/// A `Bash` event run from `cwd`.
fn bash_in(command: &str, cwd: &Path) -> String {
    serde_json::json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": { "command": command },
        "cwd": cwd,
    })
    .to_string()
}

fn bash(command: &str) -> String {
    bash_in(command, Path::new("/tmp"))
}

/// Runs `git` in `dir` with no user or system settings.
fn git(dir: &Path, args: &[&str]) {
    let _shared = FORKS.read().unwrap_or_else(PoisonError::into_inner);
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}: {out:?}");
}

/// A fresh scratch repository at `dir` on `branch`, holding no `specs/SPEC.md`.
fn scratch_on(dir: &Path, branch: &str) {
    fs::create_dir_all(dir).unwrap();
    git(dir, &["init", "-q", "-b", branch]);
}

/// A fresh repository under CLM at `dir` on `branch`: it holds `specs/SPEC.md`.
fn checkout_on(dir: &Path, branch: &str) {
    scratch_on(dir, branch);
    fs::create_dir_all(dir.join("specs")).unwrap();
    fs::write(dir.join("specs/SPEC.md"), "# Spec\n").unwrap();
}

/// A repository `work` under `dir` with a remote `origin` holding `main` and `impl/x/b`,
/// on `branch`, tracking `origin/<upstream>` or nothing.
fn tracking(dir: &Path, branch: &str, upstream: Option<&str>) -> std::path::PathBuf {
    let origin = dir.join("origin.git");
    fs::create_dir_all(&origin).unwrap();
    git(&origin, &["init", "-q", "--bare", "-b", "main"]);
    let work = dir.join("work");
    checkout_on(&work, "main");
    git(
        &work,
        &[
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "commit",
            "-q",
            "--allow-empty",
            "-m",
            "init",
        ],
    );
    git(
        &work,
        &["remote", "add", "origin", origin.to_str().unwrap()],
    );
    git(&work, &["push", "-q", "origin", "main", "main:impl/x/b"]);
    match upstream {
        Some(up) => git(
            &work,
            &[
                "checkout",
                "-q",
                "-b",
                branch,
                "--track",
                &format!("origin/{up}"),
            ],
        ),
        None => git(&work, &["checkout", "-q", "--no-track", "-b", branch]),
    }
    work
}

/// The reason `guard` blocks with, failing when it allows or asks instead.
fn blocks(event: &str) -> String {
    match guard(event) {
        Decision::Block(why) => why,
        other => panic!("{event}: expected block, got {other:?}"),
    }
}

fn blocks_push(event: &str) -> String {
    let why = blocks(event);
    assert!(why.contains("guard-blocks-push-to-main"), "{why}");
    why
}

fn allows(event: &str) {
    assert_eq!(guard(event), Decision::Allow, "{event}");
}

/// The reason `guard` asks the human with, failing when it allows or blocks instead.
fn asks_cmd(command: &str) -> String {
    match guard(&bash(command)) {
        Decision::Ask(why) => why,
        other => panic!("{command}: expected ask, got {other:?}"),
    }
}

/// A `Bash` event run from `cwd`, or with no `cwd` at all.
fn bash_in_opt(command: &str, cwd: Option<&Path>) -> String {
    let mut event = serde_json::json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": { "command": command },
    });
    if let Some(cwd) = cwd {
        event["cwd"] = serde_json::json!(cwd);
    }
    event.to_string()
}

#[test]
fn a_branch_that_pushes_to_main() {
    // req:plugin.hooks.guard/push-guard@d829891a
    let dir = TempDir::new().unwrap();
    let work = tracking(dir.path(), "impl/x/b", Some("main"));
    let why = blocks_push(&bash_in("git -c push.default=upstream push", &work));
    assert!(
        why.contains("pushes to `main`") && why.contains("HEAD:<branch>"),
        "{why}"
    );
    // After a `cd` the guard couldn't follow, the reason names it and the fallback.
    let why = blocks_push(&bash_in(
        "cd nowhere && git -c push.default=upstream push",
        &work,
    ));
    assert!(
        why.contains("pushes to `main`")
            && why.contains("`cd nowhere`")
            && why.contains("fell back to")
            && !why.contains("the checkout pushes from"),
        "{why}"
    );
    let why = blocks_push(&bash_in("git -c remote.origin.push=HEAD:main push", &work));
    assert!(why.contains("push refspec"), "{why}");
    git(&work, &["config", "push.default", "upstream"]);
    blocks_push(&bash_in("git push", &work));
    blocks_push(&bash_in("git push origin", &work));
    git(&work, &["config", "--unset", "push.default"]);
    allows(&bash_in("git push", &work));
    git(&work, &["config", "remote.origin.push", "HEAD:main"]);
    blocks_push(&bash_in("git push", &work));
    blocks_push(&bash_in("git push origin HEAD", &work));
    git(
        &work,
        &["config", "remote.origin.push", "refs/heads/*:refs/heads/*"],
    );
    blocks_push(&bash_in("git push", &work));
}

#[test]
fn only_push_and_remote_settings_reach_the_queries() {
    // req:plugin.hooks.guard/push-guard@d829891a
    let dir = TempDir::new().unwrap();
    let work = tracking(&dir.path().join("a"), "impl/x/b", Some("main"));
    // A setting outside `push.`, `remote.` and `branch.` isn't passed to git: one that
    // would make every query fail can't hide the push.
    blocks_push(&bash_in(
        "git -c push.default=upstream -c core.bare=maybe push",
        &work,
    ));
    // Nor is a `--config-env` option, whose variable git couldn't find either.
    blocks_push(&bash_in(
        "git -c push.default=upstream --config-env core.bare=UNSET_X push",
        &work,
    ));
    // The branch's own settings move `@{push}` too.
    let none = tracking(&dir.path().join("b"), "impl/x/c", None);
    blocks_push(&bash_in(
        "git -c push.default=upstream -c branch.impl/x/c.remote=origin \
         -c branch.impl/x/c.merge=refs/heads/main push",
        &none,
    ));
}

#[test]
fn another_checkouts_git_directory() {
    // req:plugin.hooks.guard/push-guard@d829891a
    let dir = TempDir::new().unwrap();
    checkout_on(dir.path(), "impl/x/b");
    checkout_on(&dir.path().join("m"), "main");
    for command in [
        "GIT_DIR=m/.git git push",
        "git --git-dir=m/.git push",
        "git --git-dir m/.git push",
        "GIT_WORK_TREE=m GIT_DIR=m/.git git push",
    ] {
        let why = blocks_push(&bash_in(command, dir.path()));
        assert!(why.contains("is on `main`"), "{command}: {why}");
    }
}

#[test]
fn pushing_branches_that_do_not_reach_main() {
    // req:plugin.hooks.guard/push-guard@d829891a
    // pushing other branches
    let dir = TempDir::new().unwrap();
    let own = tracking(&dir.path().join("own"), "impl/x/b", Some("impl/x/b"));
    allows(&bash_in("git push", &own));
    let none = tracking(&dir.path().join("none"), "impl/x/c", None);
    allows(&bash_in("git push", &none));
    let up = tracking(&dir.path().join("up"), "impl/x/b", Some("main"));
    allows(&bash_in(
        "git -c push.default=upstream push -u origin HEAD",
        &up,
    ));
    allows(&bash_in("git push", &up));
    allows(&bash_in(
        "git -c remote.origin.push=HEAD:impl/x/b push",
        &up,
    ));
}

#[test]
fn merging_into_main_through_the_host() {
    // req:plugin.hooks.guard/host-routes-guard@1e827c83
    // req:plugin.hooks.guard/push-guard@d829891a
    // merging into main through the host
    for command in [
        "git push origin 'refs/heads/*:heads/*'",
        "gh api -X POST repos/o/r/merges -f base=main -f head=impl/x/b",
        "gh api repos/o/r/merges --field=base=refs/heads/main -f head=x",
        "gh api repos/o/r/merges/ -F base=main",
        "gh api repos/o/r/merges --raw-field base=main",
        "gh api repos/o/r/merges --input body.json",
        "gh api -X PUT repos/o/r/pulls/3/merge",
        "gh api repos/o/r/pulls/3/merge -f merge_method=squash",
        "gh pr merge 3 --squash",
        "gh pr -R o/r merge 3",
        "gh api graphql -f query='mutation { mergePullRequest(input: {pullRequestId: \"P\"}) { clientMutationId } }'",
        "gh api graphql -f query='mutation { mergeBranch(input: {}) { clientMutationId } }'",
        "gh api graphql -f query='mutation { enablePullRequestAutoMerge(input: {}) { clientMutationId } }'",
        "gh api graphql -f query='mutation { updateRefs(input: {repositoryId: \"R\", refUpdates: []}) { clientMutationId } }'",
        "gh api graphql -f query='mutation { updateRef(input: {refId: \"R\", oid: \"x\"}) { clientMutationId } }'",
    ] {
        blocks_push(&bash(command));
    }
}

#[test]
fn reading_or_merging_elsewhere_through_the_host() {
    // req:plugin.hooks.guard/host-routes-guard@1e827c83
    // reading or merging elsewhere through the host
    for command in [
        "gh api -X POST repos/o/r/merges -f base=impl/x/b -f head=main",
        "gh api repos/o/r/merges -f base=mainline",
        "gh api repos/o/r/pulls/3/merge",
        "gh api repos/o/r/pulls/3/merges -X PUT",
        "gh pr view 3",
        "gh pr create --title t --body \"then gh pr merge it\"",
        "gh api graphql -f query='query { repository(owner: \"o\", name: \"r\") { id } }'",
    ] {
        allows(&bash(command));
    }
}

#[test]
fn guard_asks_to_approve() {
    // req:plugin.hooks.guard/approve-guard@5e09fed8
    // approving: the reason names the rule and the command
    let why = asks_cmd("clm approve spec/x");
    for needle in ["humans-review-intent", "`clm approve spec/x`"] {
        assert!(why.contains(needle), "{needle} missing from {why}");
    }
    // A `clm` called by path or after assignments, in any segment, and a body run by a shell.
    for command in [
        "clm approve",
        "/home/u/.claude/plugins/clm/bin/clm approve spec/x",
        "env X=1 clm approve 20261026-x",
        "clm approve --root /w spec/x",
        "CLM_WRITER=me clm approve spec/x --root .",
        "true && clm approve spec/x",
        "echo hi\nclm approve spec/x",
        "bash <<'EOF'\nclm approve spec/x\nEOF",
    ] {
        let why = asks_cmd(command);
        assert!(why.contains("humans-review-intent"), "{command}: {why}");
    }
}

#[test]
fn guard_ignores_review_word() {
    // req:plugin.hooks.guard/approve-guard@5e09fed8
    // review is no command
    for command in [
        "clm review x",
        "clm review",
        "env X=1 clm review 20261026-x",
        "clm review --root /w 20261026-x",
        "cargo run --bin=clm -- review x",
    ] {
        assert_eq!(guard(&bash(command)), Decision::Allow, "{command}");
    }
}

#[test]
fn guard_leaves_landing_and_labels_alone() {
    // req:plugin.hooks.guard/approve-guard@5e09fed8
    for command in [
        // landing
        "clm land spec/x",
        // a label is nobody's business
        "gh pr edit 30 --add-label spec-approved",
        "gh pr edit 30 --remove-label spec-review",
        "gh api repos/o/r/issues/30/labels -f 'labels[]=spec-approved'",
        "gh api graphql -f query='mutation { addLabelsToLabelable(input: {labelIds: [\"LA_9\"]}) { clientMutationId } }'",
        "gh api -X POST repos/o/r/issues/3/labels --input - <<'EOF'\n{\"labels\":[\"spec-approved\"]}\nEOF",
        // other subcommands, and prose that names the command
        "clm approved spec/x",
        "clm reviews",
        "echo clm approve spec/x",
        "git commit -m \"clm approve spec/x\"",
        "gh pr comment 3 --body 'run clm approve spec/x'",
        // quoted in a body
        "cat > body.md <<'EOF'\nclm approve spec/x\nclm review x\nEOF",
        "gh pr comment 3 --body-file - <<'EOF'\nclm approve spec/x\nEOF",
        "ls -l",
    ] {
        assert_eq!(guard(&bash(command)), Decision::Allow, "{command}");
    }
}

#[test]
fn guard_asks_through_cargo_run() {
    // req:plugin.hooks.guard/approve-guard@5e09fed8
    // approving through cargo run: the guard doesn't work out which package or binary runs,
    // or which tokens reach it (`guard-asks-any-cargo-run`).
    for command in [
        "cargo run -q -p clm -- approve spec/x",
        "cargo +stable run --package=clm -- approve spec/x",
        "~/.cargo/bin/cargo r --bin clm -- approve x",
        "CARGO_TERM_COLOR=never cargo run -pclm --release -- approve spec/x",
        "cargo --color never run -p clm -- approve spec/x",
        "cargo run -p clm --features f approve spec/x",
        "cargo run -p=clm -- approve spec/x",
        "cargo run -p clm@0.7.0 -- approve spec/x",
        "cargo run --manifest-path crates/clm/Cargo.toml -- approve spec/x",
        "cargo run -- approve spec/x",
        "cargo run -p clm --artifact-dir d approve spec/x",
        "cargo run -p clm-web -- approve spec/x",
        // a global option's value isn't the subcommand
        "cargo -C w run -p clm -- approve spec/x",
        "cargo run -p other -p clm -- approve spec/x",
        "cargo run --bin=clm -- approve x",
        "cargo run --package clm -- approve --root /w x",
        "true && cargo run -q -p clm -- approve spec/x",
        // Asking too often is the safe side: a value that is the word is asked about too.
        "cargo run -p clm -- page open x --title approve",
        "cargo run -p clm -- -- approve spec/x",
    ] {
        let why = asks_cmd(command);
        assert!(why.contains("humans-review-intent"), "{command}: {why}");
        // The reason names the segment that approves.
        let segment = command.rsplit("&& ").next().unwrap();
        assert!(why.contains(&format!("`{segment}`")), "{command}: {why}");
    }
    for global in ["--config v", "-Z v", "-C w", "--color never"] {
        let command = format!("cargo {global} run -p clm -- approve spec/x");
        let why = asks_cmd(&command);
        assert!(why.contains("humans-review-intent"), "{command}: {why}");
    }
}

#[test]
fn guard_leaves_cargo_running_something_else_alone() {
    // req:plugin.hooks.guard/approve-guard@5e09fed8
    // cargo running something else
    for command in [
        "cargo run -q -p clm -- land spec/x",
        "cargo run -q -p clm -- review x",
        "cargo run -q -p clm -- capabilities x",
        "clm architecture",
        "clm architecture x",
        "cargo test -p clm -- approve",
        "cargo build -p clm approve",
        "cargo run -p clm",
        // a word that only holds `approve`, `decide` before `capabilities`, a global option's
        // value, an alias that runs `run` under another name, and prose
        "cargo run -p clm -- approved",
        "cargo run -p clm -- decide capabilities",
        "cargo --config approve build",
        "cargo xrun -p clm -- approve spec/x",
        "cargo build -p clm -- approve spec/x",
        "echo cargo run -p clm -- approve spec/x",
        "cargo run -p clm -- review --json",
    ] {
        assert_eq!(guard(&bash(command)), Decision::Allow, "{command}");
    }
}

#[test]
fn guard_lets_a_block_win_over_ask() {
    // req:plugin.hooks.guard/approve-guard@5e09fed8
    // approval combined with a push to main
    let why = blocks(&bash("clm approve spec/x && git push origin main"));
    assert!(why.contains("guard-blocks-push-to-main"), "{why}");
    let why = blocks(&bash("clm approve spec/x; gh release create v1"));
    assert!(why.contains("humans-cut-releases"), "{why}");
}

#[test]
fn guard_allows_every_other_event() {
    // req:plugin.hooks.guard/approve-guard@5e09fed8
    let edit = serde_json::json!({
        "tool_name": "Edit",
        "tool_input": { "new_string": "clm approve spec/x" },
    });
    let other_tool = serde_json::json!({
        "tool_name": "Monitor",
        "tool_input": { "command": "clm approve spec/x" },
    });
    for input in [
        edit.to_string(),
        other_tool.to_string(),
        "not json".into(),
        "{}".into(),
        String::new(),
    ] {
        assert_eq!(guard(&input), Decision::Allow, "{input}");
    }
}

#[test]
fn guard_blocks_cutting_releases() {
    // req:plugin.hooks.guard/release-guard@1c7c5dad
    for command in [
        // dispatching a release
        "gh workflow run release.yml -f version=0.2.0",
        // creating a release directly
        "gh release create v0.2.0 --generate-notes",
        // dispatching through the API
        "gh api repos/o/r/actions/workflows/release.yml/dispatches -f ref=main -f inputs[version]=0.2.0",
        "env X=1 gh workflow run Release",
        "gh workflow run .github/workflows/release.yml",
        "bash -c \"gh release delete v0.1.0 -y\"",
        "gh release edit v0.1.0 --draft",
        "gh release upload v0.1.0 a.tar.gz",
        "gh release delete-asset v0.1.0 a.tar.gz",
        "gh api -X DELETE repos/o/r/releases/1",
        "gh api --method POST repos/o/r/releases",
        "true && gh release create v0.2.0",
        // flags before the subcommand, and gh's case-insensitive workflow names
        "gh release -R o/r create v0.2.0",
        "gh release --repo=o/r delete v0.1.0",
        "gh workflow --repo o/r run release.yml",
        "gh workflow run release",
        "gh workflow run RELEASE.YML",
        "gh workflow run Release.YAML",
        "gh workflow run RELEASE",
        "gh workflow run x/release.yaml",
        "gh workflow run -R o/r release.yaml",
        // the workflow after a flag's value
        "gh workflow run --ref main release.yml",
        "gh workflow run --ref main --json release.yml",
        "gh workflow run -f version=1 Release",
        "gh workflow run -f -r release.yml",
        "gh workflow run --json release.yml",
        "gh workflow run -- site.yml",
        // dispatching the site
        "gh workflow run site.yml",
        "gh workflow run .github/workflows/Site.YAML",
        "gh workflow run SITE",
        "gh workflow run site",
        "gh workflow run -R o/r site.yaml",
        "gh workflow run --ref main site.yml",
    ] {
        let why = blocks(&bash(command));
        assert!(why.contains("humans-cut-releases"), "{command}: {why}");
    }
    // A command that does both is blocked: the block wins over the ask.
    let why = blocks(&bash(
        "gh pr edit 1 --add-label spec-approved; gh release create v1",
    ));
    assert!(why.contains("humans-cut-releases"), "{why}");
    let why = blocks(&bash("gh release create v1; git push origin HEAD:main"));
    assert!(
        why.contains("humans-cut-releases") && why.contains("guard-blocks-push-to-main"),
        "{why}"
    );
}

#[test]
fn guard_allows_reading_releases_and_runs() {
    // req:plugin.hooks.guard/release-guard@1c7c5dad
    for command in [
        // reading releases
        "gh release list",
        "gh release view v0.1.0",
        "gh release download v0.1.0 -p 'clm-*'",
        "gh run list --workflow release.yml",
        "gh workflow run ci.yml",
        // dispatching another workflow
        "gh workflow run ci.yml --ref release",
        "gh workflow run ci.yml -f target=release.yml",
        "gh workflow run prerelease.yml",
        "gh workflow run release.yml.bak",
        "gh workflow run site.yml.bak",
        "gh workflow run website.yml",
        "gh workflow run ci.yml -f target=site.yml",
        "gh workflow view release.yml",
        "gh api repos/o/r/releases",
        "gh api repos/o/r/releases/latest --jq .tag_name",
        "gh api -X GET repos/o/r/releases",
        "gh release -R o/r view v0.1.0",
        "gh release --repo o/r list",
        "gh workflow --repo o/r view release.yml",
        // a value of `--ref` or `-r` that names a release workflow is no workflow argument
        "gh workflow run --ref release ci.yml",
        "gh workflow run -r site ci.yml",
        "gh workflow run --ref=release ci.yml",
    ] {
        assert_eq!(guard(&bash(command)), Decision::Allow, "{command}");
    }
}

#[test]
fn methods_as_gh_reads_them() {
    // req:plugin.hooks.guard/release-guard@1c7c5dad
    for command in [
        "gh api -XPOST repos/o/r/releases",
        "gh api --method=post repos/o/r/releases",
        "gh api -X GET -X POST repos/o/r/releases",
    ] {
        let why = blocks(&bash(command));
        assert!(why.contains("humans-cut-releases"), "{command}: {why}");
    }
    allows(&bash("gh api -X get repos/o/r/releases"));
}

#[test]
fn a_gh_call_behind_a_prefix() {
    // req:plugin.hooks.guard/release-guard@1c7c5dad
    for command in [
        "sudo gh release create v1",
        "/usr/local/bin/gh release create v1",
        "GH_TOKEN=x nohup gh release create v1",
        "sudo -u gh gh release create v1",
        "env -C /src/gh gh release create v1",
    ] {
        let why = blocks(&bash(command));
        assert!(why.contains("humans-cut-releases"), "{command}: {why}");
    }
}

#[test]
fn a_gh_merge_behind_a_prefix() {
    // req:plugin.hooks.guard/host-routes-guard@1e827c83
    blocks_push(&bash("sudo gh pr merge 3 --squash"));
}

#[test]
fn options_before_the_subcommand() {
    // req:plugin.hooks.guard/release-guard@1c7c5dad
    for command in [
        "gh workflow --ref x run release.yml",
        "gh workflow -f a=b run release.yml",
        "gh release --notes x create v1",
    ] {
        let why = blocks(&bash(command));
        assert!(why.contains("humans-cut-releases"), "{command}: {why}");
    }
    let why = blocks_push(&bash("gh pr --body x merge 3"));
    assert!(!why.contains("humans-cut-releases"), "{why}");
}

#[test]
fn guard_blocks_pushing_to_main() {
    // req:plugin.hooks.guard/push-guard@d829891a
    blocks_push(&bash("git push origin HEAD:main"));
}

#[test]
fn guard_blocks_other_ways_to_reach_main() {
    // req:plugin.hooks.guard/push-guard@d829891a
    for command in [
        "git push --force origin main",
        "git push origin +main",
        "git push origin impl/x/b:refs/heads/main",
        "git -C ../w push origin --delete main",
        // `--config-env`'s value is no subcommand.
        "git --config-env a=B push origin main",
        "git push --mirror origin",
        "git push --all origin",
        "FOO=1 env git push origin HEAD:main",
        "/usr/bin/git push origin main",
        "git -c push.default=current push -o ci.skip origin main",
        "cargo fmt && git push origin HEAD:main",
    ] {
        blocks_push(&bash(command));
    }
}

#[test]
fn guard_blocks_an_implicit_push_from_main() {
    // req:plugin.hooks.guard/push-guard@d829891a
    let dir = TempDir::new().unwrap();
    checkout_on(dir.path(), "main");
    for command in ["git push", "git push origin HEAD", "git push -u origin"] {
        blocks_push(&bash_in_opt(command, Some(dir.path())));
    }
    // or from a directory `sub` of one
    fs::create_dir(dir.path().join("sub")).unwrap();
    blocks_push(&bash_in_opt("git push", Some(&dir.path().join("sub"))));
}

#[test]
fn guard_allows_pushing_other_branches() {
    // req:plugin.hooks.guard/push-guard@d829891a
    for command in [
        "git push -u origin impl/x/b",
        "git push --force-with-lease origin impl/x/b",
        "git push origin --delete impl/x/b",
        "git push origin main:impl/x/b",
        "git push origin HEAD:refs/heads/impl/x/b",
        "git push origin mainline",
        "git push --repo main origin impl/x/b",
    ] {
        assert_eq!(guard(&bash(command)), Decision::Allow, "{command}");
    }
    let dir = TempDir::new().unwrap();
    let (main, w) = (dir.path().join("main"), dir.path().join("w"));
    checkout_on(&main, "main");
    checkout_on(&w, "impl/x/b");
    assert_eq!(guard(&bash_in_opt("git push", Some(&w))), Decision::Allow);
    assert_eq!(
        guard(&bash_in_opt("git -C ../w push", Some(&main))),
        Decision::Allow
    );
    assert_eq!(
        guard(&bash_in_opt("git -C../w push", Some(&main))),
        Decision::Allow
    );
    blocks_push(&bash_in_opt("git -C ../main push", Some(&w)));
    // A `cd` in an earlier segment moves the checkout, and `-C` joins onto it.
    blocks_push(&bash_in_opt("cd ../main && git push", Some(&w)));
    assert_eq!(
        guard(&bash_in_opt("cd ../w && git push", Some(&main))),
        Decision::Allow
    );
    assert_eq!(
        guard(&bash_in_opt("cd .. && git -C w push", Some(&main))),
        Decision::Allow
    );
}

#[test]
fn guard_names_a_cd_it_cannot_follow() {
    // req:plugin.hooks.guard/push-guard@d829891a
    // cd into a directory the command creates
    let dir = TempDir::new().unwrap();
    checkout_on(dir.path(), "main");
    let add = "git worktree add -q w -b spec/x && cd w";
    let why = blocks(&bash_in_opt(
        &format!("{add} && git commit -m m && git push -u origin HEAD"),
        Some(dir.path()),
    ));
    for needle in [
        "guard-blocks-push-to-main",
        "`cd w`",
        "couldn't follow",
        "fell back to",
        "HEAD:<branch>",
    ] {
        assert!(why.contains(needle), "{needle} missing from {why}");
    }
    assert!(!why.contains("pushes from is on"), "{why}");
    assert_eq!(
        guard(&bash_in_opt(
            &format!("{add} && git push -u origin spec/x"),
            Some(dir.path())
        )),
        Decision::Allow
    );
    // The `cd` is quoted as written, whichever kind the guard couldn't follow, and a body
    // a shell runs from there is judged from the same fallback.
    for (command, cd) in [
        ("cd $W && git push", "`cd $W`"),
        ("cd && git push", "`cd`"),
        ("cd - && git push", "`cd -`"),
        ("cd ~/w && git push", "`cd ~/w`"),
        ("cd `pwd` && git push", "`cd `...``"),
        ("cd $(pwd) && git push", "`cd $(...)`"),
        ("cd nowhere 2>/dev/null && git push", "`cd nowhere`"),
        ("cd . && cd nowhere; git push", "`cd nowhere`"),
        ("cd nowhere && bash -c 'git push'", "`cd nowhere`"),
        // A relative `cd` from the fallback is no surer than it.
        ("cd $W && cd . && git push", "`cd $W`"),
    ] {
        let why = blocks(&bash_in_opt(command, Some(dir.path())));
        assert!(
            why.contains(cd) && why.contains("couldn't follow"),
            "{command}: {why}"
        );
    }
    // An absolute `cd` it followed after it, an absolute `-C`, or one in a subshell that
    // ended, sets the checkout.
    let abs = dir.path().display();
    for command in [
        format!("cd nowhere && cd {abs} && git push"),
        format!("cd nowhere && git -C {abs} push"),
        format!("cd nowhere && env -C {abs} git push"),
        "(cd nowhere); git push".to_string(),
        "git push".to_string(),
    ] {
        let why = blocks(&bash_in_opt(&command, Some(dir.path())));
        assert!(
            !why.contains("couldn't follow") && why.contains("pushes from is on `main`"),
            "{command}: {why}"
        );
    }
    // A configured push refspec, read in the checkout it fell back to.
    let other = TempDir::new().unwrap();
    checkout_on(other.path(), "impl/x/b");
    let why = blocks(&bash_in_opt(
        "cd nowhere && git -c remote.origin.push=HEAD:main push",
        Some(other.path()),
    ));
    assert!(
        why.contains("`cd nowhere`") && why.contains("configured push refspec"),
        "{why}"
    );
    // A setting from the environment, judged from the checkout it fell back to.
    let why = blocks(&bash_in_opt(
        "cd nowhere && git --config-env push.default=U push",
        Some(other.path()),
    ));
    assert!(
        why.contains("`cd nowhere`")
            && why.contains("checkout it fell back to")
            && why.contains("`--config-env push.default=U`"),
        "{why}"
    );
}

#[test]
fn guard_suggests_an_explicit_branch_for_an_implicit_push() {
    // req:plugin.hooks.guard/push-guard@d829891a
    // implicit push from main
    let dir = TempDir::new().unwrap();
    checkout_on(dir.path(), "main");
    for command in ["git push", "git push origin HEAD"] {
        let why = blocks(&bash_in_opt(command, Some(dir.path())));
        assert!(
            why.contains("guard-blocks-push-to-main") && why.contains("HEAD:<branch>"),
            "{command}: {why}"
        );
    }
    let why = blocks(&bash("git push origin HEAD:main"));
    assert!(!why.contains("HEAD:<branch>"), "{why}");
    // The reason is about the checkout pushed from, which may not be the session's.
    let inner = dir.path().join("w");
    checkout_on(&inner, "impl/x/b");
    let why = blocks(&bash_in_opt("cd .. && git push", Some(&inner)));
    assert!(why.contains("pushes from is on `main`"), "{why}");
}

#[test]
fn guard_reads_push_options_and_redirections_as_what_they_are() {
    // req:plugin.hooks.guard/push-guard@d829891a
    // An option's value isn't the remote or a refspec, wherever the option sits.
    assert_eq!(
        guard(&bash("git push origin --repo main impl/x/b")),
        Decision::Allow
    );
    assert_eq!(
        guard(&bash("git push origin --repo=x impl/x/b")),
        Decision::Allow
    );
    blocks_push(&bash("git --git-dir ../main/.git push origin main"));
    let dir = TempDir::new().unwrap();
    checkout_on(dir.path(), "main");
    for command in [
        "git push -o ci.skip origin",
        "git push origin --push-option x",
        "git push -u origin 2>&1 | tail -3",
        "git push origin >/dev/null",
        "git push origin @",
    ] {
        blocks_push(&bash_in_opt(command, Some(dir.path())));
    }
    // a push of @: git's name for `HEAD`, from a checkout on `main`
    let why = blocks_push(&bash_in("git push origin @", dir.path()));
    assert!(why.contains("HEAD:<branch>"), "{why}");
}

#[test]
fn settings_in_any_case() {
    // req:plugin.hooks.guard/push-guard@d829891a
    // A setting's name is read without regard to case, as git reads it.
    let dir = TempDir::new().unwrap();
    let work = tracking(dir.path(), "impl/x/b", Some("main"));
    for command in [
        "git -c PUSH.DEFAULT=upstream push",
        "git -c REMOTE.origin.PUSH=HEAD:main push",
    ] {
        blocks_push(&bash_in(command, &work));
    }
}

#[test]
fn settings_from_the_environment() {
    // req:plugin.hooks.guard/push-guard@d829891a
    // A `--config-env` setting is read from a variable the guard can't see, so one that
    // could move where a push of the current branch or of a named branch goes blocks it.
    let dir = TempDir::new().unwrap();
    let work = tracking(dir.path(), "impl/x/b", Some("impl/x/b"));
    for (command, option) in [
        (
            "B=HEAD:main git --config-env remote.origin.push=B push",
            "`--config-env remote.origin.push=B`",
        ),
        (
            "U=upstream git --config-env=PUSH.DEFAULT=U push",
            "`--config-env=PUSH.DEFAULT=U`",
        ),
        (
            "M=refs/heads/main git --config-env branch.impl/x/b.merge=M push origin HEAD",
            "`--config-env branch.impl/x/b.merge=M`",
        ),
        (
            "git --config-env remote.origin.push=B push origin impl/x/b",
            "`--config-env remote.origin.push=B`",
        ),
    ] {
        let why = blocks_push(&bash_in(command, &work));
        assert!(
            why.contains(option)
                && why.contains("the guard can't read")
                && why.contains("HEAD:<branch>"),
            "{command}: {why}"
        );
    }
    allows(&bash_in("W=. git --config-env core.worktree=W push", &work));
    // An explicit destination is never mapped.
    allows(&bash_in(
        "git --config-env remote.origin.push=B push origin HEAD:impl/x/b",
        &work,
    ));
    // Its value is no subcommand.
    blocks_push(&bash_in("git --config-env a=B push origin main", &work));
}

#[test]
fn configuration_from_variables() {
    // req:plugin.hooks.guard/push-guard@d829891a
    // `GIT_CONFIG_COUNT`, `GIT_CONFIG_PARAMETERS`, `GIT_CONFIG_GLOBAL` and
    // `GIT_CONFIG_SYSTEM` set configuration the guard doesn't read, so a push of the current
    // branch with one of them assigned blocks, naming the first in that order.
    let dir = TempDir::new().unwrap();
    let work = tracking(dir.path(), "impl/x/b", Some("impl/x/b"));
    for (command, variable) in [
        (
            "GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=remote.origin.push \
             GIT_CONFIG_VALUE_0=HEAD:main git push",
            "GIT_CONFIG_COUNT",
        ),
        (
            "GIT_CONFIG_PARAMETERS=\"'push.default'='upstream'\" git push origin HEAD",
            "GIT_CONFIG_PARAMETERS",
        ),
        ("env GIT_CONFIG_GLOBAL=g git push", "GIT_CONFIG_GLOBAL"),
        ("GIT_CONFIG_SYSTEM=s git push origin @", "GIT_CONFIG_SYSTEM"),
        (
            "GIT_CONFIG_SYSTEM=s GIT_CONFIG_GLOBAL=g git push",
            "GIT_CONFIG_GLOBAL",
        ),
    ] {
        let why = blocks_push(&bash_in(command, &work));
        assert!(
            why.contains(&format!("`{variable}`")) && why.contains("HEAD:<branch>"),
            "{command}: {why}"
        );
        let others = [
            "GIT_CONFIG_COUNT",
            "GIT_CONFIG_PARAMETERS",
            "GIT_CONFIG_GLOBAL",
            "GIT_CONFIG_SYSTEM",
        ];
        for other in others.iter().filter(|o| **o != variable) {
            assert!(!why.contains(&format!("`{other}`")), "{command}: {why}");
        }
    }
    // After a `cd` the guard couldn't follow, the reason quotes it.
    let why = blocks_push(&bash_in("cd $W && GIT_CONFIG_COUNT=1 git push", &work));
    assert!(
        why.contains("`GIT_CONFIG_COUNT`")
            && why.contains("`cd $W`")
            && why.contains("fell back to"),
        "{why}"
    );
    allows(&bash_in(
        "GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=remote.origin.push \
         GIT_CONFIG_VALUE_0=HEAD:main git push origin HEAD:impl/x/b",
        &work,
    ));
    // A named branch may be mapped by the configuration it sets.
    let why = blocks_push(&bash_in(
        "GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=remote.origin.push \
         GIT_CONFIG_VALUE_0=HEAD:main git push origin impl/x/b",
        &work,
    ));
    assert!(why.contains("`GIT_CONFIG_COUNT`"), "{why}");
    // Decided with no checkout, as git is asked nothing.
    let (code, _, err) = guard_with_no_cwd(None, "GIT_CONFIG_GLOBAL=g git push", "/nonexistent");
    assert_eq!(code, 2, "{err}");
    assert!(err.contains("`GIT_CONFIG_GLOBAL`"), "{err}");
}

#[test]
fn a_branch_named_without_its_destination() {
    // req:plugin.hooks.guard/push-guard@d829891a
    // git maps a named branch through `remote.<name>.push`, or under
    // `push.default=upstream` to the branch's upstream.
    let dir = TempDir::new().unwrap();
    let work = tracking(dir.path(), "impl/x/b", Some("impl/x/b"));
    git(
        &work,
        &["branch", "-q", "--track", "impl/x/c", "origin/main"],
    );
    let configured = "git -c remote.origin.push=refs/heads/impl/x/b:refs/heads/main \
                      push origin impl/x/b";
    let why = blocks_push(&bash_in(configured, &work));
    assert!(
        why.contains("configured push refspec") && why.contains("HEAD:<branch>"),
        "{why}"
    );
    for command in [
        "git -c push.default=upstream push origin impl/x/c",
        "git -c push.default=tracking push origin impl/x/c",
        "git -c push.default=upstream push origin +impl/x/c",
        "git -c push.default=upstream push origin refs/heads/impl/x/c",
        "git -c push.default=upstream push origin impl/x/b impl/x/c",
    ] {
        let why = blocks_push(&bash_in(command, &work));
        assert!(
            why.contains("`impl/x/c`")
                && why.contains("push.default=upstream")
                && why.contains("HEAD:<branch>"),
            "{command}: {why}"
        );
    }
    for command in [
        "git -c push.default=upstream push origin impl/x/b",
        "git -c push.default=upstream push origin impl/x/c:impl/x/c",
        "git -c push.default=Upstream push origin impl/x/c",
        "git push origin impl/x/c",
    ] {
        allows(&bash_in(command, &work));
    }
    // The repository's own settings count the same.
    git(
        &work,
        &[
            "config",
            "remote.origin.push",
            "refs/heads/impl/x/b:refs/heads/main",
        ],
    );
    let why = blocks_push(&bash_in("git push origin impl/x/b", &work));
    assert!(
        why.contains("configured push refspec") && why.contains("HEAD:<branch>"),
        "{why}"
    );
    allows(&bash_in("git push origin HEAD:impl/x/b", &work));
    // Every value of a key counts, not only the last, which git uses.
    git(
        &work,
        &[
            "config",
            "--add",
            "remote.origin.push",
            "refs/heads/impl/x/b:refs/heads/impl/x/b",
        ],
    );
    blocks_push(&bash_in("git push origin impl/x/b", &work));
    git(&work, &["config", "--unset-all", "remote.origin.push"]);
    // A branch named as `heads/<b>`, and an upstream written `main` or `heads/main`.
    for command in [
        "git -c push.default=upstream -c branch.impl/x/b.merge=main push origin impl/x/b",
        "git -c push.default=upstream -c branch.impl/x/b.merge=heads/main \
         push origin heads/impl/x/b",
    ] {
        let why = blocks_push(&bash_in(command, &work));
        assert!(why.contains("`impl/x/b`"), "{command}: {why}");
    }
    git(&work, &["config", "push.default", "upstream"]);
    blocks_push(&bash_in("git push origin impl/x/c", &work));
}

#[test]
fn matching_and_mirroring() {
    // req:plugin.hooks.guard/push-guard@d829891a
    // With no refspec, `matching` or a mirroring remote pushes `main` with every branch.
    let dir = TempDir::new().unwrap();
    let work = tracking(dir.path(), "impl/x/b", Some("impl/x/b"));
    for command in [
        "git -c push.default=matching push",
        "git -c remote.origin.mirror=true push",
        // A setting with no value is true to git.
        "git -c remote.origin.mirror push",
        "git -c remote.origin.mirror=1 push origin",
    ] {
        let why = blocks_push(&bash_in(command, &work));
        assert!(
            why.contains("every branch") && why.contains("HEAD:<branch>"),
            "{command}: {why}"
        );
    }
    for command in [
        "git -c push.default=matching push origin HEAD",
        "git -c remote.origin.mirror=false push",
        "git -c remote.origin.mirror=OFF push",
        "git -c remote.origin.mirror=no push",
        "git -c remote.origin.mirror=0 push",
        "git -c push.default=Matching push",
    ] {
        allows(&bash_in(command, &work));
    }
    git(&work, &["config", "push.default", "matching"]);
    let why = blocks_push(&bash_in("git push", &work));
    assert!(why.contains("HEAD:<branch>"), "{why}");
}

#[test]
fn settings_from_an_included_file() {
    // req:plugin.hooks.guard/push-guard@d829891a
    // An included file can set any setting, and the guard doesn't read it.
    let dir = TempDir::new().unwrap();
    let work = tracking(dir.path(), "impl/x/b", Some("impl/x/b"));
    let p = dir.path().join("p");
    fs::write(&p, "[remote \"origin\"]\n\tpush = HEAD:main\n").unwrap();
    let p = p.display();
    for (command, option) in [
        (
            format!("git -c include.path={p} push"),
            format!("`-c include.path={p}`"),
        ),
        (
            format!("P={p} git --config-env include.path=P push"),
            "`--config-env include.path=P`".to_string(),
        ),
        (
            format!("git -c includeIf.onbranch:impl/**.path={p} push origin impl/x/b"),
            format!("`-c includeIf.onbranch:impl/**.path={p}`"),
        ),
        (
            format!("git -c INCLUDE.path={p} push origin HEAD"),
            format!("`-c INCLUDE.path={p}`"),
        ),
    ] {
        let why = blocks_push(&bash_in(&command, &work));
        assert!(
            why.contains(&option)
                && why.contains("the guard can't read")
                && why.contains("HEAD:<branch>"),
            "{command}: {why}"
        );
    }
    allows(&bash_in(
        &format!("git -c include.path={p} push origin HEAD:impl/x/b"),
        &work,
    ));
    // The first such option is named, whichever form it takes.
    let why = blocks_push(&bash_in(
        &format!("git --config-env push.default=U -c include.path={p} push"),
        &work,
    ));
    assert!(
        why.contains("`--config-env push.default=U`") && !why.contains("include.path"),
        "{why}"
    );
}

#[test]
fn a_refspec_in_a_variable() {
    // req:plugin.hooks.guard/push-guard@d829891a
    // A refspec is judged as written: only a destination written as `main` reaches it.
    let dir = TempDir::new().unwrap();
    checkout_on(dir.path(), "main");
    for command in [
        "git push origin $B",
        "git push origin HEAD:$B",
        "for b in x; do git push origin $b; done",
    ] {
        allows(&bash_in(command, dir.path()));
    }
    blocks_push(&bash_in("git push origin $B:main", dir.path()));
}

#[test]
fn a_dash_c_glob() {
    // req:plugin.hooks.guard/push-guard@d829891a
    // The walk yields a `-C` glob expanded as bash expands it: one match is the directory,
    // several are tokens of their own, and a quoted or glued one stands as bash leaves it.
    let dir = TempDir::new().unwrap();
    checkout_on(dir.path(), "impl/x/b");
    checkout_on(&dir.path().join("m"), "main");
    for plain in ["x1", "x2"] {
        fs::create_dir(dir.path().join(plain)).unwrap();
    }
    blocks_push(&bash_in("git -C m* push", dir.path()));
    for command in ["git -C 'm*' push", "git -C x* push", "git -Cm* push"] {
        allows(&bash_in(command, dir.path()));
    }
}

#[test]
fn guard_blocks_matching_and_wildcard_refspecs_that_reach_main() {
    // req:plugin.hooks.guard/push-guard@d829891a
    for command in [
        "git push origin :",
        "git push --force origin refs/heads/*:refs/heads/*",
        "git push origin HEAD:heads/main",
    ] {
        blocks_push(&bash(command));
    }
    assert_eq!(
        guard(&bash("git push origin refs/heads/impl/*:refs/heads/impl/*")),
        Decision::Allow
    );
    assert_eq!(guard(&bash("git push origin :impl/x/b")), Decision::Allow);
}

#[test]
fn guard_allows_an_implicit_push_it_cannot_decide() {
    // req:plugin.hooks.guard/push-guard@d829891a
    let dir = TempDir::new().unwrap();
    assert_eq!(
        guard(&bash_in_opt("git push", Some(dir.path()))),
        Decision::Allow,
        "not a repository"
    );
}

/// A repository with a remote `origin`, on `main`, holding no `specs/SPEC.md`.
fn scratch_tracking(dir: &Path) -> std::path::PathBuf {
    let work = tracking(dir, "impl/x/b", Some("main"));
    git(&work, &["checkout", "-q", "main"]);
    fs::remove_dir_all(work.join("specs")).unwrap();
    work
}

#[test]
fn a_repository_not_under_clm() {
    // req:plugin.hooks.guard/push-guard-scoped-to-clm@e0d1066b
    let dir = TempDir::new().unwrap();
    let work = scratch_tracking(dir.path());
    for command in [
        "git push -u origin main",
        "git push",
        "git push origin HEAD:main",
        "git push --all origin",
        "git push --mirror origin",
    ] {
        allows(&bash_in(command, &work));
    }
    // Only a push a case would block is tested, so the same repository's other pushes are
    // allowed either way.
    allows(&bash_in("git push origin impl/x/b", &work));
    // A directory of it is the same repository.
    fs::create_dir_all(work.join("sub")).unwrap();
    allows(&bash_in("git push origin main", &work.join("sub")));
}

#[test]
fn a_scratch_repository_inside_a_checkout_under_clm() {
    // req:plugin.hooks.guard/push-guard-scoped-to-clm@e0d1066b
    let dir = TempDir::new().unwrap();
    let work = tracking(dir.path(), "impl/x/b", Some("impl/x/b"));
    scratch_on(&work.join("s"), "main");
    allows(&bash_in("cd s && git push -u origin main", &work));
    allows(&bash_in("git -C s push", &work));
    blocks_push(&bash_in("git push origin main", &work));
}

#[test]
fn a_clm_repository_in_a_subdirectory() {
    // req:plugin.hooks.guard/push-guard-scoped-to-clm@e0d1066b
    let dir = TempDir::new().unwrap();
    let work = scratch_tracking(dir.path());
    fs::create_dir_all(work.join("tool/specs")).unwrap();
    fs::write(work.join("tool/specs/SPEC.md"), "# Spec\n").unwrap();
    git(&work, &["add", "tool"]);
    git(
        &work,
        &[
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "commit",
            "-q",
            "-m",
            "tool",
        ],
    );
    blocks_push(&bash_in("git push origin main", &work));
    blocks_push(&bash_in("git push", &work));
    blocks_push(&bash_in("git push", &work.join("tool")));
    // From a sibling directory the pathspec is still read from the top.
    fs::create_dir_all(work.join("other")).unwrap();
    blocks_push(&bash_in("git push origin main", &work.join("other")));
}

#[test]
fn a_push_whose_repository_the_guard_cannot_tell() {
    // req:plugin.hooks.guard/push-guard-scoped-to-clm@e0d1066b
    let dir = TempDir::new().unwrap();
    let work = scratch_tracking(dir.path());
    for command in [
        "cd $W && git push origin main",
        "GIT_DIR=.git git push origin main",
        "GIT_COMMON_DIR=.git git push origin main",
        "GIT_WORK_TREE=. git push origin main",
        "git --work-tree=. push origin main",
        "git --git-dir .git push origin main",
        "git -c core.worktree=. push origin main",
        "git -c core.workTree=. push origin main",
        "git -c core.bare=false push origin main",
        // a work tree set in any case
        "git -c Core.Worktree=. push origin main",
        "git -c CORE.BARE=false push origin main",
        "git -C missing push origin main",
        "cd nowhere && git push origin main",
    ] {
        blocks_push(&bash_in(command, &work));
    }
    // No `cwd`, judged from the guard's own working directory, this repository's, and a
    // `cwd` in no git repository.
    blocks_push(&bash_in_opt("git push origin main", None));
    let outside = TempDir::new().unwrap();
    blocks_push(&bash_in("git push origin main", outside.path()));
    // A `-C` path that is a file.
    fs::write(work.join("file"), "x").unwrap();
    blocks_push(&bash_in("git -C file push origin main", &work));
    // The push's own absolute `-C`, or an absolute `env -C`, sets the checkout outright.
    let abs = work.to_str().unwrap();
    allows(&bash_in(
        &format!("cd $W && git -C {abs} push origin main"),
        &work,
    ));
    allows(&bash_in(
        &format!("cd $W && env -C {abs} git push origin main"),
        &work,
    ));
}

#[test]
fn a_work_tree_set_from_the_environment() {
    // req:plugin.hooks.guard/push-guard-scoped-to-clm@e0d1066b
    let dir = TempDir::new().unwrap();
    let work = scratch_tracking(dir.path());
    for command in [
        "W=. git --config-env core.worktree=W push origin main",
        "B=false git --config-env=Core.Bare=B push origin main",
    ] {
        blocks_push(&bash_in(command, &work));
    }
    allows(&bash_in(
        "U=upstream git --config-env push.default=U push",
        &work,
    ));
}

#[test]
fn a_work_tree_set_by_a_variable() {
    // req:plugin.hooks.guard/push-guard-scoped-to-clm@e0d1066b
    // Each of the four variables can set `core.worktree` or `core.bare`, so a push that
    // assigns one counts as under CLM.
    let dir = TempDir::new().unwrap();
    let work = scratch_tracking(dir.path());
    for command in [
        "GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=core.worktree GIT_CONFIG_VALUE_0=. \
         git push origin main",
        "GIT_CONFIG_PARAMETERS=\"'core.bare'='false'\" git push origin main",
        "GIT_CONFIG_GLOBAL=g git push origin main",
        "env GIT_CONFIG_SYSTEM=s git push origin main",
    ] {
        blocks_push(&bash_in(command, &work));
    }
    allows(&bash_in("git push origin main", &work));
}

#[test]
fn a_work_tree_set_by_an_included_file() {
    // req:plugin.hooks.guard/push-guard-scoped-to-clm@e0d1066b
    // An included file can set `core.worktree`, so a push with one counts as under CLM.
    let dir = TempDir::new().unwrap();
    let work = scratch_tracking(dir.path());
    let p = dir.path().join("p");
    fs::write(&p, "[core]\n\tworktree = .\n").unwrap();
    let p = p.display();
    for command in [
        format!("git -c include.path={p} push origin main"),
        format!("P={p} git --config-env includeIf.onbranch:main.path=P push origin main"),
        format!("git -c IncludeIf.gitdir:/.path={p} push origin main"),
    ] {
        blocks_push(&bash_in(&command, &work));
    }
    allows(&bash_in("git push origin main", &work));
}

#[test]
fn the_hosts_routes_outside_clm() {
    // req:plugin.hooks.guard/host-routes-guard@1e827c83
    let dir = TempDir::new().unwrap();
    let work = scratch_tracking(dir.path());
    blocks_push(&bash_in("gh pr merge 3 --squash", &work));
    blocks_push(&bash_in(
        "gh api -X PATCH repos/o/r/git/refs/heads/main -f sha=abc",
        &work,
    ));
}

#[test]
fn guard_blocks_moving_main_through_the_api() {
    // req:plugin.hooks.guard/host-routes-guard@1e827c83
    blocks_push(&bash(
        "gh api -X PATCH repos/o/r/git/refs/heads/main -f sha=abc",
    ));
    blocks_push(&bash(
        "gh api repos/o/r/git/refs/heads/main?force=1 --method PATCH -F force=true",
    ));
    blocks_push(&bash("gh api -X DELETE repos/o/r/git/refs/heads/main/"));
    for command in [
        "gh api repos/o/r/git/refs/heads/main",
        "gh api -X PATCH repos/o/r/git/refs/heads/mainline -f sha=abc",
        "gh api -X PATCH repos/o/r/git/refs/heads/impl/x/b -f sha=abc",
    ] {
        assert_eq!(guard(&bash(command)), Decision::Allow, "{command}");
    }
}

#[test]
fn guard_allows_prose_about_pushing() {
    // req:plugin.hooks.guard/push-guard@d829891a
    for command in [
        r#"gh pr create --title t --body "never run git push origin main""#,
        r#"git commit -m "explain git push origin main""#,
        "echo git push origin main",
    ] {
        assert_eq!(guard(&bash(command)), Decision::Allow, "{command}");
    }
}

#[test]
fn asks_for_capability_decisions() {
    // req:plugin.hooks.guard/approve-guard@5e09fed8
    // deciding: a review decision on a capability is the human's
    for command in [
        "clm capabilities decide x --value high --decision keep",
        "cargo run -q -p clm -- capabilities decide x --value low --decision cut",
        "cargo run -- capabilities --root decide",
        "cargo r --manifest-path crates/clm/Cargo.toml capabilities x decide",
        "clm capabilities --root /w decide x --value high --decision keep",
        "clm capabilities --root=/w --format json decide x --value high --decision keep",
        "clm architecture decide x --value high --decision keep",
        "cargo run -q -p clm -- architecture decide x --value low --decision cut",
        "cargo run -- architecture --root decide",
        "clm architecture --root /w decide x --value high --decision keep",
        "clm architecture --root=/w --format json decide x --value high --decision keep",
    ] {
        let why = asks_cmd(command);
        assert!(why.contains("humans-review-intent"), "{command}: {why}");
        assert!(why.contains(&format!("`{command}`")), "{command}: {why}");
    }
}

#[test]
fn capabilities_reading_not_asked() {
    // req:plugin.hooks.guard/approve-guard@5e09fed8
    for command in [
        "clm capabilities",
        "clm capabilities x",
        "clm capabilities x --format json",
        "clm architecture",
        "clm architecture x",
        "clm architecture x --format json",
        "clm architecture --root decide",
        "clm architecture x --root decide",
        "cargo run -q -p clm -- architecture x",
        "cargo run -p clm -- decide architecture",
        // `decide` as an option's value is no subcommand.
        "clm capabilities --root decide",
        "clm capabilities --root decide x",
        "clm capabilities x --root decide",
        "cargo run -q -p clm -- capabilities x",
        "echo clm capabilities decide x --value high --decision keep",
    ] {
        assert_eq!(guard(&bash(command)), Decision::Allow, "{command}");
    }
}

/// The reason `guard` blocks a piped `clm land` with.
fn blocks_piped_land(command: &str) -> String {
    let why = blocks(&bash(command));
    assert!(why.contains("guard-refuses-piped-land"), "{command}: {why}");
    assert!(why.contains("last line"), "{command}: {why}");
    why
}

#[test]
fn check_piped_into_tail() {
    // req:plugin.hooks.guard/piped-land-guard@c9f11cbd
    for (piped, plain) in [
        ("clm land --check-only | tail -5", "clm land --check-only"),
        (
            "clm land --check-only 2>&1 | tail -20",
            "clm land --check-only",
        ),
        ("clm land impl/x |& tail", "clm land impl/x"),
        (
            "cargo run -q -p clm -- land --check-only | tail -5",
            "cargo run -q -p clm -- land --check-only",
        ),
        (
            "cargo run -p clm land spec/x | tail",
            "cargo run -p clm land spec/x",
        ),
    ] {
        let why = blocks_piped_land(piped);
        assert!(why.contains(&format!("`{plain}`")), "{piped}: {why}");
    }
}

#[test]
fn land_piped_inside_a_script() {
    // req:plugin.hooks.guard/piped-land-guard@c9f11cbd
    blocks_piped_land("bash -c 'clm land --check-only | tail'");
    blocks_piped_land("cd /tmp && clm land spec/x | tee land.log");
    blocks_piped_land("echo y | clm land spec/x | tail");
    blocks_piped_land("bash <<'EOF'\nclm land --check-only | tail\nEOF");
}

#[test]
fn a_plain_land_is_allowed() {
    // req:plugin.hooks.guard/piped-land-guard@c9f11cbd
    for command in [
        "clm land --check-only",
        "clm land spec/x",
        "clm land --check-only 2>&1",
        "clm land --check-only && git status",
    ] {
        allows(&bash(command));
    }
}

#[test]
fn land_as_another_subcommands_argument_is_allowed() {
    // req:plugin.hooks.guard/piped-land-guard@c9f11cbd
    for command in [
        "cargo run -q -p clm -- hashes land | grep x",
        "cargo run -q -p clm -- context land | head",
        "cargo run -p clm -- pending land |& head",
        "cargo run -p clm -- hashes -- land | head",
        "clm hashes land | grep x",
    ] {
        allows(&bash(command));
    }
}

#[test]
fn help_piped_is_allowed() {
    // req:plugin.hooks.guard/piped-land-guard@c9f11cbd
    for command in [
        "clm land --help | head",
        "clm land -h | head",
        "clm land impl/x --help |& head",
        "cargo run -q -p clm -- land --help | head",
        "cargo run -q -p clm -- land --help land | head",
    ] {
        allows(&bash(command));
    }
}

#[test]
fn help_elsewhere_than_the_lands_arguments_is_blocked() {
    // req:plugin.hooks.guard/piped-land-guard@c9f11cbd
    for command in [
        "clm land spec/--help | tail",
        "echo --help | clm land spec/x | tail",
        "cargo run --help -- land | tail",
    ] {
        blocks_piped_land(command);
    }
}

#[test]
fn other_pipes_are_allowed() {
    // req:plugin.hooks.guard/piped-land-guard@c9f11cbd
    for command in [
        "clm status | head",
        "clm status land | head",
        "echo y | clm land spec/x",
        "cargo run -q -p clm -- status | head",
        "cat > body.md <<'EOF'\nclm land --check-only | tail\nEOF",
    ] {
        allows(&bash(command));
    }
}

#[test]
fn the_hooks_budgets_are_twenty_and_thirty_seconds() {
    // req:plugin.hooks.guard/hook-budgets@60ab39c0
    assert_eq!(Budgets::HOOK.push, Duration::from_secs(20));
    assert_eq!(Budgets::HOOK.lookup, Duration::from_secs(30));
}

#[test]
fn the_guard_and_the_hook_judge_with_the_hooks_budgets() {
    // req:plugin.hooks.guard/hook-budgets@60ab39c0
    use clm_agent_harness::{AgentHarness::ClaudeCode, render};
    let dir = TempDir::new().unwrap();
    let w = dir.path().join("w");
    checkout_on(&w, "impl/x/b");
    let fake = TempDir::new().unwrap();
    let path = fake_rest_gh(
        fake.path(),
        &fake.path().join("calls"),
        "https://github.com/o/r",
    );
    // Git answers the push's queries and the fake `gh` the re-run's lookups at once, so the
    // hook's budgets let both through, and a spent budget blocks only what spends it.
    let push = bash_in_opt("git push", Some(&w));
    let rerun = bash_in_opt("gh run rerun 42", Some(&w));
    // Blocked with no query: a judgement that ignores the event allows it.
    let to_main = bash_in_opt("git push origin HEAD:main", Some(&w));
    let no_push = Budgets {
        push: Duration::ZERO,
        ..Budgets::HOOK
    };
    let no_lookup = Budgets {
        lookup: Duration::ZERO,
        ..Budgets::HOOK
    };
    let env = [("PATH", path.as_str())];
    let judged = {
        let _exclusive = FORKS.write().unwrap_or_else(PoisonError::into_inner);
        // The developer's own git config must not leak in, as `hook_args` keeps it out.
        let isolated = [
            ("GIT_CONFIG_GLOBAL", "/dev/null"),
            ("GIT_CONFIG_NOSYSTEM", "1"),
        ];
        let _set = ProcessEnv::set(isolated.iter().chain(&env));
        let within = clm_guard::guard_within;
        let blocked = |d: Decision, why: &str| matches!(d, Decision::Block(r) if r.contains(why));
        assert!(blocked(within(&push, &no_push), "didn't answer in time"));
        assert_eq!(within(&rerun, &no_push), Decision::Allow);
        assert_eq!(within(&push, &no_lookup), Decision::Allow);
        assert!(blocked(within(&rerun, &no_lookup), "humans-cut-releases"));
        [&push, &rerun, &to_main].map(|event| {
            (
                within(event, &Budgets::HOOK),
                clm_guard::guard(event),
                clm_guard::guard_hook(ClaudeCode, event),
            )
        })
    };
    assert_eq!(judged[0].0, Decision::Allow);
    assert_eq!(judged[1].0, Decision::Allow);
    assert!(matches!(judged[2].0, Decision::Block(_)), "{judged:?}");
    for (event, (hooks, guarded, hooked)) in [&push, &rerun, &to_main].into_iter().zip(&judged) {
        assert_eq!(guarded, hooks, "{event}");
        let rendered = render(ClaudeCode, hooks);
        assert_eq!(hooked, &rendered, "{event}");
        let out = hook_env("guard", event, &env);
        assert_eq!(out.status.code(), Some(rendered.exit_code), "{event}");
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            rendered.stdout,
            "{event}"
        );
        let stderr = String::from_utf8_lossy(&out.stderr);
        let expected = match rendered.stderr.as_str() {
            "" => String::new(),
            reason => format!("{reason}\n"),
        };
        assert_eq!(stderr, expected, "{event}");
    }
}

const WHOLE_LINE_REFUSED: &str = "nothing in it ran";

#[test]
fn a_block_says_the_whole_command_line_was_refused() {
    // req:plugin.hooks.guard/block-refuses-whole-command@050af7ab
    let dir = TempDir::new().unwrap();
    checkout_on(dir.path(), "impl/x/b");
    for command in [
        "git commit -m m && clm land impl/x | tail",
        "git add f && git push origin HEAD:main",
    ] {
        let why = blocks(&bash_in(command, dir.path()));
        let last = why.lines().last().unwrap();
        assert!(last.contains(WHOLE_LINE_REFUSED), "{command}: {why}");
    }
}

#[test]
fn two_refusing_rules_say_it_once() {
    // req:plugin.hooks.guard/block-refuses-whole-command@050af7ab
    let dir = TempDir::new().unwrap();
    checkout_on(dir.path(), "impl/x/b");
    let why = blocks(&bash_in(
        "git push origin main; clm land spec/x | tail",
        dir.path(),
    ));
    assert!(why.contains("guard-blocks-push-to-main"), "{why}");
    assert!(why.contains("guard-refuses-piped-land"), "{why}");
    assert_eq!(why.matches(WHOLE_LINE_REFUSED).count(), 1, "{why}");
    assert!(why.lines().last().unwrap().contains(WHOLE_LINE_REFUSED));
}

#[test]
fn an_ask_the_agent_harness_refuses() {
    // req:plugin.hooks.guard/block-refuses-whole-command@050af7ab
    let event = bash("git commit -m m && clm approve spec/x");
    for name in ["codex", "none"] {
        let out = hook_args(&["guard", "--agent-harness", name], &event, &[]);
        assert_eq!(out.status.code(), Some(2), "{name}: {out:?}");
        let err = String::from_utf8(out.stderr).unwrap();
        assert!(err.contains("own terminal"), "{name}: {err}");
        assert_eq!(err.matches(WHOLE_LINE_REFUSED).count(), 1, "{name}: {err}");
        let last = err.lines().last().unwrap();
        assert!(last.contains(WHOLE_LINE_REFUSED), "{name}: {err}");
    }
}

#[test]
fn allowed_and_asked_commands_carry_no_such_line() {
    // req:plugin.hooks.guard/block-refuses-whole-command@050af7ab
    allows(&bash("git commit -m m && clm land impl/x"));
    let why = asks_cmd("clm approve spec/x");
    assert!(!why.contains(WHOLE_LINE_REFUSED), "{why}");
}

#[test]
fn a_push_to_main_under_codex_is_blocked() {
    // req:plugin.hooks.guard/guard-per-agent-harness@1ce86c3d
    let dir = TempDir::new().unwrap();
    checkout_on(dir.path(), "impl/x/b");
    let out = guard_hook(
        clm_agent_harness::AgentHarness::Codex,
        &bash_in("git push origin main", dir.path()),
    );
    assert_eq!(out.exit_code, 2, "{out:?}");
    assert!(out.stderr.contains("guard-blocks-push-to-main"), "{out:?}");
}

#[test]
fn an_approve_under_codex_says_to_use_the_humans_terminal() {
    // req:plugin.hooks.guard/guard-per-agent-harness@1ce86c3d
    let out = guard_hook(
        clm_agent_harness::AgentHarness::Codex,
        &bash("clm approve spec/x"),
    );
    assert_eq!(out.exit_code, 2, "{out:?}");
    assert!(out.stderr.contains("own terminal"), "{out:?}");
    // with the line `block-refuses-whole-command` adds to an ask it gives as a refusal
    let last = out.stderr.lines().last().unwrap();
    assert!(last.contains(WHOLE_LINE_REFUSED), "{out:?}");
}

#[test]
fn claude_code_is_unchanged_and_other_events_pass() {
    // req:plugin.hooks.guard/guard-per-agent-harness@1ce86c3d
    use clm_agent_harness::AgentHarness::ClaudeCode;
    let out = guard_hook(ClaudeCode, &bash("clm approve spec/x"));
    assert_eq!(out.exit_code, 0);
    let why = asks_cmd("clm approve spec/x");
    assert_eq!(out.stdout, clm_guard::ask_json(&why));
    let v: serde_json::Value = serde_json::from_str(&out.stdout).unwrap();
    assert_eq!(v["hookSpecificOutput"]["permissionDecision"], "ask");
    let other = serde_json::json!({
        "hook_event_name": "PreToolUse", "tool_name": "Read",
        "tool_input": { "command": "git push origin main" },
    })
    .to_string();
    for event in [other.as_str(), "not json"] {
        let out = guard_hook(ClaudeCode, event);
        assert_eq!((out.exit_code, out.stdout.as_str()), (0, ""), "{event}");
    }
}

/// A git repository with a valid `specs/` and, when given, a `clm.toml`.
fn clm_repo(settings: Option<&str>) -> TempDir {
    let dir = TempDir::new().unwrap();
    git(dir.path(), &["init", "-q", "-b", "main"]);
    fs::create_dir_all(dir.path().join("specs")).unwrap();
    fs::write(
        dir.path().join("specs/SPEC.md"),
        "---\ntitle: Root\nstatus: active\nowners: []\n---\n\n## Contract\nr\n",
    )
    .unwrap();
    if let Some(settings) = settings {
        fs::write(dir.path().join("clm.toml"), settings).unwrap();
    }
    dir
}

/// Variables set on this process, put back as they were when it drops, a panic included.
/// Set only under the exclusive spawn lock, so this file's other tests spawn nothing
/// meanwhile, and under `nextest` no other test shares the process.
struct ProcessEnv(Vec<(String, Option<std::ffi::OsString>)>);

impl ProcessEnv {
    fn set<'a>(vars: impl Iterator<Item = &'a (&'a str, &'a str)>) -> ProcessEnv {
        let mut saved = Vec::new();
        for (name, value) in vars {
            saved.push((name.to_string(), std::env::var_os(name)));
            // SAFETY: see the type's comment.
            unsafe { std::env::set_var(name, value) };
        }
        ProcessEnv(saved)
    }
}

impl Drop for ProcessEnv {
    fn drop(&mut self) {
        for (name, value) in self.0.drain(..).rev() {
            match value {
                // SAFETY: see the type's comment.
                Some(value) => unsafe { std::env::set_var(&name, value) },
                None => unsafe { std::env::remove_var(&name) },
            }
        }
    }
}

/// What `clm hook guard` printed and exited with, as the call gives them.
#[derive(Debug)]
struct Output {
    status: Status,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

/// The call's exit status.
#[derive(Debug)]
struct Status(u8);

impl Status {
    fn code(&self) -> Option<i32> {
        Some(self.0.into())
    }
}

fn hook(name: &str, event: &str) -> Output {
    hook_env(name, event, &[])
}

fn hook_env(name: &str, event: &str, env: &[(&str, &str)]) -> Output {
    hook_args(&[name], event, env)
}

/// `clm hook` with `args`, `guard` and its own arguments, called as `clm-guard`'s call on
/// `event`, with `env` over this process's environment. `gh` and `git` inherit the process's
/// environment, not the lookup, so `env` is also set on this process for the length of the
/// call (`ci/nextest-runner` runs each test in a process of its own), holding the spawn lock
/// exclusively so that a plain `cargo test` runs no other spawn meanwhile.
fn hook_args(args: &[&str], event: &str, env: &[(&str, &str)]) -> Output {
    assert_eq!(args.first(), Some(&"guard"), "{args:?}");
    // The developer's own settings must not leak in either.
    let config_home = TempDir::new().unwrap();
    let config = config_home.path().to_str().unwrap().to_string();
    let lookup = |name: &str| {
        if let Some((_, value)) = env.iter().rev().find(|(n, _)| *n == name) {
            return Some(value.to_string());
        }
        match name {
            "XDG_CONFIG_HOME" => Some(config.clone()),
            "CLM_AUTOSYNC" | "CLM_DEFECTS" => None,
            _ => std::env::var(name).ok(),
        }
    };
    let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
    let _exclusive = FORKS.write().unwrap_or_else(PoisonError::into_inner);
    // The developer's own git config (signing, hooks, helpers) must not leak in.
    let isolated = [
        ("GIT_CONFIG_GLOBAL", "/dev/null"),
        ("GIT_CONFIG_NOSYSTEM", "1"),
    ];
    let _set = ProcessEnv::set(isolated.iter().chain(env));
    let code = clm_guard::guard_command(&args[1..], event, &lookup, &mut stdout, &mut stderr);
    Output {
        status: Status(code),
        stdout,
        stderr,
    }
}

#[test]
fn the_call_prints_and_exits_as_the_hook_does() {
    // req:plugin.hooks.guard/commands-in-crate@4b058acb
    let repo = clm_repo(None);
    // A block: the reason on the error stream, a line of its own, and exit 2.
    let push = bash_in("git push origin main", repo.path());
    let out = hook_args(&["guard", "--agent-harness", "codex"], &push, &[]);
    let rendered = guard_hook(clm_agent_harness::AgentHarness::Codex, &push);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    assert!(out.stdout.is_empty(), "{out:?}");
    assert_eq!(
        String::from_utf8(out.stderr).unwrap(),
        format!("{}\n", rendered.stderr)
    );
    assert!(rendered.stderr.contains("guard-blocks-push-to-main"));
    // An ask: the decision's JSON on the output stream, as it is, and exit 0.
    let approve = bash("clm approve spec/x");
    let out = hook("guard", &approve);
    assert_eq!(
        (out.status.code(), out.stderr.len()),
        (Some(0), 0),
        "{out:?}"
    );
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        clm_guard::ask_json(&asks_cmd("clm approve spec/x"))
    );
    // The same ask for the agent harness the arguments name, which can't confirm: Codex's
    // block, on the error stream, and exit 2.
    let out = hook_args(&["guard", "--agent-harness", "codex"], &approve, &[]);
    let rendered = guard_hook(clm_agent_harness::AgentHarness::Codex, &approve);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    assert!(out.stdout.is_empty(), "{out:?}");
    let err = String::from_utf8(out.stderr).unwrap();
    assert_eq!(err, format!("{}\n", rendered.stderr));
    assert!(err.contains("own terminal"), "{err}");
    // An agent harness it doesn't know: exit 1, naming it, and nothing on the output stream.
    let out = hook_args(&["guard", "--agent-harness", "nope"], &approve, &[]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(out.stdout.is_empty(), "{out:?}");
    assert_eq!(
        String::from_utf8(out.stderr).unwrap(),
        "unknown agent harness `nope`\n"
    );
}

#[test]
fn the_call_returns_2_on_arguments_it_cant_parse() {
    // req:plugin.hooks.guard/commands-in-crate@4b058acb
    // An option it doesn't have, `--root` among them, since it takes no repository root (the
    // guard places each command by its event's `cwd`); the test's process goes on after each.
    for (args, named) in [
        (&["guard", "--nope"][..], "--nope"),
        (&["guard", "--root", "."][..], "--root"),
        (&["guard", "--agent-harness"][..], "--agent-harness"),
    ] {
        let out = hook_args(args, &bash("clm approve spec/x"), &[]);
        assert_eq!(out.status.code(), Some(2), "{out:?}");
        assert!(out.stdout.is_empty(), "{out:?}");
        let err = String::from_utf8(out.stderr).unwrap();
        assert!(err.contains(named) && err.starts_with("error:"), "{err}");
    }
}

#[test]
fn the_call_prints_its_help() {
    // req:plugin.hooks.guard/commands-in-crate@4b058acb
    let description = "PreToolUse: exit 2 with the reason when a Bash command would cut a \
                       release or push to `main`; print the `ask` decision when it would run \
                       `clm approve`, or file a CLM defect as an issue";
    for flag in ["--help", "-h"] {
        let out = hook_args(&["guard", flag], &bash("clm approve spec/x"), &[]);
        assert_eq!(out.status.code(), Some(0), "{flag}: {out:?}");
        assert!(out.stderr.is_empty(), "{flag}: {out:?}");
        let help = String::from_utf8(out.stdout).unwrap();
        assert_eq!(help.lines().next(), Some(description), "{flag}: {help}");
        assert!(
            help.contains("Usage: clm hook guard [OPTIONS]"),
            "{flag}: {help}"
        );
        let option = help
            .lines()
            .find(|l| l.contains("--agent-harness <AGENT_HARNESS>"))
            .unwrap_or_else(|| panic!("{flag}: {help}"));
        assert!(option.contains("[default: claude-code]"), "{flag}: {help}");
        assert!(
            option.contains(
                "The agent harness running the hook: `claude-code` (the default), `codex` or `none`"
            ),
            "{flag}: {help}"
        );
    }
}

/// A stream whose every write fails.
struct Broken;

impl std::io::Write for Broken {
    fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
        Err(std::io::Error::other("broken"))
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Err(std::io::Error::other("broken"))
    }
}

#[test]
fn a_stream_that_fails_returns_2() {
    // req:plugin.hooks.guard/commands-in-crate@4b058acb
    let repo = clm_repo(None);
    let ask = bash("clm approve spec/x");
    let block = bash_in("git push origin HEAD:main", repo.path());
    let lookup = |name: &str| std::env::var(name).ok();
    let _shared = FORKS.read().unwrap_or_else(PoisonError::into_inner);
    let mut buffer = Vec::new();
    let code = clm_guard::guard_command(
        Vec::<String>::new(),
        &ask,
        &lookup,
        &mut Broken,
        &mut buffer,
    );
    assert_eq!(code, 2, "an ask on a failing output stream");
    let code = clm_guard::guard_command(
        Vec::<String>::new(),
        &block,
        &lookup,
        &mut buffer,
        &mut Broken,
    );
    assert_eq!(code, 2, "a block on a failing error stream");
    // An agent harness it doesn't know returns 1, its failed write included.
    let unknown = ["--agent-harness", "nope"];
    let code = clm_guard::guard_command(unknown, &ask, &lookup, &mut buffer, &mut Broken);
    assert_eq!(
        code, 1,
        "an unknown agent harness on a failing error stream"
    );
}

#[test]
fn the_call_reads_the_settings_from_its_environment() {
    // req:plugin.hooks.guard/commands-in-crate@4b058acb
    // A report the guard can't show is refused under `ask`, and let through under
    // `backlog`, which only the lookup holds, not this process's environment.
    let nowhere = TempDir::new().unwrap();
    let event = bash_in("clm defect --title x < report.md", nowhere.path());
    let call = |defects: Option<&str>| {
        let lookup = |name: &str| match name {
            "CLM_DEFECTS" => defects.map(String::from),
            "XDG_CONFIG_HOME" => Some(nowhere.path().to_str().unwrap().to_string()),
            _ => std::env::var(name).ok(),
        };
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let _shared = FORKS.read().unwrap_or_else(PoisonError::into_inner);
        let code =
            clm_guard::guard_command(Vec::<String>::new(), &event, &lookup, &mut out, &mut err);
        (code, String::from_utf8(err).unwrap())
    };
    let (code, err) = call(None);
    assert_eq!(code, 2, "{err}");
    assert!(err.contains("defect-ask-shows-report"), "{err}");
    assert_eq!(call(Some("backlog")), (0, String::new()));
}

#[test]
fn guard_command_asks_with_the_reason() {
    // req:plugin.hooks.guard/approve-guard@5e09fed8
    let event = |c: &str| {
        serde_json::json!({ "tool_name": "Bash", "tool_input": { "command": c } }).to_string()
    };
    for command in [
        "clm approve spec/x",
        "cargo run -q -p clm -- approve spec/x",
    ] {
        let out = hook("guard", &event(command));
        assert_eq!(
            (out.status.code(), out.stderr.len()),
            (Some(0), 0),
            "{command}: {out:?}"
        );
        let answer: Value = serde_json::from_slice(&out.stdout).expect("ask JSON on stdout");
        let decision = &answer["hookSpecificOutput"];
        assert_eq!(decision["hookEventName"], "PreToolUse", "{answer}");
        assert_eq!(decision["permissionDecision"], "ask", "{answer}");
        let reason = decision["permissionDecisionReason"].as_str().unwrap();
        assert!(reason.contains("humans-review-intent"), "{reason}");
    }
    // approval combined with a push to main: the block wins, on stderr, exit 2.
    let out = hook(
        "guard",
        &event("clm approve spec/x && git push origin main"),
    );
    assert_eq!(
        (out.status.code(), out.stdout.len()),
        (Some(2), 0),
        "{out:?}"
    );
    assert!(String::from_utf8_lossy(&out.stderr).contains("guard-blocks-push-to-main"));
    for command in ["gh pr edit 30 --add-label spec-approved", "clm review x"] {
        let out = hook("guard", &event(command));
        assert_eq!(
            (out.status.code(), out.stdout.len(), out.stderr.len()),
            (Some(0), 0, 0),
            "{command}"
        );
    }
    let out = hook("guard", "garbage");
    assert_eq!((out.status.code(), out.stdout.len()), (Some(0), 0));
}

#[test]
fn guard_command_blocks_dispatching_a_release() {
    // req:plugin.hooks.guard/release-guard@1c7c5dad
    let event = serde_json::json!({
        "tool_name": "Bash",
        "tool_input": { "command": "gh workflow run release.yml -f version=0.2.0" },
    })
    .to_string();
    let out = hook("guard", &event);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    assert!(String::from_utf8_lossy(&out.stderr).contains("humans-cut-releases"));
    // dispatching the site
    for command in [
        "gh workflow run site.yml",
        "gh workflow run .github/workflows/Site.YAML",
        "gh workflow run SITE",
    ] {
        let event = serde_json::json!({
            "tool_name": "Bash",
            "tool_input": { "command": command },
        })
        .to_string();
        let out = hook("guard", &event);
        assert_eq!(out.status.code(), Some(2), "{command}: {out:?}");
        assert!(String::from_utf8_lossy(&out.stderr).contains("humans-cut-releases"));
    }
}

/// A `PATH` whose `gh` is `script`, held in `dir`, ahead of the system's.
fn path_with_gh(dir: &Path, script: &str) -> String {
    executable(&dir.join("gh"), script);
    format!("{}:{}", dir.display(), std::env::var("PATH").unwrap())
}

/// A fake `gh` that logs each call's arguments, directory, `GH_REPO` and `GH_HOST` to
/// `calls`; answers `repo view --json url` with `https://ghe.example.com/o/r` when its
/// `GH_REPO` is `o/r` and with `url` otherwise; fails on anything else but `api` as `gh` 2.45
/// fails on `run view --json path`; and answers the REST lookups of runs and jobs by the
/// path's last part.
fn fake_rest_gh(dir: &Path, calls: &Path, url: &str) -> String {
    path_with_gh(
        dir,
        &format!(
            "#!/bin/sh\nprintf '%s|%s|%s|%s\\n' \"$*\" \"$(pwd)\" \"$GH_REPO\" \"$GH_HOST\" >> {calls}\n\
             if [ \"$*\" = 'repo view --json url' ]; then\n\
             if [ \"$GH_REPO\" = o/r ]; then echo '{{\"url\":\"https://ghe.example.com/o/r\"}}'; \
             else echo '{{\"url\":\"{url}\"}}'; fi; exit 0; fi\n\
             if [ \"$1\" != api ]; then echo 'Unknown JSON field: \"path\"' >&2; exit 1; fi\n\
             for last; do :; done\ncase \"$last\" in\n\
             */actions/runs/77) echo '{{\"name\":\"Release\",\"path\":\".github/workflows/release.yml\"}}' ;;\n\
             */actions/runs/76) echo '{{\"name\":\"Publish\",\"path\":\".github/workflows/release.yaml\"}}' ;;\n\
             */actions/runs/78) echo '{{\"name\":\"release\",\"path\":\".github/workflows/publish.yml\"}}' ;;\n\
             */actions/runs/79) echo '{{\"name\":\"Publish\",\"path\":\".github/workflows/release.yml@refs/heads/main\"}}' ;;\n\
             */actions/runs/80) echo '{{\"name\":\"Publish\",\"path\":\".github/workflows/Site.YML\"}}' ;;\n\
             */actions/runs/81) echo '{{\"name\":\"SITE\",\"path\":\".github/workflows/pages.yml\"}}' ;;\n\
             */actions/runs/82) echo '{{\"name\":\"Publish\",\"path\":\".github/workflows/site.yaml@refs/heads/main\"}}' ;;\n\
             */actions/runs/0) echo '{{\"name\":\"CI\",\"path\":\".github/workflows/ci.yml\"}}' ;;\n\
             */actions/runs/42) echo '{{\"name\":\"CI\",\"path\":\".github/workflows/ci.yml\"}}' ;;\n\
             */actions/runs/43) echo '{{\"name\":\"CI\"}}' ;;\n\
             */actions/runs/44) echo '[]' ;;\n\
             */actions/jobs/900) echo '{{\"run_id\":77}}' ;;\n\
             */actions/jobs/901) echo '{{\"run_id\":42}}' ;;\n\
             */actions/jobs/902) echo '{{\"run_id\":\"x\"}}' ;;\n\
             */actions/jobs/903) echo 'not json' ;;\n\
             */actions/jobs/904) echo '{{\"run_id\":0}}' ;;\n\
             *) echo 'HTTP 404: Not Found' >&2; exit 1 ;;\nesac\n",
            calls = calls.display()
        ),
    )
}

/// The calls the fake `gh` logged since the last one, clearing the log.
fn take_calls(calls: &Path) -> Vec<String> {
    let log = fs::read_to_string(calls).unwrap_or_default();
    let _ = fs::remove_file(calls);
    log.lines().map(String::from).collect()
}

#[test]
fn guard_blocks_rerunning_a_release_run() {
    // req:plugin.hooks.guard/release-guard@1c7c5dad
    // req:plugin.hooks.guard/release-run-lookup@2c3b26ad
    let fake = TempDir::new().unwrap();
    let calls = fake.path().join("calls");
    let path = fake_rest_gh(fake.path(), &calls, "https://github.com/o/r");
    let cwd = TempDir::new().unwrap();
    let env = [("PATH", path.as_str())];
    let guard = |command: &str| guard_in(cwd.path(), command, &env);
    // re-running a release run, and a release workflow by another file or name
    for command in [
        "gh run rerun 77 --failed",
        "gh run rerun 76",
        "gh run rerun 78",
        "gh run rerun 79",
        // a site run is a release run
        "gh run rerun 80",
        "gh run rerun 81",
        "gh run rerun 82",
        // a body a shell runs
        "bash <<'EOF'\ngh run rerun 77\nEOF",
    ] {
        let (code, why) = guard(command);
        assert_eq!(code, Some(2), "{command}: {why}");
        assert!(why.contains("humans-cut-releases"), "{command}: {why}");
    }
    // re-running a CI run, on a `gh` without `run view`'s `path`, in the event's `cwd`
    take_calls(&calls);
    assert_eq!(guard("gh run rerun 42 --failed"), (Some(0), String::new()));
    let dir = cwd.path().canonicalize().unwrap();
    let dir = dir.to_string_lossy();
    let expected = [
        format!("repo view --json url|{dir}||"),
        format!("api --hostname github.com repos/o/r/actions/runs/42|{dir}||"),
    ];
    assert_eq!(take_calls(&calls), expected);
    // re-running a release run's job, in each spelling, and beside a CI run
    for command in [
        "gh run rerun --job 900",
        "gh run rerun -j 900",
        "gh run rerun -j900",
        "gh run rerun --job=900",
        "gh run rerun 42 --job 900",
    ] {
        let (code, why) = guard(command);
        assert_eq!(code, Some(2), "{command}: {why}");
        assert!(why.contains("humans-cut-releases"), "{command}: {why}");
    }
    // re-running a CI run's job: the job names its run
    take_calls(&calls);
    assert_eq!(guard("gh run rerun --job 901"), (Some(0), String::new()));
    let asked: Vec<String> = take_calls(&calls)
        .iter()
        .map(|c| c.split('|').next().unwrap().to_string())
        .collect();
    assert_eq!(
        asked,
        [
            "repo view --json url",
            "api --hostname github.com repos/o/r/actions/jobs/901",
            "api --hostname github.com repos/o/r/actions/runs/42",
        ],
    );
    // the guard stops looking up at the first release run, after resolving the repository
    // once per segment
    for command in [
        "gh run rerun 77 --job 901",
        "gh run rerun 77; gh run rerun 42",
    ] {
        assert_eq!(guard(command).0, Some(2), "{command}");
        assert_eq!(take_calls(&calls).len(), 2, "{command}");
    }
    assert_eq!(guard("gh run rerun 42 --job 901").0, Some(0));
    let made = take_calls(&calls);
    let views = made.iter().filter(|c| c.starts_with("repo view")).count();
    assert_eq!((views, made.len()), (1, 4), "{made:?}");
    // run can't be looked up: an error, a run without `path`, something that isn't a run
    // or a job, a job whose `run_id` isn't a positive integer, a job ID that isn't one
    for command in [
        "gh run rerun 5",
        "gh run rerun 43",
        "gh run rerun 44",
        "gh run rerun --job 905",
        "gh run rerun --job 902",
        "gh run rerun --job 903",
        "gh run rerun --job 904",
        "gh run rerun --job $JOB",
        "gh run rerun $RUN",
        "gh run rerun `cat id`",
    ] {
        let (code, why) = guard(command);
        assert_eq!(code, Some(2), "{command}: {why}");
    }
    // an ID the guard can't know fails its lookup with no `gh` call, and a word that is no
    // ID is nothing to look up
    take_calls(&calls);
    for command in [
        "gh run rerun $RUN",
        "gh run rerun --job $JOB",
        "gh run rerun `cat id`",
    ] {
        let (code, why) = guard(command);
        assert_eq!(code, Some(2), "{command}: {why}");
        assert!(why.contains("humans-cut-releases"), "{command}: {why}");
    }
    assert_eq!(guard("gh run rerun abc"), (Some(0), String::new()));
    assert_eq!(take_calls(&calls), Vec::<String>::new());
    let no_gh = TempDir::new().unwrap();
    let path = no_gh.path().to_str().unwrap();
    for command in ["gh run rerun 42", "gh run rerun --job 901"] {
        let (code, why) = guard_in(cwd.path(), command, &[("PATH", path)]);
        assert_eq!(code, Some(2), "{command}: {why}");
    }
    // no run or job ID, or no rerun: nothing to look up, so prose that names a rerun isn't one
    take_calls(&calls);
    for command in [
        "gh run rerun",
        "gh run rerun --failed",
        "gh run list",
        "gh run view 77",
        "git commit -m \"Document gh run rerun failures\"",
        "git commit -m \"Explain gh run rerun -j usage\"",
    ] {
        assert_eq!(guard(command), (Some(0), String::new()), "{command}");
    }
    assert!(!calls.exists(), "gh was called");
}

#[test]
fn guard_looks_a_rerun_up_where_it_runs() {
    // req:plugin.hooks.guard/release-run-lookup@2c3b26ad
    let fake = TempDir::new().unwrap();
    let calls = fake.path().join("calls");
    let path = fake_rest_gh(fake.path(), &calls, "https://github.com/p/q");
    let cwd = TempDir::new().unwrap();
    fs::create_dir(cwd.path().join("other")).unwrap();
    let env = [("PATH", path.as_str())];
    let top = cwd.path().canonicalize().unwrap();
    let top = top.to_string_lossy();
    let other = format!("{top}/other");
    // the re-run's repository and host
    for (command, call) in [
        (
            "cd other && gh run rerun 42",
            format!(
                "repo view --json url|{other}||\n\
                 api --hostname github.com repos/p/q/actions/runs/42|{other}||"
            ),
        ),
        (
            "gh run rerun 42 -R o/r",
            format!("api repos/o/r/actions/runs/42|{top}||"),
        ),
        (
            "gh run rerun 42 --repo ghe.example.com/o/r",
            format!("api --hostname ghe.example.com repos/o/r/actions/runs/42|{top}||"),
        ),
        (
            "GH_REPO=o/r gh run rerun 42 -R x/y",
            format!("api repos/x/y/actions/runs/42|{top}|o/r|"),
        ),
        (
            "GH_HOST=ghe.example.com GH_REPO=o/r gh run rerun 42",
            format!(
                "repo view --json url|{top}|o/r|ghe.example.com\n\
                 api --hostname ghe.example.com repos/o/r/actions/runs/42|{top}|o/r|ghe.example.com"
            ),
        ),
        // glued flags, a `cd` it can't follow, and a job looked up on the same host
        (
            "gh run rerun -Ro/r 42",
            format!("api repos/o/r/actions/runs/42|{top}||"),
        ),
        (
            "gh run rerun --repo=o/r 42",
            format!("api repos/o/r/actions/runs/42|{top}||"),
        ),
        (
            "gh run rerun 42 -R x/y -R o/r",
            format!("api repos/o/r/actions/runs/42|{top}||"),
        ),
        (
            "cd other; cd $W && gh run rerun 42",
            format!(
                "repo view --json url|{top}||\n\
                 api --hostname github.com repos/p/q/actions/runs/42|{top}||"
            ),
        ),
        (
            "cd nonesuch && gh run rerun 42",
            format!(
                "repo view --json url|{top}||\n\
                 api --hostname github.com repos/p/q/actions/runs/42|{top}||"
            ),
        ),
        (
            "gh run rerun -j 901 -R h.example/o/r",
            format!(
                "api --hostname h.example repos/o/r/actions/jobs/901|{top}||\n\
                 api --hostname h.example repos/o/r/actions/runs/42|{top}||"
            ),
        ),
    ] {
        take_calls(&calls);
        assert_eq!(
            guard_in(cwd.path(), command, &env),
            (Some(0), String::new()),
            "{command}"
        );
        assert_eq!(take_calls(&calls).join("\n"), call, "{command}");
    }
    // a repository that can't be read
    for command in [
        "gh run rerun 42 -R https://github.com/o/r",
        "gh run rerun 42 -R r",
        "gh run rerun 42 -R o//r",
    ] {
        let (code, why) = guard_in(cwd.path(), command, &env);
        assert_eq!(code, Some(2), "{command}: {why}");
    }
}

#[test]
fn a_rerun_with_no_directory_makes_no_lookup() {
    // req:plugin.hooks.guard/release-run-lookup@2c3b26ad
    // a guard with no directory: it runs in a directory that has been removed, and the event
    // has no `cwd`, so the re-run's segment has no checkout and every lookup fails
    let fake = TempDir::new().unwrap();
    let calls = fake.path().join("calls");
    let path = fake_rest_gh(fake.path(), &calls, "https://github.com/o/r");
    let event = serde_json::json!({
        "tool_name": "Bash",
        "tool_input": { "command": "gh run rerun 42" },
    })
    .to_string();
    let (code, err) = {
        // The working directory is the process's, so it is moved only under the exclusive
        // spawn lock, as `ProcessEnv` sets variables, and put back before the lock drops.
        let _exclusive = FORKS.write().unwrap_or_else(PoisonError::into_inner);
        let _set = ProcessEnv::set([("PATH", path.as_str())].iter());
        let home = std::env::current_dir().unwrap();
        let gone = TempDir::new().unwrap();
        std::env::set_current_dir(gone.path()).unwrap();
        fs::remove_dir(gone.path()).unwrap();
        let lookup = |name: &str| std::env::var(name).ok();
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code =
            clm_guard::guard_command(Vec::<String>::new(), &event, &lookup, &mut out, &mut err);
        std::env::set_current_dir(home).unwrap();
        assert!(out.is_empty(), "{out:?}");
        (code, String::from_utf8(err).unwrap())
    };
    assert_eq!(code, 2, "{err}");
    assert!(err.contains("humans-cut-releases"), "{err}");
    assert!(!calls.exists(), "gh was called: {:?}", take_calls(&calls));
}

/// Puts the process's working directory back when it drops, a panic included.
struct Cwd(std::path::PathBuf);

impl Drop for Cwd {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.0);
    }
}

/// `clm hook guard` on `command` in an event with no `cwd`, the guard running in `dir`, or
/// in a directory that has been removed when `dir` is `None`, with `path` as `PATH`: its
/// exit code, stdout and stderr. The working directory is the process's, so it is moved
/// only under the exclusive spawn lock, as `ProcessEnv` sets variables.
fn guard_with_no_cwd(dir: Option<&Path>, command: &str, path: &str) -> (u8, String, String) {
    let event = bash_in_opt(command, None);
    let _exclusive = FORKS.write().unwrap_or_else(PoisonError::into_inner);
    let vars = [
        ("PATH", path),
        ("GIT_CONFIG_GLOBAL", "/dev/null"),
        ("GIT_CONFIG_NOSYSTEM", "1"),
    ];
    let _set = ProcessEnv::set(vars.iter());
    let _home = Cwd(std::env::current_dir().unwrap());
    let gone = TempDir::new().unwrap();
    match dir {
        Some(dir) => std::env::set_current_dir(dir).unwrap(),
        None => {
            std::env::set_current_dir(gone.path()).unwrap();
            fs::remove_dir(gone.path()).unwrap();
        }
    }
    let lookup = |name: &str| std::env::var(name).ok();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = clm_guard::guard_command(Vec::<String>::new(), &event, &lookup, &mut out, &mut err);
    (
        code,
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}

#[test]
fn an_event_with_no_cwd() {
    // req:plugin.hooks.guard/approve-guard@5e09fed8
    // The guard's own working directory stands for the event's `cwd`, for every rule.
    let fake = TempDir::new().unwrap();
    let calls = fake.path().join("calls");
    let path = fake_rest_gh(fake.path(), &calls, "https://github.com/o/r");
    let repo = TempDir::new().unwrap();
    checkout_on(repo.path(), "main");
    let (code, _, err) = guard_with_no_cwd(Some(repo.path()), "git push", &path);
    assert_eq!(code, 2, "{err}");
    assert!(err.contains("guard-blocks-push-to-main"), "{err}");
    let (code, out, err) = guard_with_no_cwd(Some(repo.path()), "gh run rerun 42", &path);
    assert_eq!((code, out.as_str(), err.as_str()), (0, "", ""));
    let here = repo.path().canonicalize().unwrap();
    let calls = take_calls(&calls);
    assert!(
        calls.iter().any(|c| c.starts_with("repo view --json url|"))
            && calls.iter().any(|c| c.starts_with("api ")),
        "{calls:?}"
    );
    for call in &calls {
        let dir = call.split('|').nth(1).unwrap();
        assert_eq!(Path::new(dir), here, "{call}");
    }
}

#[test]
fn a_guard_with_no_directory() {
    // req:plugin.hooks.guard/approve-guard@5e09fed8
    // req:plugin.hooks.guard/release-run-lookup@2c3b26ad
    // The walk starts with no checkout: only an absolute `cd` or `-C` sets one, a push left
    // with none counts as under CLM and asks git nothing, and a re-run's lookups all fail.
    let fake = TempDir::new().unwrap();
    let calls = fake.path().join("calls");
    let path = fake_rest_gh(fake.path(), &calls, "https://github.com/o/r");
    let repo = TempDir::new().unwrap();
    checkout_on(repo.path(), "main");
    // Quoted, so a temporary directory holding a space or a glob character stays one word.
    let m = format!("'{}'", repo.path().display());
    let cases = [
        (
            "git push origin main".to_string(),
            2,
            "guard-blocks-push-to-main",
        ),
        ("git push".to_string(), 0, ""),
        ("gh run rerun 42".to_string(), 2, "humans-cut-releases"),
        (
            format!("cd {m} && git push"),
            2,
            "guard-blocks-push-to-main",
        ),
        (format!("git -C {m} push"), 2, "guard-blocks-push-to-main"),
    ];
    for (command, code, rule) in cases {
        let (got, out, err) = guard_with_no_cwd(None, &command, &path);
        assert_eq!(got, code, "{command}: {err}");
        assert!(out.is_empty(), "{command}: {out}");
        assert!(err.contains(rule), "{command}: {err}");
        if code == 0 {
            assert!(err.is_empty(), "{command}: {err}");
        }
    }
    assert!(!calls.exists(), "gh was called: {:?}", take_calls(&calls));
    // The last absolute `-C` sets the checkout and a relative one after it joins it, and a
    // push it sets in a repository not under CLM isn't under CLM.
    let scratch = TempDir::new().unwrap();
    scratch_on(&scratch.path().join("s"), "main");
    let top = format!("'{}'", scratch.path().display());
    for (command, code) in [
        (format!("git -C {top} -C {m} push"), 2),
        (format!("git -C {m} -C {top} -C s push origin main"), 0),
        (format!("git -C {m} -C {top} -C s push"), 0),
    ] {
        let (got, _, err) = guard_with_no_cwd(None, &command, &path);
        assert_eq!(got, code, "{command}: {err}");
    }
    // The `defects` setting is the guard's own directory's: a report under `backlog` files
    // no issue, so nothing is asked.
    let backlog = clm_repo(Some("defects = \"backlog\"\n"));
    let report = "clm defect --title x <<'EOF'\nr\nEOF";
    let answer = guard_with_no_cwd(Some(backlog.path()), report, &path);
    assert_eq!(answer, (0, String::new(), String::new()));
}

/// A shell line printing `url` as `gh repo view --json url` would. A URL's `//` sits apart
/// from the JSON's braces: `clm-coverage` reads a test's end by counting braces outside a
/// `//` comment, so a test holding both on one line would never end.
fn echo_url(url: &str) -> String {
    format!("echo '{{\"url\":\"{url}\"}}'")
}

#[test]
fn guard_blocks_a_rerun_whose_repository_cant_be_resolved() {
    // req:plugin.hooks.guard/release-run-lookup@2c3b26ad
    // a repository that can't be resolved: `repo view` fails, or its `url` isn't
    // `<scheme>://<host>/<owner>/<repo>`
    let api_ci = "if [ \"$1\" = api ]; then \
                  echo '{\"name\":\"CI\",\"path\":\".github/workflows/ci.yml\"}'; exit 0; fi\n";
    let cwd = TempDir::new().unwrap();
    for view in [
        "echo 'no git remotes found' >&2; exit 1".to_string(),
        echo_url("o/r"),
        echo_url("https://github.com/o/r/"),
        echo_url("https://github.com/o"),
        echo_url("https:///o/r"),
        "echo '{}'".to_string(),
    ] {
        let fake = TempDir::new().unwrap();
        let path = path_with_gh(fake.path(), &format!("#!/bin/sh\n{api_ci}{view}\n"));
        let (code, why) = guard_in(cwd.path(), "gh run rerun 42", &[("PATH", &path)]);
        assert_eq!(code, Some(2), "{view}: {why}");
        assert!(why.contains("humans-cut-releases"), "{view}: {why}");
    }
    // The same fake, answering `repo view` well, lets the CI run's re-run through.
    let fake = TempDir::new().unwrap();
    let view = echo_url("https://github.com/o/r");
    let path = path_with_gh(fake.path(), &format!("#!/bin/sh\n{api_ci}{view}\n"));
    assert_eq!(
        guard_in(cwd.path(), "gh run rerun 42", &[("PATH", &path)]),
        (Some(0), String::new())
    );
}

#[test]
fn guard_command_blocks_pushing_to_main() {
    // req:plugin.hooks.guard/push-guard@d829891a
    let event = serde_json::json!({
        "tool_name": "Bash",
        "tool_input": { "command": "git push origin HEAD:main" },
    })
    .to_string();
    let out = hook("guard", &event);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    assert!(String::from_utf8_lossy(&out.stderr).contains("guard-blocks-push-to-main"));
}

/// The guard's answer to `command` run in `cwd` with `env`: its exit code, and the `ask`
/// reason or the block's stderr.
fn guard_in(cwd: &Path, command: &str, env: &[(&str, &str)]) -> (Option<i32>, String) {
    let event = serde_json::json!({
        "tool_name": "Bash",
        "tool_input": { "command": command },
        "cwd": cwd,
    })
    .to_string();
    let out = hook_env("guard", &event, env);
    if out.stdout.is_empty() {
        let stderr = String::from_utf8(out.stderr).unwrap();
        return (out.status.code(), stderr);
    }
    let answer: Value = serde_json::from_slice(&out.stdout).expect("ask JSON on stdout");
    assert_eq!(answer["hookSpecificOutput"]["permissionDecision"], "ask");
    let reason = answer["hookSpecificOutput"]["permissionDecisionReason"]
        .as_str()
        .unwrap()
        .to_owned();
    (out.status.code(), reason)
}

#[test]
fn guard_asks_before_filing_a_defect() {
    // req:plugin.hooks.guard/defect-approval@d7bcf2ca
    let nowhere = TempDir::new().unwrap();
    let report = "clm defect --title \"x\" <<'EOF'\nThe gate misfired.\nEOF";
    // asking: no settings, so `defects` is `ask`.
    let (code, reason) = guard_in(nowhere.path(), report, &[]);
    assert_eq!(code, Some(0));
    assert!(reason.contains("## CLM defects"), "{reason}");
    assert!(reason.contains("public issue"), "{reason}");
    assert!(reason.contains("pull request body"), "{reason}");
    assert!(!reason.contains("page"), "{reason}");
    assert!(!reason.contains("humans-review-intent"), "{reason}");
    // Settings that can't be loaded ask too, and so does a `clm` called by path or after
    // assignments.
    let (code, reason) = guard_in(nowhere.path(), report, &[("CLM_DEFECTS", "always")]);
    assert_eq!(code, Some(0));
    assert!(reason.contains("## CLM defects"), "{reason}");
    for command in [
        "/home/u/.claude/plugins/clm/bin/clm defect --title x <<'EOF'\nr\nEOF",
        "env X=1 clm defect --title x <<'EOF'\nr\nEOF",
        // asking through cargo run
        "cargo run -q -p clm -- defect --title \"x\" <<'EOF'\nThe gate misfired.\nEOF",
        "cargo run --bin clm defect --title x <<'EOF'\nr\nEOF",
        // whichever package it runs (`guard-asks-any-cargo-run`)
        "cargo run -p clm-web -- defect --title x <<'EOF'\nr\nEOF",
        "cargo run -- defect --title x <<'EOF'\nr\nEOF",
        "cargo r --manifest-path crates/clm/Cargo.toml defect --title x <<'EOF'\nr\nEOF",
    ] {
        let (code, reason) = guard_in(nowhere.path(), command, &[]);
        assert_eq!(code, Some(0), "{command}");
        assert!(reason.contains("## CLM defects"), "{command}: {reason}");
    }
    // backlog, and off, from the repository's settings
    let repo = clm_repo(Some("defects = \"backlog\"\n"));
    assert_eq!(guard_in(repo.path(), report, &[]), (Some(0), String::new()));
    let off = clm_repo(Some("defects = \"off\"\n"));
    assert_eq!(guard_in(off.path(), report, &[]), (Some(0), String::new()));
    let cargo = "cargo run -q -p clm -- defect --title \"x\" <<'EOF'\nreport\nEOF";
    assert_eq!(guard_in(repo.path(), cargo, &[]), (Some(0), String::new()));
    let unshown = "clm defect --title x < report.md";
    assert_eq!(
        guard_in(repo.path(), unshown, &[]),
        (Some(0), String::new())
    );
    // The command's own `CLM_DEFECTS` is the one `clm defect` reads.
    for (set, asks) in [
        ("ask", true),
        ("always", true),
        ("backlog", false),
        ("off", false),
    ] {
        let command = format!("CLM_DEFECTS={set} {report}");
        let (code, reason) = guard_in(repo.path(), &command, &[]);
        assert_eq!(code, Some(0), "{set}");
        assert_eq!(reason.contains("## CLM defects"), asks, "{set}: {reason}");
        let command = format!("env CLM_DEFECTS={set} {report}");
        let (_, reason) = guard_in(repo.path(), &command, &[]);
        assert_eq!(
            reason.contains("## CLM defects"),
            asks,
            "env {set}: {reason}"
        );
    }
    // With no settings at all, the command's own `off` stands for the setting too.
    let command = format!("CLM_DEFECTS=off {report}");
    assert_eq!(
        guard_in(nowhere.path(), &command, &[]),
        (Some(0), String::new())
    );
    // Other `clm` commands, and prose naming `clm defect`, aren't filing.
    for command in [
        "clm check",
        "echo run clm defect later",
        "clm defects",
        "cargo run -p clm -- defects",
        "cargo test -p clm -- defect",
        "cargo build -p clm defect",
    ] {
        assert_eq!(
            guard_in(nowhere.path(), command, &[]),
            (Some(0), String::new()),
            "{command}"
        );
    }
}

#[test]
fn guard_ignores_a_defect_report_body() {
    // req:plugin.hooks.guard/defect-approval@d7bcf2ca
    let nowhere = TempDir::new().unwrap();
    let quoting = "clm approve spec/x\ngit push origin main";
    // quoting a blocked command, under each form of the heredoc operator.
    for (open, close) in [
        ("<<'EOF'", "EOF"),
        ("<<\"EOF\"", "EOF"),
        ("<<EOF", "EOF"),
        ("<< EOF", "EOF"),
        ("<<-EOF", "\t\tEOF"),
        // A redirection's `&` before the operator doesn't end the segment.
        ("2>&1 <<'EOF'", "EOF"),
        ("&>log <<'EOF'", "EOF"),
    ] {
        let command = format!("clm defect --title \"x\" {open}\n{quoting}\n{close}");
        let (code, reason) = guard_in(nowhere.path(), &command, &[]);
        assert_eq!(code, Some(0), "{open}: {reason}");
        assert!(reason.contains("## CLM defects"), "{open}: {reason}");
        assert!(!reason.contains("humans-review-intent"), "{open}: {reason}");
        // With `defects = "backlog"` the body still matches no rule.
        let repo = clm_repo(Some("defects = \"backlog\"\n"));
        assert_eq!(
            guard_in(repo.path(), &command, &[]),
            (Some(0), String::new()),
            "{open}"
        );
    }
    // No heredoc after all: a quoted `<<`, `<<<`, or no terminator line.
    for command in [
        "clm defect --title \"a <<EOF\"\ngit push origin main\nEOF",
        "clm defect --title x <<<EOF\ngit push origin main\nEOF",
        "clm defect --title x <<EOF\ngit push origin main",
    ] {
        let (code, reason) = guard_in(nowhere.path(), command, &[]);
        assert_eq!(code, Some(2), "{command}: {reason}");
        assert!(reason.contains("guard-blocks-push-to-main"), "{reason}");
    }
    // Another command's heredoc isn't a report, but its body belongs to it, which isn't `gh`
    // or a shell, so it matches no rule either (`guard-heredocs`).
    let (code, _) = guard_in(
        nowhere.path(),
        "clm check <<EOF\ngit push origin main\nEOF",
        &[],
    );
    assert_eq!(code, Some(0));
}

#[test]
fn guard_block_wins_over_a_defect_ask() {
    // req:plugin.hooks.guard/defect-approval@d7bcf2ca
    let nowhere = TempDir::new().unwrap();
    // block wins: the push guard's block wins over the defect's ask.
    let (code, reason) = guard_in(
        nowhere.path(),
        "clm defect --title x <<'EOF'\nreport\nEOF\ngit push origin main",
        &[],
    );
    assert_eq!(code, Some(2), "{reason}");
    assert!(reason.contains("guard-blocks-push-to-main"), "{reason}");
    // Two asks make one decision giving both reasons.
    let (code, reason) = guard_in(
        nowhere.path(),
        "clm defect --title x <<'EOF'\nreport\nEOF\nclm approve spec/x",
        &[],
    );
    assert_eq!(code, Some(0));
    assert!(
        reason.contains("## CLM defects") && reason.contains("humans-review-intent"),
        "{reason}"
    );
}

#[test]
fn guard_quotes_the_defect_report_it_asks_about() {
    // req:plugin.hooks.guard/defect-approval@d7bcf2ca
    let nowhere = TempDir::new().unwrap();
    let command = "clm defect --title \"x\" --component gate <<'EOF'\nThe gate misfired.\nEOF";
    let (code, reason) = guard_in(nowhere.path(), command, &[]);
    assert_eq!(code, Some(0), "{reason}");
    assert!(reason.contains("## CLM defects"), "{reason}");
    assert!(
        reason.contains("clm defect --title \"x\" --component gate"),
        "{reason}"
    );
    assert!(reason.contains("The gate misfired."), "{reason}");
    assert!(reason.contains("pull request body"), "{reason}");
    assert!(!reason.contains("page"), "{reason}");
    // An unquoted heredoc with nothing to expand is sent as written.
    let (code, reason) = guard_in(
        nowhere.path(),
        "clm defect --title x <<EOF\nplain words\nEOF",
        &[],
    );
    assert_eq!(code, Some(0), "{reason}");
    assert!(reason.contains("plain words"), "{reason}");
    // A cargo run's report is quoted, a multi-line body whole, and the body once.
    let (_, reason) = guard_in(
        nowhere.path(),
        "cargo run -q -p clm -- defect --title x <<'EOF'\nThe gate misfired.\nsecond line\nEOF",
        &[],
    );
    assert!(
        reason.contains("The gate misfired.\nsecond line"),
        "{reason}"
    );
    assert_eq!(reason.matches("The gate misfired.").count(), 1, "{reason}");
    // Each segment's report is quoted, in order.
    let (_, reason) = guard_in(
        nowhere.path(),
        "clm defect --title a <<'EOF'\nfirst\nEOF\nclm defect --title b <<'EOF'\nsecond\nEOF",
        &[],
    );
    let (a, b) = (reason.find("first"), reason.find("second"));
    assert!(a.is_some() && b.is_some() && a < b, "{reason}");
}

#[test]
fn guard_refuses_a_defect_report_it_cannot_show() {
    // req:plugin.hooks.guard/defect-approval@d7bcf2ca
    let nowhere = TempDir::new().unwrap();
    for command in [
        "clm defect --title x < report.md",
        "cat report.md | clm defect --title x",
        "clm defect --title x <<< \"$R\"",
        "clm defect --title x",
        "clm defect --title x <<EOF\ncost $HOME\nEOF",
        "clm defect --title x <<EOF\nno end",
    ] {
        let (code, reason) = guard_in(nowhere.path(), command, &[]);
        assert_eq!(code, Some(2), "{command}: {reason}");
        assert!(reason.contains("defect-ask-shows-report"), "{reason}");
        assert!(reason.contains("<<'EOF'"), "{reason}");
    }
    // A refusal wins over any ask, and one unshown report refuses the lot.
    let (code, reason) = guard_in(
        nowhere.path(),
        "clm defect --title a <<'EOF'\nr\nEOF\nclm defect --title b\nclm approve spec/x",
        &[],
    );
    assert_eq!(code, Some(2), "{reason}");
    // Help files nothing, and neither does a segment under `backlog`.
    for command in [
        "clm defect --help",
        "clm defect -h",
        "cargo run -- defect -h",
    ] {
        assert_eq!(
            guard_in(nowhere.path(), command, &[]),
            (Some(0), String::new()),
            "{command}"
        );
    }
    let repo = clm_repo(Some("defects = \"backlog\"\n"));
    assert_eq!(
        guard_in(repo.path(), "clm defect --title x < report.md", &[]),
        (Some(0), String::new())
    );
}
