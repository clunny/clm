---
component: plugin.hooks.guard.shell
base: { design: "e37a270272814032" }
---

## MODIFIED Design
Module `shell` of crate `clm-guard`, at `crates/clm-guard/src/shell/` (`guard-split`): `scan.rs` (segments, tokens, heredoc bodies, here-strings and quoted substitutions), `command.rs` (the command position, `env -S`, a shell's `-c` script, whether a segment's command reads standard input) and `walk.rs` (the pieces to judge, each with its checkout), declared by `shell/mod.rs`. The module exports to the crate (`pub(crate)`) the walk, the piece it yields (`Judged`) and `command`'s `parse` and parsed `Command`, which the rules read. The guard's `lib.rs` declares the module and calls the walk once per event; nothing outside `clm-guard` sees it. It runs no process, and reads nothing outside the command but the file system, to tell whether a `cd`'s directory exists. Its tests, `crates/clm-guard/tests/shell.rs`, call `clm_guard::guard` on `PreToolUse` events as `clm hook guard` does and cite only this component's requirements: a scenario reads a rule's answer (a push to `main` blocked, an approval asked about) from the guard. Every git repository and worktree these tests make holds `specs/SPEC.md`, so it is under CLM and `push-guard` judges a push from it (`push-guard-scoped-to-clm`). The scenarios of `guard-standard-input` read the walk's pieces directly, so they are tested in unit tests inside `walk.rs`, which lies in this component's `paths`.

The guard (`guard-matches-writes`, `guard-reads-quotes-and-wrappers`) scans the command once, finding quotes and heredocs in the same scan (`guard-quotes`), and each segment keeps the separator that ended it: `&&`, `||`, `|`, `|&`, `&`, `;` or a newline, the command's end counting as a newline. Splitting at `(`, `)` and the backtick makes a subshell, a group or a command substitution (`(git push ...)`, `out=$(git push ...)`) segments of its own. A `$( )` or a backtick substitution inside double quotes stays part of its quoted token, but its text is also judged as commands (`guard-quoted-substitutions`), so `git commit -m "$(cat <<'EOF' ... EOF)"`, the usual way a session writes a commit message, keeps its body's prose out of the segments however many `"` it holds, and `echo "$(git push origin main)"` and ``echo "`git push origin main`"`` are judged as the push they run. The substitution's text stays in the outer token as written, so a rule that reads that token, such as a GraphQL mutation's name in `-f query="$(cat <<'EOF' ... EOF)"`, sees it. A `)` with no `(` inside a `$( )`, as in a `case` pattern, ends it early: a gap.

