---
component: gate
---

## MODIFIED Requirements

### Requirement: own-files <!-- base: 37282f7db7f7f363 -->
`clm-gate` SHALL offer `foreign_files(repo, base, head, component)`, returning, when the root component at the merge base of `base` and `head` sets `wiring` (`model/wiring-globs`) and the specs there hold `component`, each code file (one outside the spec paths) that `head` adds, modifies or deletes against the merge base and that is none of:
- a file `component` owns, by `check.coverage/owner` over the specs at the merge base;
- a file no component owns, which `check/unowned-files` judges;
- a file one of the root's `wiring` globs matches;
- a file `head` modifies, without adding or deleting it, whose text at the merge base holds a marker citing `capability/<slug>` anywhere in it, matched by the marker's whole ID whatever version it cites, so `capability/seeing` is not `capability/see`, for a capability that the effective `serves` of one of `component`'s requirements names (its own `#### Serves` block when it holds one, its component's `serves` otherwise);

each with the component that owns it, sorted by path. It SHALL return nothing when the root sets no `wiring` or the specs at the merge base hold no such component. It SHALL read `head` only as a revision, as `evaluate-at` does, and `clm gate`'s output and exit status SHALL NOT change with it.

#### Scenario: another component's source
- GIVEN a root with `wiring: []`, components `a` with `paths: ["crates/a/**"]` and `b` with `paths: ["crates/b/**"]`, and a branch changing `crates/a/src/lib.rs` and `crates/b/src/lib.rs`
- WHEN `foreign_files` runs for `a`, and `clm gate --base main` runs
- THEN the first returns `crates/b/src/lib.rs` with owner `b`, and the gate prints `code` and exits 0

#### Scenario: wiring
- GIVEN the same with the root's `wiring: ["Cargo.lock"]` and a branch changing `crates/a/src/lib.rs` and `Cargo.lock`, which the root owns
- WHEN `foreign_files` runs for `a`
- THEN it returns nothing

#### Scenario: a test that cites the component in another's file
- GIVEN the same with `impl/a` adding a test citing `a/x` to `crates/b/tests/cli.rs`, and deleting `crates/b/tests/old.rs`, which cited `a/y` at the merge base
- WHEN `foreign_files` runs for `a`
- THEN it returns both files with owner `b`: a marker gives a build no file

#### Scenario: a capability test the component serves
- GIVEN the same, `a/x` serving `capability/see`, a component `capability` owning `tests/capability/see.rs`, which cites `capability/see`, and a branch modifying that file
- WHEN `foreign_files` runs for `a`, and again when the branch deletes the file instead
- THEN the first returns nothing, and the second returns the file with owner `capability`

#### Scenario: not switched on
- GIVEN a root with no `wiring`, and a branch changing `crates/b/src/lib.rs`
- WHEN `foreign_files` runs for `a`
- THEN it returns nothing

#### Scenario: an unknown component
- GIVEN a root with `wiring: []` and no component `z`
- WHEN `foreign_files` runs for `z`
- THEN it returns nothing

#### Serves
- architecture/path-ownership
- capability/land-safely

#### Needs
- model/wiring-globs
- orchestrator.run/pending-prompt
