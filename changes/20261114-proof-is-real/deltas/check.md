---
component: check
---

## MODIFIED Requirements

### Requirement: dangling-markers <!-- base: 2e31c5b8395a0bb1 -->
`clm check` SHALL report every `req:<id>` marker in a scanned file whose `<id>` names no existing requirement or human-layer entry (`model/human-layers`), giving the file and line; and every marker that cites an architecture entry, or an entry holding `#### Untested`, with the message `architecture entries are proven by the rules their #### Enforced by names, not by tests` or `<id> is untested`, since a test never proves either (`human-layer-proof`); unless a change outside the archive lists `<id>` under `REMOVED Requirements` in one of its deltas, or `<id>` is a capability that a change outside the archive lists under `removes_capabilities` and `CAPABILITIES.md` doesn't hold (`check.coverage/removed-capabilities`): that code is obsolete and is removed when the component is next built (`removed-requirements-build-out`, `removed-capabilities-build-out`). The message for a marker whose `<id>` is `capability/<slug>` and names no entry, with no such change, SHALL end `; a change that removes the capability lists it under removes_capabilities`, so the author of a branch that removes a cited capability learns how to land it. Once the change is archived, the marker SHALL be reported as before. An `abandoned` change is archived (`archive-done`), so it grants no exemption; a requirement another change adds back under the same ID is a new requirement once that change applies, and the marker then cites it as usual. It SHALL judge the markers outside a test (`check.coverage/test-spans`, `markers_outside_tests`) as it judges those `scan` returns, each marker once, so a marker that proves nothing still can't name what doesn't exist.

#### Scenario: marker on an entry
- GIVEN a test citing `capability/approve`, an entry of `CAPABILITIES.md`, and another citing `capability/nope`, which isn't one
- WHEN `clm check` runs
- THEN it reports the second marker's file and line, and not the first

#### Scenario: marker on an architecture or untested entry
- GIVEN a test citing `architecture/git-is-truth`, an entry of `ARCHITECTURE.md`, and another citing `capability/watch`, a capability holding `#### Untested`
- WHEN `clm check` runs
- THEN it reports both markers' files and lines, the first saying architecture entries are proven by their `#### Enforced by` rules and the second that `capability/watch` is untested

#### Scenario: test cites a removed requirement
- GIVEN `tests/t.rs` line 3 contains the marker for `a/gone`, and `a` has no requirement `gone`
- WHEN `clm check` runs
- THEN it exits 1 reporting `tests/t.rs:3` and `a/gone`

#### Scenario: removed by a live change
- GIVEN `tests/t.rs` line 3 contains the marker for `a/gone`, `a` has no requirement `gone`, and an `applied` change `x` whose delta for `a` lists `gone` under `REMOVED Requirements`
- WHEN `clm check` runs
- THEN it reports no `check/dangling-markers` violation

#### Scenario: capability removed by a live change
- GIVEN `tests/t.rs` line 3 contains the marker for `capability/gone`, `CAPABILITIES.md` has no entry `gone`, and a `proposed` change `x` with `removes_capabilities: [gone]`
- WHEN `clm check` runs
- THEN it reports no `check/dangling-markers` violation; with `x` under `changes/archive/`, or with no `removes_capabilities`, it exits 1 reporting `tests/t.rs:3` and `capability/gone`, the line ending `; a change that removes the capability lists it under removes_capabilities`

#### Scenario: removing change archived
- GIVEN the same files, with change `x` under `changes/archive/`
- WHEN `clm check` runs
- THEN it exits 1 reporting `tests/t.rs:3` and `a/gone`

#### Scenario: a dangling marker outside a test
- GIVEN `src/lib.rs` holding `req:a/nope` in a function with no test attribute, and no requirement `a/nope`
- WHEN `clm check` runs
- THEN it reports the marker once as `check/dangling-markers`, naming the file and line

#### Needs
- check.coverage/test-spans
- model/human-layers
- check.coverage/removed-capabilities
