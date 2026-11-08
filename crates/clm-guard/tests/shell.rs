//! The guard reads a command as the shell would: quotes, prefixes, scripts, heredocs and
//! the checkout each piece runs in.

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

/// A fresh repository at `dir` on `branch`, under CLM (it holds `specs/SPEC.md`).
fn checkout_on(dir: &Path, branch: &str) {
    fs::create_dir_all(dir.join("specs")).unwrap();
    fs::write(dir.join("specs/SPEC.md"), "").unwrap();
    git(dir, &["init", "-q", "-b", branch]);
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
fn asks(event: &str) -> String {
    match guard(event) {
        Decision::Ask(why) => why,
        other => panic!("{event}: expected ask, got {other:?}"),
    }
}

/// A checkout on `main` with a worktree-like checkout `w` under it on `impl/x/b`.
fn main_with_w() -> TempDir {
    let dir = TempDir::new().unwrap();
    checkout_on(dir.path(), "main");
    checkout_on(&dir.path().join("w"), "impl/x/b");
    dir
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
fn quotes_hold_separators_and_lines() {
    // req:plugin.hooks.guard.shell/guard-quotes@49fd6b7e
    // separators inside a message
    for command in [
        "git commit -m \"build; git push origin main is forbidden\"",
        "git commit -m \"first line\ngit push origin main\nlast line\"",
        "git commit -m 'a && git push origin main | x & y ( z ) `w`'",
        "git commit -m \"say \\\"hi; git push origin main\\\"\"",
    ] {
        allows(&bash(command));
    }
    // A backslash outside quotes escapes a separator.
    allows(&bash("echo a\\; git push origin main"));
}

#[test]
fn quotes_and_heredocs_in_comments_count_for_nothing() {
    // req:plugin.hooks.guard.shell/guard-quotes@49fd6b7e
    for command in [
        "# it's here\ngit push origin main\n# that's all",
        "# use <<EOF style\ngit push origin main\nEOF",
        "true # don't\ngit push origin main # it's",
    ] {
        blocks_push(&bash(command));
    }
    // A `#` inside a word isn't a comment.
    blocks_push(&bash("echo a#'b\ngit push origin main"));
}

#[test]
fn quotes_keep_a_directory_with_a_space() {
    // req:plugin.hooks.guard.shell/guard-quotes@49fd6b7e
    // a quoted directory with a space
    let dir = TempDir::new().unwrap();
    checkout_on(dir.path(), "impl/x/b");
    checkout_on(&dir.path().join("my main"), "main");
    blocks_push(&bash_in("git -C \"my main\" push", dir.path()));
    blocks_push(&bash_in("git -C 'my main' push", dir.path()));
    blocks_push(&bash_in("git -C my\\ main push", dir.path()));
}

#[test]
fn quotes_that_do_not_balance_are_ignored() {
    // req:plugin.hooks.guard.shell/guard-quotes@49fd6b7e
    // unbalanced quotes
    blocks_push(&bash("echo don't; git push origin main"));
    blocks_push(&bash("echo \"a; git push origin 'main"));
    // A body the scan found still belongs to its segment.
    allows(&bash(
        "cat > f <<'EOF'\ngit push origin main\nEOF\necho \"oops",
    ));
}

#[test]
fn quotes_in_a_heredoc_body_do_not_count() {
    // req:plugin.hooks.guard.shell/guard-quotes@49fd6b7e
    // an apostrophe in a heredoc body
    allows(&bash(
        "cat > n.md <<'EOF'\ndon't\nEOF\ngit commit -m \"a; git push origin main\"",
    ));
}

#[test]
fn quotes_inside_quotes_stay() {
    // req:plugin.hooks.guard.shell/guard-quotes@49fd6b7e
    // a quote inside a quote
    allows(&bash("bash -c 'git commit -m \"a; git push origin main\"'"));
}

#[test]
fn ansi_c_quotes() {
    // req:plugin.hooks.guard.shell/guard-quotes@49fd6b7e
    // ANSI-C quotes
    blocks_push(&bash("echo $'\\''; git push origin main; echo \\'"));
    allows(&bash("git commit -m $'a\\'; git push origin main'"));
    allows(&bash("git commit -m $'a\\\\'; git push origin impl/x/b"));
    // Its quoting backslashes go, others stay.
    let dir = TempDir::new().unwrap();
    checkout_on(dir.path(), "impl/x/b");
    checkout_on(&dir.path().join("it's main"), "main");
    blocks_push(&bash_in("git -C $'it\\'s main' push", dir.path()));
    allows(&bash("git push origin $'mai\\n'"));
    allows(&bash("git push origin $'ma\\\\in'"));
}

#[test]
fn a_heredoc_in_a_quoted_substitution() {
    // req:plugin.hooks.guard.shell/guard-quotes@49fd6b7e
    // req:plugin.hooks.guard.shell/guard-heredocs@63eaa78e
    // a heredoc in a quoted substitution
    let commit = "git commit -m \"$(cat <<'EOF'\nSay \"a; git push origin main\" here.\nEOF\n)\"";
    allows(&bash(commit));
    let mutation =
        "gh api graphql -f query=\"$(cat <<'EOF'\nmutation { mergePullRequest }\nEOF\n)\"";
    blocks_push(&bash(mutation));
    // A body's quotes count for nothing, whatever lines they would split.
    allows(&bash(
        "git commit -m \"$(cat <<'EOF'\nx\"\ngit push origin main\n\"\nEOF\n)\"",
    ));
    allows(&bash(
        "git commit -m \"$(cat <<'EOF'\ndon't\ngit push origin main\nEOF\n)\"",
    ));
    // A quote inside the substitution opens its own.
    allows(&bash(
        "git commit -m \"$(echo \"a; git push origin main\")\"",
    ));
    // Its heredoc is the substitution's, not the body of an outer shell's input.
    allows(&bash(
        "bash \"$(cat <<'EOF'\ngit push origin main\nEOF\n)\"",
    ));
    // When other quotes don't balance, its body still isn't split into commands.
    allows(&bash(
        "git commit -m \"$(cat <<'EOF'\ngit push origin main\nEOF\n)\" && echo don't",
    ));
    // Commands after the quoted substitution are still judged.
    blocks_push(&bash(&format!("{commit} && git push origin main")));
    // A substitution that never closes leaves the quotes unbalanced.
    blocks_push(&bash("echo \"$(echo x; git push origin main"));
}

#[test]
fn redirections_separate_nothing() {
    // req:plugin.hooks.guard.shell/guard-quotes@49fd6b7e
    let dir = main_with_w();
    for command in [
        "cd w &>/dev/null && git push",
        "cd w 2>&1 && git push",
        "cd w <&0 && git push",
        "cd w >&2 && git push",
    ] {
        allows(&bash_in(command, dir.path()));
    }
}

#[test]
fn wrapped_pushes_are_judged() {
    // req:plugin.hooks.guard.shell/guard-command-position@381136c0
    // wrapped pushes
    for command in [
        "(git push origin main)",
        "{ git push origin main; }",
        "if true; then git push origin main; fi",
        "time git push origin main",
        "time -p git push origin main",
        "sudo -u me git push origin main",
        "command git push origin main",
        "nohup git push origin main &",
        "xargs -n 1 git push origin main",
        "out=$(git push origin main)",
        "x=`git push origin main`",
        "! git push origin main",
        "while true; do git push origin main; done",
        "timeout -s KILL 60 git push origin main",
        "nice -n 5 ionice -c 3 stdbuf -o L git push origin main",
        "exec -a x doas -u root git push origin main",
        "env -u HOME X=1 git push origin main",
        "if false; then :; elif true; then git push origin main; fi",
        "if false; then :; else git push origin main; fi",
        "until git push origin main; do :; done",
        "timeout --signal KILL --kill-after 5 60 git push origin main",
    ] {
        blocks_push(&bash(command));
    }
    // wrapped approval
    for command in [
        "(clm approve spec/x)",
        "if true; then clm approve spec/x; fi",
    ] {
        let why = asks(&bash(command));
        assert!(why.contains("humans-review-intent"), "{command}: {why}");
    }
    // wrapped pushes elsewhere
    for command in [
        "(git push origin impl/x/b)",
        "sudo git push -u origin impl/x/b",
    ] {
        allows(&bash(command));
    }
}

#[test]
fn clustered_options_and_split_strings() {
    // req:plugin.hooks.guard.shell/guard-command-position@381136c0
    // clustered options and split strings
    for command in [
        "sudo -Eu root git push origin main",
        "sudo -uroot git push origin main",
        "env -S 'git push origin main'",
        "env --split-string='git push origin main'",
        "env -iS 'git push origin main'",
        "timeout -vk 5 60 git push origin main",
        "env -S'git push origin main'",
        "env --split-string 'git push origin main'",
    ] {
        blocks_push(&bash(command));
    }
    blocks_push(&bash("env -S 'gh pr merge 3'"));
    let why = blocks(&bash("env -S 'gh release create v1'"));
    assert!(why.contains("humans-cut-releases"), "{why}");
    let why = asks(&bash("env -S 'clm approve spec/x'"));
    assert!(why.contains("humans-review-intent"), "{why}");
    allows(&bash("env -S 'git push origin impl/x/b'"));
    // However many split strings hold one another.
    let deep = format!("env -S '{}git push origin main'", "-S".repeat(40));
    blocks_push(&bash(&deep));
    allows(&bash("sudo -Eu root git push origin impl/x/b"));
}

#[test]
fn cd_in_a_subshell_stays_there() {
    // req:plugin.hooks.guard.shell/guard-command-position@381136c0
    let dir = main_with_w();
    // cd in a subshell
    allows(&bash_in("(cd w && git push)", dir.path()));
    blocks_push(&bash_in("(cd w); git push", dir.path()));
    // cd in a substitution
    blocks_push(&bash_in("x=`cd w`; git push", dir.path()));
    blocks_push(&bash_in("x=$(cd w); git push", dir.path()));
    // A `)` with no `(`, as in a `case` pattern, ends no scope.
    allows(&bash_in("case x in x) cd w;; esac; git push", dir.path()));
}

#[test]
fn prefixes_that_change_directory() {
    // req:plugin.hooks.guard.shell/guard-command-position@381136c0
    // a prefix that changes directory
    let dir = TempDir::new().unwrap();
    checkout_on(dir.path(), "impl/x/b");
    checkout_on(&dir.path().join("m"), "main");
    for command in [
        "env -C m git push",
        "env -iC m git push",
        "env -iCm git push",
        "sudo -ED m git push",
        "env --chdir m git push",
        "sudo -D m git push",
        "sudo --chdir m git push",
        "env --chdir=m git push",
        "timeout 60 env GIT_DIR=m/.git git push",
        // The last assignment of a name wins.
        "GIT_DIR=nowhere env GIT_DIR=m/.git git push",
    ] {
        blocks_push(&bash_in(command, dir.path()));
    }
    allows(&bash_in(
        "env -C m git push origin HEAD:impl/x/b",
        dir.path(),
    ));
}

#[test]
fn shell_scripts_are_judged() {
    // req:plugin.hooks.guard.shell/guard-shell-scripts@cd1e6da3
    // a push in a script
    for command in [
        "bash -c 'git push origin main'",
        "sh -ec \"cd . && git push origin HEAD:main\"",
        "/bin/bash -c \"git status; git push --mirror origin\"",
    ] {
        blocks_push(&bash(command));
    }
    // an approval in a script
    let why = asks(&bash("bash -c 'clm approve spec/x'"));
    assert!(why.contains("humans-review-intent"), "{why}");
    // a harmless script
    for command in [
        "bash -c 'git push origin impl/x/b'",
        "sh -c \"echo git push origin main\"",
    ] {
        allows(&bash(command));
    }
}

#[test]
fn a_script_runs_from_the_segment_checkout() {
    // req:plugin.hooks.guard.shell/guard-shell-scripts@cd1e6da3
    let dir = main_with_w();
    allows(&bash_in("cd w && bash -c 'git push'", dir.path()));
    blocks_push(&bash_in("bash -c 'cd w' && git push", dir.path()));
}

/// `command`, a line `git push origin main` and `EOF`.
fn with_push_body(command: &str) -> String {
    format!("{command}\ngit push origin main\nEOF")
}

#[test]
fn a_body_piped_into_a_shell() {
    // req:plugin.hooks.guard.shell/guard-heredocs@63eaa78e
    // a body piped into a shell
    for command in [
        "cat <<'EOF' | bash",
        "cat <<'EOF' | tee log | sh",
        "cat <<'EOF' |& sh",
        "cat <<'EOF' | env -S 'bash -e'",
    ] {
        blocks_push(&bash(&with_push_body(command)));
    }
}

#[test]
fn a_body_piped_elsewhere() {
    // req:plugin.hooks.guard.shell/guard-heredocs@63eaa78e
    // a body piped elsewhere
    for command in [
        "cat <<'EOF' | grep x",
        "cat <<'EOF' | bash -c 'cat > f'",
        "cat <<'EOF' | gh api graphql -F query=@-",
        "cat <<'EOF'; bash",
    ] {
        allows(&bash(&with_push_body(command)));
    }
}

#[test]
fn wrapped_shells_run_their_body() {
    // req:plugin.hooks.guard.shell/guard-heredocs@63eaa78e
    // wrapped shells
    for command in [
        "sudo bash <<'EOF'",
        "sudo -u root bash <<'EOF'",
        "sudo -Eu root bash <<'EOF'",
        "exec sh <<'EOF'",
        "nohup bash <<'EOF'",
        "xargs -0 sh <<'EOF'",
        "source /dev/stdin <<'EOF'",
        ". /dev/fd/0 <<'EOF'",
        ". /proc/self/fd/0 <<'EOF'",
        "bash --norc -e <<'EOF'",
    ] {
        blocks_push(&bash(&with_push_body(command)));
    }
}

#[test]
fn a_shell_running_its_argument() {
    // req:plugin.hooks.guard.shell/guard-heredocs@63eaa78e
    // a shell running its argument
    for command in ["bash -ec 'cat > f' <<'EOF'", "sh -xc 'cat > f' <<'EOF'"] {
        allows(&bash(&with_push_body(command)));
    }
    allows(&bash(
        "cat <<'A' | bash <<'B'\ngit push origin main\nA\necho hi\nB",
    ));
    allows(&bash(&with_push_body("source ./x.sh <<'EOF'")));
}

#[test]
fn a_cd_in_a_body_stays_in_its_shell() {
    // req:plugin.hooks.guard.shell/guard-heredocs@63eaa78e
    let dir = main_with_w();
    allows(&bash_in(
        "cd w && cat <<'EOF' | bash\ngit push\nEOF",
        dir.path(),
    ));
    blocks_push(&bash_in(
        "cat <<'EOF' | bash\ncd w\nEOF\ngit push",
        dir.path(),
    ));
    // A piped body runs from the shell's checkout, not the segment that holds it.
    allows(&bash_in(
        "cat <<'EOF' | env -C w bash\ngit push\nEOF",
        dir.path(),
    ));
}

#[test]
fn cd_that_does_not_move_the_push() {
    // req:plugin.hooks.guard.shell/guard-checkout@b692c0b1
    // cd that doesn't move the push
    let dir = main_with_w();
    for command in [
        "cd w | git push",
        "cd w & git push",
        "cd w || git push",
        "cd w |& git push",
    ] {
        blocks_push(&bash_in(command, dir.path()));
    }
    // cd with a redirection
    allows(&bash_in("cd w 2>&1 && git push", dir.path()));
    allows(&bash_in("cd w\ngit push", dir.path()));
}

#[test]
fn a_push_in_a_quoted_substitution() {
    // req:plugin.hooks.guard.shell/guard-quoted-substitutions@2be088d6
    // a push in a quoted substitution
    for command in [
        "echo \"$(git push origin main)\"",
        "x=\"out: $(cd . && git push origin HEAD:main)\"",
        "echo \"a $(echo \"$(git push origin main)\")\"",
    ] {
        blocks_push(&bash(command));
    }
    // an approval in a quoted substitution
    let why = asks(&bash("echo \"$(clm approve spec/x)\""));
    assert!(why.contains("humans-review-intent"), "{why}");
    // prose in a quoted substitution
    for command in [
        "git commit -m \"$(printf 'a; git push origin main')\"",
        "gh pr create --title t --body \"$(cat <<'EOF'\nclm approve spec/x\nEOF\n)\"",
    ] {
        allows(&bash(command));
    }
}

#[test]
fn a_quoted_substitution_runs_before_its_command_moves() {
    // req:plugin.hooks.guard.shell/guard-quoted-substitutions@2be088d6
    let dir = main_with_w();
    // From the checkout the earlier `cd`s leave, before the segment's own `-C`.
    allows(&bash_in("cd w && echo \"$(git push)\"", dir.path()));
    blocks_push(&bash_in("git -C w log \"$(git push)\"", dir.path()));
    blocks_push(&bash_in("env -C w echo \"$(git push)\"", dir.path()));
    // Its `cd` stays inside it.
    blocks_push(&bash_in("echo \"$(cd w)\"; git push", dir.path()));
}

#[test]
fn here_strings_fed_to_a_shell() {
    // req:plugin.hooks.guard.shell/guard-here-strings@52c32582
    // a push fed to a shell
    for command in [
        "bash <<< 'git push origin main'",
        "sh <<<\"git push origin HEAD:main\"",
        "sudo bash <<< 'cd . && git push --mirror origin'",
        "bash <<< 'git push origin main",
    ] {
        blocks_push(&bash(command));
    }
    // an approval fed to a shell
    let why = asks(&bash("sh <<< \"clm approve spec/x\""));
    assert!(why.contains("humans-review-intent"), "{why}");
    // a here-string elsewhere
    for command in [
        "grep x <<< 'git push origin main'",
        "bash -c 'cat > f' <<< 'git push origin main'",
        "gh api graphql -F query=@- <<< 'clm approve spec/x'",
    ] {
        allows(&bash(command));
    }
    // From the shell's checkout.
    let dir = main_with_w();
    allows(&bash_in("cd w && bash <<< 'git push'", dir.path()));
    blocks_push(&bash_in("bash <<< 'git push'", dir.path()));
}

#[test]
fn guard_follows_cd_into_a_worktree() {
    // req:plugin.hooks.guard.shell/guard-checkout@b692c0b1
    // cd into a worktree
    let dir = TempDir::new().unwrap();
    checkout_on(dir.path(), "main");
    checkout_on(&dir.path().join("w"), "impl/x/b");
    for command in ["cd w && git commit -m m && git push", "cd w; git push"] {
        assert_eq!(
            guard(&bash_in_opt(command, Some(dir.path()))),
            Decision::Allow,
            "{command}"
        );
    }
    // A `cd` into a directory that doesn't exist fails, and the shell may push from where it
    // was (`;`), so the checkout stays put.
    for command in [
        "cd nowhere; git push",
        "cd nowhere && git push",
        "cd ../ma* && git push",
    ] {
        blocks_push(&bash_in_opt(command, Some(dir.path())));
    }
    assert_eq!(
        guard(&bash_in_opt(
            "cd w && cd nowhere; git push",
            Some(dir.path())
        )),
        Decision::Allow
    );
    // A `cd` inside a body a shell runs stays in that shell.
    blocks_push(&bash_in_opt(
        "bash <<'EOF'\ncd w\nEOF\ngit push",
        Some(dir.path()),
    ));
    assert_eq!(
        guard(&bash_in_opt(
            "cd w && bash <<'EOF'\ngit push\nEOF",
            Some(dir.path())
        )),
        Decision::Allow
    );
}

#[test]
fn guard_follows_cd_into_main() {
    // req:plugin.hooks.guard.shell/guard-checkout@b692c0b1
    // cd into main
    let dir = TempDir::new().unwrap();
    checkout_on(dir.path(), "impl/x/b");
    checkout_on(&dir.path().join("m"), "main");
    blocks_push(&bash_in_opt("cd m && git push", Some(dir.path())));
}

#[test]
fn guard_resets_on_a_cd_it_cannot_follow() {
    // req:plugin.hooks.guard.shell/guard-checkout@b692c0b1
    // cd it can't follow
    let dir = TempDir::new().unwrap();
    checkout_on(dir.path(), "main");
    checkout_on(&dir.path().join("w"), "impl/x/b");
    for command in [
        "cd $W && git push",
        "cd ~/w && git push",
        "cd w && cd $X && git push",
        "cd w && cd - && git push",
        "cd w && cd && git push",
        "cd w && cd `pwd` && git push",
    ] {
        blocks_push(&bash_in_opt(command, Some(dir.path())));
    }
}

#[test]
fn guard_keeps_a_cd_it_could_not_follow_until_an_absolute_one() {
    // req:plugin.hooks.guard.shell/guard-checkout@b692c0b1
    // an absolute cd after one it can't follow
    let dir = TempDir::new().unwrap();
    checkout_on(dir.path(), "impl/x/b");
    checkout_on(&dir.path().join("m"), "main");
    let absolute = dir.path().join("m");
    let why = blocks_push(&bash_in_opt(
        &format!("cd gone; cd {} && git push", absolute.display()),
        Some(dir.path()),
    ));
    assert!(!why.contains("cd gone"), "{why}");
    let why = blocks_push(&bash_in_opt("cd gone; cd m && git push", Some(dir.path())));
    assert!(why.contains("cd gone"), "{why}");
}

#[test]
fn guard_ignores_guard_text_written_to_a_file() {
    // req:plugin.hooks.guard.shell/guard-heredocs@63eaa78e
    // writing guard text to a file
    for command in [
        "cat > d.md <<'EOF'\n- GIVEN the command clm approve spec/x\nclm approve spec/x\ngit push origin main\nEOF",
        "cat > d.md <<'EOF'\ncargo run -q -p clm -- approve spec/x\ngh release create v1\nEOF",
    ] {
        assert_eq!(guard(&bash(command)), Decision::Allow, "{command}");
    }
    // `<<\EOF` quotes the word too.
    assert_eq!(
        guard(&bash("cat > d.md <<\\EOF\ngit push origin main\nEOF")),
        Decision::Allow
    );
}

#[test]
fn guard_judges_no_command_in_a_gh_body() {
    // req:plugin.hooks.guard.shell/guard-heredocs@63eaa78e
    // a body sent to gh: it can't make a gh call, cut a release or approve of its own.
    for command in [
        "gh pr comment 3 --body-file - <<'EOF'\nclm approve spec/x\ngh release create v1\ngit push origin main\nEOF",
        "gh api -X POST repos/o/r/issues/3/labels --input - <<'EOF'\n{\"labels\":[\"spec-approved\"]}\nEOF",
    ] {
        assert_eq!(guard(&bash(command)), Decision::Allow, "{command}");
    }
}

#[test]
fn guard_judges_a_body_run_by_a_shell() {
    // req:plugin.hooks.guard.shell/guard-heredocs@63eaa78e
    // body run by a shell
    for command in [
        "bash <<'EOF'\nclm approve spec/x\nEOF",
        "/bin/sh <<EOF\nclm approve spec/x\nEOF",
        "env FOO=1 /usr/bin/bash <<EOF\nclm approve spec/x\nEOF",
    ] {
        let why = asks_cmd(command);
        assert!(why.contains("humans-review-intent"), "{command}: {why}");
    }
    blocks_push(&bash("bash <<'EOF'\ngit push origin main\nEOF"));
    // With `-c` the shell runs its argument, not the body.
    assert_eq!(
        guard(&bash("bash -c 'x' <<EOF\ngit push origin main\nEOF")),
        Decision::Allow
    );
}

#[test]
fn guard_judges_commands_after_the_body() {
    // req:plugin.hooks.guard.shell/guard-heredocs@63eaa78e
    // commands after the body
    blocks_push(&bash("cat > f <<'EOF'\ntext\nEOF\ngit push origin main"));
    // `<<-` ends at a tab-indented terminator.
    blocks_push(&bash("cat > f <<-EOF\ntext\n\t\tEOF\ngit push origin main"));
    assert_eq!(
        guard(&bash("cat > f <<-EOF\ngit push origin main\n\tEOF")),
        Decision::Allow
    );
}

#[test]
fn guard_finds_no_heredoc_after_all() {
    // req:plugin.hooks.guard.shell/guard-heredocs@63eaa78e
    // no heredoc after all
    for command in [
        "echo \"<<X\"\ngit push origin main",
        // A quoted operator opens nothing, even with its terminator line.
        "echo \"a <<X b\"\ngit push origin main\nX",
        "cat <<EOF\ngit push origin main",
        // `<<<` is a here-string.
        "cat <<<EOF\ngit push origin main\nEOF",
    ] {
        blocks_push(&bash(command));
    }
}

#[test]
fn guard_takes_several_heredocs_in_order() {
    // req:plugin.hooks.guard.shell/guard-heredocs@63eaa78e
    // The first body is `cat`'s, the second the shell's.
    blocks_push(&bash(
        "cat <<A | bash <<B\nclm approve spec/x\nA\ngit push origin main\nB",
    ));
    let why = asks_cmd("bash <<A | cat <<B\nclm approve spec/x\nA\ngit push origin main\nB");
    assert!(why.contains("humans-review-intent"), "{why}");
}
