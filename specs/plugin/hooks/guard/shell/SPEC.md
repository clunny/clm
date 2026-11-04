---
title: Guard shell reading
status: active
owners: ["@clunny"]
depends_on: []
paths: ["crates/clm-guard/src/shell/**", "crates/clm-guard/tests/shell.rs"]
successor: ""
serves: [capability/guard-sessions, architecture/guard-blocks-push-to-main]
---

## Contract
How the guard (`plugin.hooks.guard`) reads a Bash command as the shell would, before any rule judges it: where the command splits into segments and tokens (`guard-quotes`), which token is a segment's command (`guard-command-position`), which other text runs as commands (a heredoc body or here-string a shell reads, a shell's `-c` script, a substitution inside double quotes: `guard-heredocs`, `guard-here-strings`, `guard-shell-scripts`, `guard-quoted-substitutions`), and the checkout each piece runs in (`guard-checkout`). It decides nothing: it yields, in order, the pieces to judge, each with its text, its tokens, its checkout, the last `cd` that set that checkout when the guard couldn't follow it, and what feeds its standard input (`guard-standard-input`), and the guard's rules judge them. Its requirements are stated as `clm hook guard`'s answers because that is where they show, except `guard-standard-input`, which only the defect ask (`plugin.hooks.guard/defect-approval`) reads, so it is stated as what the walk yields.

## Design
Module `shell` of crate `clm-guard`, at `crates/clm-guard/src/shell/` (`guard-split`): `scan.rs` (segments, tokens, heredoc bodies, here-strings and quoted substitutions), `command.rs` (the command position, `env -S`, a shell's `-c` script, whether a segment's command reads standard input) and `walk.rs` (the pieces to judge, in order, each with its checkout), declared by `shell/mod.rs`. The module exports to the crate (`pub(crate)`) the walk, the piece it yields (its text, its tokens, its checkout and the `cd` kept for it: `Judged`) and `command`'s parsed `Command`, which the rules read. The guard's `lib.rs` declares the module and calls the walk once per event; nothing outside `clm-guard` sees it. It runs no process, and reads nothing outside the command but the file system, to tell whether a `cd`'s directory exists. Its tests, `crates/clm-guard/tests/shell.rs`, call `clm_guard::guard` on `PreToolUse` events as `clm hook guard` does and cite only this component's requirements: a scenario reads a rule's answer (a push to `main` blocked, an approval asked about) from the guard. The scenarios of `guard-standard-input` read the walk's pieces directly, so they are tested in unit tests inside `walk.rs`, which lies in this component's `paths`.

The guard (`guard-matches-writes`, `guard-reads-quotes-and-wrappers`) scans the command once, finding quotes and heredocs in the same scan (`guard-quotes`), and each segment keeps the separator that ended it: `&&`, `||`, `|`, `|&`, `&`, `;` or a newline, the command's end counting as a newline. Splitting at `(`, `)` and the backtick makes a subshell, a group or a command substitution (`(git push ...)`, `out=$(git push ...)`) segments of its own. A `$( )` inside double quotes stays part of its quoted token, but its text is also judged as commands (`guard-quoted-substitutions`), so `git commit -m "$(cat <<'EOF' ... EOF)"`, the usual way a session writes a commit message, keeps its body's prose out of the segments however many `"` it holds, and `echo "$(git push origin main)"` is judged as the push it runs. The substitution's text stays in the outer token as written, so a rule that reads that token, such as a GraphQL mutation's name in `-f query="$(cat <<'EOF' ... EOF)"`, sees it. A `)` with no `(` inside the substitution, as in a `case` pattern, ends it early, and a backtick inside double quotes stays part of its quoted token: both are gaps.

