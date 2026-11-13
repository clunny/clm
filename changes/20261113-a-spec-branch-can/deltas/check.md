---
component: check
---

## MODIFIED Requirements

### Requirement: dangling-markers <!-- base: de7f46e14a2b98c1 -->
`clm check` SHALL report every `req:<id>` marker in a scanned file whose `<id>` names no existing requirement or human-layer entry (`model/human-layers`), giving the file and line; and every marker that cites an architecture entry, or an entry holding `#### Untested`, with the message `architecture entries are proven by the rules their #### Enforced by names, not by tests` or `<id> is untested`, since a test never proves either (`human-layer-proof`); unless a change outside the archive lists `<id>` under `REMOVED Requirements` in one of its deltas, or `<id>` is a capability that a change outside the archive lists under `removes_capabilities` and `CAPABILITIES.md` doesn't hold (`check.coverage/removed-capabilities`): that code is obsolete and is removed when the component is next built (`removed-requirements-build-out`, `removed-capabilities-build-out`). The message for a marker whose `<id>` is `capability/<slug>` and names no entry, with no such change, SHALL end `; a change that removes the capability lists it under removes_capabilities`, so the author of a branch that removes a cited capability learns how to land it. Once the change is archived, the marker SHALL be reported as before. An `abandoned` change is archived (`archive-done`), so it grants no exemption; a requirement another change adds back under the same ID is a new requirement once that change applies, and the marker then cites it as usual.

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

#### Needs
- model/human-layers
- check.coverage/removed-capabilities

### Requirement: removed-requirement-names <!-- base: fefcc3a4ee5798eb -->
`clm check` SHALL report each inline code span outside fenced code blocks in a component's `SPEC.md`, below its frontmatter, whose whole text is a removed requirement ID `<component>/<slug>`, in any component's `SPEC.md`, or the bare `<slug>` of one, in the `SPEC.md` of `<component>` itself unless a requirement of any component has that slug, `docs/decisions/<slug>.md` exists or a human-layer entry has that slug; as `<spec file>:<line>: check/removed-requirement-names: names `<id>`, which <change> removes`. An ID is *removed* when `check.coverage`'s `removed` gives it: a change outside the archive lists the slug under `REMOVED Requirements` in its delta for the component and no component holds the ID, or lists a capability's slug under `removes_capabilities` and `CAPABILITIES.md` doesn't hold it, the ID being `capability/<slug>` and its component, for the bare slug, the graph's component `capability` (`check.coverage/removed-capabilities`); with several such changes, the message SHALL name the first by ID. It SHALL also report the same spans in the graph each `proposed` or `approved` change would leave (`apply/resulting-graph-call`), for the IDs that change's deltas list under `REMOVED Requirements` and the current graph holds, with shared slugs judged on that graph, as `<spec file>:<line>: check/removed-requirement-names: names `<id>`, which <change> would remove`, the line being that of the text the change leaves; a change whose deltas don't apply is skipped.

#### Scenario: requirement names a removed sibling
- GIVEN component `a` holding requirement `keep`, whose text names `` `gone` `` on line 30 of `specs/a/SPEC.md`, and an `applied` change `20260101-x` whose delta for `a` lists `gone` under `REMOVED Requirements`
- WHEN `clm check` runs
- THEN it exits 1 with the line ``specs/a/SPEC.md:30: check/removed-requirement-names: names `a/gone`, which 20260101-x removes``

#### Scenario: another node names the full ID
- GIVEN the same change, and component `b` whose Design names `` `a/gone` ``
- WHEN `clm check` runs
- THEN it reports that line of `specs/b/SPEC.md` with `check/removed-requirement-names`

#### Scenario: shared slug
- GIVEN the same change, component `a`'s text naming `` `gone` ``, and a file `docs/decisions/gone.md`
- WHEN `clm check` runs
- THEN it reports no `check/removed-requirement-names` violation for the bare `` `gone` ``

#### Scenario: slug of a human-layer entry
- GIVEN the same change, component `a`'s text naming `` `gone` ``, and an `ARCHITECTURE.md` whose `## Constraints` holds `### Constraint: gone`
- WHEN `clm check` runs
- THEN it reports no `check/removed-requirement-names` violation for the bare `` `gone` ``

#### Scenario: removed capability named
- GIVEN a `CAPABILITIES.md` with no entry `gone`, a `proposed` change `20260101-x` with `removes_capabilities: [gone]`, and component `b` whose Design names `` `capability/gone` `` on line 12 of `specs/b/SPEC.md`
- WHEN `clm check` runs
- THEN it exits 1 with the line ``specs/b/SPEC.md:12: check/removed-requirement-names: names `capability/gone`, which 20260101-x removes``

#### Scenario: not applied yet
- GIVEN component `a` holding requirement `keep`, a `proposed` change `20260101-y` whose delta for `a` lists `keep` under `REMOVED Requirements` with its base and which has no delta for `b`, and component `b` whose Design names `` `a/keep` `` on line 12 of `specs/b/SPEC.md`
- WHEN `clm check` runs
- THEN it exits 1 with the line ``specs/b/SPEC.md:12: check/removed-requirement-names: names `a/keep`, which 20260101-y would remove``

#### Scenario: not applied yet, name rewritten
- GIVEN component `a` holding requirements `keep` and `other`, whose text names `` `keep` ``, and a `proposed` change whose delta for `a` removes `keep` and modifies `other` to name it no more, both with their bases
- WHEN `clm check` runs
- THEN it reports no `check/removed-requirement-names` violation

#### Scenario: not applied yet, stale delta
- GIVEN the change of `not applied yet`, its delta for `a` holding a stale base for `keep`
- WHEN `clm check` runs
- THEN it reports `apply/stale-base` for the delta and no `check/removed-requirement-names` violation

#### Scenario: change archived
- GIVEN the first scenario's files with change `20260101-x` under `changes/archive/`
- WHEN `clm check` runs
- THEN it reports no `check/removed-requirement-names` violation

#### Scenario: fenced example
- GIVEN the first scenario's change, and `` `gone` `` named only inside a fenced code block in `specs/a/SPEC.md`
- WHEN `clm check` runs
- THEN it reports no `check/removed-requirement-names` violation

#### Serves
- capability/catch-drift
- architecture/spec-is-current

#### Needs
- apply/resulting-graph-call
- model/human-layers
- check.coverage/removed-capabilities