The `$( )` and backtick substitutions in an unquoted heredoc's body are judged as a quoted substitution is, since the shell runs them when it sends the body; a quoted heredoc's body, the usual commit message, holds no commands. A body judged as commands (`guard-heredocs`) is split as any command is, so its own heredocs and pipes follow the same rules; it starts at the checkout of the segment whose shell runs it (for a piped body, the shell's segment), and a `cd` inside it moves only the body's later segments, never the command's. A here-string fed to a shell over its standard input, as in `bash <<< 'git push origin main'`, is judged as a body is (`guard-here-strings`); the guard judges only heredoc bodies through a pipe.

A segment's *command* is found by `command.rs` (`guard-command-position`); every rule that speaks of a segment's first token means that command. A shell run with `-c` has its script, one token, split into segments and judged from the segment's checkout (`guard-shell-scripts`), as a heredoc body fed to a shell is.

**Checkouts** (`guard-checkout`). A `cd` ended by `|`, `|&` or `&` runs in a subshell or in the background, and one ended by `||` lets the next segment run only when it failed, so none of them moves the checkout. A `cd` the walk can't resolve leaves the checkout at the directory the walk started from, so a rule is never less guarded than without the `cd`. A `cd` that fails in the shell leaves it where it was, as the shell stays there and `;` pushes from there: one whose directory doesn't exist when the guard runs, as when an earlier segment of the same command creates it (`git worktree add <dir> -b <branch> && cd <dir>`), one with an option bash's `cd` doesn't take, and one with a second directory, which bash's `cd` refuses with "too many arguments". `-@` counts as such an option, since bash offers it only on systems with extended attributes and refuses it elsewhere. A `$VAR`, `~` or substitution in a `cd`'s directory stays an accepted gap: its value lives in the session's shell, which the guard can't read, so such a `cd` resets the checkout to the directory the walk started from, which leaves a rule no less guarded than without the `cd`, and is kept for `push-guard`'s reason. A glob in a `cd`'s directory or in an `env -C` or `sudo -D` value is no such gap: its matches lie on the file system the guard already reads, so the walk expands it as bash's default globbing does (`*`, `?` and `[...]` with `[!...]`, `[^...]`, ranges and `[[:class:]]`, matched per path component, a wildcard never matching `/`, a leading `.` matched only when the pattern writes it, an unclosed `[` taken literally, a backslash-escaped character counted as quoted). The scan notes, for each token, whether a glob character in it stood outside quotes, and only the walk reads that note: it expands a glob in `git`'s own `-C` value too, from the checkout before the segment's own `env -C` or `sudo -D` (bash expands every word of a command line in the shell's directory, before `env` or `sudo` changes it, so `env -C s git -C m* push` globs `m*` where the shell stands), and yields the match among the tokens, so `push-guard` (`plugin.hooks.guard`), which sees only the plain strings of `Judged.tokens`, reads `git -C m* push` as `git -C m push`. Matches are sorted by their bytes, as bash sorts them in the C locale; a session whose locale collates otherwise can see another order, an accepted gap: the order decides which match is git's `-C` directory and which stand as the words after it, so under such a locale a `-C` glob whose matches sort otherwise can be judged as another command than the one bash runs. The rules are bash's; zsh, which some sessions run, differs (its `cd` refuses `-e`, substitutes on two words and aborts on an unmatched glob), but no command the guard allows under these rules pushes to `main` under zsh: where zsh differs, a block over-guards. A kept `cd` survives a later relative `cd` the walk follows (only an absolute one drops it), since a directory relative to a checkout the walk fell back to is no surer than that checkout. The walk yields each segment, then the quoted substitutions it holds (from the checkout before its own `env -C` or `sudo -D`), then the bodies and here-strings its shell reads and its `-c` script (from its own checkout), then the bodies it pipes to a later shell of the same pipeline (from that shell's checkout); a `cd` inside any of these moves only that text's later segments.

**Standard input** (`guard-standard-input`). The scan already finds each heredoc operator and its word; it keeps, beside the body, whether the word was quoted, and notes each input redirection (`<`, `0<`, `<>`, `<&`, `<<<`, a heredoc) in the order written, so the walk can say which one wins. Only the source is given, never its contents: the guard reads no file to tell what a `<` redirection sends.


## MODIFIED Requirements

### Requirement: guard-checkout <!-- base: 3872903286f3b8bf -->
`clm hook guard` SHALL judge each segment from its *checkout*: the directory the guard starts from (`plugin.hooks.guard`: the event's `cwd`, or the guard's own working directory when the event has none), moved by each earlier segment that is `cd <dir>` and is ended by `&&`, `;` or a newline, with `<dir>` resolved against the checkout so far (`guard-command-position` scopes a `cd` inside `( )` or backticks), then by the segment's own `env -C` or `sudo -D` (`guard-command-position`). A `cd` segment ended by `|`, `|&`, `&` or `||` SHALL NOT move it. A `cd`'s options SHALL be its leading tokens that start with `-` and are not `-` alone, up to and including a `--`, and its directory the token after them. The guard SHALL read a `cd` in this order, as bash does:
1. A directory that contains `$`, `~` or a backtick SHALL leave the checkout at the directory the guard starts from, and is not expanded.
2. A directory that holds a `*`, `?` or `[` outside quotes SHALL be expanded against the file system, relative to the checkout so far, as bash's default globbing matches it (`*`, `?` and `[...]`, per path component, a wildcard never matching `/`, and a leading `.` matched only when the pattern writes it): one match SHALL be the directory; with none, the directory SHALL be the word as written; with several, bash's `cd` takes too many arguments, so the `cd` SHALL count as one with a token after its directory.
3. A `cd` with an option that is neither `--` nor a `-` followed by one or more of the letters `L`, `P` and `e` (`-P`, `-LPe`), or with a token after its directory, or whose directory doesn't exist or isn't a directory when the guard runs, SHALL leave the checkout where it was, as the shell's `cd` then fails and stays; the guard SHALL NOT work out what earlier segments of the command would create or switch to.
4. A `cd` with no directory, or whose directory is `-`, SHALL leave the checkout at the directory the guard starts from.

An `env -C` or `sudo -D` value that holds a `*`, `?` or `[` outside quotes SHALL be expanded in the same way, from the checkout so far: one match SHALL be the directory, and otherwise the value as written. In a segment whose command is `git` or ends in `/git` (`guard-command-position`), a `-C` before git's subcommand (its first argument that doesn't start with `-` and isn't the value of `-C`, `-c`, `--git-dir`, `--work-tree`, `--namespace` or `--config-env`) whose value is the next token and holds a `*`, `?` or `[` outside quotes SHALL be expanded in the same way, from the checkout so far, before the segment's own `env -C` or `sudo -D`, since bash expands it in the shell's directory before `env` or `sudo` changes directory, and the walk SHALL yield, in that value's place among the segment's tokens, every match, sorted by its bytes as bash sorts them in the C locale, as bash passes them to git, or the value as written when there is none. A glued `-C<value>` (`-Cm*`) is one word to bash, which globs the whole word, so only an entry whose name starts with `-C` can match it; the walk SHALL expand it so, as the whole word, and yield every match in its place, or the word as written when there is none. When the guard has no directory to start from, the walk SHALL start with no checkout: a relative `cd`, `env -C` or `sudo -D` SHALL leave it with none and a relative glob SHALL stay as written, until an absolute `cd`, `env -C` or `sudo -D` sets one. When the last `cd` that set a segment's checkout, in the segment's scope, is one it couldn't follow, of any of these kinds, the guard SHALL keep that `cd` for the segment as written (such as `cd w`, `cd $W` or `cd`), for the rules that read it (`plugin.hooks.guard/push-guard`, `plugin.hooks.guard/push-guard-scoped-to-clm`), until a `cd` it follows to an absolute directory, or an absolute `env -C` or `sudo -D`, sets the checkout.

In the scenarios below, every repository and worktree is under CLM (it holds `specs/SPEC.md`), so `push-guard` judges a push from it.

#### Scenario: cd into a worktree
- GIVEN an event whose `cwd` is on `main`, and `w` a worktree under it on `impl/x/b`
- WHEN `clm hook guard` runs on `cd w && git commit -m m && git push` and on `cd w; git push`
- THEN each exits 0

#### Scenario: cd that doesn't move the push
- GIVEN an event whose `cwd` is on `main`, and `w` a worktree under it on `impl/x/b`
- WHEN `clm hook guard` runs on `cd w | git push`, `cd w & git push` and `cd w || git push`
- THEN each exits 2

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

#### Scenario: a cd with options or several directories
- GIVEN an event whose `cwd` is on `impl/x/b`, and `m` a worktree under it on `main`
- WHEN `clm hook guard` runs on `cd -P m && git push`, `cd -LPe m && git push`, `cd -- m && git push`, `cd m; cd m x; git push`, and `cd m; cd -x .; git push`
- THEN each exits 2 naming `guard-blocks-push-to-main`

#### Scenario: a cd into a glob
- GIVEN an event whose `cwd` is on `impl/x/b`, `m` a worktree under it on `main` and the only entry there whose name starts with `m`, and plain directories `x1` and `x2` under it
- WHEN `clm hook guard` runs on `cd m* && git push`, `cd [m] && git push`, `cd m; cd ../x*; git push`, `env -C m* git push`, `sudo -D m* git push`, `git -C m* push`, `cd 'm*' && git push` and `git -C 'm*' push`
- THEN the first six exit 2 naming `guard-blocks-push-to-main`, and the last two, whose globs are quoted and name no directory, exit 0

#### Scenario: no directory to start from
- GIVEN the walk given no directory to start from, and `m` a directory
- WHEN the walk reads `cd <m's absolute path> && git push`, `env -C <m's absolute path> git push` and `cd m && git push`
- THEN the `git` piece's checkout is `m` for the first two, and it has none for the third

#### Scenario: a git -C glob after env -C
- GIVEN an event whose `cwd` is on `impl/x/b`, `m` a plain directory under it and the only entry there whose name starts with `m`, and `s` a directory under it holding `m`, a worktree on `main`, and a plain directory `m2`
- WHEN `clm hook guard` runs on `env -C s git -C m* push` and `sudo -D s git -C m* push`
- THEN each exits 2 naming `guard-blocks-push-to-main`, since bash globs `m*` to `m` where the shell stands and git pushes from `s/m`
