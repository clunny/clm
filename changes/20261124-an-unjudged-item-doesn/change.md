---
status: applied
resolves: [an-unjudged-item-doesn-t-say-which-failure-ended-the-pass-that]
---

## Intent

An `unjudged` item says why the passes that left its requirements ended, and goes only to the component whose block failed. `mutate.sh` writes `<id> failed <why>` to the result file for a block whose tool failed. The `mutated` record keeps those requirements under a new `failed` field, with their text, and how the command ended under a new `ended` field (`stopped`, `limit <m>`, `status <n>` or `signal <n>`). The filing counts a requirement against a pass only when that pass could have judged it: never when the stop request ended the pass, and never when the pass left the requirement only because another requirement's block failed first. The item's body names, for each pass, how it left the requirement.

## Problem

`backlog/an-unjudged-item-doesn-t-say-which-failure-ended-the-pass-that`: the pass after `88470edf` ended when `.github/scripts/mutate.sh` exited 1 after StrykerJS failed on five `web` blocks, with `0 killed, 0 unproven, 0 not judged, 29 left`. `clm verify` then filed seven `unjudged-<component>-88470edf` items, on `check.coverage`, `ci`, `model`, `web`, `web.views`, `web.worker` and `web.worker.replica`. Each said only that the pass "stopped, reached its limit or failed before its verdict", and pointed at `clm verify`'s stderr, which nothing keeps. Triage drafted six of them in parallel, and every draft went looking for the same cause, which lay in `mutate.sh` and in none of the components those items named.

`verify`'s Design chose this on purpose: "The ledger doesn't say why a pass left a requirement, and adding a field for it would change `ledger` for a reason the item's draft can read on the verify's own output". That premise is false. `clm verify` keeps the mutation command's output nowhere, and the filing reads records two passes apart, so the reason has gone by the time an item is filed. The decision was wrong, so the answer is to redesign it, not to work around it (`redesign-before-workaround`). No requirement says an item gives its reason, and none says which component gets the item when one block's failure starves the others, so both go into the spec (`code-fix-needs-a-written-rule`).

## Approaches

- **Taken: the command names the failed block, and the record keeps the reason.** `ci/mutate-script` already prints the tool and the block on stderr when a block fails. It now also appends `<id> failed <why>` to the result file, the one channel `verify` reads from the command. `verify/mutation-pass` reads that line: the requirement stays left, and its text goes under the record's `failed`. How the command ended, which the pass already knows and prints, goes under `ended`. Both fields are optional in `ledger/mutated-record`, so older records read as they did. `verify/files-unproven` reads them to decide which requirements a pass counts against, and to write the item's body. Keeping the reason in git follows `git-is-truth`: the item and the ledger hold it, not a log on one machine.
- **Not taken: keep the command's stderr in a log, as `kept-log` keeps a red check's, and name the log in the item.** The log lives on one machine and gets cleared, while the item lands in git and is drafted wherever triage runs. The item would then point at a file the draft can't read, which is today's bug again.
- **Not taken: have `verify` read the stderr line `mutate.sh: <tool> failed for requirement <id>`.** That ties `verify` to a sentence in one project's script. CLM names no mutation tool. The result file is the contract between the pass and its command (`mutation-pass`), so the failure belongs there.
- **Not taken: keep filing every requirement a failed pass left, and only add the reason.** The reason alone would have spared the drafts some hunting, but it would still have filed seven items for one failure. A requirement that a pass never started has nothing wrong with it. It goes first in the next pass behind the failing block, and it is judged once that block is mended.

## Outcome

