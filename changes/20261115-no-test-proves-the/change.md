---
status: applied
resolves: [no-test-proves-the-settings-clm-rebase-loads]
---

## Intent

State, as a requirement of the root, the settings `crates/clm` hands `clm rebase`, so a test that runs the binary proves them: `clm/rebase-settings`.

## Problem

`sync/commands-in-crate` moved sync's tests into `clm-sync`. The tests that ran the binary on a `clm.toml` it can't load (`[component_size] spec_tokens = 0`) and on `[health] reconsider = 2` now call `clm-sync`'s `rebase_command` with a settings closure the test writes. The closure the product passes is `sync_settings` in `crates/clm/src/main.rs`: it loads `config` from the CLM root it is given and formats a load error as `<source>: check.health/health-report: <error>`. Only `sync`'s Design states that, and no requirement does, so no test cites it and nothing proves it: a mutant of `sync_settings` that loads the defaults, or drops the rule from the line, passes every test.

The work isn't a code-only fix. `sync/judged-by-check`'s scenario `settings can't be loaded` states what `clm rebase` prints, but `clm-sync`'s own test proves it, and a root test that cites a `sync` requirement is the kind `clm/routes-only` takes out of the root's files. The root's Design already says a scenario that only the binary can prove is the root's (`commands-live-in-their-crates`), so the rule goes to the root (`code-fix-needs-a-written-rule`).

## Approaches

- **A requirement of the root, proven by running the binary.** Taken. The code it proves is the root's, the test sits in the root's own files and cites the root, and it needs nothing pending, so it builds now. It sits beside `rebase-warnings-prefixed`, the root's other rule about what it adds to `clm rebase`.
- **A scenario on `clm/routes-only`.** Not taken: that requirement waits on 23 builds and nine backlog items, so the proof would wait with it, and its rule is that the root adds nothing, while this one is what the root adds.
- **A root test citing `sync/judged-by-check`.** Not taken: it states no part of `sync_settings` (the rule name in the line, the root it loads from), and `routes-only` would move it out again.
- **Accept the gap.** Not taken: a passing check must mean the code does what the spec says, and the formatting is the only place the line's rule name comes from.

## Outcome

- **`clm/rebase-settings`**: the root hands `clm-sync` the settings `config` loads from the CLM root `clm-sync` gives it, `health` among them, and a load error as a `check.health/health-report` violation line. Two scenarios, each run as a process: a `clm.toml` that can't load, and a repository threshold of 2 that files a reconsider item the default of 3 wouldn't. Each THEN states only what the root contributes (the line's source and rule, the threshold it loaded); the outcome line and exit status stay `sync`'s to state (`sync/judged-by-check`, `sync/file-reconsider-item`), as the architecture audit advised, so no rule of `clm rebase`'s output is stated by two components.

## Constraints

- No change to code is expected: `sync_settings` does this today. The build writes the two tests, in `crates/clm/tests/sync.rs`, which already runs `clm rebase` as a process.
- The tests run with no user `config.toml` and no `clm.local.toml`, so the machine's settings never decide them.
- Edge cases and workarounds known: none.

## Open questions

None.
