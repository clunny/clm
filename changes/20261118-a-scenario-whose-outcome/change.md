---
status: applied
resolves: [a-scenario-whose-outcome-is-reversed-under-its-name-lands-on]
human_burdens:
  - About one spec branch in three that edits specs comes to need review (30 of the last 93 changed a scenario's outcome)
---

## Intent

The gate lists a scenario whose outcome changes under its name as a cut, so a spec branch that reverses what a scenario promises needs the human's yes, as one that removes the scenario does (`backlog/a-scenario-whose-outcome-is-reversed-under-its-name-lands-on`). Recorded as `a-changed-outcome-is-a-cut`, which amends `cuts-are-what-a-program-lists`.

## Problem

- `gate/cuts` lists a scenario only when its name is gone, and a requirement only when it states fewer `SHALL` sentences. `20261118-renaming-a-citing-test` reversed two scenarios named *a renamed test* ("returns `tests/t.rs::two`" to "returns nothing", "exits 1" to "lands"), so the land stopped refusing what it refused, and the gate printed `spec` with no cut.
- `removal-needs-a-yes` leaves such a reword to the human's answer in the session, but an unattended draft has no human to ask, and its prompt lands every `spec` branch on checks. The draft noticed only because the architecture audit said so, and left the change for review against the gate's line.

## Approaches

- **Taken: the gate lists a changed outcome.** A scenario of a requirement held on both sides, under the same name, whose lines from `- THEN` on differ, whitespace read as one space, is the cut `scenario "<name>" of <id> changes its outcome`. A program sees it in every branch, and it is the same reasoning that lists a renamed scenario: a program can't tell a reword from a reversal, so the human reads the line.
- **Rejected: a `change.md` key a session sets to ask for review the gate can't see.** It rests on the session noticing, the failure met here.
- **Rejected: comparing the whole scenario.** A changed `GIVEN` or `WHEN` under the same outcome promises the same thing of another case and drops nothing.
- **Rejected: listing only an outcome that loses a line or a word.** "exits 1" to "lands" loses nothing countable.

## Outcome

- `gate/cuts` (modified): a fourth kind of cut, a *changed outcome*, listed after the gone scenarios as `specs/<path>/SPEC.md: gate/cut: scenario "<name>" of <id> changes its outcome`; a scenario's *outcome* is defined; four scenarios added (*a reversed outcome*, *an outcome reflowed*, *only the setting changed*, *two scenarios of one name*). Since `cuts` returning anything makes a branch need review (`needs-review`, unchanged), such a branch needs review. `web.worker.review`, `host`, `check.health`, `land` and `orchestrator.run` print a cut through its `Display`, so they show the new line with no change of their own; no crate outside `clm-gate` matches on `Cut`.
- `gate/rules-version` (modified): `RULES` is 3; under version 2 a changed outcome makes no branch need review, under version 1 no cut does; scenario *a changed outcome under each version* added.
- `gate`'s Contract and Design name the new cut and version 3.
- `ARCHITECTURE.md`'s `removal-needs-a-yes` (its `#### Untested` text), `docs/decisions/cuts-are-what-a-program-lists.md`, `docs/decisions/yes-no-review.md`, `docs/guide/glossary.md` (**Cut**) and `docs/guide/how-it-works.md` name the new cut.

## Constraints

- No cut the gate lists today stops being listed, and the order of the existing kinds is kept, the new kind going between gone scenarios and shortened requirements.
- `cuts`, `cuts_or_unloaded` and `evaluate_by` keep their signatures; `Cut` gains a variant, which no crate outside `clm-gate` matches on.
- `cuts` still runs no process and builds for `wasm32-unknown-unknown`.

## Edge cases and workarounds

- A scenario moved to another requirement whose outcome changes on the way is not listed: a move is matched by name and count, not scenario to scenario. Accepted as the reader's, as a merged scenario is.
- A scenario renamed and reworded is listed once, as gone, not also as changed.
- A changed `GIVEN` or `WHEN` that turns the same outcome into a weaker promise is not listed; it stays a reader's.
- `plugin/skills/propose/SKILL.md` tells a session to keep a scenario's name when it rewords one; that stays true, and the gate's line now shows a reworded outcome too, so the skill needs no change.

## Open questions

- **The burden.** This makes about a third of spec branches that edit specs need review. The triage folds them into one approval, but if that is too many, the rejected key in `change.md` is the cheaper route, at the cost of resting on the session.
