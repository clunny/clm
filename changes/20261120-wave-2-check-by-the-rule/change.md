---
status: applied
---

## Intent

Wave 2 of phase 5 of `backlog/a-passing-check-proves-less-than-the-spec-claims`, its first branch of two: `check`, `check.coverage`, `check.affected`, `check.unreferenced` and `check.health` are each brought, in one pass and one delta, to the rule for a requirement (`requirements-state-what-the-contract-shows`, as amended on 2026-11-20). The wave is for a spec that is true and proven, not a smaller one. `20261120-wave-2-rest-by-the-rule` holds `effort`, `ledger`, `upgrade` and `init`; no clause moves between the two.

## Problem

Read against the rule and the code on 2026-11-20 and 2026-11-21, by one reader per component and then by one drafter per component that checked each entry against the code:

- **Every Contract was wrong or incomplete.** `check`'s left out six calls other crates use and said `clm check` runs requirement test coverage, which it doesn't. `check.coverage`'s left out five. `check.health`'s offered a crate-private call, left out three types other crates use and gave both commands' arguments wrongly. `check.unreferenced`'s offered `scan`, which nothing outside the crate calls. `check.affected`'s left out two exits.
- **About 45 clauses the code shows had no requirement**, among them what each command prints in full where a scenario said only "naming" a thing, what the calls do with `--help` and a stream that can't be written, and the handling of odd manifests, a changed submodule, a path git quotes and a tree outside git.
- **Bugs on `main`, each mended by its component's build:**
  - `check.coverage`: the TypeScript and JavaScript test scanner lets a test's first lines reach back past the call before it. A test call inside a block comment followed by another panics a debug build and gives a release build a test that ends before it starts.
  - `check.affected`: `--why` says `owned by` for an exempt test whose owner is affected though the test cites only unaffected components; `--overlap` drops a committed link that `--base` selects; and `--overlap` takes its `[affected] all` globs from the checkout it runs in, though its requirement says it reads the two commits alone, so `clm land`'s batch check judges two commits by a third tree's settings.
  - `check`: `archive-done` prints a change's status as `Done` or `Applied`, where `change.md` holds `done` and `applied`; `clm size` prints a line with no path when it can't list the files, and exits 1 for a tracked file whose parent directory has become a file, where it should leave the missing file out.
  - `check.coverage`: outside git the scan skips every directory whose name starts with a dot, follows a link to a directory, so a cycle of links never ends, and reads a file whose name isn't UTF-8, so an extracted tree's markers are not those of its checkout, against `markers-from-files`. `clm-check`'s copy of the walk has the first fault too.
  - `check.health`: `clm workaround` skips its warning that the ledger can't be read when the requirement serves no capability; the churn signal skips a file whose path git quotes; the root is stripped from a relative path of the coverage file as from an absolute one.
  - `check.unreferenced`, `check.affected`, `check.health`: `--help` through the crate's own call prints a first line that is not the command's description (a code comment, or the description with a full stop added). Each `commands-in-crate` now states the help its own call writes; that `clm <command> --help` prints the same is the root's to state, in wave 7 (`commands-live-in-their-crates`).
- **The root's `clm/detector-requires-skip` is false**: it says the skip of a detector whose `requires` is missing is the root's and that `clm-check`'s `Detector` doesn't change. Both are `clm-check`'s code.

## Outcome