Several heredocs on one line take their bodies in order. A body judged as commands (`guard-heredocs`) is split as any command is, so its own heredocs and pipes follow the same rules; it starts at the checkout of the segment whose shell runs it (for a piped body, the shell's segment), and a `cd` inside it moves only the body's later segments, never the command's. A here-string fed to a shell over its standard input, as in `bash <<< 'git push origin main'`, is judged as a body is (`guard-here-strings`); fed to any other command it stays a redirection's target, so `cat <<< '...' | bash`, like `echo '...' | bash`, is a gap: the guard judges only heredoc bodies through a pipe.

A segment's *command* is found by `command.rs` (`guard-command-position`); every rule that speaks of a segment's first token means that command. A shell run with `-c` has its script, one token, split into segments and judged from the segment's checkout (`guard-shell-scripts`), as a heredoc body fed to a shell is.

**Checkouts** (`guard-checkout`). A `cd` ended by `|`, `|&` or `&` runs in a subshell or in the background, and one ended by `||` lets the next segment run only when it failed, so none of them moves the checkout. A `cd` the walk can't resolve leaves the checkout at the event's `cwd`, and one whose directory doesn't exist when the guard runs, as when an earlier segment of the same command creates it (`git worktree add <dir> -b <branch> && cd <dir>`), leaves it where it was, so a rule is never less guarded than without the `cd`. A kept `cd` survives a later relative `cd` the walk follows (only an absolute one drops it), since a directory relative to a checkout the walk fell back to is no surer than that checkout. The walk yields each segment, then the quoted substitutions it holds (from the checkout before its own `env -C` or `sudo -D`), then the bodies and here-strings its shell reads and its `-c` script (from its own checkout), then the bodies it pipes to a later shell of the same pipeline (from that shell's checkout); a `cd` inside any of these moves only that text's later segments.


**Standard input** (`guard-standard-input`). The scan already finds each heredoc operator and its word; it keeps, beside the body, whether the word was quoted, and notes each input redirection (`<`, `0<`, `<>`, `<<<`, a heredoc) in the order written, so the walk can say which one wins. Only the source is given, never its contents: the guard reads no file to tell what a `<` redirection sends.

## Requirements

### Requirement: guard-quotes
`clm hook guard` SHALL scan a command once from left to right, as the shell quotes: a `'` opens a quote that only the next `'` closes, a `"` opens one that the next `"` not escaped by a backslash closes, a `$'` outside quotes opens an ANSI-C quote that the next `'` not escaped by a backslash closes, a backslash outside single quotes escapes the next character, and a quote opened on one line may close on a later one. Inside double quotes, a `$(` opens a substitution that runs to its matching `)`: inside it the scan SHALL quote, count `(` and `)` and find heredocs as it does outside any quote, so a `"` inside it opens a quote of its own instead of closing the outer one, and the substitution, with the double quotes around it, stays part of one token. That token SHALL hold the substitution's text as written, from `$(` to `)`, with no quotes removed and any heredoc body inside it included, and a heredoc found inside it SHALL belong to the substitution's segments (`guard-quoted-substitutions`), not to the outer segment's bodies. It SHALL find heredoc operators (`guard-heredocs`) with this scan, and a heredoc body's text SHALL NOT count toward quoting. It SHALL split the command into segments at `|`, `;`, `&`, `(`, `)`, `` ` `` and newlines only outside quotes and outside heredoc bodies, and each segment into tokens at whitespace outside quotes, removing only the quote characters and backslashes that quote (an ANSI-C quote's `$'` and closing `'`, and the backslash of its `\'` and `\\`), so a quoted string with spaces is one token and a `"` inside `'...'` stays in it. The quotes *don't balance* when the scan ends inside a quote or a substitution; the guard SHALL then split that command's segments and tokens, outside the heredoc bodies the scan found, at every separator and space as if it had no quotes, with every quote character removed. The `&` of `>&`, `<&` and `&>` SHALL NOT separate.

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