- `ci/mutate-script` (modified): a block whose own judging fails appends `<id> failed <why>` to the result file, `<why>` being the stderr line's text after `mutate.sh: `, and still writes no verdict. A tool's install, a tree the script makes and a fixture belong to no block, so they write no such line. Six scenarios whose block's tool fails now expect that line: *a tool that fails*, *a failing block lets the others finish*, *StrykerJS runs no test*, *nextest fails for a test file's mutant*, *the run that finds the lines fails* and *the run of a package's tests fails*. The tests in `crates/clm/tests/ci_suite/ci_check.rs` that assert an empty result file for those cases change with them.
- `ledger/mutated-record` (modified): the record gains `failed` (an object from requirement ID to text) and `ended` (a string), both optional. The fold skips a line where either has the wrong type. `Body::with_failed` and `Body::with_ended` set them on a body built by `Body::mutated`, as `Body::with_left` sets `left`. `clm ledger show` and `compact` don't change. `ledger`'s Contract describes the two fields and the two methods.
- `verify/mutation-pass` (modified): it reads `<id> failed [<why>]` as a left requirement with that why, and records `failed` and `ended`.
- `verify/files-unproven` (modified): a record counts a requirement it left against it only when its `ended` isn't `stopped` and its `failed` is empty or names that requirement. The `unjudged` item's body names, for each record of the run, its commit and how that pass left the requirement.
- `verify`'s Design: the paragraph *Requirements a pass can't reach* drops the sentence that rejected a ledger field, and a new paragraph, *Why a pass left it*, says why the reason is kept and which passes count against a requirement.
- `ledger`'s own test `the_same_record_as_a_literal` (`crates/clm-ledger/tests/mutated.rs`) builds a `Body::Mutated` literal, and gains the two fields in `ledger`'s build.
- `ledger/mutated-record`'s fold also skips a line whose `failed` names an ID its `left` doesn't. `verify/files-unproven` passes over a record that is stopped, or that failed on another block, without letting it end the run (scenarios *stopped twice* and *a stop between two failures*).
- Hand edits: `docs/guide/settings.md` documents the `failed` line a project's command writes and the narrowed filing, `docs/guide/how-it-works.md` the narrowed filing (`method-in-the-plugin`), and `docs/decisions/mutation-after-land.md` gains a dated section for this change.
- No capability test sees a difference. None reads the `mutated` record or an `unjudged` item's body, and the outcome line, stdout and exit status are unchanged.

## Constraints and edge cases

- A tool that fails on its fixture, before any block starts, names no requirement. Its pass leaves `failed` empty, so it counts against every requirement it left, as every pass does today, and the item's body gives the command's status. This is rare, and handling it would need a fixture failure to name a component, which none owns.
- Two passes in a row that reach `mutate_minutes` still count against every requirement they left. If the limit can't cover what is due, a person should look.
- Records written before this change have no `ended`, so an item drawn from them says `no reason recorded` for those passes. The requirements those records left still count, as they do today. A new record whose command exited 0 yet left a requirement with no line also has no `ended`, and reads the same way.
- A block that ends through the shell's `set -e`, rather than through the script's `die`, writes no `failed` line. Its requirement is left with no why, and the pass counts every requirement it left, as it does today. The build of `mutate.sh` should route every failure inside a block through `die`, which the requirement already asks for, since it names every such failure.
- Order of builds: `verify/mutation-pass` and `verify/files-unproven` need `ledger/mutated-record` built first, since they write and read its new fields (`#### Needs` already names it on both). `verify/files-unproven` already needs `ci/mutate-script`. Until `ci` is built, `verify` simply never sees a `failed` line, and every pass counts as it does today, so building `verify` before `ci` breaks nothing.

## What the requirements no longer state

- `verify/files-unproven`: "find the unbroken run of those records, newest commit first, whose `left` names it". The run now holds only the records that count the requirement against it. A pass the stop request ended, or one that left a requirement only because another requirement's block failed, is passed over: it neither adds to the run nor ends it, and no item is filed for that requirement on its account.
- `verify/files-unproven`: the item's body "saying that each of those passes left it unjudged, having stopped, reached its limit or failed before its verdict". The body now names how each pass left it.
- `ci/mutate-script`: "writing no verdict for that block" becomes "appending ... `<id> failed <why>` ..., and no verdict for that block". The script still writes no verdict, but the result file now holds a line for the block.
- `ci/mutate-script`, scenario *a tool that fails*: the outcome "the result file holds no line for it" becomes one `failed` line and no verdict.
- `ci/mutate-script`, scenario *a failing block lets the others finish*: the outcome "no line for the first" becomes a `failed` line for the first.
- `ci/mutate-script`, scenarios *StrykerJS runs no test*, *nextest fails for a test file's mutant*, *the run that finds the lines fails* and *the run of a package's tests fails*: in each, the outcome "the result file holds no line for it" becomes one `failed` line naming the tool, and no verdict.

## Open questions

- **Is this the plan's work, or does it wait for the freeze to end?** The draft judged it plan work. The mutation pass is phase 3's evidence. `20261123-the-mutation-pass-can`, which added `unjudged` items, was built as plan work, and this change corrects what those items say and where they go. The pass after `88470edf` cost six draft sessions that changed nothing but `backlog/`. If the human would rather hold it until the plan is done, as with `20261115-a-stalled-npm-install`, add its four requirements to the plan item's `blocks`.
