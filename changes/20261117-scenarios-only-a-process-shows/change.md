---
status: applied
resolves: [two-config-tests-run-other-components-commands, two-unreferenced-tests-observe-only-the-clm-process, seventeen-init-tests-can-t-move-into-its-crate, five-land-tests-observe-only-the-clm-process, apply-s-old-name-gone-test-observes-only-the-root-s-parser, clm-check-s-run-of-the-ledger-audit-is-proven-only-through-the-binary, two-defect-tests-run-clm-check-on-the-item-they-wrote, effort-scenarios-still-name-the-clm-binary, two-doc-readers-scenarios-name-the-guide-tests-old-file, the-guards-spec-still-describes-it-before-its-call, clm-hook-guard-does-not-judge-through-guard, workaround-sync-commands-in-crate-1, no-test-runs-the-affected-all-settings-crates-clm-passes, workaround-plugin.hooks.guard-hook-budgets-1]
---

## Intent

Phase 4 of the plan in `backlog/a-passing-check-proves-less-than-the-spec-claims.md`, step 2 of "Phase 4, what is next". The builds that moved each command into its crate left 30 tests in the root's files: 24 because what they observe only a `clm` process can show (another component's command run on what this one made, the binary's own file, two commands at once, the root's parser), and six of `clm guide` that a test of the root's pinned to their file. The human answered for the whole class on 2026-11-15: such a scenario is the root's, which keeps the binary (`commands-live-in-their-crates`). This change moves them into requirements of the root and deletes the items that held them, so `clm/routes-only` waits only on the tests that still have to move.

It removes one scenario and one requirement, both with the human's yes of 2026-11-17 (see "What it removes"). Everything else is moved or split, and no test is dropped.

## Problem

Seven `deferred` items block `clm/routes-only`, each naming tests that stayed in `crates/clm/tests/` citing another component. While they cite it, that component's requirement is proven from the root's files, which is what phase 4 removes, and the clause "a file that cites the component" can't leave `gate/own-files`.

Reading the tests found two things the items didn't say:

- **Ten of the scenarios are mixed.** Eight of `init`'s and two of `plugin.hooks`' state what the component wrote and, in a last clause, that another command accepts it ("all seven files exist ... and `clm check` exits 0"). Moved whole, the root would state and prove what `clm init` writes; left whole, the test can't leave the root's file.
- **Three of `init`'s requirements are proven only from the root's file**: `template-says-architectural-constraint`, `templates-say-new-words` and `architecture-template-proof` have no test in `crates/clm-init/`. And `ledger`'s `on_main`, the findings it offers `clm check`, has no test in `crates/clm-ledger/` at all.

Four further items were filed by the same builds and nobody had read them. Two say `plugin.hooks.guard`'s spec describes the guard as it was before `clm hook guard` became a call. One is a workaround on `sync`: `sync/warnings-name-the-command` bars `clm-sync` from holding a string that names `clm rebase`, proven by a scan of its source, while `sync/commands-in-crate` has its call print everything the command prints, so the root passes the word `rebase` in as an argument the Contract doesn't name; `clm/rebase-warnings-prefixed` still says the root prints the prefix, which it no longer does. One says no test runs the `[affected] all` loader `crates/clm` passes to `clm affected`.

## Approaches

For a mixed scenario, put to the human on 2026-11-17:

- **Split at the clause.** Taken. The component keeps its scenario without the clause and proves it in its own crate; the clause becomes the root's scenario in the same words. Every word is kept, each half sits with the component that can prove it, and what `clm init` writes stays proven by `init`'s own tests. It costs `init`, `plugin.hooks` and `ledger` a build each to write their half's test.
- **Move whole and trim the original.** Not taken: the root would state and prove what `clm init` writes a second time, and two texts drift apart.
- **Move whole and drop the original.** Not taken, though it is the strictest reading of "word for word": three of `init`'s requirements would have no scenario and no test of their own for what they write.

For where the scenarios land in the root:

