//! The guard's rules: which commands it asks the human about or blocks, judged in process.

use std::fs;
use std::path::Path;
use std::process::Command;

use clm_guard::{Decision, guard};
use tempfile::TempDir;

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
    // req:plugin.hooks.guard/push-guard@61eecef6
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
    // req:plugin.hooks.guard/push-guard@61eecef6
    let dir = TempDir::new().unwrap();
    let work = tracking(&dir.path().join("a"), "impl/x/b", Some("main"));
    // A setting outside `push.`, `remote.` and `branch.` isn't passed to git: one that
    // would make every query fail can't hide the push.
    blocks_push(&bash_in(
        "git -c push.default=upstream -c core.bare=maybe push",
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
    // req:plugin.hooks.guard/push-guard@61eecef6
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
    // req:plugin.hooks.guard/push-guard@61eecef6
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
    // req:plugin.hooks.guard/push-guard@61eecef6
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
    // req:plugin.hooks.guard/push-guard@61eecef6
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
    // req:plugin.hooks.guard/approve-guard@0d27c2be
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
    // req:plugin.hooks.guard/approve-guard@0d27c2be
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
    // req:plugin.hooks.guard/approve-guard@0d27c2be
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
    // req:plugin.hooks.guard/approve-guard@0d27c2be
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
    // req:plugin.hooks.guard/approve-guard@0d27c2be
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
    // req:plugin.hooks.guard/approve-guard@0d27c2be
    // approval combined with a push to main
    let why = blocks(&bash("clm approve spec/x && git push origin main"));
    assert!(why.contains("guard-blocks-push-to-main"), "{why}");
    let why = blocks(&bash("clm approve spec/x; gh release create v1"));
    assert!(why.contains("humans-cut-releases"), "{why}");
}

#[test]
fn guard_allows_every_other_event() {
    // req:plugin.hooks.guard/approve-guard@0d27c2be
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
    // req:plugin.hooks.guard/release-guard@e9278dcb
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
    // req:plugin.hooks.guard/release-guard@e9278dcb
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
    ] {
        assert_eq!(guard(&bash(command)), Decision::Allow, "{command}");
    }
}

#[test]
fn guard_blocks_pushing_to_main() {
    // req:plugin.hooks.guard/push-guard@61eecef6
    blocks_push(&bash("git push origin HEAD:main"));
}

#[test]
fn guard_blocks_other_ways_to_reach_main() {
    // req:plugin.hooks.guard/push-guard@61eecef6
    for command in [
        "git push --force origin main",
        "git push origin +main",
        "git push origin impl/x/b:refs/heads/main",
        "git -C ../w push origin --delete main",
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
    // req:plugin.hooks.guard/push-guard@61eecef6
    let dir = TempDir::new().unwrap();
    checkout_on(dir.path(), "main");
    for command in ["git push", "git push origin HEAD", "git push -u origin"] {
        blocks_push(&bash_in_opt(command, Some(dir.path())));
    }
}

#[test]
fn guard_allows_pushing_other_branches() {
    // req:plugin.hooks.guard/push-guard@61eecef6
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
    // req:plugin.hooks.guard/push-guard@61eecef6
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
}

#[test]
fn guard_suggests_an_explicit_branch_for_an_implicit_push() {
    // req:plugin.hooks.guard/push-guard@61eecef6
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
    // req:plugin.hooks.guard/push-guard@61eecef6
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
}

#[test]
fn guard_blocks_matching_and_wildcard_refspecs_that_reach_main() {
    // req:plugin.hooks.guard/push-guard@61eecef6
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
    // req:plugin.hooks.guard/push-guard@61eecef6
    let dir = TempDir::new().unwrap();
    assert_eq!(
        guard(&bash_in_opt("git push", None)),
        Decision::Allow,
        "no cwd"
    );
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
    // req:plugin.hooks.guard/push-guard@61eecef6
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
    // req:plugin.hooks.guard/push-guard@61eecef6
    let dir = TempDir::new().unwrap();
    let work = tracking(dir.path(), "impl/x/b", Some("impl/x/b"));
    scratch_on(&work.join("s"), "main");
    allows(&bash_in("cd s && git push -u origin main", &work));
    allows(&bash_in("git -C s push", &work));
    blocks_push(&bash_in("git push origin main", &work));
}

#[test]
fn a_clm_repository_in_a_subdirectory() {
    // req:plugin.hooks.guard/push-guard@61eecef6
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
}

#[test]
fn a_push_whose_repository_the_guard_cannot_tell() {
    // req:plugin.hooks.guard/push-guard@61eecef6
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
        "git -c core.bare=false push origin main",
        "git -C missing push origin main",
        "cd nowhere && git push origin main",
    ] {
        blocks_push(&bash_in(command, &work));
    }
    // No `cwd`, and a `cwd` in no git repository.
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
fn the_hosts_routes_outside_clm() {
    // req:plugin.hooks.guard/push-guard@61eecef6
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
    // req:plugin.hooks.guard/push-guard@61eecef6
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
    // req:plugin.hooks.guard/push-guard@61eecef6
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
    // req:plugin.hooks.guard/approve-guard@0d27c2be
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
    // req:plugin.hooks.guard/approve-guard@0d27c2be
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