- Each of the five has one delta: Contract and Design made right against the code, requirements reworded to what someone outside the component observes, the unstated clauses stated with one scenario each, and the cuts below.
- Where the code falls short of a requirement's plain meaning the requirement keeps its meaning, gains a scenario for the failing case, and its build mends the code. Nothing is weakened to match a bug.
- Two rules that two crates each held a copy of become one call each. `check.coverage` offers `read_files`, the files the scan reads, and `clm-check` reads its test lists through it; `check` offers `code_spans`, and `clm-health` reads code spans through it, where its own reader took the text after an unpaired backtick as a span. `check/coverage-lists`, `check.health/uncited-decisions` and `check.health/rule-signals` each need the requirement that states the call.
- `check.affected` depends on `config`, for `config/committed-settings`, by which `--overlap` loads each side's `clm.toml` from its commit.
- `check/detectors` states the `requires` skip. The root's false sentence waits for wave 7, as a clause that is the root's code does (`requirements-state-what-the-contract-shows`).
- `check.coverage/pending-report` states the priority of a pending requirement, which its code computes. `orchestrator/priority-orders-components` holds a copy until wave 4. `check.coverage/status-report` keeps the `none` row `clm-view` prints until wave 4 moves it to `view/status-report`, so it is never stated nowhere.
- `model/exemptions` keeps its sentence on a glob that matches other components' files; `check/undeclared-imports` gains the scenario for what `check`'s own rules do with one.

## Constraints