- **Three new requirements grouped by what the root does, and two existing ones.** Taken. `init-result-accepted` (other commands on what `clm init` wrote), `check-across-components` (what `clm check` is handed or runs: the ledger's audit, a detector that is the project's own command, an item `clm defect` wrote) and `land-as-a-process` (the binary's own file, two lands at once). `root-from-cwd` already states the root search `clm unreferenced` uses, and `old-command-names-retired` that an old name fails as an unknown subcommand, so `check.unreferenced`'s and `apply`'s scenarios fold there, as the plan asked.
- **One requirement per source requirement.** Not taken: eleven requirements for the root, which is over its budget, where three say the same.
- **Folding `init`'s into the `adopt-clm` capability test.** Not taken. That test runs `clm init --no-detectors` and checks the spec branch drafted afterwards; it shows none of these cases (each language's detector, a `.claude/settings.json` already there, dependencies not installed), and a capability test cites only its entry, so the cases would be stated nowhere.
- **`land`'s call taking the `clm` binary to copy**, so its crate could prove "copy removed". Not taken: "check deletes the binary" would still need a real `clm` to run the gate from, so the root keeps a test either way, and the call would gain an argument for one test.

For the `sync` workaround, put to the human on 2026-11-17:

- **Cut the bar on the literal and its source scan.** Taken, with their yes. The bar existed so the old `clm sync` name printed its own prefix; that name no longer runs (`clm/old-command-names-retired`). The crate owns what its command prints, so it writes the prefix itself.
- **Keep the scan and state the name argument in the Contract.** Not taken: it keeps a scan of source text, which phase 5 deletes, and an argument only the scan needs.

## Outcome

**The root gains three requirements and takes two scenarios into existing ones.**

