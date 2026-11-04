---
status: applied
resolves:
  - guard-treats-a-piped-clm-land-help-as-a-land
---

# Guard lets a piped `clm land --help` through

## Intent

`clm land --help | head` lands nothing and has no verdict line, yet the guard refuses it under `guard-refuses-piped-land` and gives as its reason a verdict the command doesn't print. The rule should judge only a command that lands, so a segment whose arguments ask for help passes.

## Problem

`plugin.hooks.guard/piped-land-guard` blocks a piped segment running `clm land` "with any arguments", and `clm-guard`'s `runs_land` reads no argument past `land`. So the written rule and the code agree, and the refusal follows the rule; the rule is too wide. No requirement states the behaviour the item asks for, so its fix changes `specs/` (`code-fix-needs-a-written-rule`). This draft hit the refusal itself, reading `clm land --help` through `sed`.

## Approaches

- **Exempt the help flags in the rule (chosen).** A segment runs `clm land` only when no token after `land` is exactly `--help` or `-h`. The rule exists to protect a land's verdict (`docs/decisions/guard-refuses-piped-land.md`), and a help request has none, so this narrows the rule to its own purpose, with one condition on tokens the guard already holds. It is correct, not only cheap: clap prints the help and exits whatever else is on the line, so no command the exemption lets through lands.
- **Accept the refusal.** The block already names the plain command, and a retry costs one call. Rejected: the reason it gives is false for this command, and a session that needs the help of a command it is about to run is likely to read it this way; a guard that refuses harmless commands teaches sessions to read its refusals as noise. Keeping a known false refusal to save one condition is the convenient answer, not the right one (`right-over-convenient`).
- **Exempt every read-only form.** Rejected: `--check-only` is the mistake the rule was written for, since it does print a verdict, and no other read-only form of `clm land` exists (`yagni`).
- **Point the session at `clm help land`.** It already passes, since its first argument isn't `land`, but a session's reach for `--help` is the normal one, and a sentence in the reason wouldn't make the refusal true.

## Outcome

`piped-land-guard` gains the exemption and two scenarios, one for what now passes and one for what must stay blocked. The Contract's summary ("a `clm land` whose output is piped") and the decision's reasoning stand, so neither changes. Building it adds the condition to `runs_land` or beside it, and cites the new version in the tests.

## Edge cases

- The match is on an exact token after `land`, so `clm land spec/--help | tail` stays blocked. In a `cargo run`, a `--help` before `land` is cargo's own and doesn't exempt the segment.
- `clm land -- --help | tail` passes though it asks for no help: clap reads `--help` as the branch, git refuses a branch name starting with `-`, and nothing lands. `clm land --root --help | tail` also passes, and clap refuses it as a usage error. Neither is worth a clap parser in the guard (`guard-reads-quotes-and-wrappers`: the guard is a backstop for accidents).
- Only the piped segment's own tokens count: `echo --help | clm land spec/x | tail` stays blocked.

## Constraints

Nothing here needs a human: no burdens, no human layer touched, so it lands on checks.

## Open questions

None.
