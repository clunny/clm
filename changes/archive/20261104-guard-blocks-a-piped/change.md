---
status: done
resolves:
  - guard-blocks-a-piped-clm-hashes-land-as-a-piped-land
---

# Guard reads a cargo run's land after its `--`

## Intent

`cargo run -q -p clm -- hashes land | grep x` runs `clm hashes` with the component `land` and lands nothing, yet `clm hook guard` refuses it under `guard-refuses-piped-land`. A `cargo run` that has a `--` hands `clm` the tokens after it, so the guard should count such a segment as running `clm land` only when `land` is the first of them, the same test it applies to a plain `clm` call.

## Problem

`plugin.hooks.guard/piped-land-guard` says a `cargo run` runs `clm land` when "a token after the subcommand" is exactly `land`, and `clm-guard`'s `runs_land` does just that, so the code follows the rule and the rule is too wide: a component, branch or file named `land` anywhere on the line counts. `land` is a real component ID in this repository, so `clm hashes land`, `clm context land` and `clm pending land`, which drafts run all the time through `cargo run` (`CLAUDE.md`), are refused when piped. A draft hit it and had to send the output to a file instead. No requirement states the behaviour the fix restores, so it changes `specs/` (`code-fix-needs-a-written-rule`).

## Approaches

- **Read `clm`'s subcommand after cargo's `--` (chosen).** This repository's documented form, `cargo run -q -p clm -- <subcommand>`, always has a `--`, and everything after it reaches the binary unchanged, so its first token is `clm`'s subcommand (`clm` has no global options that could come before it). That needs no list of cargo's options or spellings, which `guard-asks-any-cargo-run` rejected, only the position of one fixed token. A `cargo run` with no `--` keeps today's reading, any token `land`, since there the guard can't tell cargo's tokens from the binary's without such a list.
- **Accept the refusal.** Rejected: the reason it gives (a land's verdict hidden) is false for a command that lands nothing, and the refused commands are ones drafts need; a guard that refuses harmless commands teaches sessions to read its refusals as noise (`right-over-convenient`).
- **Require the `--`, and ignore a `cargo run` without one.** Simpler, but it would let `cargo run -p clm land spec/x | tail` through, a land the rule covers today, to save one branch of a condition. Rejected.
- **Apply the same reading to `approve-guard` and the defect rule.** Out of scope: those rules ask rather than block, and `guard-asks-any-cargo-run` deliberately takes asking too often as the safe side of a backstop, at the cost of one confirmation. No false ask has been reported (`yagni`).

## Outcome

`piped-land-guard` says where a `cargo run`'s `land` token is, and gains a scenario for `land` as another subcommand's argument plus a `cargo run` with no `--` that must stay blocked. `approve-guard`'s definition of a `cargo run`, the decisions `guard-refuses-piped-land` and `guard-asks-any-cargo-run`, and the Contract stand. Building it changes the `CargoRun` arm of `runs_land` and cites the new version in the guard's tests.

## Edge cases

- `cargo run -p clm hashes land | grep x`, with no `--`, stays blocked: the fallback can't tell `land` from cargo's own tokens. The documented form has the `--`, and the block names the plain command, so a retry costs one call.
- Only the first `--` after the subcommand counts, so `cargo run -q -p clm -- land -- x | tail` is still a land, and `cargo run -q -p clm -- hashes -- land | tail` passes.
- The `--help` exemption reads the tokens after the `land` token as before, so `cargo run --help -- land | tail` stays blocked.

## Constraints

Nothing here needs a human: no burdens, no human layer touched, so it lands on checks.

## Open questions

None.
