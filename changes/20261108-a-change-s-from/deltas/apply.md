---
component: apply
base: { contract: "5bd13058f69d2ff5" }
---

## MODIFIED Contract
- `validate(graph, change)` returns every reason the change's deltas and assets cannot be applied to the current spec graph, as violations, including every rule across components that the graph the change leaves would break, so a change that applies leaves a graph that loads.
- `resulting(graph, change)` returns the spec graph `validate` checks the change against, the graph the change would leave, or none when a delta of the change can't be applied, for `check`'s rules that judge a pending change's text (`resulting-graph-call`).
- `stale_bases(component, delta)` returns each Contract, Design or requirement `delta` MODIFIES or REMOVES whose recorded base `validate` judges stale against `component` (`stale-base`), with that base, for `sync`'s stale-delta report and merges (`stale-elements`).
- `clm apply <change-id>` applies a change's deltas to `specs/` and copies or removes its assets, deletes the `question` items its `resolves` names (`answered-questions`) and the item its `from` names (`promoted-item`), all-or-nothing, and marks the change `applied`. It exits 1 and writes nothing on any violation of the change, or of the loaded graph that the change doesn't fix (`loaded-graph`).
- `clm hashes <node-id>` prints one `<element> <hash>` line per element of the component: `frontmatter`, `contract`, `design`, then `requirement:<slug>` for each requirement, then `asset:<path>` for each asset. `clm hashes architecture` and `clm hashes capability` print one line per entry of that human layer, in file order, `requirement:<slug> <hash>` or `capability:<slug> <hash>`, the word its heading uses (`model/human-layers`). When the repository has the capability layer's component (`model/reserved-ids`), `clm hashes capability` prints that component's element lines first, then the entries' lines: the component holds no requirement, so no line name is printed twice. It isn't called `hash`, which Claude Code's worktree isolation refuses (`worktree-safe-commands`).

## ADDED Requirements
### Requirement: promoted-item
`clm apply` SHALL delete `backlog/<slug>.md` when the change's `from` is `backlog/<slug>` and that file exists, whatever the item's `kind`, among the files it removes, since `from` names the item the change was promoted from and the change now carries its work, so the item leaves the backlog when the spec branch lands and no later session takes it for open work. A `from` naming no item in `backlog/` SHALL delete nothing and be no violation, so a branch that deleted the item by hand applies as before. The deletion SHALL be one of the writes `atomic` holds back: when the change has a violation, the item stays. It SHALL print the deleted item as `removed backlog/<slug>.md`, as it prints the other files it removes, once even when `resolves` names the same item. The re-apply `clm rebase` runs on a branch's applied change deletes none, as for `answered-questions`.

#### Scenario: promoted idea
- GIVEN `backlog/i.md` with `kind: idea`, and change `c` with `from: backlog/i` and a valid delta for `a`
- WHEN `clm apply c` runs
- THEN it exits 0, `backlog/i.md` no longer exists, and it prints `removed backlog/i.md`

#### Scenario: any kind
- GIVEN `backlog/b.md` with `kind: bug`, and change `c` with `from: backlog/b` and a valid delta
- WHEN `clm apply c` runs
- THEN it exits 0 and `backlog/b.md` no longer exists

#### Scenario: refused change keeps its item
- GIVEN `backlog/i.md` with `kind: idea`, and change `c` with `from: backlog/i` and a stale delta
- WHEN `clm apply c` runs
- THEN it exits 1 and `backlog/i.md` still exists

#### Scenario: already deleted
- GIVEN change `c` with `from: backlog/i`, no `backlog/i.md`, and a valid delta
- WHEN `clm apply c` runs
- THEN it exits 0 and reports nothing about `i`

#### Scenario: named in from and resolves
- GIVEN `backlog/q.md` with `kind: question`, and change `c` with `from: backlog/q`, `resolves: [q]` and a valid delta
- WHEN `clm apply c` runs
- THEN it exits 0, `backlog/q.md` no longer exists, and `removed backlog/q.md` is printed once

#### Serves
- capability/keep-a-backlog