### Requirement: guard-heredocs
`clm hook guard` SHALL treat a heredoc's body as part of the segment that opened it, not as segments of its own, for every guard rule. The heredoc operator is `<<` or `<<-` outside quotes (the text of a substitution `guard-quotes` finds inside double quotes counting as outside) and not followed by `<`, then optional spaces and a word, bare or in single or double quotes. The body is the lines after the operator's line up to the first line that is the word (after leading tabs for `<<-`). An operator with no such line SHALL NOT start a heredoc. A segment's `clm`, `gh` and `git` calls SHALL be found only in the segment's own line, and its body SHALL match no guard rule. When that segment runs a shell over its standard input, it SHALL also judge the body's lines as segments. A segment runs a shell over its standard input when its command (`guard-command-position`) is `bash`, `sh`, `zsh`, `dash` or `ksh`, or ends in `/` and one of them, and no short option of the shell (a token after it that starts with a single `-`, among those up to the first token that doesn't start with `-`) holds the letter `c`; or when its command is `source` or `.` and the next token is `/dev/stdin`, `/dev/fd/0` or `/proc/self/fd/0`. When a segment with heredoc bodies is ended by `|` or `|&`, the guard SHALL also judge its bodies' lines as segments when a later segment of the same pipeline, the segments joined to it by `|` or `|&`, runs a shell over its standard input and has no heredoc body of its own. Body lines judged as segments SHALL follow every guard rule, these included, as a command's lines do, starting from the checkout of the segment whose shell runs them; a `cd` among them SHALL NOT move the command's later segments.

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

### Requirement: guard-command-position
Wherever a guard rule (`push-guard`, `guard-heredocs`, `approve-guard`, `defect-approval`, and the `cd` segments `push-guard` follows) speaks of a segment's first token after any `NAME=value` assignments and `env`, `clm hook guard` SHALL take the segment's *command*: its first token after skipping, repeatedly and in any order:
- `NAME=value` assignments;
- the words `!`, `{`, `if`, `then`, `elif`, `else`, `do`, `while` and `until`;
- `time`, with a following `-p`;
- `timeout`, with the tokens after it that start with `-`, the value of its `-s`, `-k`, `--signal` and `--kill-after`, and then its duration;
- `env`, `sudo`, `doas`, `command`, `exec`, `nohup`, `nice`, `ionice`, `stdbuf` and `xargs`, each with the tokens after it that start with `-`, and with the token after each of these options that takes a value: `env`'s `-u`, `-C`, `--unset` and `--chdir`; `sudo`'s `-u`, `-g`, `-h`, `-p`, `-C`, `-D`, `-r`, `-t`, `-U`, `--user`, `--group`, `--host`, `--prompt`, `--chdir`, `--close-from`, `--role`, `--type` and `--other-user`; `doas`'s `-u` and `-C`; `nice`'s `-n` and `--adjustment`; `ionice`'s `-c`, `-n` and `-p`; `exec`'s `-a`; `stdbuf`'s `-i`, `-o` and `-e`; `xargs`'s `-I`, `-n`, `-P`, `-L`, `-s`, `-d`, `-a` and `-E`; and `env`'s `-S` and `--split-string`.

A token after one of these prefix commands, or after `timeout`, that starts with a single `-` and holds more than one letter is a cluster of short options: the first of its letters that is a short option of that command taking a value SHALL take the rest of the token as its value, or, when it is the token's last letter, the next token. So `sudo -Eu root git` and `sudo -uroot git` both run `git`, and a `-C` or `-D` that moves the checkout counts in a cluster too. `env`'s `-S`/`--split-string` value, alone, glued (`-S<value>`) or after `=`, SHALL be split into tokens as `guard-quotes` splits a segment, and those tokens SHALL be read in the option's place as `env`'s further arguments, so `env -S 'git push origin main'` runs `git`. Where `release-guard` and `push-guard` find a `gh` call anywhere in the segment, they SHALL look among the segment's tokens with each such value split in place, so `env -S 'gh release create v1'` runs `gh`.

