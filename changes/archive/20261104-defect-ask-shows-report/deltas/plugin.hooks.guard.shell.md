---
component: plugin.hooks.guard.shell
base: { contract: "743d80f03d336230", design: "dac5cbce0a39f966" }
---

## MODIFIED Contract
How the guard (`plugin.hooks.guard`) reads a Bash command as the shell would, before any rule judges it: where the command splits into segments and tokens (`guard-quotes`), which token is a segment's command (`guard-command-position`), which other text runs as commands (a heredoc body or here-string a shell reads, a shell's `-c` script, a substitution inside double quotes: `guard-heredocs`, `guard-here-strings`, `guard-shell-scripts`, `guard-quoted-substitutions`), and the checkout each piece runs in (`guard-checkout`). It decides nothing: it yields, in order, the pieces to judge, each with its text, its tokens, its checkout, the last `cd` that set that checkout when the guard couldn't follow it, and what feeds its standard input (`guard-standard-input`), and the guard's rules judge them. Its requirements are stated as `clm hook guard`'s answers because that is where they show, except `guard-standard-input`, which only the defect ask (`plugin.hooks.guard/defect-approval`) reads, so it is stated as what the walk yields.


## MODIFIED Design
Module `shell` of crate `clm-guard`, at `crates/clm-guard/src/shell/` (`guard-split`): `scan.rs` (segments, tokens, heredoc bodies, here-strings and quoted substitutions), `command.rs` (the command position, `env -S`, a shell's `-c` script, whether a segment's command reads standard input) and `walk.rs` (the pieces to judge, in order, each with its checkout), declared by `shell/mod.rs`. The module exports to the crate (`pub(crate)`) the walk, the piece it yields (its text, its tokens, its checkout and the `cd` kept for it: `Judged`) and `command`'s parsed `Command`, which the rules read. The guard's `lib.rs` declares the module and calls the walk once per event; nothing outside `clm-guard` sees it. It runs no process, and reads nothing outside the command but the file system, to tell whether a `cd`'s directory exists. Its tests, `crates/clm-guard/tests/shell.rs`, call `clm_guard::guard` on `PreToolUse` events as `clm hook guard` does and cite only this component's requirements: a scenario reads a rule's answer (a push to `main` blocked, an approval asked about) from the guard. The scenarios of `guard-standard-input` read the walk's pieces directly, so they are tested in unit tests inside `walk.rs`, which lies in this component's `paths`.

The guard (`guard-matches-writes`, `guard-reads-quotes-and-wrappers`) scans the command once, finding quotes and heredocs in the same scan (`guard-quotes`), and each segment keeps the separator that ended it: `&&`, `||`, `|`, `|&`, `&`, `;` or a newline, the command's end counting as a newline. Splitting at `(`, `)` and the backtick makes a subshell, a group or a command substitution (`(git push ...)`, `out=$(git push ...)`) segments of its own. A `$( )` inside double quotes stays part of its quoted token, but its text is also judged as commands (`guard-quoted-substitutions`), so `git commit -m "$(cat <<'EOF' ... EOF)"`, the usual way a session writes a commit message, keeps its body's prose out of the segments however many `"` it holds, and `echo "$(git push origin main)"` is judged as the push it runs. The substitution's text stays in the outer token as written, so a rule that reads that token, such as a GraphQL mutation's name in `-f query="$(cat <<'EOF' ... EOF)"`, sees it. A `)` with no `(` inside the substitution, as in a `case` pattern, ends it early, and a backtick inside double quotes stays part of its quoted token: both are gaps.