- A build changes only its own component's files (`own-files-at-the-gate`). No build of this branch needs a file outside its component: `check.affected`'s new dependency changes `Cargo.lock`, which the root's `wiring` holds.
- No call's return that another component's test asserts changes. Three returns of `check.coverage` change (the scanner bug's case, `uncounted` leaving out architecture entries, and the scan of a tree outside git); no other crate's test asserts any of them.
- `check/coverage-lists` keeps "Each SHALL exit 0", on which `check.coverage/coverage-report` drops its own exit sentence.
- All of this change's components were over the spec budget before it and each grows, as "Settled on 2026-11-20" in the plan item expects of every wave: stating what was unstated costs more text than the rule's cuts save. No seam is proposed here; budgets and splits are `backlog/the-brakes-on-the-spec-s-growth-only-advise`, for after the plan (`slim-components`).
- The names other components and the human layers use stay: `check/tests-cite-their-owner`, `check/obsolete-tests-elsewhere`, `check/archive-done`, `check.affected/affected-files`, `check.unreferenced/report` and every requirement of `check.coverage`.

## What the requirements no longer state

Under the yes given in wave 1 (`requirements-state-what-the-contract-shows`):
- `git-through-layer`, whole, in `check`, `check.affected` and `check.unreferenced`, each with its scenario "covered by the scan" and its scan test. `vcs/one-git-layer` reads every crate for every call these looked for. Lost with each: the words that the crate lists `clm-git` among its dependencies, a fact of a manifest's text.
- `check/says-component`, whole: the scan of `clm-check`'s string literals for retired words and its two tests. Its scenario "an asset that isn't there" moves under its name to `check/asset-references`, which states the message.
- `check.health/says-component`: the scan sentence and its two tests. The sentence that the commands' text says component, never the old words, stays, and each message it names is still tested; no test checks it in general any more.

Sentences that only say where a test sits, cut with no separate yes (`cuts-are-what-a-program-lists`):
- `commands-in-crate` in `check`, `check.unreferenced` and `check.health`: "Every test of `<component>`'s requirements that sat in the top-level test files of `crates/clm/tests/` SHALL sit in `<component>`'s own files…".
- `check.coverage/pending-report`: "no test of this requirement SHALL be left in `crates/clm/tests/pending.rs` or in `check`'s files". The gate lists this one as a lost sentence: its words, "SHALL be left in", match none of the gate's placement phrases.
- Without `SHALL`: `check.coverage/status-report`'s clause naming a test of `crates/clm/tests/status.rs`, and `check.coverage/markers-in-tests`' sentence on what its build adds to a capability test, done long since.

New proposals, each read by the human:
- `check.coverage/coverage-report`: "It SHALL exit 0." `clm coverage`'s exit is `clm-check`'s code, and `check/coverage-lists` says "Each SHALL exit 0".
- `check/size-tracked-through-git`: the sentence that `crates/clm-check/src/size.rs` SHALL NOT run `git ls-files` itself, and the scenario "no copy of its own", a scan of that file's text. The requirement stays under its ID and now states what the scan guarded as behaviour: `clm size` counts only the files the index holds, each once even in conflict, names that aren't UTF-8 included, and never an untracked file.
- `check.affected/clm-root-paths`: "In a repository whose CLM root is the top of its git repository the output SHALL be what it is without this rule." The other sentences say it.
- `check.health/component-size-field`: "No field, variable or doc comment under `crates/clm-health/src/` SHALL keep the field's old name…". A rule over source text that no test or check holds; `clm/retired-terms` doesn't read `crates/`.
- `check.unreferenced`'s Contract no longer offers `scan(root, allow, entry)`: nothing outside the crate calls it. Nine requirements that said "`scan` SHALL" say "`clm unreferenced` SHALL", with no outcome changed.
- `check.health`'s Contract no longer offers `crowded`, which is private to the crate.

Narrowed to what is meant, each a judgement the human reads:
- `check.affected/affected-files`: an exemption selects each file its globs name that git tracks, or that is untracked and not ignored, and that is a file in the working tree, Markdown aside, whether or not its bytes are UTF-8. Left out, each for a reason of its own: a path that is ignored and untracked, gone, or a directory, and Markdown, which imports nothing. A file that isn't UTF-8 is selected, which the code doesn't do today and its build mends (`temporary-exemptions`).
- `check.health/uncited-decisions`: a decision is cited when the whole text of a code span is its name, compared with case, where the statement said its name "appears in a code span".
- "Markdown" is exactly the extension `md`, compared with case, in `check.coverage/markers-from-files` and `markers-in-tests` and in `check.health/churn-signal`, as every reader of markers decides it.
- `check/skipped-tests` and `check/tests-cite-their-owner` judge only a marker that counts, one inside a test, and `tests-cite-their-owner` never one that cites an architecture entry.
- `check/backlog-report`: "It SHALL exit 0" is "exit 0, except when the spec doesn't load".

Scenario outcomes that change under their names, none losing a case:
- `check`: `asset-references` "missing mockup", `archive-done` "finished but not archived", `dangling-markers` "test cites a removed requirement", `node-size` "unknown node" and `priority-caps` "two P1s" each give the whole line where they said "naming" or "reporting"; `node-size` "settings can't be loaded" gains a case; `commands-in-crate` "arguments it can't parse" and "lines from the crate below" name no test and no crate; `coverage-lists` "a removed requirement's tests" gains the case of a tree outside git.
- `check.unreferenced/commands-in-crate` "arguments it can't parse": "the test's process" is "the caller's process".

Design sentences that go: `check.unreferenced`'s Output paragraph, which restated `report` and misquoted `check/detectors`' line; `check.health`'s two sentences that state an exit, which `health-report` states; and the sentences of each Design that place tests or describe another component's fixture.

## What no test asserts today

Each delta's drafter read, for every newly stated clause, the assertion that fails if the behaviour breaks. The list is a floor, as the pilot found. None today: every `--help` clause; `check.coverage/test-spans` "a test in a block comment" and `markers-from-files` "a tree outside git"; `check.affected`'s changed submodule, "a link on both sides", "the settings for both sides" and "an exempt test whose owner is affected"; `check/coverage-lists`' tree outside git, `node-size`'s listing failure and `archive-done`'s status as written; `check.health`'s "nothing to count" and a quoted path's churn; `check.unreferenced`'s odd manifests and a stream that can't be written. Where a test already asserts a newly stated clause while citing a requirement that didn't say it, the build re-cites it.

## Open questions

None that holds the build. Left for later waves, and recorded in the plan item by `20261120-wave-2-rest-by-the-rule`, which lands with this branch and before any build of either (this branch edits no backlog file, so that the two don't both edit the plan item): the root's `clm/detector-requires-skip` and the unused environment argument of `clm unreferenced`'s call (wave 7: until then the root's sentence contradicts `check/detectors`, which is the price of wave 1's rule that a clause that is the root's code moves on the root's branch); `orchestrator/priority-orders-components`' copied sentence and `view/status-report`'s `none` row (wave 4); the library calls of `check.affected` nothing outside calls (wave 3, as the pilot left it); `model`'s test `priority_order_display_and_caps`, which cites `model/parse-changes` for caps `check/priority-caps` states.
