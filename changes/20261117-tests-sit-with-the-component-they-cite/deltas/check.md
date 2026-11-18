---
component: check
base: { frontmatter: "0547a6277ab808a2" }
---

## MODIFIED Frontmatter
```yaml
exemptions: [{ rule: tests-cite-their-owner, files: ["crates/clm-check/tests/state_names.rs"], until: backlog/clm-check-still-re-exports-the-coverage-library, why: "the test of the re-export cites check.coverage until the re-export goes" }]
```

## MODIFIED Requirements

### Requirement: tests-cite-their-owner <!-- base: 1ddb042d9b589d87 -->
`clm check` SHALL report each `req:` marker, in a file of its scan that a component's `tests` match, or, once the root sets `wiring` (`model/wiring-globs`), in any file of its scan that a component owns, that cites a requirement of a component other than that file's owner (`paths-disjoint`), as `<file>:<line>: check/tests-cite-their-owner: cites <requirement> of `b`, but the file is `a`'s test; move the test to `b`'s tests`, and `run` SHALL report it too. A marker citing a capability (`capability/<slug>`) SHALL be read as citing the `capability` component's requirement, which proves it (`human-layer-proof`). While the root sets no `wiring`, a file that only `paths` match SHALL NOT be judged, so a project gives its tests to their components one `tests` glob at a time (`components-own-their-tests`) before it names its `wiring`; once it does, `clm land` refuses a build's edit to another component's file (`gate/own-files`), so a marker there could never be re-cited by the build of the component it cites. A file no component owns SHALL NOT be judged, and a marker that names no existing requirement is left to `dangling-markers`. A marker in a file that an exemption of its owner names for `tests-cite-their-owner` (`model/exemptions`) SHALL NOT be reported.

#### Scenario: another component's requirement
- GIVEN components `a` with tests `["tests/a/**"]` and `b` with requirement `y`, and `tests/a/t.rs` whose line 4 cites `b/y`
- WHEN `clm check` runs
- THEN it exits 1 with the line ``tests/a/t.rs:4: check/tests-cite-their-owner: cites b/y of `b`, but the file is `a`'s test; move the test to `b`'s tests``

#### Scenario: its own requirement
- GIVEN the same components, and `tests/a/t.rs` citing only `a`'s requirements
- WHEN `clm check` runs
- THEN it reports no `check/tests-cite-their-owner` violation

#### Scenario: a file only paths own
- GIVEN a root that sets no `wiring`, component `a` with paths `["tests/**"]` and no `tests`, and `tests/t.rs` citing `b/y`
- WHEN `clm check` runs
- THEN it reports no `check/tests-cite-their-owner` violation

#### Scenario: a file only paths own, once the root names its wiring
- GIVEN the same components and file, and a root with `wiring: []`
- WHEN `clm check` runs
- THEN it exits 1 with the line ``tests/t.rs:<line>: check/tests-cite-their-owner: cites b/y of `b`, but the file is `a`'s test; move the test to `b`'s tests``

#### Scenario: a capability test under wiring
- GIVEN a root with `wiring: []`, a component `capability` owning `tests/capability/see.rs`, which cites `capability/see`, and no other marker
- WHEN `clm check` runs
- THEN it reports no `check/tests-cite-their-owner` violation

#### Scenario: exempt until split
- GIVEN the file of `another component's requirement`, and `a` with an exemption from `tests-cite-their-owner` for `tests/a/t.rs` until `backlog/x`
- WHEN `clm check` runs
- THEN it reports no `check/tests-cite-their-owner` violation

#### Serves
- architecture/path-ownership
- capability/prove-by-tests

#### Needs
- model/test-ownership-keys
- model/exemptions
- model/wiring-globs
- land/approval-policy
- land/approval-from-base
- land/changes-requested-holds
- land/signed-land
- land.approval/request-refusal
