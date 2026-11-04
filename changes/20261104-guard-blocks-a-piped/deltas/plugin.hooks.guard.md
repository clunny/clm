---
component: plugin.hooks.guard
---

## MODIFIED Requirements

### Requirement: piped-land-guard <!-- base: 8b9ca13f0972dd1b -->
`clm hook guard` SHALL block (exit 2) a `Bash` command in which a segment that runs `clm land`, with any arguments, `--check-only` among them, pipes its output to the next command: the separator that ends the segment, as `guard-quotes` reads it, is `|` or `|&`. A segment runs `clm land` when its `clm` call (`approve-guard`) is `clm` with `land` as its first argument, or a `cargo run` whose `land` token is `land`, and no token after that `land` is exactly `--help` or `-h`: that prints the help and lands nothing, so it has no verdict for a pipe to hide. A `cargo run`'s `land` token is the first token after the first `--` that follows the subcommand, since cargo hands the binary the tokens after that `--` and `clm` reads its first argument as its subcommand; in a `cargo run` with no such `--`, it is the first token after the subcommand that is exactly `land`, since the guard doesn't work out which of those tokens are cargo's (`guard-asks-any-cargo-run`). The rule SHALL judge every piece the guard judges, a `-c` script's and a heredoc body a shell runs included, and SHALL NOT judge a heredoc body that `guard-heredocs` says matches no rule. The block's reason SHALL name `guard-refuses-piped-land` and the plain command to run instead, the segment's tokens joined by spaces (its redirections left out), and SHALL say why: a pipe reports the last command's result rather than the land's and cuts its output, while the land's last line is its verdict and Claude Code keeps a long output whole. It SHALL NOT block a `clm land` that is not piped, whatever redirections it has, nor one whose only pipe is the one it reads its standard input from (`a | clm land | b` pipes its output, and is blocked), nor a pipe from any other `clm` command, a `land` that is an argument of another subcommand included.

#### Scenario: check piped into tail
- GIVEN the commands `clm land --check-only | tail -5`, `clm land --check-only 2>&1 | tail -20`, `clm land impl/x |& tail`, `cargo run -q -p clm -- land --check-only | tail -5` and `cargo run -p clm land spec/x | tail`
- WHEN `clm hook guard` runs on each
- THEN each exits 2 with a reason naming `guard-refuses-piped-land`, `last line` and the plain command (`clm land --check-only` for the first two, `clm land impl/x` for the third)

#### Scenario: piped inside a script
- GIVEN the commands `bash -c 'clm land --check-only | tail'` and `cd /tmp && clm land spec/x | tee land.log`
- WHEN `clm hook guard` runs on each
- THEN each exits 2 with a reason naming `guard-refuses-piped-land`

#### Scenario: plain land
- GIVEN the commands `clm land --check-only`, `clm land spec/x`, `clm land --check-only 2>&1` and `clm land --check-only && git status`
- WHEN `clm hook guard` runs on each
- THEN each exits 0 and prints nothing

#### Scenario: other pipes
- GIVEN the commands `clm status | head`, `echo y | clm land spec/x`, `cargo run -q -p clm -- status | head` and a `cat > body.md <<'EOF'` command whose body has a line `clm land --check-only | tail`
- WHEN `clm hook guard` runs on each
- THEN each exits 0 and prints nothing

#### Scenario: land as another subcommand's argument
- GIVEN the commands `cargo run -q -p clm -- hashes land | grep x`, `cargo run -q -p clm -- context land | head`, `cargo run -p clm -- pending land |& head` and `clm hashes land | grep x`
- WHEN `clm hook guard` runs on each
- THEN each exits 0 and prints nothing

#### Scenario: help piped
- GIVEN the commands `clm land --help | head`, `clm land -h | head`, `clm land impl/x --help |& head` and `cargo run -q -p clm -- land --help | head`
- WHEN `clm hook guard` runs on each
- THEN each exits 0 and prints nothing

#### Scenario: help elsewhere than the land's arguments
- GIVEN the commands `clm land spec/--help | tail`, `echo --help | clm land spec/x | tail` and `cargo run --help -- land | tail`
- WHEN `clm hook guard` runs on each
- THEN each exits 2 with a reason naming `guard-refuses-piped-land`

#### Serves
- capability/land-safely
