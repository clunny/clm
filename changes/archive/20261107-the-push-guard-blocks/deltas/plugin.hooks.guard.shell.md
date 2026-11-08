---
component: plugin.hooks.guard.shell
base: { design: "8eb4e6a838bca4a4" }
---

## MODIFIED Design
Module `shell` of crate `clm-guard`, at `crates/clm-guard/src/shell/` (`guard-split`): `scan.rs` (segments, tokens, heredoc bodies, here-strings and quoted substitutions), `command.rs` (the command position, `env -S`, a shell's `-c` script, whether a segment's command reads standard input) and `walk.rs` (the pieces to judge, in order, each with its checkout), declared by `shell/mod.rs`. The module exports to the crate (`pub(crate)`) the walk, the piece it yields (its text, its tokens, its checkout and the `cd` kept for it: `Judged`) and `command`'s parsed `Command`, which the rules read. The guard's `lib.rs` declares the module and calls the walk once per event; nothing outside `clm-guard` sees it. It runs no process, and reads nothing outside the command but the file system, to tell whether a `cd`'s directory exists. Its tests, `crates/clm-guard/tests/shell.rs`, call `clm_guard::guard` on `PreToolUse` events as `clm hook guard` does and cite only this component's requirements: a scenario reads a rule's answer (a push to `main` blocked, an approval asked about) from the guard. Every git repository and worktree these tests make holds `specs/SPEC.md`, so it is under CLM and `push-guard` judges a push from it (`push-guard-scoped-to-clm`); `guard-checkout`'s build makes the tests' one helper that creates a checkout write that file. The scenarios of `guard-standard-input` read the walk's pieces directly, so they are tested in unit tests inside `walk.rs`, which lies in this component's `paths`.

The guard (`guard-matches-writes`, `guard-reads-quotes-and-wrappers`) scans the command once, finding quotes and heredocs in the same scan (`guard-quotes`), and each segment keeps the separator that ended it: `&&`, `||`, `|`, `|&`, `&`, `;` or a newline, the command's end counting as a newline. Splitting at `(`, `)` and the backtick makes a subshell, a group or a command substitution (`(git push ...)`, `out=$(git push ...)`) segments of its own. A `$( )` inside double quotes stays part of its quoted token, but its text is also judged as commands (`guard-quoted-substitutions`), so `git commit -m "$(cat <<'EOF' ... EOF)"`, the usual way a session writes a commit message, keeps its body's prose out of the segments however many `"` it holds, and `echo "$(git push origin main)"` is judged as the push it runs. The substitution's text stays in the outer token as written, so a rule that reads that token, such as a GraphQL mutation's name in `-f query="$(cat <<'EOF' ... EOF)"`, sees it. A `)` with no `(` inside the substitution, as in a `case` pattern, ends it early, and a backtick inside double quotes stays part of its quoted token: both are gaps.

Several heredocs on one line take their bodies in order. A body judged as commands (`guard-heredocs`) is split as any command is, so its own heredocs and pipes follow the same rules; it starts at the checkout of the segment whose shell runs it (for a piped body, the shell's segment), and a `cd` inside it moves only the body's later segments, never the command's. A here-string fed to a shell over its standard input, as in `bash <<< 'git push origin main'`, is judged as a body is (`guard-here-strings`); fed to any other command it stays a redirection's target, so `cat <<< '...' | bash`, like `echo '...' | bash`, is a gap: the guard judges only heredoc bodies through a pipe.

A segment's *command* is found by `command.rs` (`guard-command-position`); every rule that speaks of a segment's first token means that command. A shell run with `-c` has its script, one token, split into segments and judged from the segment's checkout (`guard-shell-scripts`), as a heredoc body fed to a shell is.

**Checkouts** (`guard-checkout`). A `cd` ended by `|`, `|&` or `&` runs in a subshell or in the background, and one ended by `||` lets the next segment run only when it failed, so none of them moves the checkout. A `cd` the walk can't resolve leaves the checkout at the event's `cwd`, and one whose directory doesn't exist when the guard runs, as when an earlier segment of the same command creates it (`git worktree add <dir> -b <branch> && cd <dir>`), leaves it where it was, so a rule is never less guarded than without the `cd`. A kept `cd` survives a later relative `cd` the walk follows (only an absolute one drops it), since a directory relative to a checkout the walk fell back to is no surer than that checkout. The walk yields each segment, then the quoted substitutions it holds (from the checkout before its own `env -C` or `sudo -D`), then the bodies and here-strings its shell reads and its `-c` script (from its own checkout), then the bodies it pipes to a later shell of the same pipeline (from that shell's checkout); a `cd` inside any of these moves only that text's later segments.


