---
component: check
base: {}
---

## MODIFIED Requirements

### Requirement: obsolete-tests-elsewhere <!-- base: c840a4834c62682d -->
Once the root sets `wiring` (`model/wiring-globs`), `clm check` SHALL report each test (`check.coverage/test-spans`, groups left out) in a file of its scan that a component owns (`check.coverage/owner`) and no `wiring` glob matches, whose every marker cites a requirement that a change outside the archive removed, or a removed capability (`check.coverage`'s `removed`), whose ID names a component other than the file's owner, whether or not the spec graph still holds that component, a capability's ID naming the component `model/capability-tests-component` gives, when no approved list names the test, as ``<file>:<line>: check/obsolete-tests-elsewhere: test <name> cites <requirement>, which <change> removed, from `c`'s file; list it in that change's deleted-tests.txt, and prove what stays in the files of the component that states it``, `<line>` the test's first line as `test-spans` gives it, `<requirement>` the first such ID, `<change>` the first of its removing changes and `c` the file's owner; `run` SHALL report it too. An approved list here is a `changes/<id>/deleted-tests.txt` or `changes/archive/<id>/deleted-tests.txt` among the files `clm check` reads in the tree it checks, and it names a test by a list line (`gate/cuts`, read with its `list_lines`) whose file is the test's and whose name agrees with it as `check.coverage`'s `bare_name` gives them. It SHALL NOT report a test one of whose removed requirements `check.coverage`'s `moved_to` takes to a requirement of the file's owner, which that owner's build re-cites; a test that also holds any other marker, citing a requirement the spec holds, a requirement of the owner's own that was removed, a capability `CAPABILITIES.md` holds, or a removed capability when the owner builds the capability tests, whose markers for the other components' removed requirements and capabilities alone go; a marker outside a test; or a file no component owns. No exemption excuses it, and while the root sets no `wiring` it SHALL report nothing.

#### Scenario: a test nobody listed
- GIVEN a root with `wiring: []`, components `b` and `c` with `paths` `["crates/b/**"]` and `["crates/c/**"]`, an `applied` change `20260101-x` that removed `b/gone`, and a test `old_rule` whose first line is line 8 of `crates/c/t.rs`, citing `b/gone` alone
- WHEN `clm check` runs
- THEN it exits 1 with the line ``crates/c/t.rs:8: check/obsolete-tests-elsewhere: test old_rule cites b/gone, which 20260101-x removed, from `c`'s file; list it in that change's deleted-tests.txt, and prove what stays in the files of the component that states it``

#### Scenario: a listed test
- GIVEN the same, with `changes/20260101-x/deleted-tests.txt` holding `DELETE crates/c/t.rs::old_rule`
- WHEN `clm check` runs
- THEN it reports no `check/obsolete-tests-elsewhere` violation

#### Scenario: a removed component's test
- GIVEN a root with `wiring: []`, no component `g`, an `applied` change `20260101-x` that removed `g/x`, and a test in `crates/c/t.rs` citing `g/x` alone
- WHEN `clm check` runs
- THEN it reports that test under `check/obsolete-tests-elsewhere`

#### Scenario: moved into the file's owner
- GIVEN the same, the change adding `c/x`, the move of `g/x`
- WHEN `clm check` runs
- THEN it reports no `check/obsolete-tests-elsewhere` violation

#### Scenario: its own component's file, and a test that cites a live requirement too
- GIVEN the first scenario's repository with a test citing `b/gone` in `crates/b/t.rs`, and the test in `crates/c/t.rs` also citing `c`'s requirement `c/y`
- WHEN `clm check` runs
- THEN it reports no `check/obsolete-tests-elsewhere` violation

#### Scenario: a removed capability's test
- GIVEN a root with `wiring: []`, components `capability` and `c` with `paths` `["crates/clm/tests/capability/**"]` and `["crates/c/**"]`, a `CAPABILITIES.md` with no entry `gone`, an `applied` change `20260101-x` with `removes_capabilities: [gone]`, a test `old_cap` whose first line is line 8 of `crates/c/t.rs`, citing `capability/gone` alone, and a test citing `capability/gone` in `crates/clm/tests/capability/gone.rs`
- WHEN `clm check` runs
- THEN it exits 1 with the line ``crates/c/t.rs:8: check/obsolete-tests-elsewhere: test old_cap cites capability/gone, which 20260101-x removed, from `c`'s file; list it in that change's deleted-tests.txt, and prove what stays in the files of the component that states it``, and reports nothing for `crates/clm/tests/capability/gone.rs`

#### Scenario: a wiring file and an unowned file
- GIVEN the first scenario's repository with `wiring: ["crates/c/shared.rs"]`, the test in `crates/c/shared.rs`, and another test citing `b/gone` in `old/t.rs`, which no component owns
- WHEN `clm check` runs
- THEN it reports no `check/obsolete-tests-elsewhere` violation

#### Scenario: not switched on
- GIVEN the first scenario's repository with a root that sets no `wiring`
- WHEN `clm check` runs
- THEN it reports no `check/obsolete-tests-elsewhere` violation

#### Serves
- architecture/removal-needs-a-yes
- architecture/path-ownership

#### Needs
- model/capability-tests-component
- gate/cuts
- check.coverage/move-targets
