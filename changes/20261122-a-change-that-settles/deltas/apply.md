---
component: apply
base: { contract: "1e029fbceb2fa76d" }
---

## MODIFIED Contract
- `validate(graph, change)` returns every reason the change can't be applied to the spec graph, as violations: a status `clm apply` refuses (`change-status`), each delta and asset that can't be applied, and every rule across components that the graph the change leaves would break, so a change that applies leaves a graph that loads.
- `resulting(graph, change)` returns the spec graph `validate` checks the change against, the graph the change would leave, or none when a delta of the change can't be applied, for `check`'s rules that judge a pending change's text (`resulting-graph-call`).
- `stale_bases(component, delta)` returns each Contract, Design or requirement `delta` MODIFIES or REMOVES whose recorded base `validate` judges stale against `component` (`stale-base`), with that base, for `sync.regenerate`'s stale-delta report and merges and for `rename`'s rebase of other changes' deltas (`stale-elements`).
- `hashes(component)` returns a component's `(element, hash)` pairs in `clm hashes`' order: its frontmatter, Contract and Design, then its requirements, then its assets (`hash-command`). The entries of a human layer, which `clm hashes architecture` and `clm hashes capability` print, are the command's and not this call's.
- `apply(root, id)` is `clm apply` on a checkout: it returns the files it wrote and removed, each a `FileOp`, or the violations; when the change or the graph is at fault it has written nothing, and when a write or removal fails it has restored every file it had written or removed (`atomic`, `failed-write`). `apply_into(root, id, journal)` is the same apply performing its files through the caller's `Journal` and restoring none on a failure, so a caller that writes files of its own around the apply, as `rename` does, undoes the apply's with its own (`journal`). `reapply(graph, change)` returns the `specs/` files an already applied change writes and removes, writing none itself (`reapply`). A `FileOp` gives its path from the repository root with `path()`, and `perform(root)` writes or removes that file under `root`.
- A `Journal` on a root (`Journal::new(root)`) performs `FileOp`s under it, remembering what each file held before the first one that touched it; `undo()` puts every file it touched back as it was, and returns each file it couldn't put back with why (`journal`).
- `archive(root, id)` is `clm archive` on a checkout: it returns the directory the change moved to, or the violations.
- `clm apply <change-id>` applies a change's deltas to `specs/` and copies or removes its assets, deletes the `question` and `workaround` items its `resolves` names (`answered-questions`) and the item its `from` names (`promoted-item`), all-or-nothing, and marks the change `applied`. It prints `applied <id>` and a line per file it wrote or removed, and exits 0 (`change-status`). It exits 1, printing each violation on standard output and writing nothing, on any violation of the change, or of the loaded graph that the change doesn't fix (`loaded-graph`); and 1, printing the failure and leaving every file as it was, when a write or removal fails (`failed-write`).
- `clm archive <change-id>` moves a `done` or `abandoned` change's directory to `changes/archive/<id>/` and prints where it went; it exits 1, moving nothing, for any other change (`archive-command`).
- `clm hashes <component>` prints one `<element> <hash>` line per element of the component: `frontmatter`, `contract`, `design`, then `requirement:<slug>` for each requirement, then `asset:<path>` for each asset. `clm hashes architecture` and `clm hashes capability` print one line per entry of that human layer, in file order, `requirement:<slug> <hash>` or `capability:<slug> <hash>`, the word its heading uses (`model/human-layers`). When the repository has the capability layer's component (`model/reserved-ids`), `clm hashes capability` prints that component's element lines first, then the entries' lines: the component holds no requirement, so no line name is printed twice. For an ID that names neither, it exits 2 with a line on standard error.

`clm-apply` offers each of `clm apply`, `clm archive` and `clm hashes` as a call (`commands-in-crate`, `commands-live-in-their-crates`): it takes the command's arguments as the command line gives them, the repository root, the environment as a lookup and a stream each for standard output and standard error, and returns the exit status. `crates/clm` routes the command to it and adds nothing to what it writes.

## MODIFIED Requirements

### Requirement: resulting-graph <!-- base: ded55ba9cead51d0 -->
Once every delta of the change applies, whatever the change's status, validation SHALL check the spec graph the change would leave, the one `resulting` returns for it (`resulting-graph-call`): each component the change writes parsed from its new `SPEC.md` text as loading parses it (`model/graph-check`), with the keys live changes declare (`declared-frontmatter-keys`), at the path it is written to and with no assets, which no rule across components reads; each component it removes gone; each backlog item `clm apply` deletes with the change gone; every other component and backlog item, and the changes and decisions, as they are. It SHALL run the model's graph check on the current graph, with every item, and on that graph with the backlog's `workaround` items left out (the graph `resulting` returns keeps those the change doesn't resolve, `answered-questions`), and the component parser on the current text of each existing component the change writes. A flag's `requirement` holds the version a build built (`model/parse-backlog`), which only the current graph gives: a flag on a requirement the change rewrites or removes stays valid while the change is pending (`workaround-flags`). Each violation of the resulting graph, from the parser or the graph check, that the current graph doesn't have (the same path, rule and message) SHALL be a violation under `apply/resulting-graph`, with the model's path and the message ``change `<id>` would leave: <model rule>: <model message>``; nothing SHALL be written. Violations the current graph already has are not reported again, so `clm check`, which validates every `proposed` or `approved` change against a graph that may have violations of its own, reports each once. A violation whose message the change alters, such as a dependency cycle it lengthens, counts as new.