**Standard input** (`guard-standard-input`). The scan already finds each heredoc operator and its word; it keeps, beside the body, whether the word was quoted, and notes each input redirection (`<`, `0<`, `<>`, `<<<`, a heredoc) in the order written, so the walk can say which one wins. Only the source is given, never its contents: the guard reads no file to tell what a `<` redirection sends.

## MODIFIED Requirements

### Requirement: guard-checkout <!-- base: 9d64e7bf7909bc59 -->
`clm hook guard` SHALL judge each segment from its *checkout*: the event's `cwd`, moved by each earlier segment that is `cd <dir>` and is ended by `&&`, `;` or a newline, with `<dir>` resolved against the checkout so far (`guard-command-position` scopes a `cd` inside `( )` or backticks), then by the segment's own `env -C` or `sudo -D` (`guard-command-position`). A `cd` segment ended by `|`, `|&`, `&` or `||` SHALL NOT move it. `&&`, `||` and `|&` are each one separator, and an `&` in a redirection (`2>&1`, `>&2`, `&>f`) ends no segment, so `cd w 2>&1 && git push` moves the checkout. A `cd` with no argument, or whose directory is `-` or contains `$`, `~` or a backtick, SHALL leave the checkout at the event's `cwd`, and a `cd` whose directory doesn't exist or isn't a directory when the guard runs SHALL leave it where it was; the guard SHALL NOT work out what earlier segments of the command would create or switch to. When the last `cd` that set a segment's checkout, in the segment's scope, is one it couldn't follow, of either kind, the guard SHALL keep that `cd` for the segment as written (such as `cd w`, `cd $W` or `cd`), for the rules that read it (`push-guard`, whose reasons name it and which counts a push from a checkout so set as under CLM), until a `cd` it follows to an absolute directory, or an absolute `env -C` or `sudo -D`, sets the checkout.

In the scenarios below, every repository and worktree is under CLM (it holds `specs/SPEC.md`), so `push-guard` judges a push from it.

#### Scenario: cd into a worktree
- GIVEN an event whose `cwd` is on `main`, and `w` a worktree under it on `impl/x/b`
- WHEN `clm hook guard` runs on `cd w && git commit -m m && git push` and on `cd w; git push`
- THEN each exits 0

#### Scenario: cd that doesn't move the push
- GIVEN an event whose `cwd` is on `main`, and `w` a worktree under it on `impl/x/b`
- WHEN `clm hook guard` runs on `cd w | git push`, `cd w & git push` and `cd w || git push`
- THEN each exits 2

#### Scenario: cd with a redirection
- GIVEN an event whose `cwd` is on `main`, and `w` a worktree under it on `impl/x/b`
- WHEN `clm hook guard` runs on `cd w 2>&1 && git push`
- THEN it exits 0

#### Scenario: cd into main
- GIVEN an event whose `cwd` is on `impl/x/b`, and `m` a worktree on `main`
- WHEN `clm hook guard` runs on `cd m && git push`
- THEN it exits 2

#### Scenario: cd it can't follow
- GIVEN an event whose `cwd` is on `main`
- WHEN `clm hook guard` runs on `cd $W && git push` and `cd ~/w && git push`
- THEN each exits 2

#### Scenario: an absolute cd after one it can't follow
- GIVEN an event whose `cwd` is on `impl/x/b`, `m` a worktree under it on `main`, and no directory `gone` under it
- WHEN `clm hook guard` runs on `cd gone; cd <m's absolute path> && git push`, and on `cd gone; cd m && git push`
- THEN each exits 2; the first with a reason that doesn't say the guard couldn't follow a `cd`, the second with one that quotes `cd gone` and says it couldn't follow it