- `clm/init-result-accepted`, eight scenarios: `clm check` passes on what `clm init` wrote (for `--lang rust`, for the default, with a `.claude/settings.json` already there, and for every language with no manifest), `clm hashes` and `clm pending` list nothing, `clm check` reports a detector whose tool isn't installed as one that could not run, and `clm setup` runs the `[build] setup` it wrote. The clause "`clm check` exits 0" of three of `init`'s scenarios with the same GIVEN and WHEN (`template-says-architectural-constraint`'s, `architecture-template-proof`'s and `init-seeds-permissions`' first) is stated once, as "the written layers", which the root's `templates_say_new_words` proves too, since it runs `clm check` on the same result. "a TypeScript project" moved whole, since `clm config get`, `clm setup` and `clm check` are all other components' commands; its WHEN now names the `clm check` its THEN asserts. "a package.json added later" is new text for what `setup_installs_when_package_json_exists` already tests and no scenario stated.
- `clm/check-across-components`, six scenarios: `ledger/verify`'s "check runs it on main", `check.unreferenced/report`'s "named as a detector", `config/retired-terms-detector`'s "a retired term in a spec fails the check", `config/unreferenced-detector`'s "clean with the list", and the clause of `plugin.hooks/defect-backlog`'s two.
- `clm/land-as-a-process`, five scenarios: both of `land/land-binary-copy`'s and three of `land/land-target-lock`'s.
- `clm/root-from-cwd` names `clm unreferenced` beside `clm survey`, as the root's Design already did, and takes "from a subdirectory of a project without CLM".
- `clm/old-command-names-retired` takes `apply/hash-command`'s "old name gone", and states that `clm --help` names `hashes` and no `hash`, which the test asserts.

**Each component keeps what its crate can show.**

- `init`: seven requirements lose the clause or the scenario that moved. `init-build-setup`'s "a TypeScript project" now states the `[build]` table `clm init` writes; `init-without-manifest`'s "every language" states the script's result and each detector's `requires`, which the test asserted and the scenario didn't; `templates-say-new-words` says `clm-model` loads the tree, which `clm-init` can call. `init/commands-in-crate` says that none of `init`'s tests is left in `crates/clm/tests/init.rs` or `guide.rs`, and that the tests of `clm guide` sit in `crates/clm-init/tests/guide.rs`; it needs `clm/init-result-accepted`, so the root has re-cited its tests before `init`'s build reaches them. So `init`'s build writes a test of its own half for each of the eleven, takes its marker off the root's test, and moves the six guide tests.
- `ledger/verify` states that `clm-ledger` offers the audit's findings for `clm check` on `main` only, with a scenario its crate proves.
- `land/land-binary-copy` gets a scenario for the copy that can't be made, which `copy_failure_refused` already proves, since both of its scenarios moved.
- `config`'s two requirements lose the sentences that placed their tests in `crates/clm/tests/cli.rs`.
- `plugin.hooks`' Design names where its tests are.

**`effort/capabilities-list`** states that `clm capabilities` refuses outside a CLM repository, which only the Contract said. `effort`'s scenarios that name the `clm` binary stay as they are, as the human answered.

**`clm/doc-readers-marked`** names `crates/clm-init/tests/guide.rs` in its two scenarios, and needs `init/commands-in-crate`, so the root asserts that file alone only once `init` has moved the tests.

**`clm/routes-only`** gains a scenario that runs `clm affected` with `[affected] all` set and with it invalid, so the loader `crates/clm` passes is run by a test.

**`plugin.hooks.guard`**: the call takes no repository root, the Design says how the guard loads settings and where its tests are, and `hook-budgets` says `clm hook guard` judges with `Budgets::HOOK` through `guard_hook`. Its scenario "nothing shortens the hook's budgets" now says `clm-guard`'s `Budgets`, as its test already scans for, since `crates/clm/src/main.rs` names `clm_check`'s `Budgets`; that settles the workaround flag on the requirement, whose item is deleted.

**`sync/warnings-name-the-command`**: `clm-sync`'s call writes each warning as `clm rebase: <warning>` and takes no argument naming the command.

**What the builds do.** The root's build re-cites the 24 tests that stay where they stand (`crates/clm/tests/cli.rs`, `unreferenced.rs`, `init.rs`, `land.rs`, `ledger.rs`, `defect.rs`), and deletes `crates/clm/tests/sync.rs`'s test of the removed requirement, which reads as `obsolete`. `sync`'s build drops the name argument and, in the same branch, edits its arm of `crates/clm/src/main.rs`, which is wiring, so `main` builds at every land; it deletes the test that scans its source. `init`, `plugin.hooks` and `ledger` write the tests of their halves; `config`, `check.unreferenced`, `land`, `apply`, `effort`, `sync` and `plugin.hooks.guard` re-cite. No capability test changes what it observes.

## What it removes

With the human's yes on 2026-11-17:

- The scenario "no command name in the crate" of `sync/warnings-name-the-command`, its sentence "`clm-sync` SHALL hold no string literal that names the command", and the clause for the old `clm sync` name's own prefix, which no longer runs. Its test reads `crates/clm-sync/src` for a string; `sync`'s build deletes it.
- The requirement `clm/rebase-warnings-prefixed`. Its one scenario, "a skipped proposal", moves word for word into `sync/warnings-name-the-command`, since `clm-sync`'s call now writes the prefix; the root's test of it in `crates/clm/tests/sync.rs` is deleted by the root's build once `sync`'s crate proves the scenario.

Not behaviour, and removed without asking: the two sentences of `config` that said where its tests live, which the root's Design already overrode.

## Constraints

- No test is deleted but the two above. A root test whose scenario was split stays, re-cited to the root's requirement; the component's half is a new test in its crate.
- `clm/routes-only` still waits on `tests-of-components-with-no-crate-sit-in-the-root-s-test-files` and `some-tests-prove-a-component-from-another-component-s-files`, and on `init/commands-in-crate`.
- The twelve items this change answers by its spec edits are deleted in its branch. `workaround-sync-commands-in-crate-1` and `no-test-runs-the-affected-all-settings-crates-clm-passes` stay until the builds of `sync` and of `clm/routes-only` have done their code.
- `three-crates-format-a-violation-s-line-by-hand` was read and left: it is code only, one component at a time under `clm claim <component> --fix`.
