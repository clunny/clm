---
title: Guard shell reading
status: active
owners: ["@clunny"]
depends_on: []
paths: ["crates/clm-guard/src/shell/**", "crates/clm-guard/tests/shell.rs"]
successor: ""
serves: [capability/guard-sessions, architecture/land-locally]
---

## Contract
How the guard (`plugin.hooks.guard`) reads a Bash command as the shell would, before any rule judges it: where the command splits into segments and tokens (`guard-quotes`), which token is a segment's command (`guard-command-position`), which other text runs as commands (a heredoc body or here-string a shell reads, a shell's `-c` script, a substitution inside double quotes: `guard-heredocs`, `guard-here-strings`, `guard-shell-scripts`, `guard-quoted-substitutions`), and the checkout each piece runs in (`guard-checkout`). It decides nothing: the guard's rules judge the pieces it yields.

It offers the rest of crate `clm-guard`, and nothing outside it, two calls:
- `walk(command, cwd)`, where `cwd` is the directory the guard starts from (`plugin.hooks.guard`: the event's `cwd`, or the guard's own working directory when the event has none), absent only when the guard has none (`guard-checkout` says where the walk then starts): the pieces to judge, each a `Judged` with its `text` (the segment as written, trimmed, without its heredoc bodies), its `tokens` (as `guard-quotes` splits the segment, without its redirections, each `env -S` value split in place: `guard-command-position`), its `checkout` (`guard-checkout`; absent when the walk has none), `lost` (the last `cd` that set the checkout when the guard couldn't follow it, as written: `guard-checkout`), `piped` (whether `|` or `|&` ends the segment, so its output goes to the next segment of its pipeline: `guard-quotes`) and `stdin` (what feeds its standard input, `guard-standard-input`: `Own`, `Pipe`, `File`, `HereString`, or `Heredoc` with its `body` and `as_written`), and `command()`, the `Command` its tokens run;
- `parse(tokens)`, the `Command` a segment's tokens run (`guard-command-position`): its `args`, the tokens after the command; `is(name)`, whether the command is `name` alone or a path ending in `/<name>`; and `assigned(name)`, the value of the last `NAME=value` among the skipped tokens that assigns `name`.

Its requirements are stated as `clm hook guard`'s answers because that is where they show, except `guard-standard-input`, which only the defect ask (`plugin.hooks.guard/defect-approval`) reads, so it is stated as what the walk yields.

## Design
Module `shell` of crate `clm-guard`, at `crates/clm-guard/src/shell/` (`guard-split`): `scan.rs` (segments, tokens, heredoc bodies, here-strings and quoted substitutions), `command.rs` (the command position, `env -S`, a shell's `-c` script, whether a segment's command reads standard input) and `walk.rs` (the pieces to judge, each with its checkout), declared by `shell/mod.rs`. The module exports to the crate (`pub(crate)`) the walk, the piece it yields (`Judged`) and `command`'s `parse` and parsed `Command`, which the rules read. The guard's `lib.rs` declares the module and calls the walk once per event; nothing outside `clm-guard` sees it. It runs no process, and reads nothing outside the command but the file system, to tell whether a `cd`'s directory exists. Its tests, `crates/clm-guard/tests/shell.rs`, call `clm_guard::guard` on `PreToolUse` events as `clm hook guard` does and cite only this component's requirements: a scenario reads a rule's answer (a push to `main` blocked, an approval asked about) from the guard. Every git repository and worktree these tests make holds `specs/SPEC.md`, so it is under CLM and `push-guard` judges a push from it (`push-guard-scoped-to-clm`). The scenarios of `guard-standard-input` read the walk's pieces directly, so they are tested in unit tests inside `walk.rs`, which lies in this component's `paths`.

The guard (`guard-matches-writes`, `guard-reads-quotes-and-wrappers`) scans the command once, finding quotes and heredocs in the same scan (`guard-quotes`), and each segment keeps the separator that ended it: `&&`, `||`, `|`, `|&`, `&`, `;` or a newline, the command's end counting as a newline. Splitting at `(`, `)` and the backtick makes a subshell, a group or a command substitution (`(git push ...)`, `out=$(git push ...)`) segments of its own. A `$( )` or a backtick substitution inside double quotes stays part of its quoted token, but its text is also judged as commands (`guard-quoted-substitutions`), so `git commit -m "$(cat <<'EOF' ... EOF)"`, the usual way a session writes a commit message, keeps its body's prose out of the segments however many `"` it holds, and `echo "$(git push origin main)"` and ``echo "`git push origin main`"`` are judged as the push they run. The substitution's text stays in the outer token as written, so a rule that reads that token, such as a GraphQL mutation's name in `-f query="$(cat <<'EOF' ... EOF)"`, sees it. A `)` with no `(` inside a `$( )`, as in a `case` pattern, ends it early: a gap.

The `$( )` and backtick substitutions in an unquoted heredoc's body are judged as a quoted substitution is, since the shell runs them when it sends the body; a quoted heredoc's body, the usual commit message, holds no commands. A body judged as commands (`guard-heredocs`) is split as any command is, so its own heredocs and pipes follow the same rules; it starts at the checkout of the segment whose shell runs it (for a piped body, the shell's segment), and a `cd` inside it moves only the body's later segments, never the command's. A here-string fed to a shell over its standard input, as in `bash <<< 'git push origin main'`, is judged as a body is (`guard-here-strings`); the guard judges only heredoc bodies through a pipe.

A segment's *command* is found by `command.rs` (`guard-command-position`); every rule that speaks of a segment's first token means that command. A shell run with `-c` has its script, one token, split into segments and judged from the segment's checkout (`guard-shell-scripts`), as a heredoc body fed to a shell is.

**Checkouts** (`guard-checkout`). A `cd` ended by `|`, `|&` or `&` runs in a subshell or in the background, and one ended by `||` lets the next segment run only when it failed, so none of them moves the checkout. A `cd` the walk can't resolve leaves the checkout at the directory the walk started from, so a rule is never less guarded than without the `cd`. A `cd` that fails in the shell leaves it where it was, as the shell stays there and `;` pushes from there: one whose directory doesn't exist when the guard runs, as when an earlier segment of the same command creates it (`git worktree add <dir> -b <branch> && cd <dir>`), one with an option bash's `cd` doesn't take, and one with a second directory, which bash's `cd` refuses with "too many arguments". `-@` counts as such an option, since bash offers it only on systems with extended attributes and refuses it elsewhere. A `$VAR`, `~` or substitution in a `cd`'s directory stays an accepted gap: its value lives in the session's shell, which the guard can't read, so such a `cd` resets the checkout to the directory the walk started from, which leaves a rule no less guarded than without the `cd`, and is kept for `push-guard`'s reason. A glob in a `cd`'s directory or in an `env -C` or `sudo -D` value is no such gap: its matches lie on the file system the guard already reads, so the walk expands it as bash's default globbing does (`*`, `?` and `[...]` with `[!...]`, `[^...]`, ranges and `[[:class:]]`, matched per path component, a wildcard never matching `/`, a leading `.` matched only when the pattern writes it, an unclosed `[` taken literally, a backslash-escaped character counted as quoted). The scan notes, for each token, whether a glob character in it stood outside quotes, and only the walk reads that note: it expands a glob in `git`'s own `-C` value too and yields the match among the tokens, so `push-guard` (`plugin.hooks.guard`), which sees only the plain strings of `Judged.tokens`, reads `git -C m* push` as `git -C m push`. Matches are sorted by their bytes, as bash sorts them in the C locale; a session whose locale collates otherwise can see another order, an accepted gap that only changes which match comes first. The rules are bash's; zsh, which some sessions run, differs (its `cd` refuses `-e`, substitutes on two words and aborts on an unmatched glob), but no command the guard allows under these rules pushes to `main` under zsh: where zsh differs, a block over-guards. A kept `cd` survives a later relative `cd` the walk follows (only an absolute one drops it), since a directory relative to a checkout the walk fell back to is no surer than that checkout. The walk yields each segment, then the quoted substitutions it holds (from the checkout before its own `env -C` or `sudo -D`), then the bodies and here-strings its shell reads and its `-c` script (from its own checkout), then the bodies it pipes to a later shell of the same pipeline (from that shell's checkout); a `cd` inside any of these moves only that text's later segments.

**Standard input** (`guard-standard-input`). The scan already finds each heredoc operator and its word; it keeps, beside the body, whether the word was quoted, and notes each input redirection (`<`, `0<`, `<>`, `<&`, `<<<`, a heredoc) in the order written, so the walk can say which one wins. Only the source is given, never its contents: the guard reads no file to tell what a `<` redirection sends.

## Requirements

### Requirement: guard-quotes
`clm hook guard` SHALL scan a command once from left to right, as the shell quotes: a `'` opens a quote that only the next `'` closes, a `"` opens one that the next `"` not escaped by a backslash closes, a `$'` outside quotes opens an ANSI-C quote that the next `'` not escaped by a backslash closes, a backslash outside single quotes escapes the next character, and a quote opened on one line may close on a later one. A backslash before a newline, outside `'...'` and `$'...'` quotes, SHALL remove both, so the lines it joins are one segment. A `#` outside quotes that begins a word SHALL comment out the rest of its line: nothing in it is a command, a quote or a heredoc operator; a `#` inside a word SHALL NOT. Inside double quotes, a `$(` opens a substitution that runs to its matching `)`: inside it the scan SHALL quote, count `(` and `)` and find heredocs as it does outside any quote, so a `"` inside it opens a quote of its own instead of closing the outer one, and the substitution, with the double quotes around it, stays part of one token. Inside double quotes, a backtick SHALL open a substitution that runs to the next backtick not escaped by a backslash, which also stays, with the double quotes around it, part of one token. That token SHALL hold the substitution's text as written, from `$(` to `)` or from backtick to backtick, with no quotes removed and any heredoc body inside it included, and a heredoc found inside it SHALL belong to the substitution's segments (`guard-quoted-substitutions`), not to the outer segment's bodies. It SHALL find heredoc operators (`guard-heredocs`) with this scan, and a heredoc body's text SHALL NOT count toward quoting. It SHALL split the command into segments at `|`, `;`, `&`, `(`, `)`, `` ` `` and newlines only outside quotes and outside heredoc bodies, and each segment into tokens at whitespace outside quotes, removing only the quote characters and backslashes that quote (an ANSI-C quote's `$'` and closing `'`, and the backslash of its `\'` and `\\`), so a quoted string with spaces is one token and a `"` inside `'...'` stays in it. `&&`, `||` and `|&` SHALL each be one separator, and the `&` of `>&`, `<&` and `&>` SHALL NOT separate, so `cd w 2>&1 && git push` moves the checkout. A segment's tokens SHALL NOT include its redirections: an operator `<`, `>`, `>>`, `<>`, `<<`, `<<-`, `<<<`, `<&`, `>&`, `&>` or `&>>`, with any descriptor number written before it, and its target, glued to the operator or the next word. The quotes *don't balance* when the scan ends inside a quote or a substitution; the guard SHALL then split that command's segments and tokens, outside the heredoc bodies the scan found, at every separator and space as if it had no quotes, with every quote character removed.

#### Scenario: separators inside a message
- GIVEN the command `git commit -m "build; git push origin main is forbidden"`, and `git commit -m "first line` followed by a line `git push origin main` and a line `last line"`
- WHEN `clm hook guard` runs on each
- THEN each exits 0

#### Scenario: a quoted directory with a space
- GIVEN an event whose `cwd` is on `impl/x/b`, and a worktree `my main` under it on `main`
- WHEN `clm hook guard` runs on `git -C "my main" push`
- THEN it exits 2

#### Scenario: unbalanced quotes
- GIVEN the command `echo don't; git push origin main`
- WHEN `clm hook guard` runs on it
- THEN it exits 2

#### Scenario: an apostrophe in a heredoc body
- GIVEN the command `cat > n.md <<'EOF'`, a line `don't`, `EOF`, and a line `git commit -m "a; git push origin main"`
- WHEN `clm hook guard` runs on it
- THEN it exits 0

#### Scenario: ANSI-C quotes
- GIVEN the command `echo $'\''; git push origin main; echo \'`, whose quotes balance whether or not `$'` is read as an ANSI-C quote
- WHEN `clm hook guard` runs on it
- THEN it exits 2

#### Scenario: a heredoc in a quoted substitution
- GIVEN the command `git commit -m "$(cat <<'EOF'`, a line `Say "a; git push origin main" here.`, a line `EOF`, and a line `)"`
- WHEN `clm hook guard` runs on it
- THEN it exits 0 and prints nothing, while the same command with `gh api graphql -f query=` in place of `git commit -m ` and a body line `mutation { mergePullRequest }` exits 2

#### Scenario: a quote inside a quote
- GIVEN the command `bash -c 'git commit -m "a; git push origin main"'`
- WHEN `clm hook guard` runs on it
- THEN it exits 0

#### Scenario: a continued line
- GIVEN the commands `git push origin \` followed by a line `main`, and `git push \` followed by a line `origin main`
- WHEN `clm hook guard` runs on each
- THEN each exits 2 naming `guard-blocks-push-to-main`

#### Scenario: a comment
- GIVEN the commands `# it's here`, a line `git push origin main` and a line `# that's all`; `# use <<EOF style`, a line `git push origin main` and a line `EOF`; `true # don't` and a line `git push origin main # it's`; and `echo a#b; git push origin main`
- WHEN `clm hook guard` runs on each
- THEN each exits 2 naming `guard-blocks-push-to-main`, while `echo x # ; git push origin main` exits 0 and prints nothing

#### Scenario: cd with a redirection
- GIVEN an event whose `cwd` is on `main`, and `w` a worktree under it on `impl/x/b`
- WHEN `clm hook guard` runs on `cd w 2>&1 && git push`, `cd w &>/dev/null && git push`, `cd w <&0 && git push` and `cd w >&2 && git push`
- THEN each exits 0

#### Scenario: a pipe of standard error
- GIVEN the command `cat <<'EOF' |& sh`, a line `git push origin main` and `EOF`
- WHEN `clm hook guard` runs on it
- THEN it exits 2 naming `guard-blocks-push-to-main`

#### Scenario: redirections are no tokens
- GIVEN an event whose `cwd` is on `main`, and `w` a worktree under it on `impl/x/b`
- WHEN `clm hook guard` runs on `> out git push origin main`, `2>err git push origin main`, `cd w > /dev/null && git push` and `cd w 2> err && git push`
- THEN the first two exit 2 naming `guard-blocks-push-to-main`, and the last two exit 0

### Requirement: guard-heredocs
`clm hook guard` SHALL treat a heredoc's body as part of the segment that opened it, not as segments of its own, for every guard rule. The heredoc operator is `<<` or `<<-` outside quotes (the text of a substitution `guard-quotes` finds inside double quotes counting as outside) and not followed by `<`, then optional spaces and a word, bare or in single or double quotes. The body is the lines after the operator's line up to the first line that is the word (after leading tabs for `<<-`). An operator with no such line SHALL NOT start a heredoc. Several heredoc operators on one line SHALL take their bodies in the order written, each starting on the line after the previous one's terminator. A segment's `clm`, `gh` and `git` calls SHALL be found only in the segment's own line, and its body SHALL match no guard rule, but for its substitutions: in the body of a heredoc whose word is unquoted (no `'`, `"` or backslash in it), each `$(` and each backtick not escaped by a backslash opens a substitution that SHALL be judged as `guard-quoted-substitutions` judges one (a backtick substitution's text being what lies between its backticks, with the backslash of each `\$`, `` \` `` and `\\` in it removed), since the shell runs it when it sends the body; a quoted word's body SHALL stay unjudged. When that segment runs a shell over its standard input, it SHALL also judge the body's lines as segments. A segment runs a shell over its standard input when its command (`guard-command-position`) is `bash`, `sh`, `zsh`, `dash` or `ksh`, or ends in `/` and one of them, and no short option of the shell (a token after it that starts with a single `-`, among those up to the first token that doesn't start with `-`) holds the letter `c`; or when its command is `source` or `.` and the next token is `/dev/stdin`, `/dev/fd/0` or `/proc/self/fd/0`. When a segment with heredoc bodies is ended by `|` or `|&`, the guard SHALL also judge its bodies' lines as segments when a later segment of the same pipeline, the segments joined to it by `|` or `|&`, runs a shell over its standard input and has no heredoc body of its own. Body lines judged as segments SHALL follow every guard rule, these included, as a command's lines do, starting from the checkout of the segment whose shell runs them; a `cd` among them SHALL NOT move the command's later segments.

#### Scenario: writing guard text to a file
- GIVEN the command `cat > d.md <<'EOF'`, a line `- GIVEN the command clm approve spec/x`, a line `clm approve spec/x`, a line `git push origin main`, and `EOF`
- WHEN `clm hook guard` runs on it
- THEN it exits 0 and prints nothing

#### Scenario: a body sent to gh
- GIVEN the command `gh pr comment 3 --body-file - <<'EOF'`, a line `clm approve spec/x`, a line `gh release create v1`, a line `git push origin main`, and `EOF`
- WHEN `clm hook guard` runs on it
- THEN it exits 0 and prints nothing

#### Scenario: body run by a shell
- GIVEN the commands `bash <<'EOF'` and `/bin/sh <<EOF`, each followed by a line `clm approve spec/x` and `EOF`, and `bash <<'EOF'` followed by `git push origin main` and `EOF`
- WHEN `clm hook guard` runs on each
- THEN the first two print the `ask` decision, and the third exits 2

#### Scenario: a body piped into a shell
- GIVEN the commands `cat <<'EOF' | bash` and `cat <<'EOF' | tee log | sh`, each followed by a line `git push origin main` and `EOF`
- WHEN `clm hook guard` runs on each
- THEN each exits 2

#### Scenario: a body piped elsewhere
- GIVEN the commands `cat <<'EOF' | grep x`, `cat <<'EOF' | bash -c 'cat > f'` and `cat <<'EOF' | gh api graphql -F query=@-`, each followed by a line `git push origin main` and `EOF`
- WHEN `clm hook guard` runs on each
- THEN each exits 0 and prints nothing

#### Scenario: wrapped shells
- GIVEN the commands `sudo bash <<'EOF'`, `sudo -u root bash <<'EOF'`, `exec sh <<'EOF'`, `nohup bash <<'EOF'`, `xargs -0 sh <<'EOF'`, `source /dev/stdin <<'EOF'` and `. /dev/fd/0 <<'EOF'`, each followed by a line `git push origin main` and `EOF`
- WHEN `clm hook guard` runs on each
- THEN each exits 2

#### Scenario: a shell running its argument
- GIVEN the commands `bash -ec 'cat > f' <<'EOF'` and `sh -xc 'cat > f' <<'EOF'`, each followed by a line `git push origin main` and `EOF`, and the command `cat <<'A' | bash <<'B'` followed by a line `git push origin main`, `A`, a line `echo hi` and `B`
- WHEN `clm hook guard` runs on each
- THEN each exits 0 and prints nothing

#### Scenario: commands after the body
- GIVEN the command `cat > f <<'EOF'`, a line `text`, `EOF`, and a line `git push origin main`
- WHEN `clm hook guard` runs on it
- THEN it exits 2

#### Scenario: no heredoc after all
- GIVEN the command `echo "<<X"` followed by a line `git push origin main`, and the command `cat <<EOF` followed by a line `git push origin main` and no `EOF` line
- WHEN `clm hook guard` runs on each
- THEN each exits 2

#### Scenario: substitutions in an unquoted heredoc
- GIVEN the commands `cat > f <<EOF` and `cat > f <<'EOF'`, each followed by a line ``now `git push origin main` `` and `EOF`; `cat > f <<EOF` followed by a line `now $(clm approve spec/x)` and `EOF`; and `cat > f <<EOF` followed by a line ``now \`git push origin main\` `` and `EOF`
- WHEN `clm hook guard` runs on each
- THEN the first exits 2 naming `guard-blocks-push-to-main`, the third prints the `ask` decision naming `humans-review-intent`, and the second and fourth exit 0 and print nothing

#### Scenario: several heredocs on a line
- GIVEN the commands `cat <<A | bash <<B` and `bash <<A | cat <<B`, each followed by a line `clm approve spec/x`, `A`, a line `git push origin main` and `B`
- WHEN `clm hook guard` runs on each
- THEN the first exits 2 naming `guard-blocks-push-to-main`, and the second prints the `ask` decision naming `humans-review-intent`

### Requirement: guard-command-position
`clm hook guard` SHALL take as a segment's *command*, for every rule that reads one, its first token after skipping, repeatedly and in any order:
- `NAME=value` assignments;
- the words `!`, `{`, `if`, `then`, `elif`, `else`, `do`, `while` and `until`;
- `time`, with a following `-p`;
- `timeout`, with the tokens after it that start with `-`, the value of its `-s`, `-k`, `--signal` and `--kill-after`, and then its duration;
- `env`, `sudo`, `doas`, `command`, `exec`, `nohup`, `nice`, `ionice`, `stdbuf` and `xargs`, each with the tokens after it that start with `-`, and with the token after each of these options that takes a value: `env`'s `-u`, `-C`, `--unset` and `--chdir`; `sudo`'s `-u`, `-g`, `-h`, `-p`, `-C`, `-D`, `-r`, `-t`, `-U`, `--user`, `--group`, `--host`, `--prompt`, `--chdir`, `--close-from`, `--role`, `--type` and `--other-user`; `doas`'s `-u` and `-C`; `nice`'s `-n` and `--adjustment`; `ionice`'s `-c`, `-n` and `-p`; `exec`'s `-a`; `stdbuf`'s `-i`, `-o` and `-e`; `xargs`'s `-I`, `-n`, `-P`, `-L`, `-s`, `-d`, `-a` and `-E`; and `env`'s `-S` and `--split-string`.

A token after one of these prefix commands, or after `timeout`, that starts with a single `-` and holds more than one letter is a cluster of short options: the first of its letters that is a short option of that command taking a value SHALL take the rest of the token as its value, or, when it is the token's last letter, the next token. So `sudo -Eu root git` and `sudo -uroot git` both run `git`, and a `-C` or `-D` that moves the checkout counts in a cluster too. `env`'s `-S`/`--split-string` value, alone, glued (`-S<value>`) or after `=`, SHALL be split into tokens as `guard-quotes` splits a segment, and those tokens SHALL be read in the option's place as `env`'s further arguments, so `env -S 'git push origin main'` runs `git`. The tokens every rule reads for a segment SHALL be its tokens with each such value split in place, so a rule that looks for `gh` among them finds it in `env -S 'gh release create v1'`.

The assignments a rule reads for a segment (such as `GIT_DIR` or `GH_REPO`) SHALL be every `NAME=value` among the skipped tokens, the last one winning. `env`'s `-C`/`--chdir` and `sudo`'s `-D`/`--chdir` SHALL move the checkout of that segment's command as a `git -C` does.

A `cd` segment between a `(` and its matching `)`, or between two backticks, SHALL move the checkout only for the segments before that `)` or closing backtick; after it, the checkout SHALL be what it was at the `(` or opening backtick. A `)` with no matching `(`, as in a `case` pattern, SHALL end no scope.

#### Scenario: wrapped pushes
- GIVEN the commands `(git push origin main)`, `{ git push origin main; }`, `if true; then git push origin main; fi`, `time git push origin main`, `sudo -u me git push origin main`, `command git push origin main`, `nohup git push origin main &`, `xargs -n 1 git push origin main`, `out=$(git push origin main)` and `` x=`git push origin main` ``
- WHEN `clm hook guard` runs on each
- THEN each exits 2 naming `guard-blocks-push-to-main`

#### Scenario: wrapped approval
- GIVEN the commands `(clm approve spec/x)` and `if true; then clm approve spec/x; fi`
- WHEN `clm hook guard` runs on each
- THEN each prints the `ask` decision naming `humans-review-intent`

#### Scenario: clustered options and split strings
- GIVEN the commands `sudo -Eu root git push origin main`, `sudo -uroot git push origin main`, `env -S 'git push origin main'`, `env --split-string='git push origin main'`, `env -iS 'git push origin main'` and `timeout -vk 5 60 git push origin main`
- WHEN `clm hook guard` runs on each
- THEN each exits 2 naming `guard-blocks-push-to-main`, and `env -S 'gh release create v1'` exits 2 naming `humans-cut-releases`

#### Scenario: wrapped pushes elsewhere
- GIVEN the commands `(git push origin impl/x/b)` and `sudo git push -u origin impl/x/b`
- WHEN `clm hook guard` runs on each
- THEN each exits 0

#### Scenario: cd in a subshell
- GIVEN an event whose `cwd` is on `main`, and `w` a worktree under it on `impl/x/b`
- WHEN `clm hook guard` runs on `(cd w && git push)` and on `(cd w); git push`
- THEN the first exits 0 and the second exits 2

#### Scenario: cd in a substitution
- GIVEN an event whose `cwd` is on `main`, and `w` a worktree under it on `impl/x/b`
- WHEN `clm hook guard` runs on `` x=`cd w`; git push ``
- THEN it exits 2

#### Scenario: a prefix that changes directory
- GIVEN an event whose `cwd` is on `impl/x/b`, and `m` a worktree under it on `main`
- WHEN `clm hook guard` runs on `env -C m git push`, on `env -iC m git push` and on `timeout 60 env GIT_DIR=m/.git git push`
- THEN each exits 2

### Requirement: guard-shell-scripts
When a segment's command (`guard-command-position`) is `bash`, `sh`, `zsh`, `dash` or `ksh`, or ends in `/` and one of them, and a token after it that starts with a single `-` contains `c`, `clm hook guard` SHALL split the first token after that one that doesn't start with `-` into segments (`guard-quotes`) and judge them by every guard rule as commands, from the checkout the segment runs in, as it judges a heredoc body a shell runs (`guard-heredocs`).

#### Scenario: a push in a script
- GIVEN the commands `bash -c 'git push origin main'`, `sh -ec "cd . && git push origin HEAD:main"` and `/bin/bash -c "git status; git push --mirror origin"`
- WHEN `clm hook guard` runs on each
- THEN each exits 2

#### Scenario: an approval in a script
- GIVEN the command `bash -c 'clm approve spec/x'`
- WHEN `clm hook guard` runs on it
- THEN it prints the `ask` decision

#### Scenario: a harmless script
- GIVEN the commands `bash -c 'git push origin impl/x/b'` and `sh -c "echo git push origin main"`
- WHEN `clm hook guard` runs on each
- THEN each exits 0

### Requirement: guard-quoted-substitutions
`clm hook guard` SHALL judge the text of each substitution that `guard-quotes` finds inside double quotes, between its `$(` and its matching `)`, as commands by every guard rule, from the checkout of the segment it is in as the segment's earlier `cd`s leave it, before that segment's own `-C`, `env -C` or `sudo -D`, since the shell expands it before the command runs; it judges it as it judges a shell's `-c` script (`guard-shell-scripts`). A backtick substitution that `guard-quotes` finds inside double quotes SHALL be judged in the same way, its text being what lies between its backticks with the backslash of each `\$`, `` \` ``, `\\` and `\"` in it removed, as the shell removes it. A substitution inside one SHALL be judged too, and a `cd` among its segments SHALL NOT move the command's later segments.

#### Scenario: a push in a quoted substitution
- GIVEN the commands `echo "$(git push origin main)"` and `x="out: $(cd . && git push origin HEAD:main)"`
- WHEN `clm hook guard` runs on each
- THEN each exits 2 naming `guard-blocks-push-to-main`

#### Scenario: an approval in a quoted substitution
- GIVEN the command `echo "$(clm approve spec/x)"`
- WHEN `clm hook guard` runs on it
- THEN it prints the `ask` decision naming `humans-review-intent`

#### Scenario: prose in a quoted substitution
- GIVEN the commands `git commit -m "$(printf 'a; git push origin main')"`, `gh pr create --title t --body "$(cat <<'EOF'`, a line `clm approve spec/x`, a line `EOF`, and a line `)"`, and `git commit -m "$(cat <<'EOF'`, a line ``Use `git push origin main` or $(clm approve spec/x) here``, a line `EOF`, and a line `)"`
- WHEN `clm hook guard` runs on each
- THEN each exits 0 and prints nothing

#### Scenario: a push in a quoted backtick
- GIVEN the commands ``echo "`git push origin main`"``, ``echo "`echo \`git push origin main\``"``, ``git commit -m "x `clm approve spec/x`"``, ``git commit -m "use \`git push origin main\` here"`` and ``git commit -m "`echo \"a; clm approve spec/x\"`"``
- WHEN `clm hook guard` runs on each
- THEN the first two exit 2 naming `guard-blocks-push-to-main`, the third prints the `ask` decision naming `humans-review-intent`, and the fourth, whose backticks are escaped, and the fifth, whose substitution runs only `echo`, exit 0 and print nothing

### Requirement: guard-here-strings
When a segment runs a shell over its standard input (`guard-heredocs`), `clm hook guard` SHALL judge the word of each of its here-strings as commands by every guard rule, from the segment's checkout, as it judges a heredoc body the shell runs. A here-string is the `<<<` operator outside quotes, and its word is the token after it, or the rest of the token when the operator begins one (`<<<'x'`), with its quotes removed. In a command whose quotes don't balance (`guard-quotes`), the word SHALL be every token after the operator to the segment's end. A here-string of any other segment SHALL stay a redirection's target, matching no rule.

#### Scenario: a push fed to a shell
- GIVEN the commands `bash <<< 'git push origin main'`, `sh <<<"git push origin HEAD:main"`, `sudo bash <<< 'cd . && git push --mirror origin'` and `bash <<< 'git push origin main` with no closing quote
- WHEN `clm hook guard` runs on each
- THEN each exits 2 naming `guard-blocks-push-to-main`

#### Scenario: an approval fed to a shell
- GIVEN the command `sh <<< "clm approve spec/x"`
- WHEN `clm hook guard` runs on it
- THEN it prints the `ask` decision naming `humans-review-intent`

#### Scenario: a here-string elsewhere
- GIVEN the commands `grep x <<< 'git push origin main'`, `bash -c 'cat > f' <<< 'git push origin main'` and `gh api graphql -F query=@- <<< 'clm approve spec/x'`
- WHEN `clm hook guard` runs on each
- THEN each exits 0 and prints nothing

### Requirement: guard-checkout
`clm hook guard` SHALL judge each segment from its *checkout*: the directory the guard starts from (`plugin.hooks.guard`: the event's `cwd`, or the guard's own working directory when the event has none), moved by each earlier segment that is `cd <dir>` and is ended by `&&`, `;` or a newline, with `<dir>` resolved against the checkout so far (`guard-command-position` scopes a `cd` inside `( )` or backticks), then by the segment's own `env -C` or `sudo -D` (`guard-command-position`). A `cd` segment ended by `|`, `|&`, `&` or `||` SHALL NOT move it. A `cd`'s options SHALL be its leading tokens that start with `-` and are not `-` alone, up to and including a `--`, and its directory the token after them. The guard SHALL read a `cd` in this order, as bash does:
1. A directory that contains `$`, `~` or a backtick SHALL leave the checkout at the directory the guard starts from, and is not expanded.
2. A directory that holds a `*`, `?` or `[` outside quotes SHALL be expanded against the file system, relative to the checkout so far, as bash's default globbing matches it (`*`, `?` and `[...]`, per path component, a wildcard never matching `/`, and a leading `.` matched only when the pattern writes it): one match SHALL be the directory; with none, the directory SHALL be the word as written; with several, bash's `cd` takes too many arguments, so the `cd` SHALL count as one with a token after its directory.
3. A `cd` with an option that is neither `--` nor a `-` followed by one or more of the letters `L`, `P` and `e` (`-P`, `-LPe`), or with a token after its directory, or whose directory doesn't exist or isn't a directory when the guard runs, SHALL leave the checkout where it was, as the shell's `cd` then fails and stays; the guard SHALL NOT work out what earlier segments of the command would create or switch to.
4. A `cd` with no directory, or whose directory is `-`, SHALL leave the checkout at the directory the guard starts from.

An `env -C` or `sudo -D` value that holds a `*`, `?` or `[` outside quotes SHALL be expanded in the same way, from the checkout so far: one match SHALL be the directory, and otherwise the value as written. In a segment whose command is `git` or ends in `/git` (`guard-command-position`), a `-C` before git's subcommand (its first argument that doesn't start with `-` and isn't the value of `-C`, `-c`, `--git-dir`, `--work-tree`, `--namespace` or `--config-env`) whose value is the next token and holds a `*`, `?` or `[` outside quotes SHALL be expanded in the same way, from the segment's checkout, and the walk SHALL yield, in that value's place among the segment's tokens, every match, sorted by its bytes as bash sorts them in the C locale, as bash passes them to git, or the value as written when there is none. A glued `-C<value>` (`-Cm*`) is one word to bash, which globs the whole word, so only an entry whose name starts with `-C` can match it; the walk SHALL expand it so, as the whole word, and yield every match in its place, or the word as written when there is none. When the guard has no directory to start from, the walk SHALL start with no checkout: a relative `cd`, `env -C` or `sudo -D` SHALL leave it with none and a relative glob SHALL stay as written, until an absolute `cd`, `env -C` or `sudo -D` sets one. When the last `cd` that set a segment's checkout, in the segment's scope, is one it couldn't follow, of any of these kinds, the guard SHALL keep that `cd` for the segment as written (such as `cd w`, `cd $W` or `cd`), for the rules that read it (`plugin.hooks.guard/push-guard`, `plugin.hooks.guard/push-guard-scoped-to-clm`), until a `cd` it follows to an absolute directory, or an absolute `env -C` or `sudo -D`, sets the checkout.

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

### Requirement: guard-standard-input
The guard's walk SHALL yield, with each piece, what feeds its standard input, as the shell decides it: the last of the segment's input redirections, read left to right outside quotes and heredoc bodies (a heredoc, a here-string `<<<`, or a file: `<` or `<>` naming one, or `<&` duplicating or closing a descriptor); with none, a pipe when a `|` or `|&` ends the segment before it in the same pipeline; otherwise the command's own standard input. An input redirection SHALL count only with no descriptor number written before its operator, or with `0`: one with any other number (`3< f`, `3<<EOF`) feeds another descriptor, and a `<(` opens a process substitution, an argument and no redirection. For a heredoc it SHALL yield the body as the shell sends it, the leading tabs of its lines removed for `<<-`, and whether the shell sends it as written: its word is quoted, wholly or in part, with `'`, `"` or a backslash, or its body holds no `$`, `` ` `` or `\`. A redirection inside a quoted substitution, a shell's `-c` script or a body judged as commands SHALL belong to the piece there that holds it, never to the outer segment.

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

#### Scenario: another descriptor
- GIVEN the commands `clm defect 3< f`, `clm defect 3<<'EOF'` followed by a line `r` and `EOF`, `clm defect <(echo r)` and `clm defect 0< f`
- WHEN the walk reads each
- THEN the `clm defect` piece's standard input is the command's own for the first three, and a file for the last

#### Serves
- capability/report-defects
