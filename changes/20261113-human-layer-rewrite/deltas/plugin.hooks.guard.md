---
component: plugin.hooks.guard
base: { frontmatter: "e5ffeca413a3c925" }
---

## MODIFIED Frontmatter

```yaml
serves: [capability/guard-sessions, capability/work-from-any-agent-harness, architecture/approval-policy, architecture/land-locally, architecture/agent-harness-adapters]
```

## MODIFIED Requirements

### Requirement: approve-guard <!-- base: 0d27c2be28cf45e6 -->
`clm hook guard` SHALL read a `PreToolUse` event from stdin and, for a `Bash` command, judge each segment of it separately. It SHALL answer with the `ask` permission decision on stdout and exit 0, with a reason that names `humans-review-intent` and the command, when no other guard rule blocks the command and a segment runs `clm approve`, `clm capabilities decide` or `clm architecture decide`, outside the heredoc bodies that `guard-heredocs` says match no rule. It SHALL NOT ask about reading a capability's or an architectural constraint's cost (`clm capabilities`, `clm capabilities <slug>`, `clm architecture`, `clm architecture <slug>`), about `clm review`, which is no command, about `clm land`, or about a `gh` call that adds or removes a label. For every other event, including input it can't parse, it SHALL exit 0 and print nothing.

A segment runs `clm` when its command (`guard-command-position`) is `clm` or ends in `/clm`, and `clm`'s arguments are the tokens after it. A segment is a `cargo run` when its command is `cargo` or ends in `/cargo` and its subcommand is `run` or `r`; the subcommand is the first token after `cargo` that doesn't start with `-` or `+` and isn't the value of `--color`, `--config`, `-C` or `-Z`. The guard SHALL NOT work out which package or binary a `cargo run` runs, or which of its tokens reach the binary (`guard-asks-any-cargo-run`): it SHALL treat a `cargo run` as running `clm approve` when one of its tokens after the subcommand is exactly `approve`, as running `clm capabilities decide` when one of them is exactly `capabilities` and a later one exactly `decide`, as running `clm architecture decide` when one of them is exactly `architecture` and a later one exactly `decide`, and as running `clm defect` when one of them is exactly `defect`. A cargo alias that runs `run` under another name isn't a `cargo run`. Every guard rule that looks for a `clm` call SHALL use these definitions.

#### Scenario: approving
- GIVEN the command `clm approve spec/x`
- WHEN `clm hook guard` runs on it
- THEN it prints the `ask` decision with a reason naming `humans-review-intent` and `clm approve`, and exits 0

#### Scenario: approving through cargo run
- GIVEN the commands `cargo run -q -p clm -- approve spec/x`, `cargo +stable run --package=clm -- approve spec/x`, `~/.cargo/bin/cargo r --bin clm -- approve x`, `CARGO_TERM_COLOR=never cargo run -pclm --release -- approve spec/x`, `cargo --color never run -p clm -- approve spec/x`, `cargo run -p clm --features f approve spec/x`, `cargo run -p=clm -- approve spec/x`, `cargo run -p clm@0.7.0 -- approve spec/x`, `cargo run --manifest-path crates/clm/Cargo.toml -- approve spec/x`, `cargo run -- approve spec/x`, `cargo run -p clm --artifact-dir d approve spec/x` and `cargo run -p clm-web -- approve spec/x`
- WHEN `clm hook guard` runs on each
- THEN each prints the `ask` decision with a reason naming `humans-review-intent`, and exits 0

#### Scenario: deciding
- GIVEN the commands `clm capabilities decide x --value high --decision keep`, `cargo run -q -p clm -- capabilities decide x --value low --decision cut`, `clm architecture decide x --value high --decision keep` and `cargo run -q -p clm -- architecture decide x --value low --decision cut`
- WHEN `clm hook guard` runs on each
- THEN each prints the `ask` decision with a reason naming `humans-review-intent` and the command, and exits 0

#### Scenario: cargo running something else
- GIVEN the commands `cargo run -q -p clm -- land spec/x`, `cargo run -q -p clm -- review x`, `cargo run -q -p clm -- capabilities x`, `cargo test -p clm -- approve`, `cargo build -p clm approve`, `cargo run -p clm`, `clm capabilities`, `clm capabilities x`, `clm architecture` and `clm architecture x`
- WHEN `clm hook guard` runs on each
- THEN each exits 0 and prints nothing

