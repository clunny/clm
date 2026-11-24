---
status: applied
resolves: [the-mutation-pass-can-t-judge-a-panel-requirement-whose-test]
---

## Intent

The mutation pass judges `plugin.panel`'s requirements again, and a requirement it keeps failing to judge reaches the backlog instead of only a line on stderr. `ci/mutate-script` gives StrykerJS no range of a vitest test file (`.test.ts`, `.test.tsx`), so a test file's top-level `vi.mock` stays where vitest needs it. `verify/files-unproven` files a `bug` item, `backlog/unjudged-<component>-<short sha>.md`, for each requirement two passes in a row left unjudged, which the build loop drafts like any bug.

## Problem

`backlog/the-mutation-pass-can-t-judge-a-panel-requirement-whose-test` (filed on `plugin.panel`): for every `plugin.panel` requirement cited by `plugin-panel/test/register.test.ts`, StrykerJS's first run failed with "1 call ... was defined outside of the module's top level scope: vi.mock()", `mutate.sh` died with `StrykerJS failed for requirement plugin.panel/<slug>`, and `clm verify` printed `verify passed` with the failure on stderr after it, three passes in a row.

The cause is not the panel's tests. `verify`'s plan puts every file a component owns on `code` lines, test files included, and `mutate.sh`'s `ts_selection` takes every `plugin-panel/*.ts` among them as code, so StrykerJS mutates the test files. Checked by hand at `0623e932` with StrykerJS 10.1.1 and the panel's own vitest: mutating `test/register.test.ts:1-60` fails the first run with exactly that message; `:1-20` and `hooks/register.tsx` don't. `web/src/*.test.ts` are mutated the same way, harmlessly so far, though a mutant of a test's helper is caught however little the test asserts and hides the code's survivors.

The fix lies in `ci` (`.github/scripts/mutate.sh`), not in the component the item names, and no requirement states it: `ci/mutate-script` says what StrykerJS mutates without excluding test files, so the change states it (`code-fix-needs-a-written-rule`).

The second half of the item: a failing command leaves its block and every block not started under the `mutated` record's `left`, those go first in the next pass, and the same block fails it again, so the pass judges almost nothing, pass after pass, while every outcome line reads `verify passed`. Nothing that a human or the loop reads says so.

## Approaches

- **Rewrite the panel's tests to `vi.doMock` before each dynamic import** (the item's first suggestion). Rejected: a workaround in one directory for a flaw of the script (`redesign-before-workaround`); the next test file that mocks a module meets it again, and test files would still be mutated.
- **StrykerJS mutates no test file** (taken). One rule in `mutate.sh`'s selection; it also keeps a test helper's mutants from hiding the code's survivors, as the Rust path already does.
- **Write `not-judged` for a block a tool fails on.** Rejected: the Design keeps a tool's failure out of the verdicts because a `not-judged` is never looked at again while the test is unchanged.
- **Change the outcome line or exit status when the pass fails.** Rejected: `mutation-after-land` keeps the pass from changing verify's outcome, and the loop treats a red verify as a broken `main`.
- **A `failed` field in the `mutated` record, and file only a tool's failure.** Rejected for now: a `ledger` change and a coupled sequence for a distinction the item's draft can read off the verify's own output. Without it a pass a human stopped twice files an item too; rare, and its draft accepts it.
- **File a `bug` item for a requirement two passes in a row left** (taken). Found from the ledger, as `unproven` items are, landed by the same filing branch. Two, not three: the second pass tried the requirement first.

## Outcome

- `ci/mutate-script`: StrykerJS is given no range of a `.test.ts` or `.test.tsx` file, which counts as neither kind of code file; new scenario *StrykerJS mutates no test file*. Every existing scenario keeps its name and outcome. `ci`'s Design says why.
- `verify/files-unproven`: the `unjudged` items, named after the commit of the second oldest record of the requirement's unbroken run of records that left it, so later passes that leave it again file nothing more; the `unproven: filed` line counts both kinds; new scenarios *left by two passes in a row* and *left by one pass*. Every existing scenario keeps its name and outcome. `verify`'s Contract and Design say so.
- `docs/decisions/mutation-after-land.md` gains a dated section.
- The `plugin.panel` requirements the item names are judged by the next pass after `ci`'s build lands, since they are still under `left`.

## Constraints

- `mutate.sh` is `ci`'s; `clm-verify` is `verify`'s. Neither build touches `plugin-panel/`.
- The pass still changes no outcome line, stdout or exit status (`mutation-after-land`).
- An `unjudged` item names no `unproven` entries, so it doesn't make its requirements read as not built.

Edge cases known now: a pass stopped by a human twice in a row files an item (accepted above); a requirement under `left` that `origin/main` no longer has is not filed.

## Open questions

None.