#### Scenario: required node depends on a node made optional
- GIVEN component `a`, component `b` with `depends_on: [a]`, and change `x` whose delta for `a` sets `optional: true` in its `## MODIFIED Frontmatter`, with the correct base
- WHEN `clm apply x` runs
- THEN it exits 1 naming `apply/resulting-graph`, `model/optional-nodes`, `b` and `a`, and nothing is written

#### Scenario: dependant left behind
- GIVEN component `a`, component `b` with `depends_on: [a]`, and change `x` whose only delta is a valid removal of `a`
- WHEN `clm apply x` runs
- THEN it exits 1 naming `apply/resulting-graph`, `model/dependency-dag`, `b` and `a`, and `specs/a/SPEC.md` still exists

#### Scenario: backlog blocks a removed requirement
- GIVEN component `a` with requirements `x` and `y`, a backlog item whose `blocks` names `a/y`, and change `c` whose delta for `a` removes `y` with its base
- WHEN `clm apply c` runs
- THEN it exits 1 naming `apply/resulting-graph`, `model/parse-backlog` and `a/y`, and nothing is written

#### Scenario: answered question blocks a removed requirement
- GIVEN component `a` with requirements `x` and `y`, `backlog/q.md` with `kind: question` and `blocks: [a/y]`, and a proposed change `c` whose `resolves` is `[q]` and whose delta for `a` removes `y` with its base
- WHEN `clm check` runs, then `clm apply c`
- THEN `clm check` reports nothing under `apply/resulting-graph`, and `clm apply c` exits 0, removes `a/y` and deletes `backlog/q.md`

#### Scenario: resolved bug blocks a removed requirement
- GIVEN component `a` with requirements `x` and `y`, `backlog/b.md` with `kind: bug` and `blocks: [a/y]`, and change `c` whose `resolves` is `[b]` and whose delta for `a` removes `y` with its base
- WHEN `clm apply c` runs
- THEN it exits 1 naming `apply/resulting-graph`, `model/parse-backlog` and `a/y`, and `backlog/b.md` still exists

#### Scenario: answered question already blocks no requirement
- GIVEN component `a` with requirement `x`, `backlog/q.md` with `kind: question` and `blocks: [a/gone]`, naming no requirement, and change `c` whose `resolves` is `[q]` with a valid delta that modifies `x`
- WHEN `clm apply c` runs
- THEN it exits 0, `backlog/q.md` no longer exists, and the graph then loads with no violation

#### Scenario: violation already present
- GIVEN component `b` with `depends_on: [gone]`, naming no component, and a proposed change `x` whose valid delta modifies `a`'s Design
- WHEN `clm check` runs
- THEN it reports `b`'s `depends_on` once, under `model/dependency-dag`, and nothing under `apply/resulting-graph`

#### Scenario: workaround flag on a rewritten requirement
- GIVEN component `a` whose requirement `x` is at version `3fa2c9d1`, `backlog/workaround-a-x-1.md` with `kind: workaround` and `requirement: a/x@3fa2c9d1`, and a proposed change `c` whose delta for `a` modifies `x`'s text with its base
- WHEN `clm check` runs, then `clm apply c`, then `clm check` again
- THEN the first `clm check` reports nothing, `clm apply c` exits 0 and writes `x`'s new text, and the second `clm check` reports `backlog/workaround-a-x-1.md` under `model/parse-backlog` with `x`'s new version

#### Needs
- model/graph-check

#### Serves
- capability/catch-drift
- architecture/deterministic-enforcement

### Requirement: answered-questions <!-- base: baab55dd671a06ea -->
`clm apply` SHALL delete `backlog/<slug>.md` for each `<slug>` the change's `resolves` names whose item has `kind: question` or `kind: workaround`, among the files it removes. The spec it applies answers a question, so the requirements the item `blocks` are released when the spec branch lands (`requirement-blocks`), not when the change is archived; and it settles a flag, so a flag on a requirement the change rewrites or removes, whose version `model/parse-backlog` would no longer accept once the change is applied, leaves with it, and the build of the new text starts with no flag counted against it (`workaround-flags`). It SHALL delete no item of another kind, and a `resolves` entry naming no item in `backlog/` SHALL delete nothing and be no violation. Each deleted item SHALL be among the files it prints as removed (`change-status`), as `removed backlog/<slug>.md`.

#### Scenario: answered question
- GIVEN `backlog/q.md` with `kind: question` and `blocks: [a/x]`, and change `c` whose `resolves` is `[q]` with a valid delta for `a`
- WHEN `clm apply c` runs
- THEN it exits 0, `backlog/q.md` no longer exists, and it prints `removed backlog/q.md`

#### Scenario: other kinds stay
- GIVEN `backlog/b.md` with `kind: bug` and `backlog/h.md` with `kind: human-task`, and change `c` whose `resolves` is `[b, h]` with a valid delta
- WHEN `clm apply c` runs
- THEN both items still exist

#### Scenario: already deleted
- GIVEN change `c` whose `resolves` is `[q]`, with no `backlog/q.md`, and a valid delta
- WHEN `clm apply c` runs
- THEN it exits 0 and reports nothing about `q`

#### Scenario: settled workaround flag
- GIVEN component `a` whose requirement `x` is at version `3fa2c9d1`, `backlog/workaround-a-x-1.md` with `kind: workaround` and `requirement: a/x@3fa2c9d1`, and change `c` whose `resolves` is `[workaround-a-x-1]` with a valid delta for `a` that modifies `x`'s text
- WHEN `clm apply c` runs, then `clm check`
- THEN `clm apply` exits 0, `backlog/workaround-a-x-1.md` no longer exists and it prints `removed backlog/workaround-a-x-1.md`, and `clm check` reports nothing under `model/parse-backlog`

#### Serves
- capability/keep-a-backlog
