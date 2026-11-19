---
status: done
resolves: [an-obsolete-requirement-nobody-can-clear-keeps-its-component-ready]
---

## Intent

A removed requirement or capability whose test a build can neither delete nor re-cite no longer keeps its component ready for ever. The build's `question` item may name the removed requirement in `blocks`, once the change that removed it is `applied` (or `implementing`), so the obsolete line waits for the human's answer like any blocked requirement. And a removed capability's test in another component's file is placed and checked as a removed requirement's now is (`20261119-a-removed-requirement-s-tests-elsewhere`): on the file's owner, with the change made to list it. It answers `backlog/an-obsolete-requirement-nobody-can-clear-keeps-its-component-ready`, the two cases that change left. It is part of phase 5's pruning (`backlog/a-passing-check-proves-less-than-the-spec-claims`), which removes requirements by the hundred, so it is the plan's work and not an exception to the freeze.

## Problem

Since `land/deleted-tests-approved`, a build deletes a removed requirement's test only when an approved `deleted-tests.txt` names it, and otherwise re-cites it. Where it can do neither, its prompt says to leave the test and add a `question` item, but `model/parse-backlog` rejects a `blocks` entry naming a requirement the spec doesn't hold. So the question holds nothing back: the line stays `obsolete` and unblocked, the queue offers the component after every build that changes nothing, and the removing change never archives. Two cases are left:

- a test in the requirement's own files that no list names and that no requirement states any more, because the removing change forgot the line. Deleting it is a cut, so the human's yes is rightly needed;
- a removed capability's test in a file another component owns: `check.coverage/removed-capabilities` places it on the `capability` component, whose build `clm land` refuses the edit, and `check/obsolete-tests-elsewhere` leaves capabilities out.

## Approaches

- **Taken: a `question` may block a requirement an `applied` change removed, or a capability it removed.** `pending` already prints an item's block on any line, obsolete ones included; only the model refused the item. The human's answer is a spec change that lists the test or states what it proves and resolves the question, which `clm apply` deletes.
- **Taken: only once the change is `applied` or `implementing`.** A change still `proposed` or `approved` that removes a requirement an item blocks keeps failing `clm apply` (`apply/resulting-graph`), so an old question about building the requirement can't survive its removal unseen. Once the change is archived, the entry names nothing and is rejected as before; it can't be archived while the held-back test still cites the requirement.
- **Taken: a removed capability is a requirement of the component that builds the capability tests,** for placement under `wiring` and for `check/obsolete-tests-elsewhere`. That is how `check/tests-cite-their-owner` already reads a capability marker. Without a `capability` component, the placement stays on each marker file's owner, as today.
- **Taken: the owner's prompt says nothing about code for another component's removed capability.** A capability's tests are all it has, so the `deferred` item the prompt asks for a requirement's code would name nothing.
- **Not taken: a check that every own-file test of a removed requirement is listed.** It would take from the build the re-cite that is right whenever the behaviour stays stated, and make every removal list tests that stay.
- **Not taken: a list of tests to re-cite, written by the change.** New machinery for a forgotten line; the question covers it with what exists (`yagni`, `reuse-first`).
- **Not taken: accepting a block on a removed requirement whatever its change's status.** It would let a stale item survive a removal and hold the obsolete line back.

## Outcome

- `model/parse-backlog`: a `blocks` entry may name a requirement an `applied` or `implementing` live change lists under `REMOVED Requirements`, or a capability it lists under `removes_capabilities`.
- `check.coverage/pending-report`: an item blocks an obsolete requirement on each of its lines; a scenario proves it.
- `check.coverage/removed-capabilities`: placed as a requirement of `capability` when the graph holds it, so the `wiring` split applies; a backlog item can block it.
- `check/obsolete-tests-elsewhere`: also reports an unlisted test in another component's file that cites a removed capability.
- `orchestrator.run/pending-prompt`: the question a build files for an unlisted test it can't re-cite names the removed requirement in `blocks`, and the build leaves the code that test runs; no `deferred` item for another component's removed capability.
- Decision `a-question-holds-a-removed-requirement-s-unlisted-test`; `a-removed-requirement-s-tests-elsewhere-are-listed` and `docs/spec-format.md` follow it.

## Constraints

- A project that sets no `wiring` sees only the `blocks` widening.
- No file, key or format a released project holds changes meaning: an item that loaded before still loads.

## Edge cases

- A question blocks every line of its removed requirement, other components' lines included: one requirement, one wait, released by one answer. The answer is a change that `resolves` the question, which `clm apply` deletes; while the item stays, no build touches the line, so its test still cites the requirement and the change can't be archived with the item left naming nothing.
- Where `check/obsolete-tests-elsewhere` doesn't apply (no `wiring`, a `wiring` or unowned file, a removed component's or capability's test in the files of the component that builds it), the owner's build that meets an unlisted test of another component's removed requirement leaves it and files the same question.

## What the builds do

`model` first; then `check` and `orchestrator.run`, each by its `#### Needs`; then `check.coverage`, whose `pending-report` and `removed-capabilities` need `orchestrator.run/pending-prompt` (`expand-contract`, as in `20261119-a-removed-requirement-s-tests-elsewhere`). `check/obsolete-tests-elsewhere`, `orchestrator.run/pending-prompt` and `check.coverage/pending-report` are already pending from that change, so their builds take both texts at once; `check.coverage/removed-capabilities` is made pending by this one.

- `model`'s build widens the check of backlog `blocks` alone (`check_backlog` in `crates/clm-model/src/records.rs`), not the `blockable` lookup that `#### Enforced by` shares (`layers.rs`), and rewords the violation that says what a `blocks` entry may name.
- `check.coverage`'s `pending-report` needs no code for an item blocking an obsolete line: `pending` already applies `blocked_by` to every line. Its build adds the scenario's test.
- `orchestrator.run`'s build rewrites the prompt's general text and per-item line that say to remove the code obsolete markers mark, so a test left with a question keeps its code; `crates/clm-run/tests/prompt.rs` asserts the old question text.

## Open questions

- Not measured: whether `/clm:build` launches a component whose only pending line can't be cleared again after each build that changes nothing, or only shows it ready. Once this is built the line is blocked, so it no longer matters for these cases.
