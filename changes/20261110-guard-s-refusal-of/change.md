---
status: applied
from: backlog/guard-s-refusal-of-a-piped-clm-land-doesn-t-say-the-whole
resolves:
  - guard-s-refusal-of-a-piped-clm-land-doesn-t-say-the-whole
---

## Intent

Make every refusal of `clm hook guard` end by saying that the whole command line was refused and nothing in it ran, so a session that chained work before a refused command (`git commit ... && clm land <branch> | tail`) knows to run that work again. New requirement `plugin.hooks.guard/block-refuses-whole-command`.

## Problem

A `PreToolUse` hook's exit 2 makes Claude Code cancel the whole Bash tool call. Each guard rule's reason speaks only of the segment it judged: the piped-land refusal names the land and the plain command to run instead. A session that ran `git commit -m ... && clm land impl/x | tail` read that as "the land was refused" and believed the commit had run; it hadn't, and the work was lost from the land that followed. No requirement says what a refusal tells the session about the rest of the command.

## Approaches

1. **One closing line on every block (chosen).** The guard joins every rule's reason into one `Block` in one place (`clm-guard`'s `guard_within`), so one line appended there covers every rule, the push, release, defect and piped-land refusals alike, each of which cancels chained commands the same way. It is one sentence of code and one requirement, and needs no rule to know what else the command held.
2. **Reword only `piped-land-guard`'s reason.** Rejected: a `git commit && git push origin main` loses its commit the same way, and four rules each carrying the same sentence is duplication the single join point avoids (`reuse-first`).
3. **Name the other segments the guard saw** ("`git commit -m m` did not run"). Rejected (`yagni`): the guard would have to quote `-c` scripts, heredocs and wrappers piece by piece, while the one fact the session lacks is that nothing ran.
4. **Let the rest of the command run.** Impossible from a hook: Claude Code gives it no way to run part of a tool call.

## Outcome

- `plugin.hooks.guard/block-refuses-whole-command` (added): a refusal's reason ends with one line saying the whole command line was refused and nothing in it ran; once, however many rules refused; never on an allow or an ask.
- Code (built by `plugin.hooks.guard`): in `crates/clm-guard/src/lib.rs`, append the line where `guard_within` returns `Decision::Block(reasons.join("\n"))`; tests for the three scenarios in `crates/clm-guard/tests/guard.rs` (which call `guard` directly; no fake binaries are needed).
- Every existing rule's reason, the Contract, the Design and the decisions stand: the scenarios that check a reason check what it names, not its whole text.

## Edge cases

- The `defect-ask-shows-report` refusal (a report the guard can't show) is a block too, and gets the line: its command did not run either.
- A refusal whose command is a single segment still gets the line; saying it costs a line, and telling single from chained would need the walk's pieces counted, for no gain.

## Constraints

Nothing here needs a human: no burdens, no human layer touched, so it lands on checks.

## Open questions

None.
