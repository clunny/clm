---
component: sync.regenerate
base: { frontmatter: "c0e7682ac86b34c6" }
---

## MODIFIED Frontmatter

```yaml
serves: [capability/work-concurrently, architecture/hash-based-staleness]
```

## MODIFIED Requirements

### Requirement: regenerated-tree <!-- base: 03be684b5e4af2f3 -->
`clm-regenerate` SHALL offer `regenerated(root, onto, ids, merge)`, the one place that builds a regenerated `specs/`, which `clm rebase`'s regenerate step (`reapply`) and `land`'s derived-specs check both call: in a temporary directory, `onto`'s `specs/` (none when `onto` has none) with `HEAD`'s `changes/`, `backlog/`, `docs/decisions/`, `ARCHITECTURE.md` and `CAPABILITIES.md` (every other path `clm_model::load` reads, so the graphs it loads hold what `clm check`'s does), each taken under the CLM root's prefix in the repository when the root is a subdirectory of it, then the changes of `ids` re-applied one at a time with `clm-apply`'s `reapply`, each onto the tree the ones before it left, the next being, of the changes left, the first in ID order none of whose recorded bases is unknown against the tree so far (`stale-delta-diff`), or the first in ID order when every change left has an unknown base, whatever order `ids` lists them in. It SHALL apply them stopping at the first change that has violations under its own directory (all the violations when the change doesn't load) or that doesn't apply. It SHALL return the tree, with its stages and the `merged` lines, or, as `Regenerated::Stopped`, the `merged` lines and the lines that stopped it: the violations as `check` prints them, followed by the stale-delta report (`stale-delta-diff`). With `merge` it SHALL first merge a stale delta as `stale-delta-merge` states; without it, it SHALL change nothing in `root`'s working tree, index or history.

#### Scenario: regenerated without merging
- GIVEN a branch whose `HEAD` adds applied change `x`, whose delta adds requirement `a/r`, and an `onto` that has since added requirement `b/s`
- WHEN `regenerated(root, onto, ["x"], false)` runs
- THEN the tree's `specs/a/SPEC.md` holds `a/r` and its `specs/b/SPEC.md` holds `b/s`, and `HEAD`, the index and the working tree are unchanged

#### Scenario: change built on another sorts first
- GIVEN a branch whose `HEAD` adds applied changes `x` and `w`, `w` sorting first, where `x`'s delta rewrites the first line of `a`'s Design with base `h1`, `onto`'s hash, and `w`'s rewrites its second line with base `h2`, the Design's hash after `x` applies
- WHEN `regenerated(root, onto, ["w", "x"], false)` runs
- THEN it returns the tree, the stages are `onto`'s, then the tree after `x`, then the tree after `w`, and the tree's Design holds both edits

#### Scenario: change that adds what another modifies
- GIVEN a branch whose `HEAD` adds applied changes `x`, whose delta adds requirement `a/r`, and `w`, sorting first, whose delta modifies `a/r` with base the hash of `a/r` as `x` adds it
- WHEN `regenerated(root, onto, ["w", "x"], false)` runs
- THEN it returns the tree, and the tree's `a/r` holds `w`'s text

#### Scenario: a change stale against main goes before the one built on it
- GIVEN a branch whose `HEAD` adds applied changes `x` and `w`, `w` sorting first, where `x`'s delta rewrites the first line of `a`'s Contract with base `h0`, which `onto` has since moved by changing the same line, and `w`'s delta modifies `a`'s Contract with base the hash `x`'s delta gives on `h0`'s text
- WHEN `regenerated(root, onto, ["w", "x"], false)` runs
- THEN it returns `Stopped` whose lines are the `apply/stale-base` violation naming `x`'s delta followed by the report `main changed contract of a since h0`, with no violation naming `w`'s delta, and `HEAD`, the index and the working tree are unchanged

#### Scenario: independent changes keep ID order
- GIVEN a branch whose `HEAD` adds applied changes `x` and `y`, whose deltas both modify `a`'s Design with base `h1`, `onto`'s hash, each rewriting its first line differently
- WHEN `regenerated(root, onto, ["y", "x"], false)` runs
- THEN it returns `Stopped` with the `apply/stale-base` violation naming `y`'s delta and the report `x changed design of a since h1`

#### Scenario: human layers in the tree
- GIVEN a branch whose `HEAD` adds applied change `x` and `proposed` change `y`, whose delta adds to component `a` a requirement whose `#### Serves` names `architecture/e`, an entry of `HEAD`'s `ARCHITECTURE.md`, and on which `clm check` passes
- WHEN `regenerated(root, onto, ["x"], false)` runs
- THEN the tree holds `HEAD`'s `ARCHITECTURE.md`, `CAPABILITIES.md` and `docs/decisions/`, the graph `clm_model::load` reads from it has the entry `architecture/e`, and `clm-apply`'s `validate` of `y` against that graph reports no `apply/resulting-graph` violation

#### Serves
- architecture/reuse-first
- architecture/hash-based-staleness