`release-guard` and `push-guard` SHALL still find a `gh` call as the first token anywhere in the segment that is `gh` or ends in `/gh`, which already covers these prefixes.

The assignments a rule reads for a segment (`GIT_DIR`, `GIT_WORK_TREE`, `GH_REPO`, `GH_HOST`) SHALL be every `NAME=value` among the skipped tokens, the last one winning. `env`'s `-C`/`--chdir` and `sudo`'s `-D`/`--chdir` SHALL move the checkout of that segment's command as a `git -C` does.

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
`clm hook guard` SHALL judge the text of each substitution that `guard-quotes` finds inside double quotes, between its `$(` and its matching `)`, as commands by every guard rule, from the checkout of the segment it is in as the segment's earlier `cd`s leave it, before that segment's own `-C`, `env -C` or `sudo -D`, since the shell expands it before the command runs; it judges it as it judges a shell's `-c` script (`guard-shell-scripts`). A substitution inside one SHALL be judged too, and a `cd` among its segments SHALL NOT move the command's later segments.

#### Scenario: a push in a quoted substitution
- GIVEN the commands `echo "$(git push origin main)"` and `x="out: $(cd . && git push origin HEAD:main)"`
- WHEN `clm hook guard` runs on each
- THEN each exits 2 naming `guard-blocks-push-to-main`

#### Scenario: an approval in a quoted substitution
- GIVEN the command `echo "$(clm approve spec/x)"`
- WHEN `clm hook guard` runs on it
- THEN it prints the `ask` decision naming `humans-review-intent`

#### Scenario: prose in a quoted substitution
- GIVEN the commands `git commit -m "$(printf 'a; git push origin main')"` and `gh pr create --title t --body "$(cat <<'EOF'`, a line `clm approve spec/x`, a line `EOF`, and a line `)"`
- WHEN `clm hook guard` runs on each
- THEN each exits 0 and prints nothing

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
`clm hook guard` SHALL judge each segment from its *checkout*: the event's `cwd`, moved by each earlier segment that is `cd <dir>` and is ended by `&&`, `;` or a newline, with `<dir>` resolved against the checkout so far (`guard-command-position` scopes a `cd` inside `( )` or backticks), then by the segment's own `env -C` or `sudo -D` (`guard-command-position`). A `cd` segment ended by `|`, `|&`, `&` or `||` SHALL NOT move it. `&&`, `||` and `|&` are each one separator, and an `&` in a redirection (`2>&1`, `>&2`, `&>f`) ends no segment, so `cd w 2>&1 && git push` moves the checkout. A `cd` with no argument, or whose directory is `-` or contains `$`, `~` or a backtick, SHALL leave the checkout at the event's `cwd`, and a `cd` whose directory doesn't exist or isn't a directory when the guard runs SHALL leave it where it was; the guard SHALL NOT work out what earlier segments of the command would create or switch to. When the last `cd` that set a segment's checkout, in the segment's scope, is one it couldn't follow, of either kind, the guard SHALL keep that `cd` for the segment as written (such as `cd w`, `cd $W` or `cd`), for the rules whose reasons name it (`push-guard`), until a `cd` it follows to an absolute directory, or an absolute `env -C` or `sudo -D`, sets the checkout.

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

### Requirement: guard-standard-input
The guard's walk SHALL yield, with each piece, what feeds its standard input, as the shell decides it: the last of the segment's input redirections, read left to right outside quotes and heredoc bodies (a heredoc, a here-string `<<<`, or a file named by `<`, `0<` or `<>`); with none, a pipe when a `|` or `|&` ends the segment before it in the same pipeline; otherwise the command's own standard input. For a heredoc it SHALL yield the body and whether the shell sends it as written: its word is quoted, wholly or in part, with `'`, `"` or a backslash, or its body holds no `$`, `` ` `` or `\`. A redirection inside a quoted substitution, a shell's `-c` script or a body judged as commands SHALL belong to the piece there that holds it, never to the outer segment.

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