#### Scenario: review is no command
- GIVEN the commands `clm review x` and `clm review`
- WHEN `clm hook guard` runs on each
- THEN each exits 0 and prints nothing

#### Scenario: landing
- GIVEN the command `clm land spec/x`
- WHEN `clm hook guard` runs on it
- THEN it exits 0 and prints nothing

#### Scenario: a label is nobody's business
- GIVEN the command `gh pr edit 30 --add-label spec-approved`
- WHEN `clm hook guard` runs on it
- THEN it exits 0 and prints nothing

#### Scenario: approval combined with a push to main
- GIVEN the command `clm approve spec/x && git push origin main`
- WHEN `clm hook guard` runs on it
- THEN it exits 2, since the push block wins over `ask`

#### Scenario: quoted in a body
- GIVEN a `cat > body.md <<'EOF'` command whose body has a line `clm approve spec/x`
- WHEN `clm hook guard` runs on it
- THEN it exits 0 and prints nothing

#### Serves
- capability/guard-sessions
- capability/review-capability-cost
- architecture/approval-policy
- architecture/humans-review-intent
- architecture/land-locally

### Requirement: hook-budgets <!-- base: 0a955c815ff62856 -->
`clm-guard` SHALL offer `Budgets`, a push budget and a lookup budget, with `Budgets::HOOK` a push budget of 20 seconds and a lookup budget of 30 seconds, and `guard_within(event, budgets)`, which SHALL judge `event` as `guard` does, its `push-guard` queries sharing `budgets`' push budget and its `release-guard` lookups its lookup budget. `guard(event)` SHALL return what `guard_within(event, Budgets::HOOK)` returns, and `clm hook guard` SHALL judge through `guard`. No environment variable, setting or argument SHALL change the budgets `clm hook guard` uses: together they keep the hook under Claude Code's 60-second hook timeout, which lets the command through.

#### Scenario: the hook's budgets
- GIVEN `clm-guard`
- WHEN a test reads `Budgets::HOOK`
- THEN its push budget is 20 seconds and its lookup budget 30 seconds

#### Scenario: nothing shortens the hook's budgets
- GIVEN the workspace
- WHEN a test in `clm-guard` reads every file under `crates/clm-guard/src` and `crates/clm/src`
- THEN no file under `crates/clm/src` names `guard_within` or `Budgets`, and no file under `crates/clm-guard/src` outside a `//` comment calls `std::env::var` or `var_os` with a name holding `BUDGET`

#### Scenario: the hook judges with the hook's budgets
- GIVEN the workspace
- WHEN a test in `clm-guard` reads `crates/clm-guard/src/lib.rs`
- THEN the body of `guard` is one call of `guard_within` with `event` and `Budgets::HOOK`

#### Serves
- architecture/land-locally

### Requirement: guard-per-agent-harness <!-- base: b6263bcf82d5a2e5 -->
`clm-guard` SHALL offer `guard_hook(agent_harness, event)`, which reads the event with `agent-harness`'s `parse_event` for that agent harness, judges the event's shell command and `cwd` by the same rules whatever the agent harness, and returns the stdout and exit code `agent-harness`'s `render` gives for that agent harness, so an `ask` on an agent harness that can't confirm blocks the command. An event for another tool than the agent harness's shell tool SHALL be allowed, as a non-`Bash` event is today, and an event that can't be parsed SHALL return what the guard returns for one today. `guard` and `guard_within` SHALL return `agent-harness`'s `Decision`, and `ask_json` SHALL be `agent-harness`'s Claude Code rendering of an `Ask`, so each format is written once.

#### Scenario: push to main under Codex
- GIVEN a Codex pre-tool event for `git push origin main` in a repository under CLM
- WHEN `guard_hook` runs for `Codex`
- THEN it returns Codex's block, naming `guard-blocks-push-to-main`

#### Scenario: approve under Codex
- GIVEN a Codex pre-tool event for `clm approve spec/x`
- WHEN `guard_hook` runs for `Codex`
- THEN it returns Codex's block, its reason saying to run the command in the human's own terminal

#### Scenario: Claude Code unchanged
- GIVEN a Claude Code `PreToolUse` event for `clm approve spec/x`
- WHEN `guard_hook` runs for `ClaudeCode`
- THEN it returns the `ask` permission decision's JSON and exit 0

#### Needs
- agent-harness/hook-events
- agent-harness/abilities

#### Serves
- capability/work-from-any-agent-harness
- architecture/agent-harness-adapters
- architecture/land-locally
