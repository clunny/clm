---
component: check
base: { frontmatter: "62bcd6aeba77e13a" }
---

## MODIFIED Frontmatter
```yaml
depends_on: [model, apply, vcs, init, check.coverage, gate]
```

## ADDED Requirements

### Requirement: obsolete-tests-elsewhere
Once the root sets `wiring` (`model/wiring-globs`), `clm check` SHALL report each test (`check.coverage/test-spans`, groups left out) in a file of its scan that holds a marker citing a requirement a change outside the archive removed (`check.coverage`'s `removed`), of a component the spec graph holds, when another component owns the file (`check.coverage/owner`), no `wiring` glob matches the file, and no approved list names the test, as ``<file>:<line>: check/obsolete-tests-elsewhere: test <name> cites <requirement>, which <change> removed, from `c`'s file; list it in that change's deleted-tests.txt, and prove what stays in `b`'s own files``, `<line>` the test's first line, `<change>` the first of the removing changes, `c` the file's owner and `b` the requirement's component; `run` SHALL report it too. An approved list here is a `changes/<id>/deleted-tests.txt` or `changes/archive/<id>/deleted-tests.txt` in the tree checked, and it names a test by a list line (`gate/cuts`, read with its `list_lines`) whose file is the test's and whose name agrees with it as `check.coverage`'s `bare_name` gives them. It SHALL NOT judge a marker outside a test, a marker citing a removed capability, a file no component owns, or a file the removed requirement's own component owns, and no exemption excuses it. While the root sets no `wiring` it SHALL report nothing.

#### Scenario: a test nobody listed
- GIVEN a root with `wiring: []`, components `b` and `c` with `paths` `["crates/b/**"]` and `["crates/c/**"]`, an `applied` change `20260101-x` that removed `b/gone`, and a test `old_rule` at line 8 of `crates/c/t.rs` citing `b/gone`
- WHEN `clm check` runs
- THEN it exits 1 with the line ``crates/c/t.rs:8: check/obsolete-tests-elsewhere: test old_rule cites b/gone, which 20260101-x removed, from `c`'s file; list it in that change's deleted-tests.txt, and prove what stays in `b`'s own files``

#### Scenario: a listed test
- GIVEN the same, with `changes/20260101-x/deleted-tests.txt` holding `DELETE crates/c/t.rs::old_rule`
- WHEN `clm check` runs
- THEN it reports no `check/obsolete-tests-elsewhere` violation

#### Scenario: its own component's file
- GIVEN the first scenario's repository with the test in `crates/b/t.rs` instead
- WHEN `clm check` runs
- THEN it reports no `check/obsolete-tests-elsewhere` violation

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
- gate/cuts
- check.coverage/pending-report