Several heredocs on one line take their bodies in order. A body judged as commands (`guard-heredocs`) is split as any command is, so its own heredocs and pipes follow the same rules; it starts at the checkout of the segment whose shell runs it (for a piped body, the shell's segment), and a `cd` inside it moves only the body's later segments, never the command's. A here-string fed to a shell over its standard input, as in `bash <<< 'git push origin main'`, is judged as a body is (`guard-here-strings`); fed to any other command it stays a redirection's target, so `cat <<< '...' | bash`, like `echo '...' | bash`, is a gap: the guard judges only heredoc bodies through a pipe.

A segment's *command* is found by `command.rs` (`guard-command-position`); every rule that speaks of a segment's first token means that command. A shell run with `-c` has its script, one token, split into segments and judged from the segment's checkout (`guard-shell-scripts`), as a heredoc body fed to a shell is.

**Checkouts** (`guard-checkout`). A `cd` ended by `|`, `|&` or `&` runs in a subshell or in the background, and one ended by `||` lets the next segment run only when it failed, so none of them moves the checkout. A `cd` the walk can't resolve leaves the checkout at the event's `cwd`, and one whose directory doesn't exist when the guard runs, as when an earlier segment of the same command creates it (`git worktree add <dir> -b <branch> && cd <dir>`), leaves it where it was, so a rule is never less guarded than without the `cd`. A kept `cd` survives a later relative `cd` the walk follows (only an absolute one drops it), since a directory relative to a checkout the walk fell back to is no surer than that checkout. The walk yields each segment, then the quoted substitutions it holds (from the checkout before its own `env -C` or `sudo -D`), then the bodies and here-strings its shell reads and its `-c` script (from its own checkout), then the bodies it pipes to a later shell of the same pipeline (from that shell's checkout); a `cd` inside any of these moves only that text's later segments.


**Standard input** (`guard-standard-input`). The scan already finds each heredoc operator and its word; it keeps, beside the body, whether the word was quoted, and notes each input redirection (`<`, `0<`, `<>`, `<<<`, a heredoc) in the order written, so the walk can say which one wins. Only the source is given, never its contents: the guard reads no file to tell what a `<` redirection sends.

## ADDED Requirements

### Requirement: guard-standard-input
The guard's walk SHALL yield, with each piece, what feeds its standard input, as the shell decides it: the last of the segment's input redirections, read left to right outside quotes and heredoc bodies (a heredoc, a here-string `<<<`, or a file: `<`, `0<` or `<>` naming one, or `<&` duplicating or closing a descriptor); with none, a pipe when a `|` or `|&` ends the segment before it in the same pipeline; otherwise the command's own standard input. For a heredoc it SHALL yield the body as the shell sends it, the leading tabs of its lines removed for `<<-`, and whether the shell sends it as written: its word is quoted, wholly or in part, with `'`, `"` or a backslash, or its body holds no `$`, `` ` `` or `\`. A redirection inside a quoted substitution, a shell's `-c` script or a body judged as commands SHALL belong to the piece there that holds it, never to the outer segment.

#### Scenario: a quoted heredoc
- GIVEN the command `clm defect --title x <<'EOF'` followed by a line `cost $HOME` and `EOF`
- WHEN the walk reads it
- THEN its one piece's standard input is a heredoc with the body `cost $HOME`, sent as written

#### Scenario: an unquoted heredoc
- GIVEN the commands `clm defect <<EOF` followed by a line `plain words` and `EOF`, and `clm defect <<EOF` followed by a line `cost $HOME` and `EOF`
- WHEN the walk reads each
- THEN the first piece's heredoc is sent as written, and the second's isn't

#### Scenario: the last redirection wins
- GIVEN the commands `clm defect <<'EOF' < f` followed by a line `r` and `EOF`, and `cat r | clm defect <<'EOF'` followed by a line `r` and `EOF`
- WHEN the walk reads each
- THEN the first piece's standard input is the file `f`, and the second's is the heredoc

#### Scenario: other sources
- GIVEN the commands `clm defect < r.md`, `cat r.md | clm defect`, `clm defect <<< "$R"` and `clm defect`
- WHEN the walk reads each
- THEN the `clm defect` piece's standard input is a file, a pipe, a here-string and the command's own, in that order

#### Scenario: a heredoc inside a substitution
- GIVEN the command `git commit -m "$(cat <<'EOF'` followed by a line `msg`, `EOF` and `)"`
- WHEN the walk reads it
- THEN the `git commit` piece's standard input is the command's own, and the `cat` piece's is the heredoc

#### Serves
- capability/report-defects
